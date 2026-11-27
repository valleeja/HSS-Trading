//! 🌳 Arborescence des objets (comme Argos v1) : les indicateurs de la chart, puis ses dessins du plus récent au plus
//! ancien. Une ligne par dessin : icône de l'outil, nom, son texte ou son prix ; 👁 cacher / montrer, 🔒 verrouiller,
//! ✕ retirer. Un indicateur (index à partir de `IND`) a 👁, ⚙️ réglages et ✕ retirer.
//! Un clic sur la ligne sélectionne le dessin. Une table de zones sert au dessin et au clic.

use crate::palette as pal;
use crate::palette::{GREY, ICON, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// Index des lignes d'indicateur : `IND` + position dans `Ind::ALL` (les dessins gardent leur index).
pub const IND: usize = 1 << 20;

/// Une ligne (un dessin ou un indicateur).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeRow {
    /// Index du dessin dans la chart.
    pub index: usize,
    pub tool: String,
    pub label: String,
    /// Son texte, sinon son prix.
    pub sub: String,
    pub hidden: bool,
    pub locked: bool,
    pub selected: bool,
}

/// Ce qu'un clic demande (avec l'index du dessin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeClick {
    None,
    Close,
    Select(usize),
    Hide(usize),
    Lock(usize),
    Remove(usize),
    /// ⚙️ Réglages d'un indicateur.
    Settings(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Close,
    Row(usize),
    Hide(usize),
    Lock(usize),
    Remove(usize),
    Settings(usize),
}

const W: f64 = 360.0;
const HEAD: f64 = 52.0;
const ROW: f64 = 46.0;
const PAD: f64 = 10.0;

/// La fenêtre, sous la barre du haut à droite ; elle défile quand il y a beaucoup de dessins.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjTree {
    pub rect: Rect,
    scroll: f64,
}

impl ObjTree {
    /// Collée sous `top` (bas de la barre du haut), au bout droit de la fenêtre `win_w` × `win_h`.
    pub fn new(top: f64, win_w: f64, win_h: f64) -> Self {
        let h = (win_h - top - 16.0).clamp(160.0, 560.0);
        Self {
            rect: Rect::new((win_w - W - 60.0).max(8.0), top + 8.0, W, h),
            scroll: 0.0,
        }
    }

    fn max_scroll(&self, n: usize) -> f64 {
        (n as f64 * ROW - (self.rect.h - HEAD - PAD)).max(0.0)
    }

    /// Molette (`steps` > 0 = vers le haut).
    pub fn wheel(&mut self, steps: f64, n: usize) {
        self.scroll = (self.scroll - steps * ROW).clamp(0.0, self.max_scroll(n));
    }

    fn zones(&self, rows: &[TreeRow]) -> Vec<(Rect, Zone)> {
        let r = self.rect;
        let mut out = vec![(Rect::new(r.x + r.w - 40.0, r.y + 12.0, 28.0, 28.0), Zone::Close)];
        let top = r.y + HEAD;
        for (k, row) in rows.iter().enumerate() {
            let y = top + k as f64 * ROW - self.scroll;
            if y + ROW < top || y > r.y + r.h - PAD {
                continue;
            }
            let q = Rect::new(r.x + PAD, y, r.w - 2.0 * PAD, ROW - 4.0);
            out.push((q, Zone::Row(row.index)));
            let mid = if row.index >= IND { Zone::Settings(row.index) } else { Zone::Lock(row.index) };
            for (j, z) in [Zone::Remove(row.index), mid, Zone::Hide(row.index)].into_iter().enumerate() {
                out.push((Rect::new(q.x + q.w - 34.0 - j as f64 * 32.0, q.cy() - 14.0, 28.0, 28.0), z));
            }
        }
        out
    }

    /// Clic (coordonnées de la fenêtre de l'app).
    pub fn click(&self, rows: &[TreeRow], x: f64, y: f64) -> TreeClick {
        if !self.rect.contains(x, y) {
            return TreeClick::Close;
        }
        if y < self.rect.y + HEAD && !self.zones(rows)[0].0.contains(x, y) {
            return TreeClick::None;
        }
        // les boutons passent avant la ligne
        match self.zones(rows).into_iter().rev().find(|(q, _)| q.contains(x, y)).map(|(_, z)| z) {
            Some(Zone::Close) => TreeClick::Close,
            Some(Zone::Row(i)) => TreeClick::Select(i),
            Some(Zone::Hide(i)) => TreeClick::Hide(i),
            Some(Zone::Lock(i)) => TreeClick::Lock(i),
            Some(Zone::Remove(i)) => TreeClick::Remove(i),
            Some(Zone::Settings(i)) => TreeClick::Settings(i),
            None => TreeClick::None,
        }
    }

