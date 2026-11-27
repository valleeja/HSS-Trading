//! 📖 The Argos Script editor's Doc page (mockup validated by Jee on 22 Nov 2026): it takes the place of the code.
//! Two tabs, « Argos Script » and « Rust », whose text is the repository's own docs (`docs/indicateurs/*.md`: one
//! source, the repo file is the doc). PC: the parts (every `## ` heading, numbered) in a left column, the current one
//! highlighted and following the scroll. Cell: a « ☰ Parties » pill opens the same list as a sheet. The reading area
//! uses the Academy's Markdown reader (`argos_academy::markdown`): headings, paragraphs, lists, inline code, tables,
//! links shown as text, code blocks in monospace on a dark block with « ⧉ Copier » (and a sideways scroll with a fade
//! when a line is longer than the block: code never wraps). « Chercher dans la doc » highlights the matches (Entrée /
//! ↓ next, ↑ previous, « 3 / 12 », Échap clears); « ✨ Copier pour l'IA » copies the open tab's file for an AI
//! (`*-llms.txt`). Texts shown are in French.

use crate::academy::{Bloc, Morceau};
use crate::palette as pal;
use crate::palette::{DIM, GREY, ICON, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Mono, Rgb, Text};
use std::cell::{Cell, RefCell};
use std::sync::{Arc, OnceLock};
use tiny_skia::Pixmap;

/// The Argos Script reference (repo copy).
pub const SCRIPT_MD: &str = include_str!("../../../../docs/indicateurs/argos-script-reference.md");
/// « Créer un indicateur en Rust » (repo copy).
pub const RUST_MD: &str = include_str!("../../../../docs/indicateurs/creer-indicateur-rust.md");
/// The files for an AI, one per tab.
pub const SCRIPT_LLMS: &str = include_str!("../../../../docs/indicateurs/argos-script-llms.txt");
pub const RUST_LLMS: &str = include_str!("../../../../docs/indicateurs/creer-indicateur-rust-llms.txt");

/// Below this window width, the cell layout (its own header, « ☰ Parties », the wide pill at the bottom).
pub const ETROIT: f64 = 600.0;

/// Inline code colour of the mockup (`code` in a sentence).
const CODE_INLINE: Rgb = Rgb(0xff, 0xd5, 0x8a);
/// Code blocks: monospace size, line height, inner margins.
const CS: f32 = 12.5;
const CLH: f64 = 19.0;
const CPAD: f64 = 14.0;
const CVPAD: f64 = 12.0;
/// Space between the top of the reading area and the first line.
const HAUT_TEXTE: f64 = 6.0;
/// « Copié » stays this long on the pill.
const COPIE_MS: i64 = 1500;
/// A notice of the page (« Copié : fichier pour l'IA (…) ») stays this long: the chart's banner is under the editor.
const AVIS_MS: i64 = 2500;

/// A tab of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Onglet {
    #[default]
    Script,
    Rust,
}

impl Onglet {
    pub fn nom(self) -> &'static str {
        match self {
            Onglet::Script => "Argos Script",
            Onglet::Rust => "Rust",
        }
    }

    pub fn md(self) -> &'static str {
        match self {
            Onglet::Script => SCRIPT_MD,
            Onglet::Rust => RUST_MD,
        }
    }

    /// The tab's file for an AI (« ✨ Copier pour l'IA »).
    pub fn llms(self) -> &'static str {
        match self {
            Onglet::Script => SCRIPT_LLMS,
            Onglet::Rust => RUST_LLMS,
        }
    }
}

/// The parts of a tab (title, blocks), read once.
pub fn parties(o: Onglet) -> &'static [(String, Vec<Bloc>)] {
    static S: OnceLock<Vec<(String, Vec<Bloc>)>> = OnceLock::new();
    static R: OnceLock<Vec<(String, Vec<Bloc>)>> = OnceLock::new();
    let cell = if o == Onglet::Script { &S } else { &R };
    cell.get_or_init(|| argos_academy::markdown::parties(o.md()))
}

/// What a click asks of the editor (the rest is handled inside the page).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocAct {
    /// « ‹ Code »: back to the script, as it was.
    Code,
    /// ✕ of the cell header: close the editor.
    Fermer,
    /// « ⧉ Copier » of code block `k` of the open tab.
    CopierBloc(usize),
    /// « ✨ Copier pour l'IA ».
    CopierIa,
}

/// Places of the page (coordinates of the editor window).
#[derive(Debug, Clone, Copy)]
struct Cadre {
    etroit: bool,
    code: Rect,
    /// Cell: ✕ of the header.
    fermer: Option<Rect>,
    seg: Rect,
    onglets: [Rect; 2],
    champ: Rect,
    /// Cell: « ☰ Parties ».
    parties: Option<Rect>,
    ia: Rect,
    /// PC: the column of the parts.
    toc: Option<Rect>,
    lecture: Rect,
    /// Margins of the text inside the reading area (left, right).
    marge: (f64, f64),
}

/// Kind of letters of a piece of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Police {
    Normal,
    Gras,
    Mono,
}

#[derive(Debug, Clone)]
enum Op {
    /// Text at (x, baseline y).
    T { x: f64, y: f64, s: String, size: f32, c: Rgb, p: Police },
    /// Rounded fill (rect, colour, alpha, radius).
    F(Rect, Rgb, f32, f64),
    /// Rounded border.
    S(Rect, Rgb, f64),
}

#[derive(Debug, Clone)]
struct BlocCode {
    r: Rect,
    lignes: Vec<String>,
    /// The longest line, in points (inner margins included).
    large: f64,
}

/// A tab laid out for one width (content coordinates: x from the text margin, y from the top of the text).
#[derive(Debug, Clone, Default)]
struct Mise {
    ops: Vec<Op>,
    /// Top of each part.
    parties: Vec<f64>,
    blocs: Vec<BlocCode>,
    haut: f64,
}

/// A match of the search (content coordinates; in a code block: x before its sideways shift).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Trouve {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    bloc: Option<usize>,
}

/// The page's state (kept while the editor is open: « ‹ Code » then 📖 comes back to the same place).
#[derive(Debug, Clone, Default)]
pub struct Doc {
    pub onglet: Onglet,
    scroll: f64,
    pub recherche: String,
    /// The search field has the keyboard.
    pub saisie: bool,
    /// Current match.
    courant: usize,
    /// Cell: « ☰ Parties » sheet open.
    pub feuille: bool,
    /// Part chosen in the list (it stays highlighted until the next scroll).
    vise: Option<usize>,
    /// Sideways shift of the code blocks (block, points).
    decale: Vec<(usize, f64)>,
    /// « Copié » on this block since this time (ms).
    copie: Option<(usize, i64)>,
    /// A notice at the bottom of the page since this time (ms).
    avis: Option<(String, i64)>,
    /// Last place drawn (width, height, top) for the wheel.
    geom: Cell<(f64, f64, f64)>,
    cache: RefCell<Option<((Onglet, i64, bool), Arc<Mise>)>>,
}

