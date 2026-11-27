//! 🧩 Modules (grand écran), **comme la colonne d'Argos v1** : une colonne de modules empilés (carnet d'ordres, trades,
//! funding, watchlist, positions démo, agenda, news, compte, marché, baleines) à gauche ou à droite de la chart, ou une
//! rangée sous la chart. Chaque module se coche, se déplace (↑ ↓) et a sa taille (Étroit, Moyen, Large, Remplir).
//!
//! Ce fichier : les réglages (gardés en JSON), la place de chaque module, le cadre de la colonne et la page de réglages
//! (Réglages ▸ 🧩 Modules, dans le panneau de droite). Le contenu de chaque module est dessiné par la fenêtre.

use crate::palette as pal;
use crate::palette::{accent, GREY, ICON, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use crate::{fill, Theme};
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Largeur de fenêtre à partir de laquelle la colonne s'affiche (comme v1).
pub const MIN_WIN_W: u32 = 1200;
/// Hauteur de l'en-tête d'un module.
pub const HEAD_H: f64 = 26.0;

/// Où va la colonne.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DockPos {
    /// À gauche de la chart (après la barre d'outils).
    Left,
    /// À droite de la chart (avant la watchlist).
    #[default]
    Right,
    /// Sous la chart (les modules côte à côte).
    Bottom,
}

/// Un module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Module {
    Book,
    Trades,
    Funding,
    Watchlist,
    Positions,
    Agenda,
    News,
    Account,
    Market,
    Whales,
    World,
    Sentiment,
    Liquidations,
    KeyStats,
    Watch,
    Scanner,
    Dex,
    Backtest,
    Wallet,
    /// 🧭 Explorer Hyperliquid and 📶 Stats Hyperliquid.
    Explorer,
    Stats,
}

impl Module {
    /// Tous, dans l'ordre de départ (celui de v1, plus les baleines).
    pub const ALL: [Module; 21] = [
        Module::Book,
        Module::Trades,
        Module::Funding,
        Module::Watchlist,
        Module::Positions,
        Module::Agenda,
        Module::News,
        Module::Account,
        Module::Market,
        Module::Whales,
        Module::World,
        Module::Sentiment,
        Module::Liquidations,
        Module::KeyStats,
        Module::Watch,
        Module::Scanner,
        Module::Dex,
        Module::Backtest,
        Module::Wallet,
        Module::Explorer,
        Module::Stats,
    ];

    /// (emoji, nom).
    pub fn face(self) -> (&'static str, &'static str) {
        match self {
            Module::Book => ("📖", "Carnet d'ordres"),
            Module::Trades => ("⚡", "Trades"),
            Module::Funding => ("💸", "Funding"),
            Module::Watchlist => ("⭐", "Watchlist"),
            Module::Positions => ("💼", "Positions démo"),
            Module::Agenda => ("📅", "Agenda économique"),
            Module::News => ("📰", "News"),
            Module::Account => ("👤", "Résumé du compte"),
            Module::Market => ("🪙", "Marché crypto (gratuit)"),
            Module::Whales => ("🐋", "Baleines"),
            Module::World => ("🌍", "Marchés macro"),
            Module::Sentiment => ("😱", "Sentiment crypto"),
            Module::Liquidations => ("💥", "Liquidations"),
            Module::KeyStats => ("🎯", "Indicateurs clés"),
            Module::Watch => ("👁", "Surveillance"),
            Module::Scanner => ("📡", "Scanner"),
            Module::Dex => ("🦎", "DEX"),
            Module::Backtest => ("🧪", "Backtest"),
            Module::Wallet => ("👛", "Wallet (lecture seule)"),
            Module::Explorer => ("🧭", "Explorer Hyperliquid"),
            Module::Stats => ("📶", "Stats Hyperliquid"),
        }
    }
}

/// Taille d'un module (hauteur dans la colonne, largeur dans la rangée du bas).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ModSize {
    Small,
    #[default]
    Medium,
    Large,
    /// Prend la place qui reste (partagée entre les modules « Remplir »).
    Fill,
}

impl ModSize {
    pub const ALL: [ModSize; 4] = [ModSize::Small, ModSize::Medium, ModSize::Large, ModSize::Fill];
    pub fn label(self) -> &'static str {
        match self {
            ModSize::Small => "Étroit",
            ModSize::Medium => "Moyen",
            ModSize::Large => "Large",
            ModSize::Fill => "Remplir",
        }
    }
    /// Taille fixe (px) dans la colonne / la rangée ; `None` pour Remplir.
    fn px(self, bottom: bool) -> Option<f64> {
        match (self, bottom) {
            (ModSize::Small, false) => Some(140.0),
            (ModSize::Medium, false) => Some(240.0),
            (ModSize::Large, false) => Some(360.0),
            (ModSize::Small, true) => Some(220.0),
            (ModSize::Medium, true) => Some(320.0),
            (ModSize::Large, true) => Some(440.0),
            (ModSize::Fill, _) => None,
        }
    }
}

/// Un module de la liste.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DockItem {
    pub module: Module,
    pub on: bool,
    pub size: ModSize,
}

/// Réglages gardés entre deux lancements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DockPrefs {
    /// Colonne affichée.
    pub on: bool,
    pub pos: DockPos,
    /// Largeur de la colonne (hauteur de la rangée du bas) : 0 à 3 = S, M, L, XL.
    pub size: usize,
    pub items: Vec<DockItem>,
}

impl Default for DockPrefs {
    fn default() -> Self {
        let on = |m: Module| matches!(m, Module::Book | Module::Trades | Module::Funding);
        let size = |m: Module| match m {
            Module::Book => ModSize::Fill,
            Module::Funding | Module::Positions | Module::Account => ModSize::Small,
            _ => ModSize::Medium,
        };
        Self {
            on: false,
            pos: DockPos::Right,
            size: 1,
            items: Module::ALL
                .iter()
                .map(|&m| DockItem {
                    module: m,
                    on: on(m),
                    size: size(m),
                })
                .collect(),
        }
    }
}

/// Tailles de la colonne (S à XL) : largeur à gauche / droite, hauteur en bas.
pub const SIZES: [&str; 4] = ["S", "M", "L", "XL"];
const COL_W: [f64; 4] = [260.0, 320.0, 400.0, 480.0];
const ROW_H: [f64; 4] = [200.0, 260.0, 320.0, 400.0];

impl DockPrefs {
    /// Modules cochés, dans l'ordre (un module qui manque dans un vieux fichier est ajouté, décoché).
    pub fn shown(&self) -> Vec<DockItem> {
        self.items.iter().copied().filter(|i| i.on).collect()
    }

    /// Complète la liste avec les modules qui manquent (nouveaux depuis le fichier) et enlève les doublons.
    pub fn fix(&mut self) {
        let mut seen = vec![];
        self.items.retain(|i| {
            let new = !seen.contains(&i.module);
            seen.push(i.module);
            new
        });
        for m in Module::ALL {
            if !seen.contains(&m) {
                self.items.push(DockItem {
                    module: m,
                    on: false,
                    size: ModSize::Medium,
                });
            }
        }
        self.size = self.size.min(3);
    }

    /// La colonne prend-elle de la place, dans une fenêtre de `win_w` px (pas sur cell ni en petit écran) ?
    pub fn active(&self, win_w: u32) -> bool {
        self.on && win_w >= MIN_WIN_W && self.items.iter().any(|i| i.on)
    }

    /// Largeur de la colonne (gauche / droite) ou hauteur de la rangée (bas).
    pub fn thickness(&self) -> f64 {
        match self.pos {
            DockPos::Bottom => ROW_H[self.size.min(3)],
            _ => COL_W[self.size.min(3)],
        }
    }

    /// Place de chaque module coché dans la colonne `w` × `h` (en-tête compris). Les tailles fixes d'abord, les
    /// « Remplir » se partagent le reste ; si tout ne tient pas, tout est réduit pareil.
    pub fn layout(&self, w: f64, h: f64) -> Vec<(Module, Rect)> {
        let items = self.shown();
        let bottom = self.pos == DockPos::Bottom;
        let total = if bottom { w } else { h };
        let gap = 4.0;
        let fixed: f64 = items.iter().filter_map(|i| i.size.px(bottom)).sum();
        let fills = items.iter().filter(|i| i.size == ModSize::Fill).count();
        let room = (total - gap * items.len().saturating_sub(1) as f64).max(0.0);
        let fill_each = if fills > 0 {
            ((room - fixed) / fills as f64).max(if bottom { 200.0 } else { 120.0 })
        } else {
            0.0
        };
        let want: Vec<f64> = items.iter().map(|i| i.size.px(bottom).unwrap_or(fill_each)).collect();
        let sum: f64 = want.iter().sum();
        let k = if sum > room && sum > 0.0 { room / sum } else { 1.0 };
        let mut at = 0.0;
        items
            .iter()
            .zip(want)
            .map(|(i, len)| {
                let len = (len * k).floor();
                let r = if bottom { Rect::new(at, 0.0, len, h) } else { Rect::new(0.0, at, w, len) };
                at += len + gap;
                (i.module, r)
            })
            .collect()
    }
}

/// 🖥 Poignée entre deux modules de la colonne PC (glisser : hauteur du module au-dessus ; double-clic : auto), comme
/// la maquette (`.dk-poig`, 6 px).
pub const POIGNEE: f64 = 6.0;
/// 🖥 Corps le plus petit d'un module : une ligne (le titre reste lisible).
pub const CORPS_MIN: f64 = KV_ROW + 8.0;
/// 🖥 Largeur de la colonne réglée en glissant son bord.
pub const LARGEUR_MIN: f64 = 240.0;
pub const LARGEUR_MAX: f64 = 560.0;

/// 🖥 Hauteur la plus petite d'un module (titre + une ligne).
pub fn hauteur_min() -> f64 {
    HEAD_H + CORPS_MIN
}

