//! 📋 Menu en liste (comme TradingView) : sections repliables, lignes avec ⭐ favori, ligne choisie surlignée,
//! lignes en gris quand elles ne sont pas prêtes, défilement à la molette. Sert au menu des intervalles et à celui
//! des types de chart. Dessiné dans sa propre image, posée par-dessus la fenêtre.

use crate::palette as pal;
use crate::palette::{DIM, GREY, ICON, SNOW, WHITE};
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MenuItem {
    pub id: i64,
    pub label: String,
    pub enabled: bool,
    /// `Some(true)` : en favori (⭐ pleine) ; `None` : pas d'étoile.
    pub star: Option<bool>,
    /// Ligne choisie en ce moment.
    pub checked: bool,
    /// Icône d'Argos v1 à gauche (module `icons`).
    pub icon: Option<&'static str>,
    /// Raccourci affiché à droite (« Alt + T »).
    pub hint: String,
    /// Aide : un « ? » après le nom, la bulle au survol.
    pub tip: String,
    /// Pastille de couleur à gauche (ex. 🚩 filtre de couleur de la watchlist).
    pub color: Option<Rgb>,
    /// Nom plus foncé, mais toujours cliquable (ex. 🌐 langue pas encore téléchargée).
    pub dim: bool,
    /// ✕ at the right: removes the line (a saved drawing template).
    pub remove: bool,
}