/// `[texte](adresse)` → `texte` (links shown as text).
fn sans_liens(s: &str) -> String {
    let mut out = String::new();
    let mut reste = s;
    while let Some(d) = reste.find('[') {
        let apres = &reste[d + 1..];
        let lien = apres
            .find(']')
            .filter(|f| apres[f + 1..].starts_with('('))
            .and_then(|f| apres[f + 2..].find(')').map(|p| (f, f + 2 + p)));
        match lien {
            Some((f, p)) => {
                out.push_str(&reste[..d]);
                out.push_str(&apres[..f]);
                reste = &apres[p + 1..];
            }
            None => {
                out.push_str(&reste[..=d]);
                reste = apres;
            }
        }
    }
    out.push_str(reste);
    out
}

/// Pieces of a line: (text, kind) — `code` in monospace, `**gras**` in bold, links as text.
fn pieces(m: &[Morceau]) -> Vec<(String, Police)> {
    let mut out = vec![];
    for x in m {
        for (k, s) in x.texte.split('`').enumerate() {
            if s.is_empty() {
                continue;
            }
            if k % 2 == 1 {
                out.push((s.to_string(), Police::Mono));
            } else {
                out.push((sans_liens(s), if x.gras { Police::Gras } else { Police::Normal }));
            }
        }
    }
    out
}

fn largeur(text: &Text, gras: &Text, s: &str, size: f32, p: Police) -> f64 {
    match p {
        Police::Normal => text.width(s, size) as f64,
        Police::Gras => gras.width(s, size) as f64,
        Police::Mono => Mono::advance(size - 1.0) as f64 * s.chars().count() as f64,
    }
}

/// Lays out a line of rich text (`prefixe` then `m`) on `w` from `x0`; inline code in a small chip.
#[allow(clippy::too_many_arguments)]
fn riche(ops: &mut Vec<Op>, text: &Text, gras: &Text, y: &mut f64, x0: f64, w: f64, prefixe: &str, m: &[Morceau], size: f32, col: Rgb) {
    // words: (text, kind, space before)
    let mut mots: Vec<(String, Police, bool)> = vec![];
    let mut tout = vec![(prefixe.to_string(), Police::Normal)];
    tout.extend(pieces(m));
    let mut espace = false;
    for (s, p) in tout {
        let mut mot = String::new();
        for c in s.chars() {
            if c == ' ' {
                if !mot.is_empty() {
                    mots.push((std::mem::take(&mut mot), p, espace));
                }
                espace = true;
            } else {
                mot.push(c);
            }
        }
        if !mot.is_empty() {
            mots.push((mot, p, espace));
            espace = false;
        }
    }
    // lines of (x, text, kind)
    let mut lignes: Vec<Vec<(f64, String, Police)>> = vec![vec![]];
    let mut x = x0;
    let mut avant: Option<Police> = None;
    for (mot, p, esp) in mots {
        let sp = if esp && x > x0 {
            largeur(
                text,
                gras,
                " ",
                size,
                if p == Police::Mono && avant == Some(Police::Mono) {
                    Police::Mono
                } else {
                    Police::Normal
                },
            )
        } else {
            0.0
        };
        // chip margins: 5 points in and out of inline code
        let pad = match (avant, p) {
            (Some(Police::Mono), Police::Mono) => 0.0,
            (_, Police::Mono) | (Some(Police::Mono), _) => 5.0,
            _ => 0.0,
        };
        let mw = largeur(text, gras, &mot, size, p);
        if x + sp + pad + mw > x0 + w && x > x0 {
            lignes.push(vec![]);
            x = x0 + if p == Police::Mono { 4.0 } else { 0.0 };
            avant = None;
        } else {
            x += sp + pad;
        }
        if let Some(l) = lignes.last_mut() {
            match l.last_mut() {
                Some((_, s, q)) if *q == p && avant == Some(p) => {
                    if esp {
                        s.push(' ');
                    }
                    s.push_str(&mot);
                }
                _ => l.push((x, mot, p)),
            }
        }
        x += mw;
        avant = Some(p);
    }
    for l in lignes {
        let base = *y + size as f64 * 1.15;
        for (x, s, p) in l {
            if p == Police::Mono {
                let w = largeur(text, gras, &s, size, p);
                ops.push(Op::F(
                    Rect::new(x - 4.0, base - size as f64 * 0.95, w + 8.0, size as f64 * 1.3),
                    WHITE,
                    0.08,
                    5.0,
                ));
                ops.push(Op::T {
                    x,
                    y: base,
                    s,
                    size: size - 1.0,
                    c: CODE_INLINE,
                    p,
                });
            } else {
                ops.push(Op::T {
                    x,
                    y: base,
                    s,
                    size,
                    c: col,
                    p,
                });
            }
        }
        *y += size as f64 * 1.6;
    }
}