impl DockPrefs {
    /// 🖥 Colonne PC (gauche / droite) : place de chaque module coché, du haut de la colonne (sans défilement), et la
    /// hauteur totale. Rien n'est écrasé : chaque module garde sa hauteur (réglée à la main, sinon son contenu plafonné
    /// par Étroit / Moyen / Large, au moins titre + une ligne) ; « Remplir » prend la place qui reste s'il y en a,
    /// sinon sa hauteur confortable. Si le total dépasse `h`, la colonne défile.
    pub fn layout_pc(&self, w: f64, h: f64, contenu: &dyn Fn(Module) -> f64, manuel: &dyn Fn(Module) -> Option<f64>) -> (Vec<(Module, Rect)>, f64) {
        let items = self.shown();
        let min = hauteur_min();
        let auto = |i: &DockItem| {
            let c = contenu(i.module);
            // (contenu inconnu, comme le carnet qui remplit sa place : la taille choisie)
            let plafond = i.size.px(false).unwrap_or(ModSize::Medium.px(false).unwrap_or(240.0));
            if c <= 0.0 {
                plafond
            } else if i.module.liste() {
                (HEAD_H + c).min(plafond)
            } else {
                HEAD_H + c
            }
            .max(min)
        };
        let mut want: Vec<(f64, bool)> = items
            .iter()
            .map(|i| match manuel(i.module) {
                Some(m) => (m.max(min).round(), false),
                None => (auto(i).round(), i.size == ModSize::Fill),
            })
            .collect();
        let gaps = POIGNEE * items.len().saturating_sub(1) as f64;
        let sum: f64 = want.iter().map(|w| w.0).sum::<f64>() + gaps;
        let fills = want.iter().filter(|w| w.1).count();
        if fills > 0 && sum < h {
            let extra = ((h - sum) / fills as f64).floor();
            for w in want.iter_mut().filter(|w| w.1) {
                w.0 += extra;
            }
        }
        let mut at = 0.0;
        let places = items
            .iter()
            .zip(&want)
            .map(|(i, (len, _))| {
                let r = Rect::new(0.0, at, w, *len);
                at += len + POIGNEE;
                (i.module, r)
            })
            .collect::<Vec<_>>();
        (places, (at - POIGNEE).max(0.0))
    }

    /// 🖥 Déplace le module `m` à la place `vers` (parmi les modules cochés), comme les ↑ ↓ du volet Modules.
    pub fn deplacer(&mut self, m: Module, vers: usize) {
        let shown: Vec<Module> = self.shown().iter().map(|i| i.module).collect();
        let Some(de) = shown.iter().position(|&x| x == m) else { return };
        let vers = vers.min(shown.len() - 1);
        if de == vers {
            return;
        }
        // le module prend la place (dans toute la liste) du module coché qui était à `vers`
        let cible = shown[vers];
        let Some(i) = self.items.iter().position(|x| x.module == m) else { return };
        let item = self.items.remove(i);
        let j = self.items.iter().position(|x| x.module == cible).unwrap_or(self.items.len());
        let j = if de < vers { j + 1 } else { j };
        self.items.insert(j.min(self.items.len()), item);
    }
}

impl Module {
    /// 🖥 Liste qui grandit avec les données (trades, news, baleines…) : sa hauteur automatique est plafonnée par
    /// Étroit / Moyen / Large et elle défile dedans ; les fiches (funding, compte, positions…) montrent tout leur contenu.
    pub fn liste(self) -> bool {
        matches!(
            self,
            Module::Book | Module::Trades | Module::Watchlist | Module::Agenda | Module::News | Module::Whales | Module::Liquidations | Module::Scanner
        )
    }
}

/// 🖥 Barre de défilement de la colonne (fine, en verre, sur le bord extérieur) : `None` si tout tient.
pub fn barre_defil(pos: DockPos, w: f64, h: f64, total: f64, defil: f64) -> Option<Rect> {
    if total <= h + 0.5 || h <= 0.0 {
        return None;
    }
    let th = (h * h / total).max(28.0).min(h);
    let y = (defil / (total - h)).clamp(0.0, 1.0) * (h - th);
    let x = if pos == DockPos::Left { 2.0 } else { w - 7.0 };
    Some(Rect::new(x, y, 5.0, th))
}

/// 🖥 Défilement de la colonne le plus grand (le dernier module arrive en bas).
pub fn defil_max(h: f64, total: f64) -> f64 {
    (total - h).max(0.0)
}

/// 🖥 Poignée sous un module (`y` dans la colonne affichée, défilement compris) : le module au-dessus.
pub fn poignee_a(places: &[(Module, Rect)], defil: f64, y: f64) -> Option<Module> {
    places
        .iter()
        .find(|(_, r)| {
            let bas = r.y + r.h - defil;
            y >= bas - 2.0 && y <= bas + POIGNEE + 1.0
        })
        .map(|(m, _)| *m)
}

/// 🖥 ✕ de l'en-tête d'un module (place `r` du module, déjà décalée du défilement) : 24 × 24, juste avant ⋮⋮.
pub fn croix_rect(r: Rect) -> Rect {
    Rect::new(r.x + r.w - 62.0, r.y + 1.0, 24.0, 24.0)
}

/// 📐 Nom d'un module dans son en-tête (place `r` du module) : coupé (…) avant son ✕ et ⋮⋮.
pub fn titre_module(text: &Text, nom: &str, r: Rect) -> crate::entete::Entete {
    crate::entete::placer(text, nom, 12.0, (r.x + 28.0, r.y + 17.0), &[croix_rect(r)])
}

/// 🖥 ✕ d'un module sous (`x`, `y`) (colonne affichée) : ce module se décoche.
pub fn croix_a(places: &[(Module, Rect)], defil: f64, x: f64, y: f64) -> Option<Module> {
    places
        .iter()
        .find(|(_, r)| croix_rect(Rect::new(r.x, r.y - defil, r.w, r.h)).contains(x, y))
        .map(|(m, _)| *m)
}

/// 🖥 Titre d'un module sous (`x`, `y`) (colonne affichée), hors de son ✕.
pub fn titre_a(places: &[(Module, Rect)], defil: f64, x: f64, y: f64) -> Option<Module> {
    if croix_a(places, defil, x, y).is_some() {
        return None;
    }
    places
        .iter()
        .find(|(_, r)| Rect::new(r.x, r.y - defil, r.w, HEAD_H).contains(x, y))
        .map(|(m, _)| *m)
}

impl DockPrefs {
    /// 🖥 ✕ d'un module : il est décoché (comme sa case du volet Modules) ; le dernier décoché masque la colonne.
    pub fn fermer(&mut self, m: Module) {
        if let Some(i) = self.items.iter_mut().find(|i| i.module == m) {
            i.on = false;
        }
        if !self.items.iter().any(|i| i.on) {
            self.on = false;
        }
    }
}

/// 🖥 Où tombe un module glissé jusqu'à `y` (colonne affichée) : sa nouvelle place parmi les modules cochés, et la hauteur
/// de la ligne qui la montre (entre deux modules).
pub fn depot(places: &[(Module, Rect)], de: Module, defil: f64, y: f64) -> (usize, f64) {
    let autres: Vec<&(Module, Rect)> = places.iter().filter(|(m, _)| *m != de).collect();
    let i = autres.iter().filter(|(_, r)| r.y - defil + r.h / 2.0 < y).count();
    let ligne = if i == 0 {
        autres.first().map_or(0.0, |(_, r)| r.y - defil)
    } else {
        let (_, r) = autres[i - 1];
        r.y - defil + r.h + POIGNEE / 2.0
    };
    (i, ligne)
}

/// 🖥 Cadre de la colonne PC (`w` × `h`) d'après sa mise en page (`places`, `total`) et son défilement : fond, chaque
/// module visible avec son titre (⋮⋮ : il se glisse), la poignée sous chaque module, la barre de défilement ; pendant
/// un glisser de titre, le module glissé estompé et la ligne cyan où il tombera. Renvoie l'image et la zone de contenu
/// de chaque module visible (dans la colonne affichée).
#[allow(clippy::too_many_arguments)]
pub fn frame_pc(
    p: &DockPrefs,
    text: &Text,
    (w, h): (u32, u32),
    (places, total, defil): (&[(Module, Rect)], f64, f64),
    mouse: Option<(f64, f64)>,
    glisse: Option<(Module, f64)>,
    th: &Theme,
) -> Option<(Pixmap, Vec<(Module, Rect)>)> {
    let (wf, hf) = (w as f64, h as f64);
    let mut pm = crate::px::new(w.max(1), h.max(1))?;
    fill(&mut pm, 0.0, 0.0, wf, hf, th.chrome, 1.0);
    let survol_poignee = mouse.and_then(|(_, y)| poignee_a(places, defil, y));
    let mut bodies = vec![];
    for (m, r) in places {
        let r = Rect::new(r.x, r.y - defil, r.w, r.h);
        if r.y + r.h + POIGNEE < 0.0 || r.y > hf {
            continue;
        }
        fill(&mut pm, r.x, r.y, r.w, r.h, th.background, 1.0);
        let head = Rect::new(r.x, r.y, r.w, HEAD_H);
        let over = mouse.is_some_and(|(x, y)| head.contains(x, y));
        fill(&mut pm, head.x, head.y, head.w, head.h, if over { pal::hover() } else { pal::panel() }, 1.0);
        fill(&mut pm, head.x, head.y + head.h - 1.0, head.w, 1.0, pal::edge_soft(), 1.0);
        let (ic, name) = m.face();
        crate::icons::draw(&mut pm, ic, r.x + 15.0, r.y + HEAD_H / 2.0, 15, ICON);
        let x = croix_rect(r);
        text.draw(
            &mut pm,
            &titre_module(text, name, r).titre,
            ((r.x + 28.0) as f32, (r.y + 17.0) as f32),
            12.0,
            WHITE,
            Align::Left,
        );
        // ✕ : le module se décoche (gris, fond discret au survol)
        if mouse.is_some_and(|(mx, my)| x.contains(mx, my)) {
            round_fill(&mut pm, x, 5.0, WHITE, 0.10);
        }
        crate::icons::draw(&mut pm, "close", x.x + x.w / 2.0, x.y + x.h / 2.0, 12, GREY);
        // ⋮⋮ : le titre se glisse pour changer l'ordre (comme la maquette)
        text.draw(&mut pm, "⋮⋮", ((r.x + r.w - 12.0) as f32, (r.y + 17.0) as f32), 11.0, GREY, Align::Right);
        bodies.push((*m, Rect::new(r.x, r.y + HEAD_H, r.w, r.h - HEAD_H)));
        // poignée : un trait fin, bleu au survol (maquette `.dk-poig`)
        let py = r.y + r.h;
        if survol_poignee == Some(*m) {
            fill(&mut pm, r.x, py, r.w, POIGNEE, accent(), 0.85);
        } else {
            fill(&mut pm, r.x + 8.0, py + 2.0, r.w - 16.0, 2.0, WHITE, 0.12);
        }
        if glisse.is_some_and(|(g, _)| g == *m) {
            fill(&mut pm, r.x, r.y, r.w, r.h, th.chrome, 0.45);
            round_stroke(&mut pm, Rect::new(r.x + 1.0, r.y + 1.0, r.w - 2.0, r.h - 2.0), 4.0, CYAN, 1.0);
        }
    }
    if let Some((_, y)) = glisse {
        let y = y.clamp(0.0, hf - 3.0);
        fill(&mut pm, 0.0, y - 4.0, wf, 9.0, CYAN, 0.18);
        fill(&mut pm, 0.0, y - 1.0, wf, 3.0, CYAN, 1.0);
    }
    if let Some(b) = barre_defil(p.pos, wf, hf, total, defil) {
        let sur = mouse.is_some_and(|(x, y)| Rect::new(b.x - 4.0, 0.0, b.w + 8.0, hf).contains(x, y));
        round_fill(&mut pm, b, b.w / 2.0, Rgb(76, 141, 255), if sur { 0.55 } else { 0.25 });
    }
    match p.pos {
        DockPos::Left => fill(&mut pm, wf - 1.0, 0.0, 1.0, hf, pal::edge(), 1.0),
        DockPos::Right => fill(&mut pm, 0.0, 0.0, 1.0, hf, pal::edge(), 1.0),
        DockPos::Bottom => fill(&mut pm, 0.0, 0.0, wf, 1.0, pal::edge(), 1.0),
    }
    Some((pm, bodies))
}

