//! 🧰 Barre des favoris flottante sur la chart — **copie de celle d'Argos v1** (localhost) :
//! - ⋮⋮ poignée : toucher = fermer / ouvrir la barre (il reste le petit carré), glisser = la déplacer ;
//! - 🕐 unité de temps (la grille des unités), type de graphique, fx (indicateurs), ⏰ alertes ;
//! - tes outils ⭐ favoris (clic = choisir, glisser = changer l'ordre) ;
//! - un bouton par catégorie d'outils (la fenêtre de l'outil en petits carrés, avec ☆ pour les favoris) ;
//! - ⇅ barre verticale / horizontale. Clic maintenu n'importe où = déplacer la barre ; clic droit = menu de l'outil.
//!
//! Une table de zones (`layout`) sert au dessin et au clic ; la fenêtre des outils / des unités de temps (`Pop`) aussi.

use crate::shapes::{arc, lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use argos_core::interval::{DEFAULT_FAVORITES, SECTIONS};
use argos_core::Interval;
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Ce qui est gardé entre deux lancements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Coin haut-gauche de la barre, dans la chart.
    pub x: f64,
    pub y: f64,
    /// Intervalles favoris (secondes) : les ⭐ du menu des intervalles.
    pub favorites: Vec<i64>,
    /// Barre fermée : seulement la poignée (toucher la poignée = fermer / ouvrir).
    pub retracted: bool,
    /// Barre verticale (⇅).
    pub vertical: bool,
    /// Barre masquée (clic droit ▸ Masquer la barre de favoris ; clic droit sur la barre de gauche pour la revoir).
    pub hidden: bool,
    /// Dernier intervalle choisi (secondes).
    pub interval: i64,
    /// ⏱ Intervalles personnalisés ajoutés (secondes), dans le menu des intervalles.
    pub custom: Vec<i64>,
    /// Sections repliées du menu des intervalles.
    pub menu_collapsed: Vec<String>,
    /// Outils ⭐ favoris (ceux de la barre de gauche), remis à jour par la fenêtre avant chaque image.
    #[serde(skip)]
    pub favs: Vec<&'static str>,
    /// Catégories : (catégorie, outil affiché = le dernier choisi).
    #[serde(skip)]
    pub groups: Vec<(&'static str, &'static str)>,
    /// Outil choisi en ce moment (surligné).
    #[serde(skip)]
    pub active: &'static str,
    /// Icône du type de graphique choisi.
    #[serde(skip)]
    pub style_icon: &'static str,
    /// Émoji choisi pour l'outil Icône.
    #[serde(skip)]
    pub emoji: &'static str,
    /// Type de graphique choisi (index dans `CandleStyle::all()`).
    #[serde(skip)]
    pub style: usize,
    /// Indicateurs natifs affichés (cases allumées dans la fenêtre fx).
    #[serde(skip)]
    pub inds_on: Vec<crate::indicators::Ind>,
    /// Indicateurs ⭐ favoris (ceux du Marketplace) : l'étoile des cases fx.
    #[serde(skip)]
    pub ind_favs: Vec<crate::indicators::Ind>,
    /// 📱 Largeur visible de la chart (0 : pas de limite) : plus longue, la barre défile au doigt.
    #[serde(skip)]
    pub max_w: f64,
    /// 📱 Défilement de la barre (points depuis le début), borné par [`max_scroll`].
    #[serde(skip)]
    pub scroll: f64,
    /// 📱 Cell : les boutons − + RAZ Zen sont au bout de la barre (plus de rangée à part).
    #[serde(skip)]
    pub pad: bool,
    /// Mode Zen allumé (bouton Zen surligné).
    #[serde(skip)]
    pub zen: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            x: 60.0,
            y: 96.0,
            favorites: DEFAULT_FAVORITES.to_vec(),
            custom: vec![],
            retracted: false,
            vertical: false,
            hidden: false,
            interval: 1800,
            menu_collapsed: vec![],
            favs: vec![],
            groups: vec![],
            active: "cursor",
            style_icon: "candles",
            emoji: crate::emoji::DEFAULT,
            style: 0,
            inds_on: vec![],
            ind_favs: vec![],
            max_w: 0.0,
            scroll: 0.0,
            pad: false,
            zen: false,
        }
    }
}

impl Prefs {
    /// Ajoute / enlève un intervalle des favoris (gardés en ordre croissant).
    /// ⏱ Ajoute un intervalle personnalisé (1 s à 12 mois ; pas en double ; 24 au plus) et le met en favori (il
    /// apparaît dans la barre). `false` : refusé.
    pub fn add_custom(&mut self, secs: i64) -> bool {
        const MAX_S: i64 = 12 * 2_592_000;
        if !(1..=MAX_S).contains(&secs) {
            return false;
        }
        if !self.custom.contains(&secs) {
            if self.custom.len() >= 24 {
                return false;
            }
            self.custom.push(secs);
            self.custom.sort_unstable();
        }
        if !self.favorites.contains(&secs) {
            self.toggle_favorite(secs);
        }
        true
    }

    pub fn toggle_favorite(&mut self, secs: i64) -> bool {
        match self.favorites.iter().position(|s| *s == secs) {
            Some(k) => {
                self.favorites.remove(k);
                false
            }
            None => {
                self.favorites.push(secs);
                self.favorites.sort_unstable();
                true
            }
        }
    }
}

/// La catégorie « Outils » de la barre (règle, gomme, curseur), en plus des groupes de la barre de gauche.
pub const MISC: &str = "misc";
pub const MISC_TOOLS: [&str; 3] = ["measure", "erase", "cursor"];

/// Outils d'une catégorie (groupe de la barre de gauche, ou « Outils »).
pub fn group_tools(group: &str) -> Vec<&'static str> {
    if group == MISC {
        return MISC_TOOLS.to_vec();
    }
    crate::tools::RAIL
        .iter()
        .filter_map(|r| match r {
            crate::tools::RailItem::Group(g) if g.id == group => Some(g.sections.iter().flat_map(|(_, l)| l.iter().copied()).collect()),
            _ => None,
        })
        .next()
        .unwrap_or_default()
}

/// Nom d'une catégorie.
pub fn group_label(group: &str) -> &'static str {
    if group == MISC {
        "Outils"
    } else {
        crate::rail::group_label(group)
    }
}