/// A table: columns on a wide reading area, one card per row on a narrow one.
fn tableau(ops: &mut Vec<Op>, text: &Text, y: &mut f64, w: f64, rows: &[Vec<String>]) {
    const S: f32 = 12.5;
    const LH: f64 = 18.0;
    let Some((tete, corps)) = rows.split_first() else { return };
    let propre = |s: &str| sans_liens(&s.replace('`', ""));
    let wrap = |s: &str, w: f64| crate::assistant::wrap(text, &propre(s), S, w.max(30.0));
    let n = tete.len().max(1);
    if w >= 460.0 {
        let len = |c: usize| rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count().clamp(6, 40)).max().unwrap_or(6) as f64;
        let total: f64 = (0..n).map(len).sum();
        let ws: Vec<f64> = (0..n).map(|c| w * len(c) / total).collect();
        let top = *y;
        for (k, r) in rows.iter().enumerate() {
            let cells: Vec<Vec<String>> = ws
                .iter()
                .enumerate()
                .map(|(c, cw)| wrap(r.get(c).map_or("", |s| s.as_str()), cw - 16.0))
                .collect();
            let h = cells.iter().map(Vec::len).max().unwrap_or(1).max(1) as f64 * LH + 10.0;
            if k == 0 {
                ops.push(Op::F(Rect::new(0.0, *y, w, h), pal::field(), 1.0, 6.0));
            } else if k % 2 == 0 {
                ops.push(Op::F(Rect::new(0.0, *y, w, h), WHITE, 0.03, 0.0));
            }
            let mut x = 0.0;
            for (c, ls) in cells.into_iter().enumerate() {
                for (i, l) in ls.into_iter().enumerate() {
                    let p = if k == 0 { Police::Gras } else { Police::Normal };
                    ops.push(Op::T {
                        x: x + 8.0,
                        y: *y + 18.0 + i as f64 * LH,
                        s: l,
                        size: S,
                        c: if k == 0 { GREY } else { WHITE },
                        p,
                    });
                }
                x += ws[c];
            }
            *y += h;
        }
        ops.push(Op::S(Rect::new(0.0, top, w, *y - top), pal::edge_soft(), 6.0));
    } else {
        for r in corps {
            let mut ls: Vec<(String, Rgb)> = wrap(r.first().map_or("", |s| s.as_str()), w - 24.0).into_iter().map(|l| (l, WHITE)).collect();
            for (h, v) in tete.iter().zip(r).skip(1).filter(|(_, v)| !v.is_empty()) {
                ls.extend(wrap(&format!("{h} : {v}"), w - 24.0).into_iter().map(|l| (l, GREY)));
            }
            let rect = Rect::new(0.0, *y, w, ls.len() as f64 * LH + 14.0);
            ops.push(Op::F(rect, pal::card(), 1.0, 8.0));
            ops.push(Op::S(rect, pal::edge_soft(), 8.0));
            for (i, (l, c)) in ls.into_iter().enumerate() {
                ops.push(Op::T {
                    x: 12.0,
                    y: *y + 20.0 + i as f64 * LH,
                    s: l,
                    size: S,
                    c,
                    p: Police::Normal,
                });
            }
            *y += rect.h + 6.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn bloc(m: &mut Mise, text: &Text, gras: &Text, y: &mut f64, b: &Bloc, w: f64, size: f32) {
    match b {
        Bloc::Para(ms) => {
            riche(&mut m.ops, text, gras, y, 0.0, w, "", ms, size, WHITE);
            *y += 6.0;
        }
        Bloc::Titre(_, t) => {
            *y += 8.0;
            for l in crate::assistant::wrap(gras, t, 15.5, w) {
                *y += 15.5 * 1.4;
                m.ops.push(Op::T {
                    x: 0.0,
                    y: *y - 5.0,
                    s: l,
                    size: 15.5,
                    c: WHITE,
                    p: Police::Gras,
                });
            }
            *y += 6.0;
        }
        Bloc::Liste { numerotee, elements } => {
            for (k, e) in elements.iter().enumerate() {
                let pre = if *numerotee { format!("{}. ", k + 1) } else { "• ".into() };
                riche(&mut m.ops, text, gras, y, 6.0, w - 6.0, &pre, e, size, WHITE);
                *y += 2.0;
            }
            *y += 6.0;
        }
        Bloc::Tableau(rows) => {
            *y += 4.0;
            tableau(&mut m.ops, text, y, w, rows);
            *y += 12.0;
        }
        Bloc::Details { resume, blocs } => {
            let r = vec![Morceau {
                texte: resume.clone(),
                gras: true,
            }];
            riche(&mut m.ops, text, gras, y, 0.0, w, "▸ ", &r, size, WHITE);
            for b in blocs {
                bloc(m, text, gras, y, b, w, size);
            }
        }
        Bloc::Code(lignes) => {
            let adv = Mono::advance(CS) as f64;
            let r = Rect::new(0.0, *y + 2.0, w, 2.0 * CVPAD + lignes.len().max(1) as f64 * CLH);
            let large = lignes.iter().map(|l| l.chars().count()).max().unwrap_or(0) as f64 * adv + 2.0 * CPAD;
            m.blocs.push(BlocCode {
                r,
                lignes: lignes.clone(),
                large,
            });
            *y = r.y + r.h + 12.0;
        }
    }
}

fn construire(o: Onglet, text: &Text, w: f64, etroit: bool) -> Mise {
    let gras = Text::graisse(Graisse::Bold);
    let size = if etroit { 14.0 } else { 13.5 };
    let mut m = Mise::default();
    let mut y = 0.0;
    for (k, (titre, blocs)) in parties(o).iter().enumerate() {
        if k > 0 {
            y += 22.0;
        }
        m.parties.push(y);
        for l in crate::assistant::wrap(&gras, &format!("{}. {titre}", k + 1), 19.0, w) {
            y += 19.0 * 1.35;
            m.ops.push(Op::T {
                x: 0.0,
                y: y - 4.0,
                s: l,
                size: 19.0,
                c: WHITE,
                p: Police::Gras,
            });
        }
        y += 10.0;
        for b in blocs {
            bloc(&mut m, text, &gras, &mut y, b, w, size);
        }
    }
    m.haut = y + 24.0;
    m
}

/// The pill text of a code block (the « ⧉ » of the mockup is not in the app's fonts: the word alone).
fn copier_libelle(_text: &Text) -> &'static str {
    "Copier"
}

/// « ⧉ Copier » of a block whose box is `r` (same coordinates).
fn copier_rect(text: &Text, r: Rect) -> Rect {
    let pw = text.width(copier_libelle(text), 11.5) as f64 + 20.0;
    Rect::new(r.x + r.w - 8.0 - pw, r.y + 8.0, pw, 24.0)
}

fn bas_de_casse(s: &str) -> Vec<char> {
    s.chars().map(|c| c.to_lowercase().next().unwrap_or(c)).collect()
}

/// Positions (in characters) where `q` (already in lower case) starts in `s`.
fn positions(s: &str, q: &[char]) -> Vec<usize> {
    let cs = bas_de_casse(s);
    if q.is_empty() || cs.len() < q.len() {
        return vec![];
    }
    (0..=cs.len() - q.len()).filter(|&i| &cs[i..i + q.len()] == q).collect()
}

impl Doc {
    fn cadre(&self, text: &Text, w: f64, h: f64, top: f64) -> Cadre {
        let pill = |s: &str, size: f32| text.width(s, size) as f64 + 24.0;
        if w < ETROIT {
            let code = Rect::new(14.0, top + 11.0, pill("‹ Code", 13.0), 30.0);
            let fermer = Rect::new(w - 14.0 - 30.0, top + 11.0, 30.0, 30.0);
            let seg = Rect::new(14.0, top + 60.0, w - 28.0, 34.0);
            let pw = pill("☰ Parties", 13.0);
            let champ = Rect::new(14.0, top + 104.0, w - 28.0 - pw - 8.0, 32.0);
            let parties = Rect::new(champ.x + champ.w + 8.0, champ.y, pw, 32.0);
            let ia = Rect::new(14.0, h - 18.0 - 44.0, w - 28.0, 44.0);
            let ly = champ.y + champ.h + 8.0;
            Cadre {
                etroit: true,
                code,
                fermer: Some(fermer),
                seg,
                onglets: Self::moities(seg),
                champ,
                parties: Some(parties),
                ia,
                toc: None,
                lecture: Rect::new(0.0, ly, w, (ia.y - 10.0 - ly).max(40.0)),
                marge: (16.0, 16.0),
            }
        } else {
            let y = top + 10.0;
            let code = Rect::new(14.0, y, pill("‹ Code", 12.5), 30.0);
            let seg = Rect::new(code.x + code.w + 10.0, y, 260.0, 30.0);
            let iw = pill("✨ Copier pour l'IA", 12.5);
            let ia = Rect::new(w - 14.0 - iw, y, iw, 30.0);
            let champ = Rect::new(seg.x + seg.w + 10.0, y, (ia.x - 10.0 - seg.x - seg.w - 10.0).max(80.0), 30.0);
            let by = top + 50.0;
            let bh = (h - 10.0 - by).max(40.0);
            let tw = if w >= 900.0 { 240.0 } else { 200.0 };
            let toc = Rect::new(10.0, by, tw, bh);
            let lx = toc.x + toc.w + 6.0;
            Cadre {
                etroit: false,
                code,
                fermer: None,
                seg,
                onglets: Self::moities(seg),
                champ,
                parties: None,
                ia,
                toc: Some(toc),
                lecture: Rect::new(lx, by, (w - 10.0 - lx).max(80.0), bh),
                marge: (18.0, 22.0),
            }
        }
    }

    fn moities(seg: Rect) -> [Rect; 2] {
        let half = (seg.w - 10.0) / 2.0;
        [
            Rect::new(seg.x + 3.0, seg.y + 3.0, half, seg.h - 6.0),
            Rect::new(seg.x + 7.0 + half, seg.y + 3.0, half, seg.h - 6.0),
        ]
    }

    fn largeur_texte(c: &Cadre) -> f64 {
        (c.lecture.w - c.marge.0 - c.marge.1).max(60.0)
    }

    fn mise(&self, text: &Text, c: &Cadre) -> Arc<Mise> {
        let w = Self::largeur_texte(c);
        let cle = (self.onglet, w.round() as i64, c.etroit);
        if let Some((k, m)) = self.cache.borrow().as_ref() {
            if *k == cle {
                return m.clone();
            }
        }
        let m = Arc::new(construire(self.onglet, text, w, c.etroit));
        *self.cache.borrow_mut() = Some((cle, m.clone()));
        m
    }

    fn max_scroll(m: &Mise, c: &Cadre) -> f64 {
        (m.haut - c.lecture.h + 20.0).max(0.0)
    }

    fn decalage(&self, k: usize) -> f64 {
        self.decale.iter().find(|(b, _)| *b == k).map_or(0.0, |(_, d)| *d)
    }

    fn poser_decalage(&mut self, k: usize, d: f64) {
        self.decale.retain(|(b, _)| *b != k);
        if d > 0.0 {
            self.decale.push((k, d));
        }
    }

    /// Rows of the list of parts (in the column, or in the sheet).
    fn rangees(&self, c: &Cadre) -> Vec<Rect> {
        let n = parties(self.onglet).len().max(1);
        match c.toc {
            Some(t) => {
                let rh = (t.h / n as f64).min(28.5);
                (0..n).map(|k| Rect::new(t.x + 4.0, t.y + 4.0 + k as f64 * rh, t.w - 8.0, rh - 1.0)).collect()
            }
            None => {
                let f = self.feuille_rect(c);
                let rh = ((f.h - 16.0) / n as f64).min(40.0);
                (0..n).map(|k| Rect::new(f.x + 8.0, f.y + 8.0 + k as f64 * rh, f.w - 16.0, rh - 1.0)).collect()
            }
        }
    }

    /// The « ☰ Parties » sheet (cell), under the search row.
    fn feuille_rect(&self, c: &Cadre) -> Rect {
        let n = parties(self.onglet).len() as f64;
        let h = (n * 40.0 + 16.0).min(c.ia.y + c.ia.h - c.lecture.y);
        Rect::new(10.0, c.lecture.y, c.lecture.w - 20.0, h)
    }

    /// The part on screen: the one chosen in the list, else the last one whose title is above the top + 24.
    fn partie_courante(&self, m: &Mise) -> usize {
        self.vise
            .unwrap_or_else(|| m.parties.iter().rposition(|y| *y <= self.scroll + 24.0).unwrap_or(0))
    }

    fn trouvailles(&self, m: &Mise) -> Vec<Trouve> {
        let q = bas_de_casse(self.recherche.trim());
        if q.is_empty() {
            return vec![];
        }
        let text = Text::default();
        let gras = Text::graisse(Graisse::Bold);
        let mut out = vec![];
        let n = q.len();
        for op in &m.ops {
            let Op::T { x, y, s, size, p, .. } = op else { continue };
            for i in positions(s, &q) {
                let avant: String = s.chars().take(i).collect();
                let mot: String = s.chars().skip(i).take(n).collect();
                let (dx, w) = match p {
                    Police::Normal => (text.width(&avant, *size) as f64, text.width(&mot, *size) as f64),
                    Police::Gras => (gras.width(&avant, *size) as f64, gras.width(&mot, *size) as f64),
                    Police::Mono => {
                        let a = Mono::advance(*size) as f64;
                        (i as f64 * a, n as f64 * a)
                    }
                };
                out.push(Trouve {
                    x: x + dx,
                    y: y - *size as f64 * 0.95,
                    w,
                    h: *size as f64 * 1.3,
                    bloc: None,
                });
            }
        }
        let adv = Mono::advance(CS) as f64;
        for (k, b) in m.blocs.iter().enumerate() {
            for (j, l) in b.lignes.iter().enumerate() {
                for i in positions(l, &q) {
                    out.push(Trouve {
                        x: CPAD + i as f64 * adv,
                        y: b.r.y + CVPAD + j as f64 * CLH,
                        w: n as f64 * adv,
                        h: CLH,
                        bloc: Some(k),
                    });
                }
            }
        }
        out.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
        out
    }

    /// Scrolls to match `i` (and shifts its code block sideways when it is past the right edge).
    fn montrer(&mut self, i: usize) {
        let text = Text::default();
        let (w, h, top) = self.geom.get();
        let c = self.cadre(&text, w, h, top);
        let m = self.mise(&text, &c);
        let t = self.trouvailles(&m);
        let Some(f) = t.get(i).copied() else { return };
        self.courant = i;
        self.vise = None;
        self.scroll = (f.y - c.lecture.h / 3.0).clamp(0.0, Self::max_scroll(&m, &c));
        if let Some(k) = f.bloc {
            let vis = Self::largeur_texte(&c) - 2.0 * CPAD;
            let d = self.decalage(k);
            if f.x + f.w - CPAD > d + vis || f.x - CPAD < d {
                self.poser_decalage(k, (f.x - CPAD - vis / 2.0).max(0.0));
            }
        }
    }

    /// Number of matches of the search in the open tab.
    pub fn nombre_trouves(&self, text: &Text, w: f64, h: f64, top: f64) -> usize {
        let c = self.cadre(text, w, h, top);
        self.trouvailles(&self.mise(text, &c)).len()
    }

    /// Text typed (or pasted) in the search field.
    pub fn taper(&mut self, s: &str) {
        if !self.saisie {
            return;
        }
        self.recherche.extend(s.chars().filter(|c| !c.is_control()).take(60));
        self.montrer(0);
    }

    /// ⌫ in the search field.
    pub fn effacer(&mut self) {
        if self.saisie {
            self.recherche.pop();
            self.montrer(0);
        }
    }

    /// Entrée / ↓: next match; ↑: previous one (around the end).
    pub fn suivant(&mut self, sens: i64) {
        let text = Text::default();
        let (w, h, top) = self.geom.get();
        let c = self.cadre(&text, w, h, top);
        let n = self.trouvailles(&self.mise(&text, &c)).len();
        if n > 0 {
            self.montrer((self.courant as i64 + sens).rem_euclid(n as i64) as usize);
        }
    }

    /// Échap: closes the sheet, then clears the search; `true` = nothing left to close (back to the code).
    pub fn escape(&mut self) -> bool {
        if self.feuille {
            self.feuille = false;
            return false;
        }
        if self.saisie || !self.recherche.is_empty() {
            self.saisie = false;
            self.recherche.clear();
            self.courant = 0;
            return false;
        }
        true
    }

    /// Wheel / finger: `dy` points (positive = further down).
    pub fn wheel(&mut self, dy: f64) {
        let text = Text::default();
        let (w, h, top) = self.geom.get();
        let c = self.cadre(&text, w, h, top);
        let m = self.mise(&text, &c);
        self.scroll = (self.scroll + dy).clamp(0.0, Self::max_scroll(&m, &c));
        self.vise = None;
    }

    /// The code block under (x, y) (editor window coordinates), if any.
    pub fn bloc_sous(&self, x: f64, y: f64) -> Option<usize> {
        let text = Text::default();
        let (w, h, top) = self.geom.get();
        let c = self.cadre(&text, w, h, top);
        if self.feuille || !c.lecture.contains(x, y) {
            return None;
        }
        let m = self.mise(&text, &c);
        let (cx, cy) = (x - c.lecture.x - c.marge.0, y - c.lecture.y - HAUT_TEXTE + self.scroll);
        m.blocs.iter().position(|b| b.r.contains(cx, cy))
    }

    /// A code block slides sideways by `dx` (finger, Maj + wheel); `false` = no block there.
    pub fn pan(&mut self, x: f64, y: f64, dx: f64) -> bool {
        let Some(k) = self.bloc_sous(x, y) else { return false };
        let text = Text::default();
        let (w, h, top) = self.geom.get();
        let c = self.cadre(&text, w, h, top);
        let m = self.mise(&text, &c);
        let max = (m.blocs[k].large - m.blocs[k].r.w + 46.0).max(0.0);
        let d = (self.decalage(k) + dx).clamp(0.0, max);
        self.poser_decalage(k, d);
        true
    }

    /// Goes to part `i`.
    fn aller(&mut self, i: usize, text: &Text, c: &Cadre) {
        let m = self.mise(text, c);
        if let Some(y) = m.parties.get(i) {
            self.scroll = (y - 6.0).clamp(0.0, Self::max_scroll(&m, c));
            self.vise = Some(i);
        }
    }

    /// The exact text of code block `k` of the open tab (« ⧉ Copier »).
    pub fn texte_bloc(&self, k: usize) -> Option<String> {
        parties(self.onglet).iter().flat_map(|(_, b)| b.iter().flat_map(codes)).nth(k)
    }

    /// « Copié » is on screen (the window wakes up to take it away).
    pub fn anime(&self) -> bool {
        let now = crate::journal::now_ms();
        self.copie.is_some_and(|(_, t)| now - t < COPIE_MS) || self.avis.as_ref().is_some_and(|(_, t)| now - t < AVIS_MS)
    }

    /// Shows a notice at the bottom of the page for a moment (« Copié : fichier pour l'IA (Argos Script) »).
    pub fn avis(&mut self, s: String) {
        self.avis = Some((s, crate::journal::now_ms()));
    }

    /// The search field (window coordinates): the only place where the cell keyboard opens.
    pub fn champ(&self, text: &Text, w: f64, h: f64, top: f64) -> Rect {
        self.cadre(text, w, h, top).champ
    }

    /// A click at (x, y) (window coordinates, the page drawn from `top`).
    #[allow(clippy::too_many_arguments)]
    pub fn click(&mut self, text: &Text, w: f64, h: f64, top: f64, x: f64, y: f64) -> Option<DocAct> {
        self.geom.set((w, h, top));
        let c = self.cadre(text, w, h, top);
        let dans = |r: Rect| r.contains(x, y);
        if c.fermer.is_some_and(dans) {
            return Some(DocAct::Fermer);
        }
        if dans(c.code) {
            self.saisie = false;
            self.feuille = false;
            return Some(DocAct::Code);
        }
        self.saisie = dans(c.champ);
        if self.saisie {
            self.feuille = false;
            return None;
        }
        if let Some(k) = c.onglets.iter().position(|r| dans(*r)) {
            let o = if k == 0 { Onglet::Script } else { Onglet::Rust };
            if o != self.onglet {
                self.onglet = o;
                self.scroll = 0.0;
                self.courant = 0;
                self.vise = None;
                self.decale.clear();
                self.copie = None;
                self.avis = None;
            }
            self.feuille = false;
            return None;
        }
        if c.parties.is_some_and(dans) {
            self.feuille = !self.feuille;
            return None;
        }
        if self.feuille {
            // (a tap outside the sheet only closes it)
            if let Some(i) = self.rangees(&c).iter().position(|r| dans(*r)) {
                self.aller(i, text, &c);
            }
            self.feuille = false;
            return None;
        }
        if dans(c.ia) {
            return Some(DocAct::CopierIa);
        }
        if let Some(i) = c.toc.and_then(|_| self.rangees(&c).iter().position(|r| dans(*r))) {
            self.aller(i, text, &c);
            return None;
        }
        if dans(c.lecture) {
            let m = self.mise(text, &c);
            let (cx, cy) = (x - c.lecture.x - c.marge.0, y - c.lecture.y - HAUT_TEXTE + self.scroll);
            if let Some(k) = m.blocs.iter().position(|b| copier_rect(text, b.r).contains(cx, cy)) {
                self.copie = Some((k, crate::journal::now_ms()));
                return Some(DocAct::CopierBloc(k));
            }
        }
        None
    }

    /// Draws the page on the editor's image (`w` × `h`), from `top` (PC: under the editor header; cell: 0).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, w: f64, h: f64, top: f64, mouse: Option<(f64, f64)>) {
        self.geom.set((w, h, top));
        let c = self.cadre(text, w, h, top);
        let gras = Text::graisse(Graisse::Bold);
        let medium = Text::graisse(Graisse::Medium);
        let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let pill = |pm: &mut Pixmap, r: Rect, s: &str, size: f32| {
            round_fill(pm, r, r.h / 2.0, if hover(&r) { pal::on() } else { pal::hover() }, 1.0);
            text.draw(pm, s, (r.cx() as f32, (r.cy() + size as f64 * 0.36) as f32), size, ICON, Align::Center);
        };
        // cell header: ‹ Code · 📖 Doc · ✕
        if let Some(f) = c.fermer {
            pill(pm, c.code, "‹ Code", 13.0);
            gras.draw(pm, "📖 Doc", ((w / 2.0) as f32, (top + 31.0) as f32), 14.0, WHITE, Align::Center);
            round_fill(pm, f, f.h / 2.0, if hover(&f) { pal::on() } else { pal::hover() }, 1.0);
            crate::shapes::lines(pm, f.cx(), f.cy(), &[&[(-5.0, -5.0), (5.0, 5.0)], &[(5.0, -5.0), (-5.0, 5.0)]], GREY, 1.5);
            crate::fill(pm, 0.0, top + 50.0, w, 1.0, pal::edge(), 1.0);
        } else {
            pill(pm, c.code, "‹ Code", 12.5);
        }
        // tabs
        round_fill(pm, c.seg, c.seg.h / 2.0, pal::field(), 1.0);
        for (k, r) in c.onglets.iter().enumerate() {
            let o = if k == 0 { Onglet::Script } else { Onglet::Rust };
            let size = if c.etroit { 13.5 } else { 13.0 };
            let at = (r.cx() as f32, (r.cy() + size as f64 * 0.36) as f32);
            if o == self.onglet {
                round_fill(pm, *r, r.h / 2.0, pal::accent(), 0.30);
                gras.draw(pm, o.nom(), at, size, WHITE, Align::Center);
            } else {
                if hover(r) {
                    round_fill(pm, *r, r.h / 2.0, pal::hover(), 1.0);
                }
                text.draw(pm, o.nom(), at, size, ICON, Align::Center);
            }
        }
        // search field and its counter
        let m = self.mise(text, &c);
        let t = self.trouvailles(&m);
        let f = c.champ;
        round_fill(pm, f, f.h / 2.0, pal::field(), 1.0);
        if self.saisie {
            round_stroke(pm, f, f.h / 2.0, pal::accent(), 1.0);
        }
        let compte = if self.recherche.trim().is_empty() {
            String::new()
        } else if t.is_empty() {
            "0 / 0".to_string()
        } else {
            format!("{} / {}", self.courant.min(t.len() - 1) + 1, t.len())
        };
        let cw = if compte.is_empty() { 0.0 } else { text.width(&compte, 11.5) as f64 + 14.0 };
        let fy = (f.cy() + 4.5) as f32;
        if self.recherche.is_empty() {
            let vide = if c.etroit { "🔎 Chercher" } else { "🔎 Chercher dans la doc" };
            text.draw(pm, &text.fit(vide, 12.5, f.w - 24.0), ((f.x + 12.0) as f32, fy), 12.5, GREY, Align::Left);
        } else {
            let s = text.fit(&format!("🔎 {}", self.recherche), 12.5, f.w - 24.0 - cw);
            text.draw(pm, &s, ((f.x + 12.0) as f32, fy), 12.5, WHITE, Align::Left);
            if self.saisie {
                crate::fill(pm, f.x + 13.0 + text.width(&s, 12.5) as f64, f.y + 8.0, 1.5, f.h - 16.0, pal::accent(), 1.0);
            }
            text.draw(pm, &compte, ((f.x + f.w - 12.0) as f32, fy), 11.5, GREY, Align::Right);
        }
        if let Some(p) = c.parties {
            pill(pm, p, "☰ Parties", 13.0);
        }
        // reading area
        let cur = self.partie_courante(&m);
        if let Some(mut body) = crate::px::new(c.lecture.w.max(1.0) as u32, c.lecture.h.max(1.0) as u32) {
            self.dessiner_lecture(&mut body, text, &gras, &c, &m, &t);
            crate::px::put(pm, c.lecture.x, c.lecture.y, &body);
        }
        // PC: the parts in the left column
        if let Some(tc) = c.toc {
            crate::fill(pm, tc.x + tc.w + 5.0, tc.y, 1.0, tc.h, pal::edge_soft(), 1.0);
            for (k, (r, (titre, _))) in self.rangees(&c).iter().zip(parties(self.onglet)).enumerate() {
                self.rangee(pm, text, &medium, *r, k, titre, k == cur, hover(r), 12.5);
            }
        }
        // ✨ Copier pour l'IA
        let ia_size = if c.etroit { 14.0 } else { 12.5 };
        pill(pm, c.ia, "✨ Copier pour l'IA", ia_size);
        // cell: the « ☰ Parties » sheet over the text
        if self.feuille {
            crate::fill(pm, 0.0, c.lecture.y, w, c.ia.y + c.ia.h - c.lecture.y, pal::backdrop(), 0.55);
            let fr = self.feuille_rect(&c);
            round_fill(pm, fr, 14.0, pal::popup(), 1.0);
            round_stroke(pm, fr, 14.0, pal::edge(), 1.0);
            for (k, (r, (titre, _))) in self.rangees(&c).iter().zip(parties(self.onglet)).enumerate() {
                self.rangee(pm, text, &medium, *r, k, titre, k == cur, hover(r), 14.0);
            }
        }
        // a notice, at the bottom of the reading area (above the cell's wide pill)
        if let Some((s, _)) = self.avis.as_ref().filter(|(_, t0)| crate::journal::now_ms() - t0 < AVIS_MS) {
            let s = text.fit(s, 12.5, w - 56.0);
            let aw = text.width(&s, 12.5) as f64 + 28.0;
            let ay = if c.etroit {
                c.ia.y - 10.0 - 32.0
            } else {
                c.lecture.y + c.lecture.h - 12.0 - 32.0
            };
            let r = Rect::new((c.lecture.x + (c.lecture.w - aw) / 2.0).max(14.0), ay, aw, 32.0);
            round_fill(pm, r, 16.0, pal::popup(), 0.97);
            round_stroke(pm, r, 16.0, pal::edge(), 1.0);
            text.draw(pm, &s, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, WHITE, Align::Center);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rangee(&self, pm: &mut Pixmap, text: &Text, medium: &Text, r: Rect, k: usize, titre: &str, on: bool, survol: bool, size: f32) {
        if on {
            round_fill(pm, r, 10.0, pal::accent(), 0.22);
        } else if survol {
            round_fill(pm, r, 10.0, pal::hover(), 1.0);
        }
        let by = (r.cy() + size as f64 * 0.36) as f32;
        text.draw(pm, &(k + 1).to_string(), ((r.x + 12.0) as f32, by), size, DIM, Align::Left);
        let t = text.fit(titre, size, r.w - 46.0);
        if on {
            medium.draw(pm, &t, ((r.x + 34.0) as f32, by), size, WHITE, Align::Left);
        } else {
            text.draw(pm, &t, ((r.x + 34.0) as f32, by), size, ICON, Align::Left);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn dessiner_lecture(&self, body: &mut Pixmap, text: &Text, gras: &Text, c: &Cadre, m: &Mise, t: &[Trouve]) {
        let (ox, oy) = (c.marge.0, HAUT_TEXTE - self.scroll);
        let vh = c.lecture.h;
        let vu = |y0: f64, y1: f64| y1 + oy > -10.0 && y0 + oy < vh + 10.0;
        for op in &m.ops {
            match op {
                Op::F(r, col, a, rad) if vu(r.y, r.y + r.h) => {
                    round_fill(body, Rect::new(r.x + ox, r.y + oy, r.w, r.h), *rad, *col, *a);
                }
                Op::S(r, col, rad) if vu(r.y, r.y + r.h) => round_stroke(body, Rect::new(r.x + ox, r.y + oy, r.w, r.h), *rad, *col, 1.0),
                Op::T { x, y, s, size, c: col, p } if vu(*y - 20.0, *y + 6.0) => {
                    let at = ((x + ox) as f32, (y + oy) as f32);
                    match p {
                        Police::Normal => text.draw(body, s, at, *size, *col, Align::Left),
                        Police::Gras => gras.draw(body, s, at, *size, *col, Align::Left),
                        Police::Mono => Mono::draw(body, s, at, *size, *col),
                    }
                }
                _ => {}
            }
        }
        // code blocks: dark block, coloured lines (shifted sideways), fade on the right, « ⧉ Copier »
        let adv = Mono::advance(CS) as f64;
        let now = crate::journal::now_ms();
        for (k, b) in m.blocs.iter().enumerate() {
            if !vu(b.r.y, b.r.y + b.r.h) {
                continue;
            }
            let r = Rect::new(b.r.x + ox, b.r.y + oy, b.r.w, b.r.h);
            round_fill(body, r, 12.0, pal::night(), 0.9);
            round_stroke(body, r, 12.0, pal::edge_soft(), 1.0);
            let d = self.decalage(k);
            let debut = (d / adv).floor() as usize;
            let vis = ((r.w - 2.0 * CPAD) / adv).floor().max(1.0) as usize;
            for (j, l) in b.lignes.iter().enumerate() {
                let by = r.y + CVPAD + j as f64 * CLH + 14.0;
                for (from, piece, col) in crate::editor::spans(l) {
                    let n = piece.chars().count();
                    let (a, z) = (from.max(debut), (from + n).min(debut + vis));
                    if a >= z {
                        continue;
                    }
                    let s: String = piece.chars().skip(a - from).take(z - a).collect();
                    Mono::draw(body, &s, ((r.x + CPAD + (a - debut) as f64 * adv) as f32, by as f32), CS, col);
                }
            }
            if b.large - d > r.w {
                for i in 0..12 {
                    let a = 0.95 * (i as f32 + 1.0) / 12.0;
                    crate::fill(
                        body,
                        r.x + r.w - 46.0 + i as f64 * 46.0 / 12.0,
                        r.y + 1.0,
                        46.0 / 12.0 + 0.5,
                        r.h - 2.0,
                        pal::night(),
                        a,
                    );
                }
            }
            let p = copier_rect(text, r);
            let copie = self.copie.is_some_and(|(b, t0)| b == k && now - t0 < COPIE_MS);
            round_fill(body, p, 12.0, if copie { pal::up() } else { WHITE }, if copie { 0.35 } else { 0.10 });
            let s = if copie { "Copié" } else { copier_libelle(text) };
            text.draw(body, s, (p.cx() as f32, (p.y + 16.0) as f32), 11.5, ICON, Align::Center);
        }
        // search: every match highlighted, the current one stronger
        for (i, f) in t.iter().enumerate() {
            let (mut x, y) = (f.x + ox, f.y + oy);
            if let Some(k) = f.bloc {
                let b = &m.blocs[k];
                x = b.r.x + ox + f.x - (self.decalage(k) / adv).floor() * adv;
                if x < b.r.x + ox + CPAD - 1.0 || x + f.w > b.r.x + ox + b.r.w - 4.0 {
                    continue;
                }
            }
            if vu(f.y, f.y + f.h) {
                let (col, a) = if i == self.courant { (pal::ORANGE, 0.45) } else { (pal::GOLD, 0.25) };
                round_fill(body, Rect::new(x - 1.0, y, f.w + 2.0, f.h), 3.0, col, a);
            }
        }
    }
}

/// Code blocks of a block, in reading order (a folded block's included).
fn codes(b: &Bloc) -> Vec<String> {
    match b {
        Bloc::Code(l) => vec![l.join("\n")],
        Bloc::Details { blocs, .. } => blocs.iter().flat_map(codes).collect(),
        _ => vec![],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn heading_count(md: &str) -> usize {
        let mut dans = false;
        md.lines()
            .filter(|l| {
                if l.trim_start().starts_with("```") {
                    dans = !dans;
                }
                !dans && l.starts_with("## ")
            })
            .count()
    }

    /// D8: one entry per `## ` heading of each doc (14 for Argos Script, 8 for Rust today), counted from the files.
    #[test]
    fn parties_des_deux_docs() {
        for o in [Onglet::Script, Onglet::Rust] {
            assert_eq!(parties(o).len(), heading_count(o.md()), "{o:?}");
        }
        assert_eq!(parties(Onglet::Script).len(), 14);
        assert_eq!(parties(Onglet::Rust).len(), 8);
        assert!(RUST_MD.contains("compilée sans changement le 22 nov. 2026 (ma_moyenne.wasm, 32 549 octets)."));
    }

    /// D8: the tabs switch the content; the search finds « input.int » with a count; Entrée / ↑ move, Échap clears.
    #[test]
    fn onglets_et_recherche() {
        let t = Text::default();
        let (w, h, top) = (1060.0, 700.0, 40.0);
        let mut d = Doc::default();
        let c = d.cadre(&t, w, h, top);
        assert_eq!(d.click(&t, w, h, top, c.onglets[1].cx(), c.onglets[1].cy()), None);
        assert_eq!(d.onglet, Onglet::Rust);
        assert_eq!(d.rangees(&c).len(), 8);
        d.click(&t, w, h, top, c.onglets[0].cx(), c.onglets[0].cy());
        assert_eq!(d.onglet, Onglet::Script);
        assert_eq!(d.rangees(&c).len(), 14);
        // search
        d.click(&t, w, h, top, c.champ.cx(), c.champ.cy());
        assert!(d.saisie);
        d.taper("Input.Int");
        let n = d.nombre_trouves(&t, w, h, top);
        assert!(n > 0, "« input.int » is found");
        assert_eq!(d.courant, 0);
        d.suivant(1);
        assert_eq!(d.courant, 1 % n);
        d.suivant(-1);
        d.suivant(-1);
        assert_eq!(d.courant, n - 1, "↑ from the first goes to the last");
        assert!(d.scroll > 0.0, "the match is scrolled into view");
        assert!(!d.escape(), "Échap clears the search first");
        assert!(d.recherche.is_empty() && d.nombre_trouves(&t, w, h, top) == 0);
        assert!(d.escape(), "then nothing is left: back to the code");
        // a part of the list: the reading area goes there, the part is highlighted
        let rows = d.rangees(&c);
        d.click(&t, w, h, top, rows[10].cx(), rows[10].cy());
        let m = d.mise(&t, &c);
        assert_eq!(d.partie_courante(&m), 10);
        d.wheel(-1e9);
        assert_eq!((d.scroll, d.partie_courante(&m)), (0.0, 0), "the highlight follows the scroll");
    }

    /// D8: « ⧉ Copier » gives the exact text of its block; « Copier pour l'IA » the whole llms file of the tab.
    #[test]
    fn copier() {
        let t = Text::default();
        let (w, h, top) = (1060.0, 700.0, 40.0);
        let mut d = Doc::default();
        let c = d.cadre(&t, w, h, top);
        let m = d.mise(&t, &c);
        let b = m.blocs[0].r;
        let p = copier_rect(&t, b);
        let (x, y) = (c.lecture.x + c.marge.0 + p.cx(), c.lecture.y + HAUT_TEXTE + p.cy());
        assert_eq!(d.click(&t, w, h, top, x, y), Some(DocAct::CopierBloc(0)));
        let premier = "//@version=6\nindicator(\"Ma moyenne\", overlay = true)\nlongueur = input.int(20, \"Longueur\", minval = 1)\nmoy = ta.sma(close, longueur)\nplot(moy, \"Moyenne\", color = color.orange, linewidth = 2)";
        assert_eq!(d.texte_bloc(0).as_deref(), Some(premier));
        assert!(d.anime(), "« Copié » shows");
        // every block of both docs: its text is in the file, as written
        for o in [Onglet::Script, Onglet::Rust] {
            d.onglet = o;
            let n = o.md().matches("```").count() / 2;
            assert_eq!((0..n).filter_map(|k| d.texte_bloc(k)).count(), n, "{o:?}");
            for k in 0..n {
                assert!(o.md().contains(&d.texte_bloc(k).unwrap_or_default()));
            }
        }
        d.onglet = Onglet::Script;
        assert_eq!(d.click(&t, w, h, top, c.ia.cx(), c.ia.cy()), Some(DocAct::CopierIa));
        assert_eq!(Onglet::Script.llms(), SCRIPT_LLMS);
        d.avis("Copié : fichier pour l'IA (Argos Script)".into());
        assert!(d.anime(), "the notice shows");
        let mut pm = crate::px::new(w as u32, h as u32).expect("image");
        d.draw(&mut pm, &t, w, h, top, None);
        assert!(Onglet::Rust.llms().len() > 1000 && SCRIPT_LLMS.len() > 1000);
    }

    fn separes(a: Rect, b: Rect) -> bool {
        a.x + a.w <= b.x || b.x + b.w <= a.x || a.y + a.h <= b.y || b.y + b.h <= a.y
    }

    /// D7 / D8: at 390 × 844 (cell, the doc over the whole editor), 820 × 1180 and 1440 × 900, the header, tabs,
    /// search, « ☰ Parties », reading area and the bottom pill never overlap and stay in the window.
    #[test]
    fn rien_ne_se_chevauche() {
        let t = Text::default();
        for (w, h, top) in [(390.0, 780.0, 0.0), (772.0, 1100.0, 40.0), (1100.0, 820.0, 40.0)] {
            let mut d = Doc::default();
            let c = d.cadre(&t, w, h, top);
            let mut v = vec![c.code, c.seg, c.champ, c.ia, c.lecture];
            v.extend(c.fermer);
            v.extend(c.parties);
            v.extend(c.toc);
            for (i, a) in v.iter().enumerate() {
                assert!(a.x >= 0.0 && a.y >= top && a.x + a.w <= w + 0.5 && a.y + a.h <= h + 0.5, "{w}: {a:?} out");
                for b in &v[i + 1..] {
                    assert!(separes(*a, *b), "{w}: {a:?} / {b:?}");
                }
            }
            assert_eq!(c.etroit, w < ETROIT);
            if c.etroit {
                d.feuille = true;
                let f = d.feuille_rect(&c);
                assert!(f.y + f.h <= h && d.rangees(&c).iter().all(|r| r.y + r.h <= f.y + f.h + 0.5));
            }
            let mut pm = crate::px::new(w as u32, h as u32).expect("image");
            d.draw(&mut pm, &t, w, h, top, None);
        }
    }

    /// Cell: « ☰ Parties » opens the sheet, a part closes it there; a code block longer than the screen slides sideways.
    #[test]
    fn feuille_et_code_de_cote() {
        let t = Text::default();
        let (w, h) = (390.0, 780.0);
        let mut d = Doc::default();
        let c = d.cadre(&t, w, h, 0.0);
        let p = c.parties.expect("pill");
        d.click(&t, w, h, 0.0, p.cx(), p.cy());
        assert!(d.feuille);
        let r = d.rangees(&c)[3];
        d.click(&t, w, h, 0.0, r.cx(), r.cy());
        assert!(!d.feuille);
        let m = d.mise(&t, &c);
        assert_eq!(d.partie_courante(&m), 3);
        // the first block (the 5-line example) is wider than 358 points: it slides, and stops at its end
        let b = m.blocs[0].r;
        d.scroll = b.y;
        let (x, y) = (c.lecture.x + c.marge.0 + b.x + 30.0, c.lecture.y + HAUT_TEXTE + 10.0);
        assert!(m.blocs[0].large > b.w);
        assert!(d.pan(x, y, 50.0));
        assert_eq!(d.decalage(0), 50.0);
        d.pan(x, y, 1e6);
        assert!(d.decalage(0) < m.blocs[0].large);
        assert!(!d.pan(c.lecture.x + 2.0, c.lecture.y + 2.0, 10.0), "on a title: nothing slides");
    }

    #[test]
    fn liens_et_code_en_ligne() {
        assert_eq!(sans_liens("voir [la doc](https://x.y/z) ici"), "voir la doc ici");
        assert_eq!(sans_liens("close[1] et [a](b)"), "close[1] et a");
        let p = pieces(&argos_academy::markdown::morceaux("`input.int` crée **un réglage**"));
        assert_eq!(p[0], ("input.int".into(), Police::Mono));
        assert_eq!(p[2], ("un réglage".into(), Police::Gras));
        assert_eq!(positions("Input.INT x input.int", &bas_de_casse("input.int")), vec![0, 12]);
    }
}