/// Cyan de la maquette (`--cyan`) : la ligne où tombe un module glissé.
const CYAN: Rgb = Rgb(56, 214, 255);

/// Dessine le cadre de la colonne (`w` × `h`) : fond, chaque module avec son en-tête ; le contenu se pose ensuite
/// dans `content(rect)`. Renvoie l'image et la zone de contenu de chaque module.
pub fn frame(p: &DockPrefs, text: &Text, w: u32, h: u32, mouse: Option<(f64, f64)>, th: &Theme) -> Option<(Pixmap, Vec<(Module, Rect)>)> {
    let mut pm = crate::px::new(w.max(1), h.max(1))?;
    fill(&mut pm, 0.0, 0.0, w as f64, h as f64, th.chrome, 1.0);
    let mut bodies = vec![];
    for (m, r) in p.layout(w as f64, h as f64) {
        if r.w < 20.0 || r.h < HEAD_H + 4.0 {
            continue;
        }
        fill(&mut pm, r.x, r.y, r.w, r.h, th.background, 1.0);
        let head = Rect::new(r.x, r.y, r.w, HEAD_H);
        let over = mouse.is_some_and(|(x, y)| head.contains(x, y));
        fill(&mut pm, head.x, head.y, head.w, head.h, if over { pal::hover() } else { pal::panel() }, 1.0);
        fill(&mut pm, head.x, head.y + head.h - 1.0, head.w, 1.0, pal::edge_soft(), 1.0);
        let (ic, name) = m.face();
        crate::icons::draw(&mut pm, ic, r.x + 15.0, r.y + HEAD_H / 2.0, 15, ICON);
        text.draw(
            &mut pm,
            &text.fit(name, 12.0, r.w - 34.0),
            ((r.x + 28.0) as f32, (r.y + 17.0) as f32),
            12.0,
            WHITE,
            Align::Left,
        );
        bodies.push((m, Rect::new(r.x, r.y + HEAD_H, r.w, r.h - HEAD_H)));
    }
    // bordure du côté de la chart
    match p.pos {
        DockPos::Left => fill(&mut pm, w as f64 - 1.0, 0.0, 1.0, h as f64, pal::edge(), 1.0),
        DockPos::Right => fill(&mut pm, 0.0, 0.0, 1.0, h as f64, pal::edge(), 1.0),
        DockPos::Bottom => fill(&mut pm, 0.0, 0.0, w as f64, 1.0, pal::edge(), 1.0),
    }
    Some((pm, bodies))
}

/// Rangée d'une liste « nom · valeur » (compte, positions, funding).
pub const KV_ROW: f64 = 24.0;

/// Liste « nom · valeur » d'un module (valeur à droite, de sa couleur si donnée) ; `empty` quand la liste est vide.
/// A value too long for its row is cut (…) and leaves the name up to 45 % of the width: never written over the name.
pub fn draw_kv(pm: &mut Pixmap, text: &Text, w: f64, rows: &[(String, String, Option<crate::text::Rgb>)], empty: &str) {
    if rows.is_empty() {
        text.draw(pm, &text.fit(empty, 12.0, w - 20.0), (10.0, 20.0), 12.0, GREY, Align::Left);
    }
    for (k, (name, value, c)) in rows.iter().enumerate() {
        let y = 6.0 + k as f64 * KV_ROW;
        let nw = (text.width(name, 12.0) as f64).min(w * 0.45);
        let value = text.fit(value, 12.5, (w - nw - 26.0).max(30.0));
        let vw = text.width(&value, 12.5) as f64;
        text.draw(
            pm,
            &text.fit(name, 12.0, (w - vw - 26.0).max(30.0)),
            (10.0, (y + 15.0) as f32),
            12.0,
            GREY,
            Align::Left,
        );
        text.draw(pm, &value, ((w - 10.0) as f32, (y + 15.0) as f32), 12.5, c.unwrap_or(WHITE), Align::Right);
    }
}

/// Rangée du carnet et des trades.
pub const LINE_H: f64 = 18.0;

/// 📖 Where the Book module's order book comes from (the only book feed is Hyperliquid's).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CarnetSource {
    /// The chart is on Hyperliquid: its own book.
    Hyperliquid,
    /// The chart is on another exchange: Hyperliquid's book of the same coin, said in a band at the top.
    MemePiece,
    /// No book for this market (the coin is not on Hyperliquid, or the market has no public book): the message.
    Indisponible(String),
    /// Books read from the exchanges chosen in the heatmap's ⚙ (« 📖 Carnets : Binance USDT · Bybit USDT 2/2 »).
    Choisis(String),
}

/// 📖 Band at the top of the Book module when the book is Hyperliquid's for a chart on another exchange.
pub const CARNET_MEME_PIECE: &str = "📖 Carnet d'Hyperliquid (même pièce)";

/// Height of that band.
const BANDE_SOURCE_H: f64 = 24.0;

/// 📖 Carnet d'ordres dans `w` × `h` : ventes en haut (la meilleure en bas), l'écart au milieu, achats dessous ;
/// barre de profondeur (quantité cumulée) derrière chaque niveau. `src` : d'où vient le carnet, dit tel quel.
#[allow(clippy::too_many_arguments)]
pub fn draw_book(pm: &mut Pixmap, text: &Text, w: f64, h: f64, book: Option<&argos_data::book::Book>, dec: usize, th: &Theme, src: &CarnetSource) {
    if let CarnetSource::Indisponible(m) = src {
        text.draw(pm, &text.fit(m, 12.0, w - 20.0), (10.0, 20.0), 12.0, GREY, Align::Left);
        return;
    }
    let bande = match src {
        CarnetSource::MemePiece => Some(CARNET_MEME_PIECE.to_string()),
        CarnetSource::Choisis(t) => Some(format!("📖 Carnets : {t}")),
        _ => None,
    };
    let y0 = if let Some(bande) = bande {
        fill(pm, 0.0, 0.0, w, BANDE_SOURCE_H - 2.0, pal::panel(), 1.0);
        text.draw(pm, &text.fit(&bande, 12.0, w - 20.0), (10.0, 16.0), 12.0, crate::palette::GOLD, Align::Left);
        BANDE_SOURCE_H
    } else {
        0.0
    };
    let Some(b) = book.filter(|b| !b.bids.is_empty() || !b.asks.is_empty()) else {
        let attente = if matches!(src, CarnetSource::Choisis(_)) {
            "Lecture des carnets…"
        } else {
            "Connexion au carnet d'Hyperliquid…"
        };
        text.draw(pm, attente, (10.0, (y0 + 20.0) as f32), 12.0, GREY, Align::Left);
        return;
    };
    let head = y0 + 20.0;
    text.draw(pm, "Prix", (10.0, (y0 + 14.0) as f32), 11.0, GREY, Align::Left);
    text.draw(pm, "Taille", ((w * 0.62) as f32, (y0 + 14.0) as f32), 11.0, GREY, Align::Right);
    text.draw(pm, "Total", ((w - 10.0) as f32, (y0 + 14.0) as f32), 11.0, GREY, Align::Right);
    let n = (((h - head - 22.0) / 2.0 / LINE_H).floor().max(1.0)) as usize;
    let cum = |l: &[argos_data::book::Level]| {
        l.iter()
            .take(n)
            .scan(0.0, |a, x| {
                *a += x.sz;
                Some(*a)
            })
            .collect::<Vec<f64>>()
    };
    let (ca, cb) = (cum(&b.asks), cum(&b.bids));
    let max = ca.last().copied().unwrap_or(0.0).max(cb.last().copied().unwrap_or(0.0)).max(1e-12);
    // one format for the whole book (« 170 500 » and « 165,000 » side by side read as the same size)
    let plus_grande = b.asks.iter().chain(&b.bids).take(2 * n).map(|l| l.sz).fold(0.0, f64::max);
    let dec_taille = decimales_taille(plus_grande);
    let size = |v: f64| crate::viewport::format_number(v, dec_taille);
    let row = |pm: &mut Pixmap, y: f64, l: &argos_data::book::Level, total: f64, c: crate::text::Rgb| {
        fill(pm, w - w * total / max, y, w * total / max, LINE_H - 1.0, c, 0.16);
        text.draw(pm, &crate::viewport::format_number(l.px, dec), (10.0, (y + 13.0) as f32), 11.5, c, Align::Left);
        text.draw(pm, &size(l.sz), ((w * 0.62) as f32, (y + 13.0) as f32), 11.5, WHITE, Align::Right);
        text.draw(pm, &size(total), ((w - 10.0) as f32, (y + 13.0) as f32), 11.5, GREY, Align::Right);
    };
    // ventes : la meilleure juste au-dessus de l'écart
    for (k, (l, total)) in b.asks.iter().zip(&ca).enumerate() {
        row(pm, head + (n - 1 - k) as f64 * LINE_H, l, *total, th.bear);
    }
    let mid = head + n as f64 * LINE_H;
    fill(pm, 0.0, mid, w, 22.0, pal::panel(), 1.0);
    let spread = b.spread().map_or("—".into(), |(s, p)| {
        format!("Écart {} · {} %", crate::viewport::format_number(s, dec), crate::viewport::format_number(p, 3))
    });
    text.draw(
        pm,
        &text.fit(&spread, 11.5, w - 20.0),
        ((w / 2.0) as f32, (mid + 15.0) as f32),
        11.5,
        WHITE,
        Align::Center,
    );
    for (k, (l, total)) in b.bids.iter().zip(&cb).enumerate() {
        row(pm, mid + 22.0 + k as f64 * LINE_H, l, *total, th.bull);
    }
}