/// Un bouton de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Handle,
    /// Intervalle favori (⭐ du menu des intervalles) — gardé de la barre Rust d'avant.
    Favorite(i64),
    Intervals,
    Style,
    Indicators,
    Alerts,
    /// Outil ⭐ favori.
    Fav(&'static str),
    /// Catégorie d'outils (sa fenêtre en petits carrés).
    Group(&'static str),
    /// ⇅ barre verticale / horizontale.
    Rotate,
    /// 📱 − + RAZ Zen (au bout de la barre sur cell) : 0 −, 1 +, 2 RAZ, 3 Zen.
    Pad(u8),
}

impl Btn {
    pub fn enabled(self) -> bool {
        true
    }
    fn tip(self, p: &Prefs) -> String {
        match self {
            Btn::Handle => if p.retracted {
                "Toucher pour ouvrir · glisser pour déplacer"
            } else {
                "Toucher pour fermer · glisser pour déplacer"
            }
            .into(),
            Btn::Favorite(s) => Interval(s).label(),
            Btn::Intervals => format!("Unités de temps ({})", Interval(p.interval).short()),
            Btn::Style => "Type de graphique".into(),
            Btn::Indicators => "Indicateurs".into(),
            Btn::Alerts => "Alertes".into(),
            Btn::Fav(t) => crate::tools::def(t).map_or(t, |d| d.label).to_string(),
            Btn::Group(g) => group_label(g).to_string(),
            Btn::Rotate => "Barre verticale / horizontale".into(),
            Btn::Pad(k) => pad_label(k).into(),
        }
    }
}

/// Épaisseur de la barre, taille d'un bouton, écart (maquette `#favbar{padding:5px;gap:3px}`, boutons de 36, l.122-124).
const H: f64 = 46.0;
const B: f64 = 36.0;
const GAP: f64 = 3.0;
const SEP: f64 = 9.0;

use crate::palette as pal;
use crate::palette::{DIM, GREY, ICON, WHITE};

/// Barre et ses boutons (coordonnées de la chart). Masquée : rien.
pub fn layout(p: &Prefs, text: &Text) -> (Rect, Vec<(Rect, Btn)>) {
    if p.hidden {
        return (Rect::new(p.x, p.y, 0.0, 0.0), vec![]);
    }
    // 📐 cell, barre repliée : la petite pilule ⋮⋮ de la maquette (mesurée : 48 × 28, poignée 36 × 16)
    if p.pad && p.retracted && !p.vertical {
        let m = crate::ecrans::cell();
        let (b, h) = (m.zone("favbar").map(|(z, _)| z.boite), m.zone("favbar.0").map(|(z, _)| z.boite));
        if let (Some(b), Some(h)) = (b, h) {
            return (
                Rect::new(p.x, p.y, b[2], b[3]),
                vec![(Rect::new(p.x + h[0] - b[0], p.y + h[1] - b[1], h[2], h[3]), Btn::Handle)],
            );
        }
    }
    let v = p.vertical;
    let mut at = 5.0; // position le long de la barre
    let side = (H - B) / 2.0;
    let mut out = vec![];
    let mut push = |len: f64, b: Btn, at: &mut f64| {
        let r = if v {
            Rect::new(p.x + side, p.y + *at, B, len)
        } else {
            Rect::new(p.x + *at, p.y + side, len, B)
        };
        out.push((r, b));
        *at += len + GAP;
    };
    push(16.0, Btn::Handle, &mut at);
    if !p.retracted {
        // intervalles favoris : barre horizontale seulement (verticale, comme v1 : 🕐 ouvre la grille)
        for s in p.favorites.iter().filter(|_| !v) {
            let w = (text.width(&Interval(*s).short(), 14.5) as f64 + 16.0).max(B);
            push(w, Btn::Favorite(*s), &mut at);
        }
        for b in [Btn::Intervals, Btn::Style, Btn::Indicators, Btn::Alerts] {
            push(B, b, &mut at);
        }
        at += SEP;
        for t in &p.favs {
            push(B, Btn::Fav(t), &mut at);
        }
        if !p.favs.is_empty() {
            at += SEP;
        }
        for (g, _) in &p.groups {
            push(B, Btn::Group(g), &mut at);
        }
        push(26.0, Btn::Rotate, &mut at);
        // 📱 − + RAZ Zen au bout (cell)
        if p.pad && !v {
            at += SEP;
            for k in 0..5u8 {
                let w = (text.width(pad_label(k), 13.0) as f64 + 18.0).max(B);
                push(w, Btn::Pad(k), &mut at);
            }
        }
    }
    let len = at + 2.0;
    // 📱 plus longue que la chart : la barre garde sa poignée et fait défiler le reste (au doigt)
    if !v && p.max_w > 0.0 && len > visible_w(p) {
        let vis = visible_w(p);
        let s = p.scroll.clamp(0.0, len - vis);
        let start = p.x + 6.0 + 16.0 + GAP;
        let out = out
            .into_iter()
            .filter_map(|(r, b)| {
                if b == Btn::Handle {
                    return Some((r, b));
                }
                let r = Rect::new(r.x - s, r.y, r.w, r.h);
                (r.x >= start - 0.5 && r.x + r.w <= p.x + vis + 0.5).then_some((r, b))
            })
            .collect();
        return (Rect::new(p.x, p.y, vis, H), out);
    }
    let bar = if v { Rect::new(p.x, p.y, H, len) } else { Rect::new(p.x, p.y, len, H) };
    (bar, out)
}

/// Bouton sous la souris (coordonnées de la chart).
pub fn hit(p: &Prefs, text: &Text, x: f64, y: f64) -> Option<Btn> {
    let (bar, btns) = layout(p, text);
    // 👆 la poignée (16 points de large) se touche au doigt : 44 points au moins autour d'elle
    if btns
        .first()
        .is_some_and(|(r, b)| *b == Btn::Handle && poignee_doigt(*r).contains(x, y) && !btns.iter().skip(1).any(|(r, _)| r.contains(x, y)))
    {
        return Some(Btn::Handle);
    }
    if !bar.contains(x, y) {
        return None;
    }
    btns.into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, b)| b)
}

/// Dans la barre (même entre deux boutons) ?
pub fn contains(p: &Prefs, text: &Text, x: f64, y: f64) -> bool {
    let (bar, btns) = layout(p, text);
    bar.w > 0.0 && (bar.contains(x, y) || btns.first().is_some_and(|(r, _)| poignee_doigt(*r).contains(x, y)))
}

/// 👆 Zone de toucher de la poignée : au moins 44 × 44 points, centrée sur elle.
fn poignee_doigt(r: Rect) -> Rect {
    let (w, h) = (r.w.max(44.0), r.h.max(44.0));
    Rect::new(r.cx() - w / 2.0, r.cy() - h / 2.0, w, h)
}

/// Rectangle d'un bouton (pour coller une fenêtre dessous).
pub fn button_rect(p: &Prefs, text: &Text, b: Btn) -> Option<Rect> {
    layout(p, text).1.into_iter().find(|(_, x)| *x == b).map(|(r, _)| r)
}