/// Une section. Avec `strip`, elle se dessine sur **une seule rangée** : son titre, puis ses choix côte à côte
/// (« 🚩 Voir : Tout ● ● ● », comme Argos v1) ; sinon une ligne par choix (avec sa pastille de couleur s'il en a une).
#[derive(Debug, Clone, PartialEq)]
pub struct MenuGroup {
    /// Titre de la section (vide : pas de titre).
    pub title: String,
    pub items: Vec<MenuItem>,
    /// Section sur une seule rangée de pastilles (titre puis choix côte à côte, ex. « 🚩 Voir : Tout ● ● ● »).
    pub strip: bool,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuClick {
    /// À côté du menu (il se ferme).
    Outside,
    /// Dans le menu, rien à faire de plus (section repliée, ligne grise…).
    Inside,
    Pick(i64),
    Star(i64),
    /// ✕ of a line with `remove`.
    Remove(i64),
}

const W: f64 = 300.0;

/// The hint written at the right of a row: 📱 none when it is a keyboard shortcut in phone view (no keyboard).
fn indice(h: &str) -> &str {
    if !crate::actions::montrer_raccourcis() && crate::actions::est_raccourci(h) {
        ""
    } else {
        h
    }
}

/// Largeur du menu : 300, ou plus pour que la plus longue ligne (icône, raccourci, ⭐ compris) tienne.
fn width(groups: &[MenuGroup]) -> f64 {
    let t = Text::default();
    let need = groups
        .iter()
        .flat_map(|g| g.items.iter().filter(|_| !g.is_strip()))
        .map(|i| {
            t.width(&i.label, 14.0) as f64
                + if indice(&i.hint).is_empty() {
                    0.0
                } else {
                    t.width(indice(&i.hint), 12.0) as f64 + 20.0
                }
                + 96.0
        })
        .fold(W, f64::max);
    need.min(460.0)
}
const HEAD: f64 = 34.0;
const SEP: f64 = 9.0;
/// Height of an item row (PC, and most menus).
pub const ROW: f64 = 40.0;
const PAD: f64 = 6.0;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Line {
    Group(usize),
    Item(usize, usize),
    /// Section de pastilles sur une rangée.
    Strip(usize),
    /// Trait entre deux sections sans titre (menu du clic droit sur un dessin).
    Sep,
}

/// Rangée de pastilles : où commencent les choix, largeur d'une pastille de couleur, écart entre deux choix, hauteur
/// d'une rangée de plus (les choix passent à la ligne quand la place manque).
const STRIP_X: f64 = 76.0;
const STRIP_DOT_PC: f64 = 24.0;
const STRIP_GAP: f64 = 6.0;
const STRIP_ROW2_PC: f64 = 30.0;

/// Width of a colour dot's slot (📱 phone view: 36, the dot 26 across, for the finger; Jee, 21 Nov, list 2).
fn strip_dot() -> f64 {
    if crate::actions::tactile() {
        36.0
    } else {
        STRIP_DOT_PC
    }
}

/// Height of a second row of dots (📱 phone view: 40).
fn strip_row2() -> f64 {
    if crate::actions::tactile() {
        40.0
    } else {
        STRIP_ROW2_PC
    }
}
/// Taille du texte d'un choix et marge de chaque côté dans sa pilule.
const STRIP_FONT: f32 = 12.5;
const STRIP_PAD: f64 = 10.0;

/// Largeur d'un choix : sa pastille de couleur, ou une pilule de la largeur de son texte.
fn strip_item_w(it: &MenuItem) -> f64 {
    if it.color.is_some() {
        strip_dot()
    } else {
        Text::default().width(&it.label, STRIP_FONT) as f64 + 2.0 * STRIP_PAD
    }
}

impl MenuGroup {
    fn is_strip(&self) -> bool {
        self.strip
    }
}

/// Hauteur de la ligne du titre quand il passe au-dessus des pastilles.
const STRIP_TITLE_H: f64 = 22.0;

/// Où commencent les pastilles quand le titre est à leur gauche : après le titre, jamais dessous.
fn strip_x0(g: &MenuGroup) -> f64 {
    let tw = if g.title.is_empty() {
        0.0
    } else {
        Text::default().width(&g.title, 13.0) as f64 + 14.0
    };
    STRIP_X.max(16.0 + tw)
}

/// 📱 Le titre tient-il à gauche des pastilles dans un menu de largeur `w` ? Sinon (titre long sur cell, « FRAIS
/// (entrée et sortie) » du menu Backtest), il passe sur sa propre ligne au-dessus et les pastilles partent du bord.
fn strip_title_above(g: &MenuGroup, w: f64) -> bool {
    let total: f64 = g.items.iter().map(|it| strip_item_w(it) + STRIP_GAP).sum();
    strip_x0(g) + total > w - 12.0
}

/// Choix d'une rangée de pastilles : (index, centre x depuis le bord gauche du menu, demi-largeur, rangée), dans un
/// menu de largeur `w`. Chaque choix a la largeur de son texte ; ce qui ne tient pas passe à la rangée suivante.
fn strip_slots(g: &MenuGroup, w: f64) -> Vec<(usize, f64, f64, usize)> {
    let x0 = if strip_title_above(g, w) { 16.0 } else { strip_x0(g) };
    let (mut x, mut rang) = (x0, 0);
    g.items
        .iter()
        .enumerate()
        .map(|(i, it)| {
            let iw = strip_item_w(it);
            if x > x0 && x + iw > w - 12.0 {
                (x, rang) = (16.0, rang + 1);
            }
            let slot = (i, x + iw / 2.0, iw / 2.0, rang);
            x += iw + STRIP_GAP;
            slot
        })
        .collect()
}

/// Nombre de rangées de choix.
fn strip_rows(g: &MenuGroup, w: f64) -> usize {
    strip_slots(g, w).last().map_or(1, |s| s.3 + 1)
}

/// Centre (hauteur) de la rangée `rang` depuis le haut des choix.
fn strip_cy(rang: usize) -> f64 {
    ROW / 2.0 + rang as f64 * strip_row2()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ListMenu {
    pub groups: Vec<MenuGroup>,
    /// Sections repliées (par titre).
    pub collapsed: Vec<String>,
    /// Place du menu dans la fenêtre.
    pub rect: Rect,
    /// Bouton qui l'a ouvert (pour rouvrir au même endroit).
    pub anchor: Rect,
    pub scroll: f64,
    /// 🩶 A tool menu (drawing bar): grey icons and a light grey patch on the chosen row, never blue (Jee, 17 Nov).
    pub gris: bool,
    /// Height of an item row: `ROW`, or the finger's touch minimum for some menus a touch opens ([`ListMenu::new_lignes`]).
    pub ligne: f64,
}

impl ListMenu {
    /// Menu collé au bouton `anchor` : en dessous s'il y a de la place, sinon au-dessus ; jamais hors de la fenêtre.
    /// S'ouvre défilé jusqu'à la ligne choisie.
    pub fn new(groups: Vec<MenuGroup>, collapsed: Vec<String>, anchor: Rect, win_w: f64, win_h: f64) -> Self {
        Self::new_lignes(groups, collapsed, anchor, win_w, win_h, ROW)
    }

    /// 👆 [`ListMenu::new`] with item rows `ligne` high (by the finger: [`crate::formfactor::TOUCH_MIN_PT`]).
    pub fn new_lignes(groups: Vec<MenuGroup>, collapsed: Vec<String>, anchor: Rect, win_w: f64, win_h: f64, ligne: f64) -> Self {
        let mut m = Self {
            groups,
            collapsed,
            rect: anchor,
            anchor,
            scroll: 0.0,
            gris: false,
            ligne,
        };
        // la largeur d'abord : la hauteur d'une rangée de pastilles en dépend (titre au-dessus si trop étroit)
        m.rect.w = width(&m.groups).min(win_w - 8.0);
        let content = m.content_h() + 2.0 * PAD;
        let below = win_h - (anchor.y + anchor.h) - 8.0;
        let above = anchor.y - 8.0;
        let h = content.min(below.max(above)).max(80.0);
        let y = if below >= h || below >= above {
            anchor.y + anchor.h + 4.0
        } else {
            anchor.y - 4.0 - h
        };
        // 📱 jamais plus large que l'écran (cell : le menu ⋯ d'un indicateur débordait à droite)
        let w = width(&m.groups).min(win_w - 8.0);
        let x = anchor.x.min(win_w - w - 4.0).max(4.0);
        m.rect = Rect::new(x, y.max(4.0), w, h);
        m.scroll_to_checked();
        m
    }

    /// Menu à droite du bouton `anchor` (sous-menus de la barre de gauche), le haut aligné sur le bouton.
    pub fn beside(groups: Vec<MenuGroup>, collapsed: Vec<String>, anchor: Rect, win_w: f64, win_h: f64) -> Self {
        let mut m = Self {
            groups,
            collapsed,
            rect: anchor,
            anchor,
            scroll: 0.0,
            gris: false,
            ligne: ROW,
        };
        m.rect.w = width(&m.groups).min(win_w - 8.0);
        let h = (m.content_h() + 2.0 * PAD).min(win_h - 8.0).max(80.0);
        let y = (anchor.y - PAD).min(win_h - h - 4.0).max(4.0);
        // 📱 jamais plus large que l'écran (cell : le menu ⋯ d'un indicateur débordait à droite)
        let w = width(&m.groups).min(win_w - 8.0);
        let x = (anchor.x + anchor.w + 6.0).min(win_w - w - 4.0).max(4.0);
        m.rect = Rect::new(x, y, w, h);
        m.scroll_to_checked();
        m
    }

    fn scroll_to_checked(&mut self) {
        let m = self;
        if let Some(top) = m.lines_at().iter().find(|(_, l)| m.is_checked(*l)).map(|(t, _)| *t) {
            m.scroll = (top - m.rect.y - m.rect.h / 2.0).clamp(0.0, m.max_scroll());
        }
    }

    fn is_checked(&self, l: Line) -> bool {
        matches!(l, Line::Item(g, i) if self.groups[g].items[i].checked)
    }

    fn lines(&self) -> Vec<Line> {
        let mut out = vec![];
        for (g, grp) in self.groups.iter().enumerate() {
            if grp.is_strip() {
                out.push(Line::Strip(g));
                continue;
            }
            if !grp.title.is_empty() {
                out.push(Line::Group(g));
            } else if g > 0 {
                out.push(Line::Sep);
            }
            if !self.collapsed.contains(&grp.title) || grp.title.is_empty() {
                out.extend((0..grp.items.len()).map(|i| Line::Item(g, i)));
            }
        }
        out
    }

    fn h(&self, l: Line) -> f64 {
        match l {
            Line::Group(_) => HEAD,
            Line::Sep => SEP,
            Line::Strip(g) => {
                let grp = &self.groups[g];
                let dessus = if strip_title_above(grp, self.rect.w) { STRIP_TITLE_H } else { 0.0 };
                ROW + dessus + (strip_rows(grp, self.rect.w) - 1) as f64 * strip_row2()
            }
            _ => self.ligne,
        }
    }

    fn content_h(&self) -> f64 {
        self.lines().into_iter().map(|l| self.h(l)).sum()
    }

    fn max_scroll(&self) -> f64 {
        (self.content_h() + 2.0 * PAD - self.rect.h).max(0.0)
    }

    /// Lignes avec leur haut (fenêtre).
    fn lines_at(&self) -> Vec<(f64, Line)> {
        let mut y = self.rect.y + PAD - self.scroll;
        self.lines()
            .into_iter()
            .map(|l| {
                let t = y;
                y += self.h(l);
                (t, l)
            })
            .collect()
    }

    pub fn wheel(&mut self, steps: f64) {
        self.scroll = (self.scroll - steps * ROW).clamp(0.0, self.max_scroll());
    }

    pub fn click(&mut self, x: f64, y: f64) -> MenuClick {
        if !self.rect.contains(x, y) {
            return MenuClick::Outside;
        }
        let Some((top, line)) = self.lines_at().into_iter().find(|(t, l)| y >= *t && y < t + self.h(*l)) else {
            return MenuClick::Inside;
        };
        match line {
            Line::Group(g) => {
                let t = self.groups[g].title.clone();
                match self.collapsed.iter().position(|c| *c == t) {
                    Some(k) => {
                        self.collapsed.remove(k);
                    }
                    None => self.collapsed.push(t),
                }
                self.scroll = self.scroll.clamp(0.0, self.max_scroll());
                MenuClick::Inside
            }
            Line::Sep => MenuClick::Inside,
            Line::Strip(g) => {
                let grp = &self.groups[g];
                // titre au-dessus des pastilles : le toucher sur le titre ne choisit rien
                if strip_title_above(grp, self.rect.w) && y < top + STRIP_TITLE_H {
                    return MenuClick::Inside;
                }
                let dy = if strip_title_above(grp, self.rect.w) { STRIP_TITLE_H } else { 0.0 };
                match strip_slots(grp, self.rect.w)
                    .into_iter()
                    .find(|(_, cx, hw, rang)| (x - self.rect.x - cx).abs() <= *hw && (y - top - dy - strip_cy(*rang)).abs() <= strip_row2() / 2.0)
                {
                    Some((i, _, _, _)) if grp.items[i].enabled => MenuClick::Pick(grp.items[i].id),
                    _ => MenuClick::Inside,
                }
            }
            Line::Item(g, i) => {
                let it = &self.groups[g].items[i];
                if it.star.is_some() && x > self.rect.x + self.rect.w - 44.0 {
                    MenuClick::Star(it.id)
                } else if it.remove && x > self.rect.x + self.rect.w - 44.0 {
                    MenuClick::Remove(it.id)
                } else if it.enabled {
                    MenuClick::Pick(it.id)
                } else {
                    MenuClick::Inside
                }
            }
        }
    }

    /// Change l'étoile d'une ligne (après un clic ⭐), sans refaire le menu.
    pub fn set_star(&mut self, id: i64, on: bool) {
        for it in self.groups.iter_mut().flat_map(|g| g.items.iter_mut()).filter(|it| it.id == id) {
            it.star = Some(on);
        }
    }

    /// Aide de la ligne sous la souris (le « ? » d'Argos v1) : (texte, haut de la ligne dans la fenêtre).
    pub fn tip_at(&self, x: f64, y: f64) -> Option<(String, f64)> {
        if !self.rect.contains(x, y) {
            return None;
        }
        self.lines_at().into_iter().find_map(|(t, l)| match l {
            Line::Item(g, i) if y >= t && y < t + self.ligne => {
                let it = &self.groups[g].items[i];
                // (📱 phone view: without its keyboard part, « double-clic (ou Entrée) »)
                let tip = crate::actions::sans_touches(&it.tip);
                (!tip.is_empty()).then_some((tip, t))
            }
            _ => None,
        })
    }

    /// Image du menu (taille de `rect`). `mouse` : position dans la fenêtre.
    pub fn draw(&self, text: &Text, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let local = Rect::new(0.0, 0.0, r.w, r.h);
        crate::shapes::menu_card(&mut pm, local);
        let (white, grey, dim) = (WHITE, GREY, DIM);
        for (top, line) in self.lines_at() {
            let y = top - r.y;
            let lh = self.h(line);
            if y + lh < 0.0 || y > r.h {
                continue;
            }
            match line {
                // (entre deux sections sans titre : un espace, sans trait, comme les `.pop` de la maquette)
                Line::Sep => {}
                Line::Strip(g) => {
                    // « 🚩 Voir : Tout ● ● ● » : le choix courant en pilule bleue ; titre trop long pour la largeur
                    // (cell) : sur sa propre ligne, au-dessus des pastilles ; chaque choix est une pilule de la
                    // largeur de son texte, à la ligne quand la place manque (sans trait autour, comme la maquette)
                    let grp = &self.groups[g];
                    let dessus = strip_title_above(grp, r.w);
                    let dy = if dessus { STRIP_TITLE_H } else { 0.0 };
                    let ty = if dessus { y + 17.0 } else { y + 25.0 };
                    text.draw(&mut pm, &text.fit(&grp.title, 13.0, r.w - 32.0), (16.0, ty as f32), 13.0, grey, Align::Left);
                    for (i, cx, hw, rang) in strip_slots(grp, r.w) {
                        let it = &grp.items[i];
                        let cy = y + dy + strip_cy(rang);
                        let hover = mouse.is_some_and(|(mx, my)| (mx - r.x - cx).abs() <= hw && (my - r.y - cy).abs() <= strip_row2() / 2.0);
                        match it.color {
                            Some(c) => {
                                // (📱 phone view: a dot of 26 across)
                                let base = if crate::actions::tactile() { 13.0 } else { 7.0 };
                                if it.checked {
                                    let k = base + 3.0;
                                    round_stroke(&mut pm, Rect::new(cx - k, cy - k, 2.0 * k, 2.0 * k), k, c, 1.6);
                                }
                                let rad = if hover { base + 1.0 } else { base };
                                round_fill(&mut pm, Rect::new(cx - rad, cy - rad, 2.0 * rad, 2.0 * rad), rad, c, 1.0);
                            }
                            None => {
                                crate::shapes::pilule(&mut pm, Rect::new(cx - hw, cy - 12.0, 2.0 * hw, 24.0), it.checked, hover);
                                text.draw(&mut pm, &it.label, (cx as f32, (cy + 4.5) as f32), STRIP_FONT, white, Align::Center);
                            }
                        }
                    }
                }
                Line::Group(g) => {
                    let title = &self.groups[g].title;
                    text.draw(&mut pm, title, (16.0, (y + 22.0) as f32), 11.5, grey, Align::Left);
                    let up: [(f32, f32); 3] = [(-4.0, 2.0), (0.0, -2.0), (4.0, 2.0)];
                    let down: [(f32, f32); 3] = [(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)];
                    let chev = if self.collapsed.contains(title) { down } else { up };
                    lines(&mut pm, r.w - 26.0, y + HEAD / 2.0, &[&chev], grey, 1.4);
                }
                Line::Item(g, i) => {
                    let it = &self.groups[g].items[i];
                    let row = Rect::new(6.0, y + 1.0, r.w - 12.0, self.ligne - 2.0);
                    let hover = mouse.is_some_and(|(mx, my)| row.contains(mx - r.x, my - r.y));
                    if self.gris && it.checked {
                        round_fill(&mut pm, row, row.h / 2.0, crate::zoompad::CHOISI.0, crate::zoompad::CHOISI.1);
                    } else {
                        crate::shapes::menu_row(&mut pm, row, it.checked, hover && it.enabled);
                    }
                    let col = match (it.enabled && !it.dim, self.gris) {
                        (true, true) => crate::zoompad::ENCRE,
                        (true, false) => white,
                        (false, _) => dim,
                    };
                    let lx = if let Some(ic) = it.icon {
                        crate::icons::draw(&mut pm, ic, 26.0, y + self.ligne / 2.0, 20, col);
                        46.0
                    } else if let Some(c) = it.color {
                        round_fill(&mut pm, Rect::new(18.0, y + self.ligne / 2.0 - 8.0, 16.0, 16.0), 8.0, c, 1.0);
                        46.0
                    } else {
                        16.0
                    };
                    // 📱 menu plus étroit que le texte (cell) : coupé avec « … » plutôt que tranché au bord
                    let reste = r.w
                        - lx
                        - if it.star.is_some() || it.remove { 44.0 } else { 14.0 }
                        - if indice(&it.hint).is_empty() {
                            0.0
                        } else {
                            text.width(indice(&it.hint), 12.0) as f64 + 20.0
                        };
                    let label = text.fit(&it.label, 14.5, reste);
                    text.draw(&mut pm, &label, (lx as f32, (y + self.ligne / 2.0 + 6.0) as f32), 14.5, col, Align::Left);
                    if !it.tip.is_empty() {
                        let qx = lx + text.width(&it.label, 14.5) as f64 + 14.0;
                        round_fill(&mut pm, Rect::new(qx - 8.0, y + self.ligne / 2.0 - 8.0, 16.0, 16.0), 8.0, pal::on(), 1.0);
                        text.draw(&mut pm, "?", (qx as f32, (y + self.ligne / 2.0 + 4.5) as f32), 11.5, ICON, Align::Center);
                    }
                    let right = if it.star.is_some() || it.remove { r.w - 44.0 } else { r.w - 14.0 };
                    if !indice(&it.hint).is_empty() {
                        text.draw(
                            &mut pm,
                            indice(&it.hint),
                            (right as f32, (y + self.ligne / 2.0 + 6.0) as f32),
                            12.0,
                            grey,
                            Align::Right,
                        );
                    }
                    if let Some(on) = it.star {
                        star(&mut pm, r.w - 26.0, y + self.ligne / 2.0, on);
                    }
                    if it.remove {
                        lines(
                            &mut pm,
                            r.w - 26.0,
                            y + self.ligne / 2.0,
                            &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]],
                            grey,
                            1.5,
                        );
                    }
                }
            }
        }
        // indicateur de défilement
        let max = self.max_scroll();
        if max > 0.0 {
            let th = (r.h * r.h / (self.content_h() + 2.0 * PAD)).max(24.0);
            let ty = self.scroll / max * (r.h - th);
            round_fill(&mut pm, Rect::new(r.w - 6.0, ty + 2.0, 4.0, th - 4.0), 2.0, GREY, 0.5);
        }
        Some(pm)
    }
}