/// 📖 The decimals of every size of a book, from its largest level: whole numbers from 1 000 (a coin worth a fraction
/// of a cent, « 170 500 »), 2 from 10, 4 below (BTC, « 0,0350 »). Thousands are grouped by the number format.
pub fn decimales_taille(plus_grande: f64) -> usize {
    if plus_grande >= 1000.0 {
        0
    } else if plus_grande >= 10.0 {
        2
    } else {
        4
    }
}

/// ● Hauteur de la ligne du bouton « Enregistrer » en haut du module ⚡ Trades.
const REC_H: f64 = 30.0;

/// ● Bouton « Enregistrer » du module ⚡ Trades (coordonnées du module, largeur `w`).
pub fn tape_rec(w: f64) -> Rect {
    Rect::new(w - 150.0, 4.0, 140.0, 22.0)
}

/// ⚡ The fallback line of the Trades module (`repli`: the exchange of the chart, without a trades flow here); the
/// exchange read instead is on the line above.
pub fn tape_repli_txt(repli: &str) -> String {
    format!("{repli} : pas de flux de trades pour cette pièce")
}

/// ⚡ Height of the fallback line.
const REPLI_H: f64 = 20.0;

/// ⚡ Trades en direct (le plus récent en haut) : prix (vert = achat, rouge = vente), taille, heure. `rec` : le marché
/// de la chart est-il ● enregistré en continu (`None` : pas de bouton, marché non branché). `source` : la bourse lue,
/// et la bourse de la chart quand c'est un repli (dit en or sur la première ligne).
#[allow(clippy::too_many_arguments)]
pub fn draw_tape(
    pm: &mut Pixmap,
    text: &Text,
    w: f64,
    trades: &[argos_data::whales::Whale],
    dec: usize,
    tz_offset_s: i64,
    rec: Option<bool>,
    (bourse, repli): (&str, Option<&str>),
    th: &Theme,
) {
    let place = rec.map_or(w - 20.0, |on| {
        let r = tape_rec(w);
        round_fill(pm, r, 7.0, if on { accent() } else { pal::field() }, if on { 0.3 } else { 1.0 });
        round_stroke(pm, r, 7.0, if on { accent() } else { pal::edge() }, 1.0);
        let label = if on { "● Enregistré en continu" } else { "○ Enregistrer" };
        text.draw(pm, label, (r.cx() as f32, (r.cy() + 4.0) as f32), 11.5, WHITE, Align::Center);
        r.x - 20.0
    });
    let nom = if repli.is_some() {
        format!("⚠ {bourse} (repli)")
    } else {
        bourse.to_string()
    };
    let couleur = if repli.is_some() { crate::palette::GOLD } else { GREY };
    text.draw(pm, &text.fit(&nom, 11.5, place), (10.0, 19.0), 11.5, couleur, Align::Left);
    let mut y0 = REC_H;
    if let Some(r) = repli {
        text.draw(
            pm,
            &text.fit(&tape_repli_txt(r), 11.5, w - 20.0),
            (10.0, (y0 + 12.0) as f32),
            11.5,
            couleur,
            Align::Left,
        );
        y0 += REPLI_H;
    }
    if trades.is_empty() {
        text.draw(
            pm,
            &format!("En attente des trades de {bourse}…"),
            (10.0, (y0 + 20.0) as f32),
            12.0,
            GREY,
            Align::Left,
        );
        return;
    }
    let ht = (y0 + 14.0) as f32;
    text.draw(pm, "Prix", (10.0, ht), 11.0, GREY, Align::Left);
    text.draw(pm, "Taille", ((w * 0.62) as f32, ht), 11.0, GREY, Align::Right);
    text.draw(pm, "Heure", ((w - 10.0) as f32, ht), 11.0, GREY, Align::Right);
    for (k, t) in trades.iter().enumerate() {
        let y = y0 + 20.0 + k as f64 * LINE_H;
        let c = if t.buy { th.bull } else { th.bear };
        if t.usd() >= 100_000.0 {
            fill(pm, 0.0, y, w, LINE_H - 1.0, c, 0.14); // gros trade surligné
        }
        let s = (t.time_ms / 1000 + tz_offset_s).rem_euclid(86_400);
        text.draw(
            pm,
            &crate::viewport::format_number(t.price, dec),
            (10.0, (y + 13.0) as f32),
            11.5,
            c,
            Align::Left,
        );
        text.draw(
            pm,
            &crate::viewport::format_number(t.size, if t.size >= 1000.0 { 0 } else { 4 }),
            ((w * 0.62) as f32, (y + 13.0) as f32),
            11.5,
            WHITE,
            Align::Right,
        );
        text.draw(
            pm,
            &format!("{:02}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60),
            ((w - 10.0) as f32, (y + 13.0) as f32),
            11.5,
            GREY,
            Align::Right,
        );
    }
}

/// Hauteur des trades (`n` trades), avec la ligne de la source (et du bouton ● Enregistrer), et celle du repli.
pub fn tape_h(n: usize, repli: bool) -> f64 {
    24.0 + n as f64 * LINE_H + REC_H + if repli { REPLI_H } else { 0.0 }
}

/// 💸 Historique du funding : une barre par paiement (vert positif, rouge négatif), à partir de `y`, hauteur `h`.
pub fn draw_funding_bars(pm: &mut Pixmap, text: &Text, w: f64, y: f64, h: f64, hist: &[argos_data::book::Funding], th: &Theme) {
    text.draw(pm, "Dernières 24 h", (10.0, (y + 12.0) as f32), 11.0, GREY, Align::Left);
    if hist.is_empty() {
        return;
    }
    let (top, bh) = (y + 18.0, h - 22.0);
    let max = hist.iter().map(|f| f.rate.abs()).fold(1e-12, f64::max);
    let zero = top + bh / 2.0;
    fill(pm, 10.0, zero, w - 20.0, 1.0, pal::edge(), 1.0);
    let bw = (w - 20.0) / hist.len() as f64;
    for (k, f) in hist.iter().enumerate() {
        let len = f.rate.abs() / max * (bh / 2.0 - 2.0);
        let x = 10.0 + k as f64 * bw;
        let c = if f.rate >= 0.0 { th.bull } else { th.bear };
        fill(
            pm,
            x + 1.0,
            if f.rate >= 0.0 { zero - len } else { zero + 1.0 },
            (bw - 2.0).max(1.0),
            len.max(1.0),
            c,
            0.9,
        );
    }
}

/// Tuile des marchés macro (hauteur) et nombre de colonnes selon la largeur.
const TILE_H: f64 = 62.0;
fn tile_cols(w: f64) -> usize {
    if w >= 420.0 {
        3
    } else if w >= 240.0 {
        2
    } else {
        1
    }
}

/// Hauteur des marchés macro (`n` tuiles).
pub fn world_h(n: usize, w: f64) -> f64 {
    n.div_ceil(tile_cols(w)).max(1) as f64 * (TILE_H + 6.0) + 10.0
}

/// 🌍 Marchés macro : une tuile par actif (nom, prix, variation du jour, courbe des 5 jours).
pub fn draw_world(pm: &mut Pixmap, text: &Text, w: f64, tiles: &[argos_data::world::Tile], note: &str, th: &Theme) {
    if tiles.is_empty() {
        let msg = if note.is_empty() {
            "Chargement des marchés (Yahoo Finance)…"
        } else {
            note
        };
        text.draw(pm, &text.fit(msg, 12.0, w - 20.0), (10.0, 20.0), 12.0, GREY, Align::Left);
        return;
    }
    let cols = tile_cols(w);
    let tw = (w - 12.0 - 6.0 * (cols - 1) as f64) / cols as f64;
    for (k, t) in tiles.iter().enumerate() {
        let (x, y) = (6.0 + (k % cols) as f64 * (tw + 6.0), 6.0 + (k / cols) as f64 * (TILE_H + 6.0));
        let r = Rect::new(x, y, tw, TILE_H);
        round_fill(pm, r, 6.0, pal::panel(), 1.0);
        let c = if t.chg_pct >= 0.0 { th.bull } else { th.bear };
        text.draw(
            pm,
            &text.fit(&t.name, 11.0, tw - 12.0),
            ((x + 8.0) as f32, (y + 15.0) as f32),
            11.0,
            GREY,
            Align::Left,
        );
        let dec = if t.price >= 1000.0 {
            0
        } else if t.price >= 10.0 {
            2
        } else {
            3
        };
        text.draw(
            pm,
            &crate::viewport::format_number(t.price, dec),
            ((x + 8.0) as f32, (y + 34.0) as f32),
            14.0,
            WHITE,
            Align::Left,
        );
        text.draw(
            pm,
            &format!("{:+.2} %", t.chg_pct),
            ((x + tw - 8.0) as f32, (y + 34.0) as f32),
            12.0,
            c,
            Align::Right,
        );
        // courbe des 5 jours
        let (lo, hi) = t.spark.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| (a.min(*v), b.max(*v)));
        if t.spark.len() > 1 && hi > lo {
            let pts: Vec<(f32, f32)> = t
                .spark
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    (
                        (x + 8.0 + i as f64 / (t.spark.len() - 1) as f64 * (tw - 16.0)) as f32,
                        (y + TILE_H - 6.0 - (v - lo) / (hi - lo) * 16.0) as f32,
                    )
                })
                .collect();
            crate::shapes::lines_alpha(pm, 0.0, 0.0, &[&pts], c, 1.2, 0.9);
        }
    }
}