/// Garde la barre dans la chart (largeur `w`, hauteur `h`).
pub fn clamp(p: &mut Prefs, text: &Text, w: f64, h: f64) {
    let bar = layout(p, text).0;
    p.x = p.x.min(w - bar.w - 4.0).max(4.0);
    p.y = p.y.min(h - bar.h - 4.0).max(4.0);
}

/// Glisser un outil ⭐ : l'outil favori à la place de la souris (le long de la barre), pour changer l'ordre.
pub fn fav_at(p: &Prefs, text: &Text, x: f64, y: f64) -> Option<&'static str> {
    layout(p, text).1.into_iter().find_map(|(r, b)| match b {
        Btn::Fav(t) if (if p.vertical { y >= r.y && y <= r.y + r.h } else { x >= r.x && x <= r.x + r.w }) => Some(t),
        _ => None,
    })
}

/// Texte d'un bouton − + RAZ Zen.
fn pad_label(k: u8) -> &'static str {
    ["−", "+", "RAZ", "Zen", "Replay"][k.min(4) as usize]
}

/// Ce que fait un bouton − + RAZ Zen de la barre.
pub fn pad_action(k: u8) -> crate::zoompad::PadBtn {
    use crate::actions::ChartAction as A;
    use crate::zoompad::PadBtn;
    match k {
        0 => PadBtn::Action(A::ZoomOut),
        1 => PadBtn::Action(A::ZoomIn),
        2 => PadBtn::Action(A::ResetView),
        3 => PadBtn::Zen,
        _ => PadBtn::Replay,
    }
}

/// 📱 Défilement le plus loin permis (0 si la barre tient dans la largeur visible).
pub fn max_scroll(p: &Prefs, text: &Text) -> f64 {
    let full = Prefs {
        max_w: 0.0,
        scroll: 0.0,
        ..p.clone()
    };
    let len = layout(&full, text).0.w;
    if p.vertical || p.max_w <= 0.0 {
        return 0.0;
    }
    (len - visible_w(p)).max(0.0)
}

/// 📱 Bord gauche de la barre horizontale ouverte sur cell : décalée vers la gauche pour tenir entière dans la chart
/// (`max_w`), ou collée à gauche si elle est plus longue (le reste défile).
pub fn place_ouverte(p: &Prefs, text: &Text) -> f64 {
    let full = Prefs {
        max_w: 0.0,
        scroll: 0.0,
        retracted: false,
        ..p.clone()
    };
    let len = layout(&full, text).0.w;
    p.x.min(p.max_w - 4.0 - len).max(4.0)
}

/// Largeur visible de la barre horizontale (de son bord gauche au bord droit de la chart).
fn visible_w(p: &Prefs) -> f64 {
    (p.max_w - p.x - 4.0).max(120.0)
}

fn icon(pm: &mut Pixmap, text: &Text, b: Btn, r: Rect, p: &Prefs) {
    let (cx, cy) = (r.cx(), r.cy());
    let c = if b.enabled() { crate::zoompad::ENCRE } else { DIM };
    let img = |pm: &mut Pixmap, id: &str| crate::icons::draw(pm, id, cx, cy, 24, c);
    match b {
        Btn::Handle => {
            for k in 0..6 {
                let (dx, dy) = if p.vertical {
                    ((k / 2) as f64 * 5.0 - 5.0, (k % 2) as f64 * 5.0 - 2.5)
                } else {
                    ((k % 2) as f64 * 5.0 - 2.5, (k / 2) as f64 * 5.0 - 5.0)
                };
                round_fill(pm, Rect::new(cx + dx - 1.2, cy + dy - 1.2, 2.4, 2.4), 1.2, DIM, 1.0);
            }
        }
        Btn::Pad(k) => {
            text.draw(pm, pad_label(k), (cx as f32, (cy + 5.0) as f32), 13.0, c, Align::Center);
        }
        Btn::Favorite(s) => {
            text.draw(pm, &Interval(s).short(), (cx as f32, (cy + 5.0) as f32), 14.5, c, Align::Center);
        }
        Btn::Intervals => {
            let circle = arc(7.0, 0.0, 360.0, 28);
            lines(pm, cx, cy - 3.0, &[&circle, &[(0.0, -4.0), (0.0, 0.0), (3.0, 2.0)]], c, 1.4);
            text.draw(pm, &Interval(p.interval).short(), (cx as f32 + 5.0, (cy + 13.0) as f32), 9.0, c, Align::Center);
        }
        Btn::Style => img(pm, p.style_icon),
        Btn::Indicators => {
            text.draw(pm, "fx", (cx as f32, (cy + 5.0) as f32), 15.0, c, Align::Center);
        }
        Btn::Alerts => {
            let circle = arc(6.5, 0.0, 360.0, 28);
            lines(
                pm,
                cx,
                cy + 1.0,
                &[
                    &circle,
                    &[(0.0, -3.5), (0.0, 0.0), (2.5, 1.5)],
                    &[(-7.0, -6.0), (-4.0, -8.5)],
                    &[(7.0, -6.0), (4.0, -8.5)],
                ],
                c,
                1.4,
            );
        }
        Btn::Fav(t) => img(pm, t),
        Btn::Group(g) => {
            let id = p.groups.iter().find(|(k, _)| *k == g).map_or(g, |(_, i)| *i);
            img(pm, id);
            // petit triangle ◢ : la catégorie ouvre sa fenêtre d'outils
            let tri: [(f32, f32); 4] = [(16.0, 16.0), (16.0, 12.0), (12.0, 16.0), (16.0, 16.0)];
            lines(pm, cx, cy, &[&tri], DIM, 1.2);
        }
        Btn::Rotate => {
            let (a, b2) = if p.vertical {
                ([(-7.0, -3.0), (7.0, -3.0)], [(7.0, 3.0), (-7.0, 3.0)])
            } else {
                ([(-3.0, -7.0), (-3.0, 7.0)], [(3.0, 7.0), (3.0, -7.0)])
            };
            let heads: [[(f32, f32); 3]; 2] = if p.vertical {
                [[(4.0, -6.0), (7.0, -3.0), (4.0, 0.0)], [(-4.0, 0.0), (-7.0, 3.0), (-4.0, 6.0)]]
            } else {
                [[(-6.0, -4.0), (-3.0, -7.0), (0.0, -4.0)], [(0.0, 4.0), (3.0, 7.0), (6.0, 4.0)]]
            };
            lines(pm, cx, cy, &[&a, &heads[0], &b2, &heads[1]], c, 1.3);
        }
    }
}