    /// Image ; `symbol` pour le message « aucun dessin ».
    pub fn draw(&self, text: &Text, rows: &[TreeRow], symbol: &str, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::jetons::fond_fenetre(&mut pm, loc(r), 12.0);
        round_stroke(&mut pm, loc(r), 12.0, pal::edge(), 1.0);
        text.draw(&mut pm, "Arborescence des objets", (16.0, 32.0), 15.5, WHITE, Align::Left);
        if rows.is_empty() {
            text.draw(
                &mut pm,
                &format!("{} {symbol}.", crate::i18n::tr("Aucun dessin sur")),
                ((r.w / 2.0) as f32, (HEAD + 40.0) as f32),
                13.0,
                GREY,
                Align::Center,
            );
        }
        let over = |q: &Rect| q.contains(mouse.0, mouse.1);
        for (q, z) in self.zones(rows) {
            let lq = loc(q);
            match z {
                Zone::Close => {
                    round_fill(&mut pm, lq, 8.0, if over(&q) { pal::hover() } else { pal::tile() }, 1.0);
                    lines(
                        &mut pm,
                        lq.cx(),
                        lq.cy(),
                        &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]],
                        ICON,
                        1.5,
                    );
                }
                Zone::Row(i) => {
                    let Some(row) = rows.iter().find(|x| x.index == i) else { continue };
                    if row.selected || over(&q) {
                        round_fill(&mut pm, lq, 8.0, if row.selected { pal::on() } else { pal::hover() }, 1.0);
                    }
                    let c = if row.hidden { GREY } else { WHITE };
                    crate::icons::draw(&mut pm, &row.tool, lq.x + 18.0, lq.cy(), 18, if row.hidden { GREY } else { ICON });
                    text.draw(
                        &mut pm,
                        &text.fit(&row.label, 13.5, lq.w - 150.0),
                        ((lq.x + 38.0) as f32, (lq.y + 18.0) as f32),
                        13.5,
                        c,
                        Align::Left,
                    );
                    text.draw(
                        &mut pm,
                        &text.fit(&row.sub, 11.0, lq.w - 150.0),
                        ((lq.x + 38.0) as f32, (lq.y + 34.0) as f32),
                        11.0,
                        GREY,
                        Align::Left,
                    );
                }
                Zone::Hide(i) | Zone::Lock(i) | Zone::Remove(i) | Zone::Settings(i) => {
                    let Some(row) = rows.iter().find(|x| x.index == i) else { continue };
                    if over(&q) {
                        round_fill(&mut pm, lq, 6.0, pal::tile(), 1.0);
                    }
                    let (icon, on) = match z {
                        Zone::Hide(_) => (if row.hidden { "eyeoff2" } else { "eye2" }, !row.hidden),
                        Zone::Lock(_) => (if row.locked { "lock2" } else { "unlock2" }, row.locked),
                        Zone::Settings(_) => ("gear", true),
                        _ => ("trash", true),
                    };
                    crate::icons::draw(&mut pm, icon, lq.cx(), lq.cy(), 16, if on { ICON } else { GREY });
                }
            }
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<TreeRow> {
        (0..3)
            .rev()
            .map(|i| TreeRow {
                index: i,
                tool: "trend".into(),
                label: format!("Ligne {i}"),
                sub: "76.86".into(),
                ..Default::default()
            })
            .collect()
    }

    #[test]
    fn arborescence() {
        let t = ObjTree::new(60.0, 1400.0, 800.0);
        let r = rows();
        let z = t.zones(&r);
        let at = |w: Zone| z.iter().find(|(_, x)| *x == w).unwrap().0;
        let q = at(Zone::Row(2));
        assert_eq!(t.click(&r, q.x + 40.0, q.cy()), TreeClick::Select(2), "1re ligne = le plus récent");
        let q = at(Zone::Hide(1));
        assert_eq!(t.click(&r, q.cx(), q.cy()), TreeClick::Hide(1));
        let q = at(Zone::Lock(0));
        assert_eq!(t.click(&r, q.cx(), q.cy()), TreeClick::Lock(0));
        let q = at(Zone::Remove(0));
        assert_eq!(t.click(&r, q.cx(), q.cy()), TreeClick::Remove(0));
        assert_eq!(t.click(&r, 5.0, 5.0), TreeClick::Close);
        assert!(t.draw(&Text::default(), &r, "BTC", (0.0, 0.0)).is_some());
        assert!(t.draw(&Text::default(), &[], "BTC", (0.0, 0.0)).is_some());
        // un indicateur : ⚙️ à la place du cadenas
        let ind = vec![TreeRow {
            index: IND + 3,
            tool: "📈".into(),
            label: "Argos Cipher Pro".into(),
            ..Default::default()
        }];
        let z = t.zones(&ind);
        let q = z.iter().find(|(_, x)| *x == Zone::Settings(IND + 3)).unwrap().0;
        assert_eq!(t.click(&ind, q.cx(), q.cy()), TreeClick::Settings(IND + 3));
        // beaucoup de dessins : ça défile
        let many: Vec<TreeRow> = (0..40)
            .map(|i| TreeRow {
                index: i,
                ..Default::default()
            })
            .collect();
        let mut t2 = t.clone();
        t2.wheel(-100.0, many.len());
        assert!(t2.scroll > 0.0 && t2.scroll == t2.max_scroll(many.len()));
    }
}