/// 🎯 Indicateurs clés de la chart ouverte (lignes « nom · valeur ») : RSI 14 et son mot, SMA 20 contre EMA 50
/// (golden / death cross et depuis quand), amplitude et volatilité sur 24 h. Vide s'il manque des bougies.
pub fn key_stats(bars: &[argos_core::Bar], tf_seconds: i64) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    use argos_script::ta::{ema, rsi, sma};
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let mut out = vec![];
    let close: Vec<f64> = bars.iter().map(|b| b.close).collect();
    if let Some(r) = rsi(&close, 14).last().copied().filter(|v| v.is_finite()) {
        let (word, c) = match r {
            r if r >= 70.0 => ("surachat", down),
            r if r > 55.0 => ("fort", up),
            r if r <= 30.0 => ("survente", up),
            r if r < 45.0 => ("faible", down),
            _ => ("neutre", GREY),
        };
        out.push(("RSI 14".to_string(), format!("{r:.1} · {word}"), Some(c)));
    }
    let (s20, e50) = (sma(&close, 20), ema(&close, 50));
    let n = close.len();
    if n > 0 && s20[n - 1].is_finite() && e50[n - 1].is_finite() {
        let golden = s20[n - 1] > e50[n - 1];
        // depuis combien de bougies l'ordre SMA / EMA est le même
        let since = (1..n)
            .take_while(|&k| s20[n - 1 - k].is_finite() && e50[n - 1 - k].is_finite() && (s20[n - 1 - k] > e50[n - 1 - k]) == golden)
            .count();
        let what = if golden { "Golden cross" } else { "Death cross" };
        out.push((
            "SMA 20 / EMA 50".to_string(),
            format!("{what} · {since} b."),
            Some(if golden { up } else { down }),
        ));
    }
    // 24 h : amplitude (plus haut − plus bas) et volatilité (écart-type des rendements, ramené à 24 h)
    let per_day = (86_400 / tf_seconds.max(1)).max(1) as usize;
    if tf_seconds <= 86_400 && n > per_day {
        let day = &bars[n - per_day..];
        let (hi, lo) = day.iter().fold((f64::MIN, f64::MAX), |a, b| (a.0.max(b.high), a.1.min(b.low)));
        let last = close[n - 1];
        if last > 0.0 {
            out.push(("Amplitude 24 h".to_string(), format!("{:.2} %", (hi - lo) / last * 100.0), None));
        }
        let rets: Vec<f64> = close[n - per_day - 1..].windows(2).filter(|w| w[0] > 0.0).map(|w| (w[1] / w[0]).ln()).collect();
        if rets.len() > 1 {
            let m = rets.iter().sum::<f64>() / rets.len() as f64;
            let sd = (rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (rets.len() - 1) as f64).sqrt();
            out.push(("Volatilité 24 h".to_string(), format!("{:.2} %", sd * (rets.len() as f64).sqrt() * 100.0), None));
        }
    }
    out
}

/// Ce que la carte Surveillance sait du symbole : prix de l'exchange, prix d'il y a 24 h, volume 24 h en dollars.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Live {
    pub mark: f64,
    pub prev: f64,
    pub volume: f64,
}

/// Lignes du carnet compact de la carte Surveillance (ventes au-dessus, achats en dessous).
pub const WATCH_DEPTH: usize = 3;

/// 👁 Surveillance du symbole ouvert : prix, variation, plus haut / plus bas et volume sur 24 h, puis un carnet compact
/// (3 ventes, l'écart, 3 achats). Sans données de l'exchange : le prix et le volume viennent des bougies.
pub fn watch_rows(
    bars: &[argos_core::Bar],
    tf_seconds: i64,
    live: Option<Live>,
    book: Option<&argos_data::book::Book>,
    dec: usize,
) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let num = |v: f64| crate::viewport::format_number(v, dec);
    let big = |v: f64| {
        if v >= 1e9 {
            format!("{} G$", crate::viewport::format_number(v / 1e9, 2))
        } else if v >= 1e6 {
            format!("{} M$", crate::viewport::format_number(v / 1e6, 2))
        } else {
            format!("{} $", crate::viewport::format_number(v, 0))
        }
    };
    let mut out = vec![];
    let last = live.map(|l| l.mark).filter(|v| *v > 0.0).or(bars.last().map(|b| b.close));
    let Some(price) = last else { return out };
    out.push(("Prix".to_string(), num(price), None));
    // fenêtre de 24 h dans les bougies (unité ≤ 1 jour)
    let day = (tf_seconds > 0 && tf_seconds <= 86_400 && !bars.is_empty()).then(|| {
        let n = ((86_400 / tf_seconds) as usize).clamp(1, bars.len());
        &bars[bars.len() - n..]
    });
    let prev = live.map(|l| l.prev).filter(|v| *v > 0.0).or(day.map(|d| d[0].open));
    if let Some(p) = prev {
        let pct = (price / p - 1.0) * 100.0;
        out.push((
            "Variation 24 h".to_string(),
            argos_core::nombres::format_pct(pct),
            Some(if pct >= 0.0 { up } else { down }),
        ));
    }
    if let Some(d) = day {
        let (hi, lo) = d.iter().fold((f64::MIN, f64::MAX), |a, b| (a.0.max(b.high), a.1.min(b.low)));
        out.push(("Plus haut 24 h".to_string(), num(hi.max(price)), None));
        out.push(("Plus bas 24 h".to_string(), num(lo.min(price)), None));
    }
    let vol = live
        .map(|l| l.volume)
        .filter(|v| *v > 0.0)
        .or(day.map(|d| d.iter().map(|b| b.volume * b.close).sum()));
    if let Some(v) = vol {
        out.push(("Volume 24 h".to_string(), big(v), None));
    }
    if let Some(b) = book.filter(|b| !b.bids.is_empty() && !b.asks.is_empty()) {
        for l in b.asks.iter().take(WATCH_DEPTH).rev() {
            out.push((format!("Vente {}", num(l.px)), crate::viewport::format_number(l.sz, 3), Some(down)));
        }
        if let Some((abs, pct)) = b.spread() {
            out.push((
                "Écart".to_string(),
                format!("{} · {} %", num(abs), crate::viewport::format_number(pct, 3)),
                Some(GREY),
            ));
        }
        for l in b.bids.iter().take(WATCH_DEPTH) {
            out.push((format!("Achat {}", num(l.px)), crate::viewport::format_number(l.sz, 3), Some(up)));
        }
    }
    out
}

/// Hauteur de la carte Surveillance (avec le carnet compact).
pub fn watch_h() -> f64 {
    (6 + 2 * WATCH_DEPTH) as f64 * KV_ROW + 12.0
}

/// 📡 Une ligne du scanner double structure (Daily + 4H) : `dir` = 1 LONG, −1 SHORT, 2 à surveiller, 0 pas aligné.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanLine {
    pub coin: String,
    pub dir: i8,
    /// R:R du plan proposé (aligné seulement).
    pub rr: Option<f64>,
    /// Exchange scanné (le plus liquide) et les autres qui ont aussi ce symbole.
    pub ex: String,
    pub also: Vec<String>,
}

/// « BTC · Binance (+ OKX, Hyperliquid) » : symbole, exchange gardé, autres en petit.
pub fn venue_label(coin: &str, ex: &str, also: &[String]) -> String {
    if also.is_empty() {
        format!("{coin} · {ex}")
    } else {
        format!("{coin} · {ex} (+ {})", also.join(", "))
    }
}

/// Lignes du scanner : les alignés d'abord (LONG / SHORT), puis à surveiller, puis le reste.
pub fn scan_rows(lines: &[ScanLine]) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let rank = |d: i8| match d {
        1 | -1 => 0,
        2 => 1,
        _ => 2,
    };
    let mut v: Vec<&ScanLine> = lines.iter().collect();
    v.sort_by_key(|l| rank(l.dir));
    v.into_iter()
        .map(|l| {
            let rr = l.rr.map(|r| format!(" · R:R {r:.1}")).unwrap_or_default();
            let (word, c) = match l.dir {
                1 => (format!("▲ LONG{rr}"), Some(up)),
                -1 => (format!("▼ SHORT{rr}"), Some(down)),
                2 => ("👀 à surveiller".to_string(), Some(crate::palette::accent())),
                _ => ("pas aligné".to_string(), Some(GREY)),
            };
            (venue_label(&l.coin, &l.ex, &l.also), word, c)
        })
        .collect()
}

/// Hauteur du scanner (une ligne par symbole, et la ligne d'état).
pub fn scan_h(n: usize) -> f64 {
    (n.max(1) + 1) as f64 * KV_ROW + 12.0
}

/// 👛 Lignes du wallet en lecture seule : adresse, valeur, retirable, marge, PnL latent, positions, soldes spot,
/// ordres ouverts, derniers trades.
pub fn wallet_rows(address: &str, s: Option<&argos_data::wallet::Snapshot>, tz_offset_s: i64) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let money = |v: f64| format!("{} $", crate::viewport::format_number(v, 2));
    let col = |v: f64| Some(if v >= 0.0 { up } else { down });
    let short = if address.len() > 12 {
        format!("{}…{}", &address[..6], &address[address.len() - 4..])
    } else {
        address.to_string()
    };
    let mut out = vec![("Adresse (lecture seule)".to_string(), short, Some(GREY))];
    let Some(s) = s else { return out };
    out.push(("Valeur du compte".to_string(), money(s.value), None));
    out.push(("Retirable".to_string(), money(s.withdrawable), None));
    out.push(("Marge utilisée".to_string(), money(s.margin_used), None));
    out.push(("PnL latent".to_string(), format!("{:+.2} $", s.upnl()), col(s.upnl())));
    for p in &s.positions {
        let side = if p.size >= 0.0 { "▲" } else { "▼" };
        let liq = p.liq.map(|l| format!(" · liq. {}", crate::viewport::format_number(l, 2))).unwrap_or_default();
        out.push((
            format!(
                "{side} {} {}× · {} @ {}{liq}",
                p.coin,
                p.leverage,
                crate::viewport::format_number(p.size.abs(), 4),
                crate::viewport::format_number(p.entry, 2)
            ),
            format!("{:+.2} $", p.upnl),
            col(p.upnl),
        ));
    }
    for (coin, total, hold) in &s.balances {
        let h = if *hold > 0.0 {
            format!(" (dont {} bloqué)", crate::viewport::format_number(*hold, 4))
        } else {
            String::new()
        };
        out.push((format!("Spot {coin}{h}"), crate::viewport::format_number(*total, 4), None));
    }
    for o in &s.orders {
        out.push((
            format!("Ordre {} {} {}", if o.buy { "achat" } else { "vente" }, o.coin, o.kind),
            format!("{} @ {}", crate::viewport::format_number(o.size, 4), crate::viewport::format_number(o.px, 2)),
            Some(if o.buy { up } else { down }),
        ));
    }
    for x in s.fills.iter().take(5) {
        let t = (x.time_ms / 1000 + tz_offset_s).rem_euclid(86_400);
        out.push((
            format!("{:02}:{:02} {} {}", t / 3600, t % 3600 / 60, x.dir, x.coin),
            format!("{} @ {}", crate::viewport::format_number(x.size, 4), crate::viewport::format_number(x.px, 2)),
            Some(if x.buy { up } else { down }),
        ));
    }
    if s.positions.is_empty() && s.balances.is_empty() && s.orders.is_empty() {
        out.push(("Aucune position ni solde".to_string(), String::new(), Some(GREY)));
    }
    out
}