/// Étoile : pleine (favori) ou en contour.
pub fn star(pm: &mut Pixmap, cx: f64, cy: f64, full: bool) {
    let pts: Vec<(f32, f32)> = (0..=10)
        .map(|k| {
            let r = if k % 2 == 0 { 8.0f32 } else { 3.6 };
            let a = (k as f32 * 36.0).to_radians();
            (r * a.sin(), -r * a.cos())
        })
        .collect();
    if full {
        poly_fill(pm, cx, cy, &pts, SNOW, 1.0);
    } else {
        lines(pm, cx, cy, &[&pts], GREY, 1.3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn menu() -> ListMenu {
        let item = |id: i64, label: &str, enabled: bool, checked: bool| MenuItem {
            id,
            label: label.into(),
            enabled,
            star: Some(id == 60),
            checked,
            ..Default::default()
        };
        let groups = vec![
            MenuGroup {
                title: "SECONDES".into(),
                items: vec![item(1, "1 seconde", false, false)],
                strip: false,
            },
            MenuGroup {
                title: "MINUTES".into(),
                items: vec![item(60, "1 minute", true, false), item(300, "5 minutes", true, true)],
                strip: false,
            },
        ];
        ListMenu::new(groups, vec![], Rect::new(100.0, 50.0, 40.0, 30.0), 1400.0, 900.0)
    }

    /// 👆 A menu only a touch opens (⋯ of the tablet's dock, « Accueil »): rows of 55 points (44 real px at 0.8), each
    /// touched over its whole height; the other menus keep 40.
    #[test]
    fn lignes_au_doigt_de_55() {
        let m = menu();
        let t = crate::formfactor::TOUCH_MIN_PT;
        let d = ListMenu::new_lignes(m.groups.clone(), vec![], m.anchor, 1400.0, 900.0, t);
        assert_eq!(m.ligne, ROW);
        let items: Vec<(f64, Line)> = d.lines_at().into_iter().filter(|(_, l)| matches!(l, Line::Item(..))).collect();
        assert_eq!(items.len(), 3);
        for w in items.windows(2) {
            assert!(w[1].0 - w[0].0 >= t, "{w:?}");
        }
        assert_eq!(d.content_h() - m.content_h(), 3.0 * (t - ROW));
        assert!(d.draw(&Text::default(), None).is_some());
    }

    /// 📱 Cell (390) : un titre long (« FRAIS (entrée et sortie) » du menu Backtest) ne passe plus sous les pastilles :
    /// à côté si ça tient, sinon sur sa propre ligne au-dessus ; chaque pastille se touche à sa place.
    #[test]
    fn titre_long_jamais_sous_les_pastilles() {
        let t = Text::default();
        let it = |id: i64, l: &str| MenuItem {
            id,
            label: l.into(),
            enabled: true,
            ..Default::default()
        };
        let strip = |titre: &str| MenuGroup {
            title: titre.into(),
            items: (0..5).map(|k| it(50 + k, &format!("0.0{k} %"))).collect(),
            strip: true,
        };
        let groups = vec![
            MenuGroup {
                title: "STOP".into(),
                items: vec![it(1, "1 %")],
                strip: false,
            },
            strip("FRAIS (entrée et sortie)"),
            strip("GLISSEMENT (entrée et sortie)"),
        ];
        let mut m = ListMenu::new(groups, vec![], Rect::new(8.0, 100.0, 10.0, 10.0), 390.0, 844.0);
        for g in 1..3 {
            let grp = &m.groups[g];
            let fin_titre = 16.0 + t.width(&grp.title, 13.0) as f64;
            let slots = strip_slots(grp, m.rect.w);
            let premiere = slots[0].1 - slots[0].2;
            assert!(
                slots.iter().all(|(_, cx, hw, _)| cx + hw <= m.rect.w - 8.0),
                "les pastilles tiennent dans le menu"
            );
            assert!(
                strip_title_above(grp, m.rect.w) || premiere >= fin_titre,
                "{} : pas de chevauchement",
                grp.title
            );
            // chaque pastille se touche à sa place
            let (top, _) = m.lines_at().into_iter().find(|(_, l)| *l == Line::Strip(g)).unwrap();
            let dy = if strip_title_above(grp, m.rect.w) { STRIP_TITLE_H } else { 0.0 };
            for (i, cx, _, rang) in slots {
                let id = m.groups[g].items[i].id;
                assert_eq!(m.click(m.rect.x + cx, top + dy + strip_cy(rang)), MenuClick::Pick(id));
            }
            if strip_title_above(&m.groups[g], m.rect.w) {
                assert_eq!(m.click(m.rect.x + 30.0, top + 8.0), MenuClick::Inside, "sur le titre : rien");
            }
        }
        // un titre court reste à gauche des pastilles, sur une seule rangée (comme avant)
        let court = MenuGroup {
            title: "🚩 Voir :".into(),
            items: vec![it(9, "Tout")],
            strip: true,
        };
        assert!(!strip_title_above(&court, 300.0));
        assert!(m.draw(&t, None).is_some());
    }
    /// 🔔 « DÉCLENCHEMENT » et « VALIDITÉ » de la Nouvelle alerte (bug de Jee, 16 nov.) : chaque choix est une pilule de
    /// la largeur de son texte, espacées, à la ligne si la place manque ; aucun texte ne se chevauche (au pixel : les
    /// pilules ne se touchent pas), la 1re ne colle pas au titre, chacune se clique à sa place ; à 3 largeurs.
    #[test]
    fn choix_en_pilules_sans_chevauchement() {
        let t = Text::default();
        let it = |id: i64, l: &str, on: bool| MenuItem {
            id,
            label: l.into(),
            enabled: true,
            checked: on,
            ..Default::default()
        };
        let groups = || {
            vec![
                MenuGroup {
                    title: "NOUVELLE ALERTE".into(),
                    items: vec![it(1, "Alerte de prix à 76 000,00 (BTCUSDT)", false)],
                    strip: false,
                },
                MenuGroup {
                    title: "DÉCLENCHEMENT".into(),
                    items: ["Une fois", "À chaque fois", "15 min", "1 h"]
                        .iter()
                        .enumerate()
                        .map(|(k, l)| it(30 + k as i64, l, k == 0))
                        .collect(),
                    strip: true,
                },
                MenuGroup {
                    title: "VALIDITÉ".into(),
                    items: ["Sans fin", "24 h", "7 jours"]
                        .iter()
                        .enumerate()
                        .map(|(k, l)| it(40 + k as i64, l, k == 0))
                        .collect(),
                    strip: true,
                },
            ]
        };
        for win_w in [300.0, 390.0, 1440.0] {
            let mut m = ListMenu::new(groups(), vec![], Rect::new(4.0, 40.0, 10.0, 10.0), win_w, 900.0);
            for g in 1..3 {
                let grp = m.groups[g].clone();
                let slots = strip_slots(&grp, m.rect.w);
                let fin_titre = 16.0 + t.width(&grp.title, 13.0) as f64;
                for (k, (_, cx, hw, rang)) in slots.iter().enumerate() {
                    assert!(cx + hw <= m.rect.w - 8.0 && cx - hw >= 8.0, "{} dans le menu ({win_w})", grp.title);
                    if *rang == 0 && !strip_title_above(&grp, m.rect.w) {
                        assert!(cx - hw >= fin_titre, "{} : la 1re pilule ne colle pas au titre ({win_w})", grp.title);
                    }
                    for (_, cx2, hw2, rang2) in slots.iter().skip(k + 1) {
                        assert!(
                            rang != rang2 || (cx2 - hw2) - (cx + hw) >= STRIP_GAP - 0.01,
                            "{} : chevauchement ({win_w})",
                            grp.title
                        );
                    }
                }
                let (top, _) = m.lines_at().into_iter().find(|(_, l)| *l == Line::Strip(g)).unwrap();
                let dy = if strip_title_above(&grp, m.rect.w) { STRIP_TITLE_H } else { 0.0 };
                for (i, cx, _, rang) in slots {
                    assert_eq!(m.click(m.rect.x + cx, top + dy + strip_cy(rang)), MenuClick::Pick(grp.items[i].id), "{win_w}");
                }
            }
            assert!(m.draw(&t, None).is_some());
        }
    }

    #[test]
    fn rangee_de_pastilles() {
        let dot = |id: i64, c: u8| MenuItem {
            id,
            label: String::new(),
            enabled: true,
            color: Some(Rgb(c, 0, 0)),
            ..Default::default()
        };
        let all = MenuItem {
            id: 9,
            label: "Tout".into(),
            enabled: true,
            checked: true,
            ..Default::default()
        };
        let groups = vec![MenuGroup {
            title: "🚩 Voir :".into(),
            items: vec![all, dot(1, 200), dot(2, 100)],
            strip: true,
        }];
        let mut m = ListMenu::new(groups, vec![], Rect::new(0.0, 0.0, 10.0, 10.0), 800.0, 600.0);
        assert_eq!(m.lines(), vec![Line::Strip(0)], "une seule rangée, sans en-tête");
        let (_, top) = (0, m.rect.y + PAD);
        let slots = strip_slots(&m.groups[0], m.rect.w);
        assert_eq!(m.click(m.rect.x + slots[0].1, top + 10.0), MenuClick::Pick(9));
        assert_eq!(m.click(m.rect.x + slots[2].1, top + 10.0), MenuClick::Pick(2));
        assert_eq!(m.click(m.rect.x + 20.0, top + 10.0), MenuClick::Inside, "sur le titre");
        assert!(m.draw(&Text::default(), None).is_some());
    }

    #[test]
    fn clics_du_menu() {
        let mut m = menu();
        assert!(m.rect.y > 80.0, "sous le bouton");
        let row_y = |m: &ListMenu, k: usize| m.lines_at()[k].0 + 5.0;
        // lignes : SECONDES, 1 seconde, MINUTES, 1 minute, 5 minutes
        assert_eq!(m.click(m.rect.x + 50.0, row_y(&m, 1)), MenuClick::Inside, "1 seconde : grise");
        assert_eq!(m.click(m.rect.x + 50.0, row_y(&m, 3)), MenuClick::Pick(60));
        assert_eq!(m.click(m.rect.x + m.rect.w - 20.0, row_y(&m, 4)), MenuClick::Star(300));
        assert_eq!(m.click(5.0, 5.0), MenuClick::Outside);
        // replier SECONDES
        m.click(m.rect.x + 50.0, row_y(&m, 0));
        assert_eq!(m.collapsed, vec!["SECONDES"]);
        assert_eq!(m.lines().len(), 4);
        m.set_star(300, true);
        assert_eq!(m.groups[1].items[1].star, Some(true));
        assert!(m.draw(&Text::default(), None).is_some());
    }

    #[test]
    fn au_dessus_quand_pas_de_place() {
        let groups = vec![MenuGroup {
            title: String::new(),
            items: (0..5)
                .map(|i| MenuItem {
                    id: i,
                    label: format!("{i}"),
                    enabled: true,
                    ..Default::default()
                })
                .collect(),
            strip: false,
        }];
        let m = ListMenu::new(groups, vec![], Rect::new(1300.0, 860.0, 40.0, 30.0), 1400.0, 900.0);
        assert!(m.rect.y + m.rect.h <= 860.0, "au-dessus du bouton du bas");
        assert!(m.rect.x + m.rect.w <= 1400.0, "dans la fenêtre");
    }
}