/// Image de la barre et sa place (coordonnées de la chart). `mouse` : position dans la chart ; `lifted` : on la
/// déplace (bord bleu) ; `open` : bouton dont la fenêtre est ouverte.
pub fn draw(p: &Prefs, text: &Text, mouse: Option<(f64, f64)>, lifted: bool, open: Option<Btn>) -> Option<(Pixmap, Rect)> {
    let (bar, btns) = layout(p, text);
    if btns.is_empty() {
        return None;
    }
    let mut pm = crate::px::new(bar.w.ceil() as u32 + 1, bar.h.ceil() as u32 + 1)?;
    let local = |r: Rect| Rect::new(r.x - bar.x, r.y - bar.y, r.w, r.h);
    // 🫧 une pilule en verre qui flotte sur la chart (maquette verre) ; bord bleu quand on la déplace
    let rayon = crate::jetons::R_FAVBAR.min(bar.h / 2.0).min(bar.w / 2.0);
    round_fill(&mut pm, local(bar), rayon, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
    round_stroke(
        &mut pm,
        local(bar),
        rayon,
        if lifted {
            crate::zoompad::ENCRE
        } else {
            crate::jetons::sur_fond(crate::jetons::LISERE)
        },
        if lifted { 2.0 } else { 1.0 },
    );
    let hovered = mouse.and_then(|(x, y)| btns.iter().find(|(r, _)| r.contains(x, y)).map(|(_, b)| *b));
    for (r, b) in &btns {
        let lr = local(*r);
        let active = match b {
            Btn::Favorite(s) => *s == p.interval,
            Btn::Pad(3) => p.zen,
            Btn::Fav(t) => *t == p.active,
            Btn::Group(g) => p.active != "cursor" && !p.favs.contains(&p.active) && p.groups.iter().any(|(k, i)| k == g && *i == p.active),
            _ => false,
        } || open == Some(*b);
        if active {
            round_fill(&mut pm, lr, crate::jetons::pilule(lr), crate::zoompad::CHOISI.0, crate::zoompad::CHOISI.1);
        } else if hovered == Some(*b) && b.enabled() && *b != Btn::Handle {
            round_fill(&mut pm, lr, crate::jetons::pilule(lr), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
        }
        icon(&mut pm, text, *b, lr, p);
    }
    // séparateurs : après ⏰ (raccourcis | outils), puis après le dernier outil ⭐ (favoris | catégories)
    let last_fav = btns.iter().rev().find(|(_, b)| matches!(b, Btn::Fav(_)));
    for (r, _) in btns.iter().filter(|(_, b)| *b == Btn::Alerts).chain(last_fav) {
        let lr = local(*r);
        let line = crate::jetons::sur_fond(crate::jetons::LISERE);
        if p.vertical {
            crate::fill(&mut pm, 9.0, lr.y + lr.h + GAP / 2.0 + SEP / 2.0, H - 18.0, 1.0, line, 1.0);
        } else {
            crate::fill(&mut pm, lr.x + lr.w + GAP / 2.0 + SEP / 2.0, 9.0, 1.0, H - 18.0, line, 1.0);
        }
    }
    Some((pm, bar))
}

/// Bulle du bouton survolé : (texte, sous quel bouton). Dessinée par-dessus la chart.
pub fn tooltip(p: &Prefs, text: &Text, mouse: Option<(f64, f64)>) -> Option<(String, Rect)> {
    let (x, y) = mouse?;
    let (_, btns) = layout(p, text);
    btns.into_iter().find(|(r, _)| r.contains(x, y)).map(|(r, b)| (b.tip(p), r))
}

/// Image d'une bulle (texte d'aide sous un bouton).
pub fn tip_pixmap(text: &Text, tip: &str) -> Option<Pixmap> {
    // 📱 phone view: without its keyboard part (« Défaire (Ctrl + Z) » → « Défaire »); only keys: no tooltip
    let tip = &crate::actions::sans_touches(tip);
    if tip.is_empty() {
        return None;
    }
    // (maquette `#infobulle` : 6 × 9 de marge, 11 px, rayon 9, en verre avec son liseré)
    let w = text.width(tip, 11.5) as f64 + 18.0;
    let mut pm = crate::px::new(w.ceil() as u32, 26)?;
    let r = Rect::new(0.5, 0.5, w - 1.0, 25.0);
    round_fill(&mut pm, r, crate::jetons::pilule(r), pal::hover(), 1.0);
    round_stroke(&mut pm, r, crate::jetons::pilule(r), crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    text.draw(&mut pm, tip, (9.0, 17.0), 11.5, WHITE, Align::Left);
    Some(pm)
}

// ── fenêtre des outils d'une catégorie / des unités de temps (comme v1 : petits carrés, ☆ en coin, ✕) ─────────────

/// Ce que montre la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopKind {
    /// Unités de temps.
    Tf,
    /// Outils d'une catégorie.
    Group(&'static str),
    /// Émojis de l'outil Icône (choisir, puis poser sur la chart).
    Emoji,
    /// Types de graphique (bougies, barres, ligne…).
    Style,
    /// fx : la gamme Argos (indicateurs officiels), en cases ; la 1re case ouvre le panneau Indicateurs.
    Ind,
}

/// 🧰 La gamme Argos de la fenêtre fx : (indicateur, émoji de sa catégorie, nom court).
/// Libellé d'une case sur une ou deux lignes (coupé en mots), chaque ligne tenant dans `w` : jamais de « … » si
/// deux lignes suffisent.
pub fn two_lines(text: &Text, label: &str, size: f32, w: f64) -> (String, Option<String>) {
    if text.width(label, size) as f64 <= w {
        return (label.to_string(), None);
    }
    let words: Vec<&str> = label.split(' ').collect();
    // la coupure qui laisse la plus longue première ligne qui tient
    for k in (1..words.len()).rev() {
        let (a, b) = (words[..k].join(" "), words[k..].join(" "));
        if text.width(&a, size) as f64 <= w {
            let b = if text.width(&b, size) as f64 <= w { b } else { text.fit(&b, size, w) };
            return (a, Some(b));
        }
    }
    (text.fit(label, size, w), None)
}

/// Dessine le libellé d'une case (une ligne en bas, ou deux lignes).
fn tile_label(pm: &mut Pixmap, text: &Text, lq: Rect, label: &str) {
    match two_lines(text, label, 10.5, lq.w - 6.0) {
        (one, None) => text.draw(pm, &one, (lq.cx() as f32, (lq.y + lq.h - 10.0) as f32), 10.5, GREY, Align::Center),
        (a, Some(b)) => {
            text.draw(pm, &a, (lq.cx() as f32, (lq.y + lq.h - 19.0) as f32), 10.0, GREY, Align::Center);
            text.draw(pm, &b, (lq.cx() as f32, (lq.y + lq.h - 7.0) as f32), 10.0, GREY, Align::Center);
        }
    }
}

pub fn argos_range() -> Vec<(crate::indicators::Ind, &'static str, &'static str)> {
    crate::catalogue::officiels()
        .iter()
        .map(|(ind, cat, _, _)| {
            let label = crate::marketplace::CATS.iter().find(|(k, _)| k == cat).map_or("📈 ", |(_, l)| *l);
            let emoji = label.split(' ').next().unwrap_or("📈");
            (*ind, emoji, ind.name().strip_prefix("Argos ").unwrap_or(ind.name()))
        })
        .collect()
}

/// Résultat d'un clic dans la fenêtre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopClick {
    /// Dans la fenêtre, rien à faire.
    None,
    /// ✕ ou à côté : fermer.
    Close,
    /// Choisir cet outil (la fenêtre se ferme).
    Tool(&'static str),
    /// ☆ : ajouter / retirer des favoris (la fenêtre reste ouverte).
    Star(&'static str),
    /// Choisir cette unité de temps (secondes).
    Tf(i64),
    /// Choisir cet émoji pour l'outil Icône.
    Emoji(&'static str),
    /// Choisir ce type de graphique (index dans `CandleStyle::all()`).
    Style(usize),
    /// Ouvrir le panneau Indicateurs (1re case de fx : l'action d'avant).
    /// Ajouter / retirer cet indicateur de la chart.
    Ind(crate::indicators::Ind),
    /// ☆ d'une unité de temps : dans la barre ou pas (la fenêtre reste ouverte).
    StarTf(i64),
    /// ☆ d'un indicateur : favori du Marketplace ou pas (la fenêtre reste ouverte).
    StarInd(crate::indicators::Ind),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Zone {
    Close,
    Tile(usize),
    Star(usize),
}

/// La fenêtre ouverte, collée sous (ou au-dessus de) son bouton.
#[derive(Debug, Clone, PartialEq)]
pub struct Pop {
    pub kind: PopKind,
    pub rect: Rect,
    /// 😀 Le sélecteur de tout Twemoji (seulement pour `PopKind::Emoji`).
    pub pick: Option<crate::emojipick::EmojiPick>,
}

const TILE_W: f64 = 70.0;
/// 📊 Largeur d'une case du type de graphique (maquette : environ 84).
const STYLE_W: f64 = 84.0;
const TILE_H: f64 = 62.0;
const TF_W: f64 = 62.0;
const POP_HEAD: f64 = 44.0;
const POP_PAD: f64 = 10.0;

/// Les émojis de la fenêtre (toutes les sections à la suite).
pub fn emojis() -> Vec<&'static str> {
    crate::emoji::SECTIONS.iter().flat_map(|(_, l)| l.iter().copied()).collect()
}

/// Les unités de temps de la grille (toutes celles du menu).
fn tfs() -> Vec<i64> {
    SECTIONS.iter().flat_map(|(_, l)| l.iter().copied()).collect()
}

impl Pop {
    /// Fenêtre sous le bouton `anchor` (coordonnées de la chart), dans la chart (`w` × `h`).
    pub fn new(kind: PopKind, anchor: Rect, w: f64, h: f64) -> Self {
        if kind == PopKind::Emoji {
            let pick = crate::emojipick::EmojiPick::new(anchor, w, h);
            return Self {
                kind,
                rect: pick.rect,
                pick: Some(pick),
            };
        }
        let (cols, n, tw, th) = Self::grid(kind);
        let rows = n.div_ceil(cols);
        let pw = cols as f64 * (tw + 6.0) - 6.0 + 2.0 * POP_PAD;
        let ph = (POP_HEAD + rows as f64 * (th + 6.0) + POP_PAD - 6.0).min(h - 8.0);
        let mut y = anchor.y + anchor.h + 6.0;
        if y + ph > h - 4.0 {
            y = anchor.y - ph - 6.0;
        }
        let x = (anchor.cx() - pw / 2.0).min(w - pw - 4.0).max(4.0);
        Self {
            kind,
            rect: Rect::new(x, y.max(4.0), pw, ph),
            pick: None,
        }
    }

    /// (colonnes, cases, largeur, hauteur d'une case).
    fn grid(kind: PopKind) -> (usize, usize, f64, f64) {
        match kind {
            PopKind::Tf => (6, tfs().len(), TF_W, TILE_H),
            PopKind::Group(g) => (3, group_tools(g).len(), TILE_W, TILE_H),
            PopKind::Emoji => (8, emojis().len(), 38.0, 38.0),
            // (maquette `.tgrille` : 3 colonnes larges, le nom entier sous l'icône)
            PopKind::Style => (3, crate::CandleStyle::all().len(), STYLE_W, TILE_H),
            PopKind::Ind => (3, argos_range().len(), TILE_W, TILE_H),
        }
    }

    fn zones(&self) -> Vec<(Rect, Zone)> {
        let r = self.rect;
        let (cols, n, tw, th) = Self::grid(self.kind);
        let mut out = vec![(Rect::new(r.x + r.w - 36.0, r.y + 9.0, 26.0, 26.0), Zone::Close)];
        for i in 0..n {
            let t = Rect::new(
                r.x + POP_PAD + (i % cols) as f64 * (tw + 6.0),
                r.y + POP_HEAD + (i / cols) as f64 * (th + 6.0),
                tw,
                th,
            );
            if t.y + t.h > r.y + r.h {
                break;
            }
            out.push((t, Zone::Tile(i)));
            // l'étoile après sa case : dessinée par-dessus, et trouvée d'abord au clic (recherche à l'envers)
            if matches!(self.kind, PopKind::Group(_) | PopKind::Tf | PopKind::Ind) {
                out.push((Rect::new(t.x + t.w - 18.0, t.y + 1.0, 17.0, 17.0), Zone::Star(i)));
            }
        }
        out
    }

    /// Clic (coordonnées de la chart).
    pub fn click(&self, x: f64, y: f64) -> PopClick {
        if !self.rect.contains(x, y) {
            return PopClick::Close;
        }
        let tools = match self.kind {
            PopKind::Group(g) => group_tools(g),
            _ => vec![],
        };
        match self.zones().into_iter().rev().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) {
            Some(Zone::Close) => PopClick::Close,
            Some(Zone::Star(i)) => match self.kind {
                PopKind::Tf => tfs().get(i).map_or(PopClick::None, |s| PopClick::StarTf(*s)),
                PopKind::Ind => argos_range().get(i).map_or(PopClick::None, |(ind, _, _)| PopClick::StarInd(*ind)),
                _ => tools.get(i).map_or(PopClick::None, |t| PopClick::Star(t)),
            },
            Some(Zone::Tile(i)) => match self.kind {
                PopKind::Emoji => emojis().get(i).map_or(PopClick::None, |e| PopClick::Emoji(e)),
                PopKind::Tf => tfs().get(i).map_or(PopClick::None, |s| PopClick::Tf(*s)),
                PopKind::Group(_) => tools.get(i).map_or(PopClick::None, |t| PopClick::Tool(t)),
                PopKind::Style => PopClick::Style(i),
                PopKind::Ind => argos_range().get(i).map_or(PopClick::None, |(ind, _, _)| PopClick::Ind(*ind)),
            },
            None => PopClick::None,
        }
    }

    /// Dessin (coordonnées de la chart) : ⭐ = `p.favs`, case choisie = `p.active` / `p.interval`.
    pub fn draw(&self, text: &Text, p: &Prefs, mouse: Option<(f64, f64)>) -> Option<(Pixmap, Rect)> {
        if let Some(k) = &self.pick {
            return Some((k.draw(text, p.emoji, mouse)?, k.rect));
        }
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        // (`.pop` de la maquette : rayon du menu)
        crate::shapes::menu_card(&mut pm, loc(r));
        let title = match self.kind {
            PopKind::Tf => "UNITÉ DE TEMPS".to_string(),
            PopKind::Group(g) => group_label(g).to_uppercase(),
            PopKind::Emoji => "ICÔNES ET ÉMOJIS".to_string(),
            PopKind::Style => "TYPE DE GRAPHIQUE".to_string(),
            PopKind::Ind => "GAMME ARGOS".to_string(),
        };
        let title = text.fit(&title, 11.5, r.w - 60.0);
        text.draw(&mut pm, &title, (POP_PAD as f32 + 2.0, 27.0), 11.5, GREY, Align::Left);
        let tools = match self.kind {
            PopKind::Group(g) => group_tools(g),
            _ => vec![],
        };
        let tf = tfs();
        let emo = emojis();
        let over = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        for (q, z) in self.zones() {
            let lq = loc(q);
            match z {
                Zone::Close => {
                    // (✕ en pilule, seulement au survol, comme les autres fenêtres)
                    if over(&q) {
                        round_fill(&mut pm, lq, lq.h / 2.0, pal::hover(), 1.0);
                    }
                    lines(
                        &mut pm,
                        lq.cx(),
                        lq.cy(),
                        &[&[(-4.5, -4.5), (4.5, 4.5)], &[(4.5, -4.5), (-4.5, 4.5)]],
                        ICON,
                        1.5,
                    );
                }
                Zone::Tile(i) => {
                    let on = match self.kind {
                        PopKind::Emoji => emo.get(i) == Some(&p.emoji),
                        PopKind::Tf => tf.get(i) == Some(&p.interval),
                        PopKind::Group(_) => tools.get(i) == Some(&p.active),
                        PopKind::Style => i == p.style,
                        PopKind::Ind => argos_range().get(i).is_some_and(|(ind, _, _)| p.inds_on.contains(ind)),
                    };
                    // (maquette `.tgrille .tt` : sans cadre ; the current choice in light grey glass, hover in glass)
                    if on {
                        round_fill(&mut pm, lq, crate::jetons::pilule(lq), crate::zoompad::CHOISI.0, crate::zoompad::CHOISI.1);
                    } else if over(&q) {
                        round_fill(&mut pm, lq, crate::jetons::pilule(lq), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                    }
                    match self.kind {
                        PopKind::Emoji => {
                            if let Some(e) = emo.get(i) {
                                crate::icons::draw_emoji(&mut pm, e, lq.cx(), lq.cy(), 28);
                            }
                        }
                        PopKind::Tf => {
                            // case comme les outils : le court en gros (« 4h »), le long en petit (« 4 heures »)
                            let iv = tf.get(i).map(|s| Interval(*s));
                            let (short, long) = iv.map_or((String::new(), String::new()), |v| (v.short(), v.label()));
                            text.draw(&mut pm, &short, (lq.cx() as f32, (lq.y + 29.0) as f32), 16.0, WHITE, Align::Center);
                            text.draw(
                                &mut pm,
                                &text.fit(&long, 10.0, lq.w - 6.0),
                                (lq.cx() as f32, (lq.y + lq.h - 10.0) as f32),
                                10.0,
                                GREY,
                                Align::Center,
                            );
                        }
                        PopKind::Style => {
                            let all = crate::CandleStyle::all();
                            let Some(st) = all.get(i) else { continue };
                            let two = two_lines(text, st.name(), 10.5, lq.w - 6.0).1.is_some();
                            crate::icons::draw(&mut pm, st.icon(), lq.cx(), lq.y + if two { 17.0 } else { 22.0 }, 20, crate::zoompad::ENCRE);
                            tile_label(&mut pm, text, lq, st.name());
                        }
                        PopKind::Ind => {
                            let (emoji, label) = argos_range().get(i).map_or(("", ""), |(_, e, l)| (*e, *l));
                            let two = two_lines(text, label, 10.5, lq.w - 6.0).1.is_some();
                            crate::icons::draw(&mut pm, emoji, lq.cx(), lq.y + if two { 17.0 } else { 22.0 }, 20, crate::zoompad::ENCRE);
                            tile_label(&mut pm, text, lq, label);
                        }
                        PopKind::Group(_) => {
                            let Some(t) = tools.get(i) else { continue };
                            let label = crate::tools::def(t).map_or(*t, |d| d.label);
                            let two = two_lines(text, label, 10.5, lq.w - 6.0).1.is_some();
                            crate::icons::draw(&mut pm, t, lq.cx(), lq.y + if two { 17.0 } else { 22.0 }, 20, crate::zoompad::ENCRE);
                            tile_label(&mut pm, text, lq, label);
                        }
                    }
                }
                Zone::Star(i) => {
                    let fav = match self.kind {
                        PopKind::Tf => tf.get(i).is_some_and(|s| p.favorites.contains(s)),
                        PopKind::Ind => argos_range().get(i).is_some_and(|(ind, _, _)| p.ind_favs.contains(ind)),
                        _ => tools.get(i).is_some_and(|t| p.favs.contains(t)),
                    };
                    star(&mut pm, lq.cx(), lq.cy(), 5.5, fav);
                }
            }
        }
        Some((pm, r))
    }
}

/// Étoile ☆ (contour gris) / ★ (pleine grise : Jee, 17 Nov) de rayon `r`.
fn star(pm: &mut Pixmap, cx: f64, cy: f64, r: f32, on: bool) {
    let pts: Vec<(f32, f32)> = (0..=10)
        .map(|k| {
            let a = std::f32::consts::PI * (k as f32 / 5.0) - std::f32::consts::FRAC_PI_2;
            let rr = if k % 2 == 0 { r } else { r * 0.45 };
            (rr * a.cos(), rr * a.sin())
        })
        .collect();
    if on {
        crate::shapes::poly_fill(pm, cx, cy, &pts, crate::zoompad::ENCRE, 1.0);
    } else {
        lines(pm, cx, cy, &[&pts], DIM, 1.1);
    }
}

// ── clic droit sur la barre : menu de l'outil / de la barre d'outils (comme v1) ─────────────────────────────────

/// Lignes du menu du clic droit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    Use = 1,
    Fav,
    Ai,
    Keep,
    Magnet,
    Vertical,
    Hide,
    LockAll,
    HideDrawings,
    DeleteAll,
}

impl MenuAction {
    pub fn from_id(id: i64) -> Option<Self> {
        use MenuAction::*;
        [Use, Fav, Ai, Keep, Magnet, Vertical, Hide, LockAll, HideDrawings, DeleteAll]
            .into_iter()
            .find(|a| *a as i64 == id)
    }
}

/// Ce que le menu affiche (coche, nom de l'aimant…).
pub struct MenuState<'a> {
    /// Outil sous la souris (`None` : la barre elle-même).
    pub tool: Option<&'static str>,
    pub fav: bool,
    pub keep: bool,
    pub magnet: &'a str,
    pub vertical: bool,
    pub hidden: bool,
    pub drawings_hidden: bool,
}

/// Le menu : l'outil (Utiliser, favoris, assistant) puis la barre d'outils, comme v1.
pub fn context_menu(s: &MenuState) -> Vec<crate::listmenu::MenuGroup> {
    use crate::listmenu::{MenuGroup, MenuItem};
    let row = |a: MenuAction, icon: &'static str, label: String, checked: bool| MenuItem {
        id: a as i64,
        label,
        enabled: true,
        icon: Some(icon),
        checked,
        ..Default::default()
    };
    let mut out = vec![];
    if let Some(t) = s.tool {
        let name = crate::tools::def(t).map_or(t, |d| d.label);
        out.push(MenuGroup {
            title: name.to_uppercase(),
            items: vec![
                row(MenuAction::Use, "✏️", "Utiliser l'outil".into(), false),
                row(
                    MenuAction::Fav,
                    if s.fav { "➖" } else { "⭐" },
                    if s.fav { "Retirer des favoris" } else { "Ajouter aux favoris" }.into(),
                    false,
                ),
                row(MenuAction::Ai, crate::icons::ARGOS_A, "Assistant : comment s'en servir".into(), false),
            ],
            strip: false,
        });
    }
    out.push(MenuGroup {
        title: "BARRE D'OUTILS".into(),
        items: vec![
            row(MenuAction::Keep, "🔒", "Garder l'outil actif après chaque dessin".into(), s.keep),
            row(MenuAction::Magnet, "🧲", format!("Aimant : {} (changer)", s.magnet), false),
            row(
                MenuAction::Vertical,
                "↕️",
                if s.vertical {
                    "Barre de favoris horizontale"
                } else {
                    "Barre de favoris verticale"
                }
                .into(),
                false,
            ),
            row(
                MenuAction::Hide,
                "👁",
                if s.hidden {
                    "Afficher la barre de favoris"
                } else {
                    "Masquer la barre de favoris"
                }
                .into(),
                false,
            ),
        ],
        strip: false,
    });
    out.push(MenuGroup {
        title: String::new(),
        items: vec![
            row(MenuAction::LockAll, "🔐", "Verrouiller / déverrouiller tous les dessins".into(), false),
            row(
                MenuAction::HideDrawings,
                "🙈",
                if s.drawings_hidden { "Afficher les dessins" } else { "Masquer les dessins" }.into(),
                false,
            ),
            row(MenuAction::DeleteAll, "🗑", "Supprimer…".into(), false),
        ],
        strip: false,
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📸 La barre d'outils flottante en pilule de verre : seulement si `ARGOS_CAPTURES`.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let p = Prefs {
            favs: vec!["trend", "hline", "rect"],
            active: "trend",
            ..Default::default()
        };
        draw(&p, &Text::default(), None, false, None)
            .unwrap()
            .0
            .save_png(format!("{dir}/app-barre-outils.png"))
            .unwrap();
    }

    #[test]
    fn cell_barre_qui_defile_avec_zoom_au_bout() {
        let t = Text::default();
        let mut p = Prefs {
            x: 8.0,
            y: 500.0,
            pad: true,
            max_w: 300.0,
            ..Prefs::default()
        };
        let max = max_scroll(&p, &t);
        assert!(max > 0.0, "plus longue que la chart : elle défile");
        let (bar, btns) = layout(&p, &t);
        assert!(bar.x + bar.w <= 300.0, "coupée au bord de la chart");
        assert!(btns.iter().all(|(r, _)| r.x + r.w <= bar.x + bar.w + 0.5), "aucun bouton dehors");
        assert!(!btns.iter().any(|(_, b)| matches!(b, Btn::Pad(_))), "au début : − + RAZ Zen hors de vue");
        p.scroll = 1e9;
        let (_, btns) = layout(&p, &t);
        assert!(btns.iter().any(|(_, b)| *b == Btn::Pad(3)), "défilée au bout : Zen visible");
        assert!(btns.iter().any(|(_, b)| *b == Btn::Handle), "la poignée reste");
        assert_eq!(pad_action(3), crate::zoompad::PadBtn::Zen);
        assert!(btns.iter().any(|(_, b)| *b == Btn::Pad(4)), "⏪ Replay au bout aussi");
        assert_eq!(pad_action(4), crate::zoompad::PadBtn::Replay);
        // ordi : pas de limite, pas de boutons de zoom
        let d = Prefs::default();
        assert_eq!(max_scroll(&d, &t), 0.0);
        assert!(!layout(&d, &t).1.iter().any(|(_, b)| matches!(b, Btn::Pad(_))));
    }

    fn prefs() -> Prefs {
        Prefs {
            favs: vec!["trend", "fib"],
            groups: vec![("lines", "trend"), ("fib", "fib"), (MISC, "measure")],
            ..Prefs::default()
        }
    }

    #[test]
    fn comme_v1() {
        let t = Text::default();
        let mut p = prefs();
        let (bar, btns) = layout(&p, &t);
        let kinds: Vec<Btn> = btns.iter().map(|(_, b)| *b).collect();
        let n = p.favorites.len();
        assert_eq!(kinds[0], Btn::Handle);
        assert_eq!(kinds[1], Btn::Favorite(60), "intervalles favoris gardés, 1m en premier");
        assert_eq!(&kinds[n + 1..n + 5], &[Btn::Intervals, Btn::Style, Btn::Indicators, Btn::Alerts]);
        assert_eq!(&kinds[n + 5..n + 7], &[Btn::Fav("trend"), Btn::Fav("fib")]);
        assert_eq!(kinds.last(), Some(&Btn::Rotate));
        assert!(kinds.contains(&Btn::Group(MISC)));
        let r = button_rect(&p, &t, Btn::Fav("fib")).unwrap();
        assert_eq!(hit(&p, &t, r.cx(), r.cy()), Some(Btn::Fav("fib")));
        assert_eq!(
            fav_at(&p, &t, r.cx(), bar.y - 50.0),
            Some("fib"),
            "réordonner : seulement la position le long de la barre"
        );
        // ⇅ : verticale, mêmes boutons
        p.vertical = true;
        let (vbar, vbtns) = layout(&p, &t);
        assert!(vbar.h > vbar.w && vbtns.len() == btns.len() - n, "verticale : sans les intervalles favoris");
        // poignée touchée : il reste le petit carré
        p.retracted = true;
        assert_eq!(layout(&p, &t).1.len(), 1);
        // masquée : rien, la souris passe à travers
        p.hidden = true;
        assert!(!contains(&p, &t, p.x + 5.0, p.y + 5.0) && draw(&p, &t, None, false, None).is_none());
        // dans la chart
        let mut p = prefs();
        p.x = 5000.0;
        clamp(&mut p, &t, 1200.0, 700.0);
        assert!(p.x + layout(&p, &t).0.w <= 1200.0);
        assert!(draw(&p, &t, Some((p.x + 10.0, p.y + 10.0)), true, None).is_some());
    }

    #[test]
    fn fenetre_des_outils_et_des_unites() {
        let t = Text::default();
        let p = prefs();
        let pop = Pop::new(PopKind::Group("fib"), Rect::new(300.0, 100.0, 32.0, 32.0), 1200.0, 1400.0);
        assert!(pop.rect.y > 132.0, "sous le bouton");
        let z = pop.zones();
        let (tile, _) = z.iter().find(|(_, z)| *z == Zone::Tile(0)).unwrap();
        let (star, _) = z.iter().find(|(_, z)| *z == Zone::Star(1)).unwrap();
        assert_eq!(pop.click(tile.x + 10.0, tile.cy() + 10.0), PopClick::Tool(group_tools("fib")[0]));
        assert_eq!(pop.click(star.cx(), star.cy()), PopClick::Star(group_tools("fib")[1]));
        assert_eq!(pop.click(0.0, 0.0), PopClick::Close);
        assert!(pop.draw(&t, &p, None).is_some());
        let tf = Pop::new(PopKind::Tf, Rect::new(300.0, 600.0, 32.0, 32.0), 1200.0, 700.0);
        assert!(tf.rect.y + tf.rect.h <= 600.0, "pas de place dessous : au-dessus");
        let (t0, _) = tf.zones().into_iter().find(|(_, z)| *z == Zone::Tile(0)).unwrap();
        assert_eq!(tf.click(t0.cx(), t0.cy()), PopClick::Tf(1));
        assert_eq!(group_tools(MISC), MISC_TOOLS.to_vec());
        let em = Pop::new(PopKind::Emoji, Rect::new(300.0, 100.0, 32.0, 32.0), 1200.0, 700.0);
        let (e1, _) = em.zones().into_iter().find(|(_, z)| *z == Zone::Tile(1)).unwrap();
        assert_eq!(em.click(e1.cx(), e1.cy()), PopClick::Emoji(emojis()[1]));
    }

    #[test]
    fn menu_du_clic_droit() {
        let s = MenuState {
            tool: Some("fib"),
            fav: true,
            keep: false,
            magnet: "faible",
            vertical: false,
            hidden: false,
            drawings_hidden: false,
        };
        let m = context_menu(&s);
        assert_eq!(m.len(), 3);
        assert!(m[0].items.iter().any(|i| i.label == "Retirer des favoris"));
        let bar = context_menu(&MenuState { tool: None, ..s });
        assert_eq!(bar[0].title, "BARRE D'OUTILS");
        assert!(MenuAction::from_id(MenuAction::DeleteAll as i64).is_some());
    }
}
#[cfg(test)]
mod tests_fx {

    #[test]
    fn intervalle_personnalise() {
        let mut p = Prefs::default();
        assert!(p.add_custom(90 * 60));
        assert!(p.custom.contains(&5400) && p.favorites.contains(&5400), "ajouté et en favori (dans la barre)");
        assert!(p.add_custom(90 * 60));
        assert_eq!(p.custom.iter().filter(|s| **s == 5400).count(), 1, "pas en double");
        assert!(!p.add_custom(0) && !p.add_custom(400_000_000), "hors limites");
        for k in 1..40 {
            p.add_custom(7 * k);
        }
        assert_eq!(p.custom.len(), 24);
    }

    use super::*;
    #[test]
    /// ☆ des cases fx et unités de temps : l'étoile de la case cliquée.
    fn etoiles_fx_et_tf() {
        let pop = Pop::new(PopKind::Ind, Rect::new(300.0, 30.0, 32.0, 32.0), 1200.0, 900.0);
        // ☆ du 1er indicateur (TVD) : cliquer = StarInd
        let z = pop.zones().into_iter().find(|(_, z)| matches!(z, Zone::Star(0))).unwrap().0;
        assert_eq!(pop.click(z.cx(), z.cy()), PopClick::StarInd(crate::indicators::Ind::Tvd));
        let tf = Pop::new(PopKind::Tf, Rect::new(300.0, 30.0, 32.0, 32.0), 1200.0, 900.0);
        let z = tf.zones().into_iter().find(|(_, z)| matches!(z, Zone::Star(0))).unwrap().0;
        assert_eq!(tf.click(z.cx(), z.cy()), PopClick::StarTf(tfs()[0]));
    }

    #[test]
    /// Cases de la gamme Argos : chaque libellé tient en entier sur une ou deux lignes de la case (pas de « … »).
    fn libelles_des_cases_entiers() {
        let t = Text::default();
        for (_, _, label) in argos_range() {
            let (a, b) = two_lines(&t, label, 10.5, TILE_W - 6.0);
            let joined = match &b {
                Some(b) => format!("{a} {b}"),
                None => a.clone(),
            };
            if label.split(' ').all(|w| t.width(w, 10.5) as f64 <= TILE_W - 6.0) && label.split(' ').count() <= 2 {
                assert_eq!(joined, label, "« {label} » coupé");
            }
            assert!(t.width(&a, 10.5) as f64 <= TILE_W - 6.0);
        }
        assert_eq!(two_lines(&t, "TBO", 10.5, 64.0), ("TBO".to_string(), None));
        assert_eq!(two_lines(&t, "EMA multiples", 10.5, 64.0), ("EMA".to_string(), Some("multiples".to_string())));
    }
}