/// Hauteur de la carte Wallet (`n` lignes).
pub fn wallet_h(n: usize) -> f64 {
    n.max(2) as f64 * KV_ROW + 12.0
}

/// 🧪 Un backtest terminé : stratégie, réglages, bougies testées, rapport du moteur `argos_core::backtest`, et le
/// résultat d'un simple achat-conservation (buy & hold) sur la même période, et le test contre le hasard.
#[derive(Debug, Clone, PartialEq)]
pub struct BtView {
    pub name: String,
    pub settings: String,
    /// 📜 A strategy's costs (commission, slippage, pyramiding, on the close), on their own row; empty for a built-in
    /// (its costs are in `settings`).
    pub frais: String,
    pub symbol: String,
    pub bars: usize,
    pub start: f64,
    pub report: argos_core::backtest::BacktestReport,
    pub buy_hold_pct: Option<f64>,
    /// 🎲 La stratégie contre des entrées au hasard (`None` sans trade).
    pub hasard: Option<argos_core::backtest::Hasard>,
    /// 📜 A strategy written in Argos Script (`strategy(...)`): its own orders and costs, its arrows drawn by the
    /// script, no R (its stop is not known when it enters).
    pub script: bool,
    /// 📊 What the full report page needs (lot 6b).
    pub detail: BtDetail,
    /// 📜 A strategy's position still open at the last bar: its result ($), not counted in the net (as TradingView).
    pub ouverte: Option<f64>,
}

/// 📊 The full report's data (lot 6b): the closed trades as TradingView lists them, and the account at each bar.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BtDetail {
    pub lignes: Vec<argos_script::lang::rapport::Ligne>,
    /// The account at each bar's close (a strategy script: with its open trade) and the bars' times.
    pub equite: Vec<f64>,
    pub temps: Vec<i64>,
    /// The decimals of the symbol's prices (as its bars write them).
    pub dec: usize,
    /// 🎯 A strategy's positions still open at the last bar: (side, entry time, result at the last close).
    pub en_cours: Vec<(argos_core::Side, i64, f64)>,
}

/// Derniers trades montrés dans la carte Backtest.
pub const BT_TRADES: usize = 8;

/// Résumé d'une ligne (bandeau).
pub fn bt_summary(v: &BtView) -> String {
    let r = &v.report;
    let pf = if r.profit_factor.is_finite() {
        format!("{:.2}", r.profit_factor)
    } else {
        "∞".into()
    };
    format!(
        "🧪 {} · {} · {} trades · {:+.2} % · {:.0} % gagnants · PF {pf} · DD −{:.1} %",
        v.name,
        v.symbol,
        r.trades.len(),
        r.net_pnl.to_f64() / v.start * 100.0,
        r.win_rate,
        r.max_drawdown_pct
    )
}

/// Lignes de la carte Backtest.
pub fn bt_rows(v: &BtView, tz_offset_s: i64) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let r = &v.report;
    let col = |x: f64| Some(if x >= 0.0 { up } else { down });
    let longs = r.trades.iter().filter(|t| t.side == argos_core::Side::Long).count();
    let net = r.net_pnl.to_f64() / v.start * 100.0;
    let mut out = vec![
        ("Stratégie".to_string(), v.name.clone(), None),
        (
            if v.script { "Capital · taille" } else { "Réglages" }.to_string(),
            v.settings.clone(),
            Some(GREY),
        ),
        ("Frais".to_string(), v.frais.clone(), Some(GREY)),
        ("Symbole · bougies".to_string(), format!("{} · {}", v.symbol, v.bars), None),
        (
            "Position ouverte".to_string(),
            v.ouverte.map_or_else(String::new, |x| format!("{x:+.2} $ · hors résultat")),
            v.ouverte.and_then(col),
        ),
        ("Résultat".to_string(), format!("{net:+.2} % ({:+.0} $)", r.net_pnl.to_f64()), col(net)),
        (
            "Trades".to_string(),
            format!("{} ({longs} L · {} S)", r.trades.len(), r.trades.len() - longs),
            None,
        ),
        (
            "% gagnants".to_string(),
            format!("{:.1} %", r.win_rate),
            Some(if r.win_rate >= 50.0 { up } else { down }),
        ),
        (
            "Profit factor".to_string(),
            if r.profit_factor.is_finite() {
                format!("{:.2}", r.profit_factor)
            } else {
                "∞".into()
            },
            Some(if r.profit_factor >= 1.0 { up } else { down }),
        ),
        ("Drawdown max".to_string(), format!("−{:.1} %", r.max_drawdown_pct), Some(down)),
        ("R moyen".to_string(), format!("{:+.2} R", r.avg_r), col(r.avg_r)),
        ("Courbe du solde".to_string(), sparkline(&r.equity_curve, 24), col(net)),
    ];
    if v.script {
        out.retain(|r| r.0 != "R moyen");
    }
    if v.ouverte.is_none() {
        out.retain(|r| r.0 != "Position ouverte");
    }
    if v.frais.is_empty() {
        out.retain(|r| r.0 != "Frais");
    }
    if let Some(bh) = v.buy_hold_pct {
        out.push(("Buy & hold".to_string(), format!("{bh:+.2} %"), col(bh)));
    }
    if let Some(h) = &v.hasard {
        out.push((
            "🎲 Contre le hasard".to_string(),
            // (the rank said the same thing again and was cut on a narrow card)
            format!("mieux que {:.0} % des tirages", h.battus_pct),
            Some(if h.battus_pct >= 50.0 { up } else { down }),
        ));
    }
    // (the last trades in the rows left: the card keeps its height)
    let place = (13 + BT_TRADES).saturating_sub(out.len() + 1).min(BT_TRADES);
    for t in r.trades.iter().rev().take(place) {
        let s = (t.opened_at + tz_offset_s).rem_euclid(86_400);
        let pct = (t.exit / t.entry - 1.0) * 100.0 * if t.side == argos_core::Side::Long { 1.0 } else { -1.0 };
        out.push((
            format!(
                "{} {} {:02}:{:02} · {}",
                if t.side == argos_core::Side::Long { "▲" } else { "▼" },
                crate::civil_day(t.opened_at + tz_offset_s),
                s / 3600,
                s % 3600 / 60,
                t.reason
            ),
            argos_core::nombres::format_pct(pct),
            col(pct),
        ));
    }
    out.push((
        "Simulation".to_string(),
        if v.script {
            "comme TradingView"
        } else {
            "entrée à l'ouverture suivante · frais et glissement à l'entrée et à la sortie"
        }
        .to_string(),
        Some(GREY),
    ));
    out
}

/// Hauteur de la carte Backtest.
pub fn bt_h() -> f64 {
    (13 + BT_TRADES) as f64 * KV_ROW + 12.0
}

/// Mini-courbe en caractères (▁ à █) de `n` points au plus : la courbe du solde du backtest.
pub fn sparkline(v: &[argos_core::Money], n: usize) -> String {
    if v.is_empty() || n == 0 {
        return "—".into();
    }
    let step = (v.len() as f64 / n as f64).max(1.0);
    let pts: Vec<f64> = (0..v.len().min(n)).map(|k| v[((k as f64 * step) as usize).min(v.len() - 1)].to_f64()).collect();
    let (lo, hi) = pts.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), x| (a.min(*x), b.max(*x)));
    const BARS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    pts.iter()
        .map(|x| {
            if hi > lo {
                BARS[(((x - lo) / (hi - lo)) * 7.0).round() as usize]
            } else {
                BARS[3]
            }
        })
        .collect()
}

/// Derniers trades montrés dans la carte DEX.
pub const DEX_TRADES: usize = 10;

/// 🦎 Carte DEX de la pool affichée : prix, variation, liquidité, volume, achats / ventes et acheteurs / vendeurs
/// distincts sur 24 h (GeckoTerminal), puis les derniers trades (▲ achat, ▼ vente, montant, heure).
pub fn dex_rows(
    name: &str,
    info: Option<&argos_data::dex::PoolInfo>,
    trades: &[argos_data::dex::Trade],
    tz_offset_s: i64,
) -> Vec<(String, String, Option<crate::text::Rgb>)> {
    let (up, down) = (crate::palette::up(), crate::palette::down());
    let money = |v: Option<f64>| v.map_or("—".to_string(), crate::whales::money);
    let mut out = vec![("Paire".to_string(), name.to_string(), None)];
    if let Some(i) = info {
        let price = i.price_usd.map_or("—".to_string(), |p| crate::viewport::format_price(p, 8));
        out.push(("Prix".to_string(), format!("{price} $"), None));
        if let Some(c) = i.chg_24h {
            out.push((
                "Variation 24 h".to_string(),
                argos_core::nombres::format_pct(c),
                Some(if c >= 0.0 { up } else { down }),
            ));
        }
        out.push(("Liquidité".to_string(), money(i.liq_usd), None));
        out.push(("Volume 24 h".to_string(), money(i.vol_24h), None));
        out.push((
            "Achats / ventes 24 h".to_string(),
            format!("{} / {}", i.buys, i.sells),
            Some(if i.buys >= i.sells { up } else { down }),
        ));
        out.push((
            "Acheteurs / vendeurs".to_string(),
            format!("{} / {}", i.buyers, i.sellers),
            Some(if i.buyers >= i.sellers { up } else { down }),
        ));
        // variations courtes, volumes récents, valeur, âge (même appel, gratuit)
        let pct = |c: Option<f64>| c.map(|c| (argos_core::nombres::format_pct(c), Some(if c >= 0.0 { up } else { down })));
        for (l, c) in [("Variation 5 min", i.chg_5m), ("Variation 1 h", i.chg_1h), ("Variation 6 h", i.chg_6h)] {
            if let Some((v, col)) = pct(c) {
                out.push((l.to_string(), v, col));
            }
        }
        if i.vol_1h.is_some() || i.vol_6h.is_some() {
            out.push(("Volume 1 h / 6 h".to_string(), format!("{} / {}", money(i.vol_1h), money(i.vol_6h)), None));
        }
        if i.buys_1h + i.sells_1h > 0 {
            out.push((
                "Achats / ventes 1 h".to_string(),
                format!("{} / {}", i.buys_1h, i.sells_1h),
                Some(if i.buys_1h >= i.sells_1h { up } else { down }),
            ));
        }
        if i.mcap.is_some() {
            out.push(("Capitalisation".to_string(), money(i.mcap), None));
        }
        if i.fdv.is_some() {
            out.push(("FDV (diluée)".to_string(), money(i.fdv), None));
        }
        if let Some(c) = i.created_s {
            let now = web_time::SystemTime::now()
                .duration_since(web_time::UNIX_EPOCH)
                .map_or(0, |d| d.as_secs() as i64);
            let days = ((now - c) / 86_400).max(0);
            out.push((
                "Âge de la pool".to_string(),
                if days >= 1 { format!("{days} j") } else { "moins d'un jour".into() },
                (days < 7).then_some(down),
            ));
        }
        if let Some(l) = i.locked_pct {
            out.push(("Liquidité verrouillée".to_string(), format!("{l:.0} %"), None));
        }
        // 🛡️ fiche du jeton : signaux de risque (à vérifier soi-même, pas un conseil)
        if let Some(t) = &i.token {
            if let Some(h) = t.holders {
                out.push(("Détenteurs".to_string(), crate::viewport::format_number(h as f64, 0), None));
            }
            if let Some(p) = t.top10_pct {
                out.push(("Top 10 détenteurs".to_string(), format!("{p:.1} %"), (p > 50.0).then_some(down)));
            }
            if let Some(s) = t.gt_score {
                out.push((
                    "Score GeckoTerminal".to_string(),
                    format!("{s:.0} / 100{}", if t.verified { " ✓" } else { "" }),
                    Some(if s >= 60.0 { up } else { down }),
                ));
            }
            if t.honeypot == Some(true) {
                out.push(("⚠️ Honeypot".to_string(), "signalé : revente bloquée ?".into(), Some(down)));
            }
            if t.mint_authority || t.freeze_authority {
                let what = match (t.mint_authority, t.freeze_authority) {
                    (true, true) => "création et gel actifs",
                    (true, false) => "création active",
                    _ => "gel actif",
                };
                out.push(("⚠️ Autorités".to_string(), what.into(), Some(down)));
            }
            if !t.twitter.is_empty() {
                out.push(("X".to_string(), format!("@{}", t.twitter), None));
            }
        }
    }
    for t in trades.iter().take(DEX_TRADES) {
        let s = (t.time_s + tz_offset_s).rem_euclid(86_400);
        out.push((
            format!("{} {:02}:{:02}:{:02}", if t.buy { "▲" } else { "▼" }, s / 3600, s % 3600 / 60, s % 60),
            crate::whales::money(t.usd),
            Some(if t.buy { up } else { down }),
        ));
    }
    out
}

/// Hauteur de la carte DEX.
pub fn dex_h() -> f64 {
    (7 + DEX_TRADES) as f64 * KV_ROW + 12.0
}

/// Hauteur des indicateurs clés.
pub fn key_stats_h() -> f64 {
    4.0 * KV_ROW + 12.0
}

/// Hauteur du sentiment crypto.
pub fn sentiment_h() -> f64 {
    64.0 + 7.0 * KV_ROW + 12.0
}

/// 😱 Sentiment crypto : jauge Fear & Greed, puis dominance, capitalisation, volume et saison des altcoins.
pub fn draw_sentiment(pm: &mut Pixmap, text: &Text, w: f64, m: &argos_data::market::MarketInfo, th: &Theme) {
    let Some(f) = &m.fng else {
        let msg = m.error.clone().unwrap_or_else(|| "Chargement (alternative.me, CoinGecko)…".into());
        text.draw(pm, &text.fit(&msg, 12.0, w - 20.0), (10.0, 20.0), 12.0, GREY, Align::Left);
        return;
    };
    // jauge 0 → 100 : rouge (peur) → vert (avidité)
    let bar = Rect::new(10.0, 34.0, w - 20.0, 10.0);
    let steps = 40;
    for k in 0..steps {
        let t = k as f64 / (steps - 1) as f64;
        let c = crate::text::Rgb(
            (0xef as f64 * (1.0 - t) + 0x08 as f64 * t) as u8,
            (0x44 as f64 * (1.0 - t) + 0x99 as f64 * t) as u8,
            (0x44 as f64 * (1.0 - t) + 0x81 as f64 * t) as u8,
        );
        fill(pm, bar.x + bar.w * k as f64 / steps as f64, bar.y, bar.w / steps as f64 + 1.0, bar.h, c, 0.85);
    }
    let x = bar.x + bar.w * f.value.min(100) as f64 / 100.0;
    round_fill(pm, Rect::new(x - 5.0, bar.y - 4.0, 10.0, 18.0), 3.0, WHITE, 1.0);
    let fc = if f.value >= 55 {
        th.bull
    } else if f.value <= 45 {
        th.bear
    } else {
        WHITE
    };
    text.draw(pm, &format!("Fear & Greed {} · {}", f.value, f.label), (10.0, 22.0), 13.0, fc, Align::Left);
    let mut rows = vec![];
    let ago = |v: Option<u32>| v.map_or("—".to_string(), |v| v.to_string());
    rows.push(("Hier · il y a 7 j".to_string(), format!("{} · {}", ago(f.yesterday), ago(f.week_ago)), None));
    if let Some(g) = &m.global {
        let pct = |v: Option<f64>| v.map_or("—".into(), |v| format!("{v:.1} %"));
        let big = |v: f64| format!("{} $", crate::topbar::big(v));
        rows.push(("Dominance BTC".to_string(), pct(g.btc_d), None));
        rows.push(("Dominance ETH · USDT".to_string(), format!("{} · {}", pct(g.eth_d), pct(g.usdt_d)), None));
        rows.push(("Capitalisation totale".to_string(), big(g.total), None));
        rows.push((
            "Total 2 · Total 3".to_string(),
            format!("{} · {}", g.total2().map_or("—".into(), big), g.total3().map_or("—".into(), big)),
            None,
        ));
        rows.push(("Volume 24 h".to_string(), big(g.volume), None));
    }
    if let Some(a) = m.alt_season {
        rows.push((
            "Saison des altcoins".to_string(),
            format!(
                "{a} / 100{}",
                if a >= 75 {
                    " 🚀"
                } else if a <= 25 {
                    " · saison BTC"
                } else {
                    ""
                }
            ),
            None,
        ));
    }
    if let Some(mut sub) = crate::px::new(w.max(1.0) as u32, (rows.len() as f64 * KV_ROW + 12.0) as u32) {
        draw_kv(&mut sub, text, w, &rows, "");
        crate::px::put(pm, 0.0, 56.0, &sub);
    }
}

/// Hauteur des liquidations (`n` rangées).
pub fn liqs_h(n: usize) -> f64 {
    52.0 + n as f64 * LINE_H
}

/// 💥 Liquidations : longs et shorts liquidés de la dernière heure (barre), puis chacune (la plus récente en haut) ;
/// 100 k$ et plus surlignées.
pub fn draw_liqs(pm: &mut Pixmap, text: &Text, w: f64, liqs: &[argos_data::liq::Liq], now_ms: i64, tz_offset_s: i64, th: &Theme) {
    let (l, s) = liqs
        .iter()
        .filter(|q| now_ms - q.time_ms <= 3_600_000)
        .fold((0.0, 0.0), |(l, s), q| if q.long { (l + q.usd(), s) } else { (l, s + q.usd()) });
    let money = |v: f64| crate::whales::money(v);
    text.draw(
        pm,
        &text.fit(&format!("1 h · longs liquidés {} · shorts {}", money(l), money(s)), 11.5, w - 20.0),
        (10.0, 16.0),
        11.5,
        GREY,
        Align::Left,
    );
    let bar = Rect::new(10.0, 24.0, w - 20.0, 8.0);
    round_fill(pm, bar, 4.0, pal::field(), 1.0);
    if l + s > 0.0 {
        let wl = bar.w * l / (l + s);
        fill(pm, bar.x, bar.y, wl, bar.h, th.bear, 1.0);
        fill(pm, bar.x + wl, bar.y, bar.w - wl, bar.h, th.bull, 1.0);
    }
    if liqs.is_empty() {
        text.draw(pm, "En attente des liquidations de Binance et Bybit…", (10.0, 56.0), 12.0, GREY, Align::Left);
        return;
    }
    for (k, q) in liqs.iter().enumerate() {
        let y = 40.0 + k as f64 * LINE_H;
        // longue liquidée = vente forcée (rouge) ; short liquidé = achat forcé (vert)
        let c = if q.long { th.bear } else { th.bull };
        if q.usd() >= 100_000.0 {
            fill(pm, 0.0, y, w, LINE_H - 1.0, c, 0.16);
        }
        let t = (q.time_ms / 1000 + tz_offset_s).rem_euclid(86_400);
        text.draw(pm, &q.coin, (10.0, (y + 13.0) as f32), 11.5, crate::palette::GOLD, Align::Left);
        // exchange en petit sous le symbole : B = Binance, Y = Bybit
        let cw = text.width(&q.coin, 11.5) as f64;
        let tag = if q.venue == "Bybit" { "Y" } else { "B" };
        text.draw(pm, tag, ((14.0 + cw) as f32, (y + 13.0) as f32), 9.5, GREY, Align::Left);
        text.draw(
            pm,
            if q.long { "LONG" } else { "SHORT" },
            ((10.0 + w * 0.2) as f32, (y + 13.0) as f32),
            11.0,
            c,
            Align::Left,
        );
        text.draw(pm, &money(q.usd()), ((w * 0.72) as f32, (y + 13.0) as f32), 11.5, WHITE, Align::Right);
        text.draw(
            pm,
            &format!("{:02}:{:02}:{:02}", t / 3600, t % 3600 / 60, t % 60),
            ((w - 10.0) as f32, (y + 13.0) as f32),
            11.5,
            GREY,
            Align::Right,
        );
    }
}

// ---------- page de réglages (panneau de droite) ----------

const PAD: f64 = 14.0;
const TOP: f64 = 48.0;
const CHIP_H: f64 = 28.0;
const ROW: f64 = 44.0;

/// Ce qui se touche dans la page.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    On,
    Pos(DockPos),
    Size(usize),
    Check(usize),
    ModSize(usize),
    Up(usize),
    Down(usize),
}

const POS: [(DockPos, &str); 3] = [
    (DockPos::Left, "⬅️ Gauche"),
    (DockPos::Right, "➡️ Droite"),
    (DockPos::Bottom, "⬇️ Sous la chart"),
];

/// Haut de la liste des modules, sous `n` rangées de puces (colonne et côtés) et la rangée Hauteur.
fn rows_top_de(n: usize) -> f64 {
    TOP + (n as f64 + 1.0) * (CHIP_H + 10.0) + 30.0
}

/// Au plus 3 rangées de puces en haut (volet le plus étroit : la colonne, puis les côtés passés à la ligne).
fn rows_top() -> f64 {
    rows_top_de(3)
}

/// Puces du haut : la colonne affichée / masquée puis les 3 côtés, passés à la ligne quand ils ne tiennent pas (aucun
/// n'est jamais enlevé faute de place) ; et le nombre de rangées.
fn puces_haut(p: &DockPrefs, text: &Text, pw: f64) -> (Vec<(Rect, Zone)>, usize) {
    let mut z = vec![];
    let (mut x, mut y, mut n) = (PAD, TOP, 1);
    let lab = if p.on { ON_TXT } else { OFF_TXT };
    let w = text.width(lab, 12.5) as f64 + 22.0;
    z.push((Rect::new(x, y, w, CHIP_H), Zone::On));
    x += w + 6.0;
    for (k, l) in POS {
        let w = text.width(l, 12.5) as f64 + 22.0;
        if x + w > pw - PAD && x > PAD {
            (x, y, n) = (PAD, y + CHIP_H + 10.0, n + 1);
        }
        z.push((Rect::new(x, y, w, CHIP_H), Zone::Pos(k)));
        x += w + 6.0;
    }
    (z, n)
}

/// Haut de la liste des modules pour un volet de largeur `pw`.
fn rows_top_pour(p: &DockPrefs, text: &Text, pw: f64) -> f64 {
    rows_top_de(puces_haut(p, text, pw).1)
}

/// Hauteur de la page (elle défile).
pub fn content_h(p: &DockPrefs) -> f64 {
    rows_top() + p.items.len() as f64 * ROW + 80.0
}

/// Le nom d'un module dans la liste (volet de largeur `pw`) : (décalage de la ligne, texte, taille) — une ligne de 13,5
/// s'il tient, sinon deux lignes de 12 (jamais coupé).
fn nom_module(text: &Text, nom: &str, pw: f64) -> Vec<(f64, String, f32)> {
    let place = pw - 2.0 * PAD - 196.0;
    if text.width(nom, 13.5) as f64 <= place {
        return vec![(18.0, nom.to_string(), 13.5)];
    }
    let lignes = crate::assistant::wrap(text, nom, 12.0, place);
    let (a, b) = (
        lignes.first().map_or(String::new(), |l| l.trim().to_string()),
        lignes[1.min(lignes.len())..].join(" ").trim().to_string(),
    );
    if b.is_empty() {
        return vec![(18.0, a, 12.0)];
    }
    vec![(12.0, a, 12.0), (26.0, text.fit(&b, 12.0, place), 12.0)]
}

/// Le bouton de la colonne : son état, en bascule (« Masquée » seul se lisait comme un choix à côté de Gauche / Droite).
const ON_TXT: &str = "👁 Colonne affichée";
const OFF_TXT: &str = "🚫 Colonne masquée";

fn zones(p: &DockPrefs, text: &Text, pw: f64) -> Vec<(Rect, Zone)> {
    let (mut z, n) = puces_haut(p, text, pw);
    let haut_h = TOP + n as f64 * (CHIP_H + 10.0);
    let mut x = PAD + text.width("Hauteur", 12.0) as f64 + 12.0;
    for k in 0..SIZES.len() {
        z.push((Rect::new(x, haut_h, 36.0, CHIP_H), Zone::Size(k)));
        x += 42.0;
    }
    let top = rows_top_de(n);
    for i in 0..p.items.len() {
        let y = top + i as f64 * ROW;
        z.push((Rect::new(PAD, y + 8.0, 26.0, 26.0), Zone::Check(i)));
        // (plus serrés : le nom du module se lit en entier)
        z.push((Rect::new(pw - PAD - 130.0, y + 7.0, 74.0, 28.0), Zone::ModSize(i)));
        z.push((Rect::new(pw - PAD - 54.0, y + 7.0, 26.0, 28.0), Zone::Up(i)));
        z.push((Rect::new(pw - PAD - 26.0, y + 7.0, 26.0, 28.0), Zone::Down(i)));
    }
    z
}

/// Clic dans la page (coordonnées du contenu) : `true` si un réglage a changé.
pub fn click(p: &mut DockPrefs, text: &Text, pw: f64, x: f64, y: f64) -> bool {
    let Some(z) = zones(p, text, pw).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) else {
        return false;
    };
    match z {
        Zone::On => p.on = !p.on,
        // (choisir un côté ou cocher un module allume la colonne : demande de Jee, rien ne se passait)
        Zone::Pos(k) => (p.pos, p.on) = (k, true),
        Zone::Size(k) => p.size = k,
        Zone::Check(i) => {
            p.items[i].on = !p.items[i].on;
            if p.items[i].on {
                p.on = true;
            }
        }
        Zone::ModSize(i) => {
            let k = ModSize::ALL.iter().position(|s| *s == p.items[i].size).unwrap_or(0);
            p.items[i].size = ModSize::ALL[(k + 1) % ModSize::ALL.len()];
        }
        Zone::Up(i) if i > 0 => p.items.swap(i, i - 1),
        Zone::Down(i) if i + 1 < p.items.len() => p.items.swap(i, i + 1),
        Zone::Up(_) | Zone::Down(_) => return false,
    }
    true
}

/// Dessine la page de réglages (largeur `pw`) ; `big` : la fenêtre est assez grande pour la colonne.
pub fn draw_settings(p: &DockPrefs, pm: &mut Pixmap, text: &Text, pw: f64, big: bool, mouse: Option<(f64, f64)>, th: &Theme) {
    let h = crate::px::lh(pm);
    fill(pm, 0.0, 0.0, pw, h, th.background, 1.0);
    text.draw(pm, "🧩 Modules", (PAD as f32, 30.0), 15.0, WHITE, Align::Left);
    fill(pm, 0.0, TOP - 8.0, pw, 1.0, th.grid, 1.0);
    let hover = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
    let chip = |pm: &mut Pixmap, r: Rect, label: &str, on: bool| {
        round_fill(
            pm,
            r,
            7.0,
            if on {
                accent()
            } else if hover(&r) {
                pal::hover()
            } else {
                pal::field()
            },
            if on { 0.3 } else { 1.0 },
        );
        round_stroke(pm, r, 7.0, if on { accent() } else { pal::edge() }, 1.0);
        text.draw(pm, label, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, WHITE, Align::Center);
    };
    let n = puces_haut(p, text, pw).1;
    text.draw(
        pm,
        "Hauteur",
        (PAD as f32, (TOP + n as f64 * (CHIP_H + 10.0) + 18.0) as f32),
        12.0,
        GREY,
        Align::Left,
    );
    for (r, z) in zones(p, text, pw) {
        let y = r.y;
        match z {
            Zone::On => chip(pm, r, if p.on { ON_TXT } else { OFF_TXT }, p.on),
            Zone::Pos(k) => chip(pm, r, POS.iter().find(|(d, _)| *d == k).map_or("", |x| x.1), p.pos == k),
            Zone::Size(k) => chip(pm, r, SIZES[k], p.size == k),
            Zone::Check(i) => {
                let it = &p.items[i];
                let row = Rect::new(PAD - 4.0, y - 6.0, pw - 2.0 * PAD + 8.0, ROW - 4.0);
                round_fill(pm, row, 8.0, pal::panel(), 1.0);
                round_fill(pm, r, 5.0, if it.on { accent() } else { pal::field() }, 1.0);
                round_stroke(pm, r, 5.0, if it.on { accent() } else { pal::edge() }, 1.2);
                if it.on {
                    crate::shapes::lines(pm, r.cx(), r.cy(), &[&[(-6.0, 0.0), (-2.0, 4.5), (6.0, -5.0)]], WHITE, 2.0);
                }
                let (ic, name) = it.module.face();
                crate::icons::draw(pm, ic, PAD + 46.0, y + 13.0, 17, if it.on { ICON } else { GREY });
                // le nom en entier : sur une ligne s'il tient, sinon sur deux lignes un peu plus petites
                let c = if it.on { WHITE } else { GREY };
                for (k, l, taille) in nom_module(text, name, pw) {
                    text.draw(pm, &l, ((PAD + 60.0) as f32, (y + k) as f32), taille, c, Align::Left);
                }
            }
            Zone::ModSize(i) => {
                round_fill(pm, r, 6.0, if hover(&r) { pal::hover() } else { pal::field() }, 1.0);
                round_stroke(pm, r, 6.0, pal::edge(), 1.0);
                text.draw(
                    pm,
                    &format!("{} ▾", p.items[i].size.label()),
                    (r.cx() as f32, (r.cy() + 4.5) as f32),
                    12.5,
                    WHITE,
                    Align::Center,
                );
            }
            Zone::Up(_) | Zone::Down(_) => {
                let up = matches!(z, Zone::Up(_));
                if hover(&r) {
                    round_fill(pm, r, 6.0, pal::hover(), 1.0);
                }
                text.draw(
                    pm,
                    if up { "↑" } else { "↓" },
                    (r.cx() as f32, (r.cy() + 5.0) as f32),
                    14.0,
                    GREY,
                    Align::Center,
                );
            }
        }
    }
    let y = rows_top_pour(p, text, pw) + p.items.len() as f64 * ROW + 8.0;
    let note = if big {
        "Les modules s'empilent dans la colonne, à part du panneau de droite. Taille : touche Étroit / Moyen / Large / Remplir pour changer."
    } else {
        "📏 Les modules s'affichent seulement quand la fenêtre fait 1200 px de large ou plus : agrandis-la."
    };
    for (k, l) in crate::assistant::wrap(text, note, 12.0, pw - 2.0 * PAD).iter().enumerate() {
        text.draw(pm, l, (PAD as f32, (y + 14.0 + k as f64 * 17.0) as f32), 12.0, GREY, Align::Left);
    }
}

#[cfg(test)]
mod tests;
