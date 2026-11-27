//! 🧰 Barre de gauche — **la même qu'Argos v1** : curseur, les 6 groupes d'outils de dessin (icône = dernier outil
//! choisi du groupe, sous-menu à droite avec sections, raccourcis, « ? » et ⭐), la règle, puis l'aimant (faible / fort),
//! le mode dessin continu, verrouiller, masquer, la gomme, le ratio RR des positions, défaire et supprimer.
//! Les outils viennent de la table unique `tools`, les icônes d'Argos v1 (`icons`).

use crate::listmenu::{MenuGroup, MenuItem};
use crate::palette::DIM;
use crate::shapes::{round_fill, Rect};
use crate::text::Rgb;
use crate::tools::{def, RailItem, RAIL};
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Largeur de la barre.
pub const RAIL_W: u32 = 54;
const B: f64 = crate::formfactor::BOUTON_PC + 4.0;
const STEP: f64 = B + 2.0;

/// Aimant : colle les clics sur l'ouverture, le haut, le bas ou la clôture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Magnet {
    #[default]
    Off,
    Weak,
    Strong,
}

/// Ce qui est gardé entre deux lancements.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    /// Dernier outil choisi de chaque groupe (son icône dans la barre).
    pub last: Vec<(String, String)>,
    /// Outils ⭐ favoris (barre flottante).
    pub favs: Vec<String>,
    pub magnet: Magnet,
    /// S'accrocher aussi aux indicateurs.
    pub snap_ind: bool,
    /// Mode dessin continu : l'outil reste actif après un dessin.
    pub keep: bool,
    /// Ratio risque / rendement des positions longue / courte.
    pub rr: f64,
    pub hide_draw: bool,
    pub hide_ind: bool,
    pub hide_pos: bool,
    pub ind_locked: bool,
    /// Sections repliées des sous-menus.
    pub collapsed: Vec<String>,
    /// Émoji choisi pour l'outil Icône (posé au prochain clic sur la chart).
    pub emoji: String,
    /// ‹ › Barre de gauche repliée (même geste que la barre de droite).
    pub folded: bool,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            last: vec![],
            favs: vec![],
            magnet: Magnet::Weak,
            snap_ind: false,
            keep: false,
            rr: 2.0,
            hide_draw: false,
            hide_ind: false,
            hide_pos: false,
            ind_locked: false,
            collapsed: vec![],
            emoji: crate::emoji::DEFAULT.into(),
            folded: false,
        }
    }
}

impl Prefs {
    /// Outil affiché pour un groupe : le dernier choisi, sinon le premier.
    pub fn current(&self, group: &str) -> &'static str {
        let Some(g) = groups().find(|g| g.id == group) else { return "cursor" };
        let first = g.sections[0].1[0];
        let last = self.last.iter().find(|(k, _)| k == group).map(|(_, v)| v.as_str());
        last.and_then(def).map_or(first, |d| d.id)
    }

    pub fn set_last(&mut self, group: &str, tool: &str) {
        self.last.retain(|(k, _)| k != group);
        self.last.push((group.to_string(), tool.to_string()));
    }

    pub fn toggle_fav(&mut self, tool: &str) -> bool {
        match self.favs.iter().position(|t| t == tool) {
            Some(i) => {
                self.favs.remove(i);
                false
            }
            None => {
                self.favs.push(tool.to_string());
                true
            }
        }
    }

    fn hidden_all(&self) -> bool {
        self.hide_draw || self.hide_ind || self.hide_pos
    }
}

/// Les groupes d'outils, dans l'ordre de la barre de gauche (pour la barre flottante).
pub fn group_ids() -> impl Iterator<Item = &'static str> {
    groups().map(|g| g.id)
}

/// Nom d'un groupe (« Lignes, canaux, fourchettes »).
pub fn group_label(group: &str) -> &'static str {
    groups().find(|g| g.id == group).map_or("", |g| g.label)
}

/// Groupe d'un outil (`None` : outil seul, ou curseur).
pub fn group_of(tool: &str) -> Option<&'static str> {
    groups().find(|g| g.sections.iter().any(|(_, l)| l.contains(&tool))).map(|g| g.id)
}

fn groups() -> impl Iterator<Item = &'static crate::tools::RailGroup> {
    RAIL.iter().filter_map(|r| match r {
        RailItem::Group(g) => Some(g),
        RailItem::Tool(_) => None,
    })
}

/// Un bouton de la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Btn {
    Tool(&'static str),
    Group(&'static str),
    Magnet,
    Keep,
    Lock,
    Eye,
    Erase,
    Undo,
    Redo,
    Trash,
}

/// 🫧 The bar's glass pill on the PC (Jee, 21 Nov: « la barre de gauche pareille comme la barre de droite ») in an
/// image `h` high: the right column's place (6 from the window's edge, 44 wide, from 6 under the top rows to 10 above
/// the bottom); `None` by the finger (the phone has no bar, the tablet keeps its own).
pub fn pilule_rect(h: f64) -> Option<Rect> {
    (!crate::actions::tactile()).then(|| Rect::new(PILULE_X, PILULE_HAUT, PILULE_W, (h - PILULE_HAUT - PILULE_BAS).max(0.0)))
}

const PILULE_X: f64 = 6.0;
const PILULE_W: f64 = 44.0;
const PILULE_HAUT: f64 = 6.0;
const PILULE_BAS: f64 = 10.0;

/// Boutons et leur place (coordonnées de la barre). Les séparateurs sont entre les blocs, comme Argos v1.
/// 🖥 In its glass pill: centred in it, the first one 9 under its top (like the right column's icons).
pub fn layout(h: f64) -> Vec<(Rect, Btn)> {
    let (mut y, x) = match pilule_rect(h) {
        Some(p) => (p.y + 9.0, p.x + (p.w - B) / 2.0),
        None => (8.0, (RAIL_W as f64 - B) / 2.0),
    };
    let bas = pilule_rect(h).map_or(h - 4.0, |p| p.y + p.h - 4.0);
    let mut out = vec![];
    let mut push = |b: Btn, y: &mut f64| {
        out.push((Rect::new(x, *y, B, B), b));
        *y += STEP;
    };
    for item in RAIL {
        let b = match item {
            RailItem::Tool(id) => Btn::Tool(id),
            RailItem::Group(g) => Btn::Group(g.id),
        };
        push(b, &mut y);
    }
    y += 10.0;
    for b in [Btn::Magnet, Btn::Keep, Btn::Lock, Btn::Eye, Btn::Erase] {
        push(b, &mut y);
    }
    y += 10.0;
    for b in [Btn::Undo, Btn::Redo, Btn::Trash] {
        push(b, &mut y);
    }
    // fenêtre trop basse : on garde ce qui rentre
    out.retain(|(r, _)| r.y + r.h <= bas);
    out
}

/// La souris est-elle sur la petite flèche du coin d'un groupe (bas à droite) ? Elle ouvre le sous-menu ; le reste du bouton prend l'outil.
pub fn on_arrow(r: &Rect, x: f64, y: f64) -> bool {
    r.contains(x, y) && x >= r.x + r.w - 14.0 && y >= r.y + r.h - 14.0
}

pub fn hit(x: f64, y: f64, h: f64) -> Option<(Btn, Rect)> {
    layout(h).into_iter().find(|(r, _)| r.contains(x, y)).map(|(r, b)| (b, r))
}

/// Nom du bouton (bulle).
pub fn tip(p: &Prefs, b: Btn) -> String {
    match b {
        // infobulle « nom · raccourci » quand l'outil a une vraie touche (table d'actions)
        // (📱 phone view: the name only, no keyboard)
        Btn::Tool("hline") if !crate::actions::tactile() => format!(
            "{} · {}",
            def("hline").map_or("", |d| d.label),
            crate::actions::shortcut(crate::actions::ChartAction::HLineAtMouse).unwrap_or_default()
        ),
        Btn::Tool(id) => def(id).map_or(id.to_string(), |d| d.label.to_string()),
        Btn::Group(g) => {
            let label = groups().find(|x| x.id == g).map_or("", |x| x.label);
            let cur = def(p.current(g)).map_or("", |d| d.label);
            format!("{label} · {cur}")
        }
        Btn::Magnet => format!(
            "Aimant : {}{}",
            match p.magnet {
                Magnet::Off => "désactivé",
                Magnet::Weak => "faible",
                Magnet::Strong => "fort",
            },
            if p.snap_ind { " + indicateurs" } else { "" }
        ),
        Btn::Keep => "Mode dessin continu : garder l'outil actif".into(),
        Btn::Lock => "Verrouiller (dessins, indicateurs, tout)".into(),
        Btn::Eye => "Masquer (dessins, indicateurs, ordres, tout)".into(),
        Btn::Erase => "Gomme".into(),
        Btn::Undo => "Défaire (Ctrl + Z)".into(),
        Btn::Redo => "Refaire (Ctrl + Maj + Z ou Ctrl + Y)".into(),
        Btn::Trash => "Supprimer".into(),
    }
}

/// Sous-menu d'un groupe : sections, icônes, raccourcis, « ? » et ⭐ (l'id d'une ligne = son rang dans `tools::TOOLS`).
pub fn group_menu(p: &Prefs, group: &str, active: &str) -> Vec<MenuGroup> {
    let Some(g) = groups().find(|g| g.id == group) else { return vec![] };
    g.sections
        .iter()
        .map(|(title, ids)| MenuGroup {
            title: title.to_uppercase(),
            items: ids
                .iter()
                .filter_map(|id| {
                    let (k, d) = crate::tools::TOOLS.iter().enumerate().find(|(_, t)| t.id == *id)?;
                    Some(MenuItem {
                        id: k as i64,
                        label: d.label.to_string(),
                        enabled: true,
                        star: Some(p.favs.iter().any(|f| f == id)),
                        checked: *id == active,
                        icon: Some(d.id),
                        hint: if d.key.is_empty() { String::new() } else { format!("Alt + {}", d.key) },
                        tip: d.tip.to_string(),
                        ..Default::default()
                    })
                })
                .collect(),
            strip: false,
        })
        .collect()
}

/// Menus des boutons du bas (id des lignes : voir `MenuAction`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    MagnetOff = 1,
    MagnetWeak,
    MagnetStrong,
    SnapInd,
    LockDraw,
    LockInd,
    LockAll,
    HideDraw,
    HideInd,
    HidePos,
    HideAll,
    DelDraw,
    DelInd,
    DelAll,
}

impl MenuAction {
    pub fn from_id(id: i64) -> Option<Self> {
        use MenuAction::*;
        [
            MagnetOff,
            MagnetWeak,
            MagnetStrong,
            SnapInd,
            LockDraw,
            LockInd,
            LockAll,
            HideDraw,
            HideInd,
            HidePos,
            HideAll,
            DelDraw,
            DelInd,
            DelAll,
        ]
        .into_iter()
        .find(|a| *a as i64 == id)
    }
}

fn row(a: MenuAction, label: String, checked: bool, icon: Option<&'static str>) -> MenuItem {
    MenuItem {
        id: a as i64,
        label,
        enabled: true,
        checked,
        icon,
        ..Default::default()
    }
}

/// Menu de l'aimant, du cadenas, de l'œil ou de la poubelle (`drawings` : nombre de dessins, `locked` : tous verrouillés).
pub fn button_menu(p: &Prefs, b: Btn, drawings: usize, locked: bool) -> Vec<MenuGroup> {
    use MenuAction::*;
    let one = |items: Vec<MenuItem>| {
        vec![MenuGroup {
            title: String::new(),
            items,
            strip: false,
        }]
    };
    match b {
        Btn::Magnet => vec![
            MenuGroup {
                title: String::new(),
                items: vec![
                    row(MagnetOff, "Désactivé".into(), p.magnet == Magnet::Off, None),
                    row(MagnetWeak, "Aimant faible".into(), p.magnet == Magnet::Weak, Some("magnet")),
                    row(MagnetStrong, "Aimant fort".into(), p.magnet == Magnet::Strong, Some("magnet2")),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![row(
                    SnapInd,
                    format!("S'accrocher aux indicateurs : {}", if p.snap_ind { "oui" } else { "non" }),
                    p.snap_ind,
                    None,
                )],
                strip: false,
            },
        ],
        Btn::Lock => one(vec![
            row(
                LockDraw,
                if locked { "Déverrouiller les dessins" } else { "Verrouiller les dessins" }.into(),
                false,
                None,
            ),
            row(
                LockInd,
                if p.ind_locked {
                    "Déverrouiller les indicateurs"
                } else {
                    "Verrouiller les indicateurs"
                }
                .into(),
                false,
                None,
            ),
            row(
                LockAll,
                if locked && p.ind_locked { "Tout déverrouiller" } else { "Verrouiller tout" }.into(),
                false,
                None,
            ),
        ]),
        Btn::Eye => one(vec![
            row(
                HideDraw,
                if p.hide_draw { "Afficher les dessins" } else { "Masquer les dessins" }.into(),
                false,
                None,
            ),
            row(
                HideInd,
                if p.hide_ind { "Afficher les indicateurs" } else { "Masquer les indicateurs" }.into(),
                false,
                None,
            ),
            row(
                HidePos,
                if p.hide_pos {
                    "Afficher les ordres et positions"
                } else {
                    "Masquer les ordres et positions"
                }
                .into(),
                false,
                None,
            ),
            row(HideAll, if p.hidden_all() { "Tout afficher" } else { "Tout masquer" }.into(), false, None),
        ]),
        Btn::Trash => {
            let s = if drawings > 1 { "s" } else { "" };
            vec![
                MenuGroup {
                    title: String::new(),
                    items: vec![
                        row(DelDraw, format!("Supprimer {drawings} dessin{s}"), false, Some("trash")),
                        row(DelInd, "Supprimer 0 indicateur".into(), false, Some("trash")),
                    ],
                    strip: false,
                },
                MenuGroup {
                    title: String::new(),
                    items: vec![row(DelAll, "Supprimer les dessins et les indicateurs".into(), false, Some("trash"))],
                    strip: false,
                },
            ]
        }
        _ => vec![],
    }
}

/// Applique une ligne d'un menu du bas aux réglages (verrouiller / supprimer les dessins : c'est la fenêtre qui les fait).
pub fn apply(p: &mut Prefs, a: MenuAction) {
    use MenuAction::*;
    match a {
        MagnetOff => p.magnet = Magnet::Off,
        MagnetWeak => p.magnet = Magnet::Weak,
        MagnetStrong => p.magnet = Magnet::Strong,
        SnapInd => p.snap_ind = !p.snap_ind,
        LockInd => p.ind_locked = !p.ind_locked,
        LockAll => p.ind_locked = true,
        HideDraw => p.hide_draw = !p.hide_draw,
        HideInd => p.hide_ind = !p.hide_ind,
        HidePos => p.hide_pos = !p.hide_pos,
        HideAll => {
            let on = !p.hidden_all();
            (p.hide_draw, p.hide_ind, p.hide_pos) = (on, on, on);
        }
        LockDraw | DelDraw | DelInd | DelAll => {}
    }
}

thread_local! {
    /// 🖱 Icône du 1er bouton : le curseur choisi (« cursor » = la croix, « curseur-point », « curseur-fleche »).
    static ICONE_CURSEUR: std::cell::Cell<&'static str> = const { std::cell::Cell::new("cursor") };
}

/// 🖱 L'icône du curseur choisi, au 1er bouton de la barre (posée par l'app).
pub fn set_icone_curseur(id: &'static str) {
    ICONE_CURSEUR.with(|c| c.set(id));
}

fn icone_curseur() -> &'static str {
    ICONE_CURSEUR.with(|c| c.get())
}

/// Image de la barre (largeur `RAIL_W`, hauteur `h`). `active` : outil choisi ; `mouse` : position dans la barre.
pub fn draw(p: &Prefs, h: u32, active: &str, locked: bool, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
    let mut pm = crate::px::new(RAIL_W, h.max(1))?;
    // the app's background, then (🖥 PC, Jee 21 Nov) the right column's own glass pill, drawn by the same function
    let fond = Rgb(0x0a, 0x0b, 0x0f);
    crate::fill(&mut pm, 0.0, 0.0, RAIL_W as f64, h as f64, fond, 1.0);
    if let Some(r) = pilule_rect(h as f64) {
        let mut p = crate::px::new(r.w as u32, r.h.max(1.0) as u32)?;
        crate::watchlist::pilule_bande(&mut p, r.w, r.h, &crate::Theme::current());
        crate::px::put(&mut pm, r.x, r.y, &p);
    }
    // (one grey ink, like the navigation pill: the chosen tool is shown by its light patch)
    let ink = crate::zoompad::ENCRE;
    let btns = layout(h as f64);
    for (r, b) in &btns {
        // (avant l'aimant et avant défaire : un espace, sans trait, comme la maquette)
        let pressed = match b {
            Btn::Tool(id) => *id == active,
            Btn::Group(g) => p.current(g) == active,
            Btn::Magnet => p.magnet != Magnet::Off,
            Btn::Keep => p.keep,
            Btn::Lock => locked,
            Btn::Eye => p.hidden_all(),
            Btn::Erase => active == "erase",
            _ => false,
        };
        let hover = mouse.is_some_and(|(x, y)| r.contains(x, y));
        // (maquette : `#rail button:hover,#rail button.on{background:var(--verre2)}`, rayon 11, l.69-70)
        if pressed {
            round_fill(&mut pm, *r, crate::jetons::pilule(*r), crate::zoompad::CHOISI.0, crate::zoompad::CHOISI.1);
        } else if hover {
            round_fill(&mut pm, *r, crate::jetons::pilule(*r), crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
        }
        let c = ink;
        let icon = match b {
            // 🖱 1er bouton : l'icône du curseur choisi (Croix, Point, Flèche)
            Btn::Tool("cursor") => icone_curseur(),
            Btn::Tool(id) => *id,
            Btn::Group(g) => p.current(g),
            Btn::Magnet => {
                if p.magnet == Magnet::Strong {
                    "magnet2"
                } else {
                    "magnet"
                }
            }
            Btn::Keep => "keep",
            Btn::Lock => {
                if locked {
                    "lock2"
                } else {
                    "unlock2"
                }
            }
            Btn::Eye => {
                if p.hidden_all() {
                    "eyeoff2"
                } else {
                    "eye2"
                }
            }
            Btn::Erase => "erase",
            Btn::Undo => "undo",
            Btn::Redo => "redo",
            Btn::Trash => "trash",
        };
        crate::icons::draw(&mut pm, icon, r.cx(), r.cy(), 26, c);
        // petit triangle : le groupe a un sous-menu (clic dessus = la liste) ; plus gros et clair quand la souris est dessus
        if matches!(b, Btn::Group(_) | Btn::Tool("cursor")) {
            let on = mouse.is_some_and(|(x, y)| on_arrow(r, x, y));
            if on {
                round_fill(
                    &mut pm,
                    Rect::new(r.x + r.w - 14.0, r.y + r.h - 14.0, 14.0, 14.0),
                    7.0,
                    Rgb(0x3a, 0x3f, 0x4c),
                    1.0,
                );
            }
            let k = if on { 6.0 } else { 4.0 };
            let tri: [(f32, f32); 3] = [(0.0, 0.0), (0.0, -k), (-k, 0.0)];
            crate::shapes::poly_fill(&mut pm, r.x + r.w - 3.0, r.y + r.h - 3.0, &tri, if on { ink } else { DIM }, 1.0);
        }
    }
    Some(pm)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// « ok meme nom » (Jee 18 Nov): the « Prévisions et mesures » group has TradingView's section titles, names and
    /// order (French); Règle and Loupe are Argos extras, last.
    #[test]
    fn previsions_et_mesures_comme_tradingview() {
        let m = group_menu(&Prefs::default(), "pos", "");
        let vu: Vec<(String, Vec<String>)> = m.iter().map(|g| (g.title.clone(), g.items.iter().map(|i| i.label.clone()).collect())).collect();
        let attendu = [
            (
                "PRÉVISIONS",
                vec![
                    "Position longue",
                    "Position courte",
                    "Prévision de position",
                    "Motif à barres",
                    "Ghost feed",
                    "Secteur",
                ],
            ),
            (
                "BASÉ SUR LE VOLUME",
                vec!["VWAP ancré", "Profil de volume à gamme fixe", "Profil de volume ancré"],
            ),
            (
                "MESUREURS",
                vec!["Plage de prix", "Plage de dates", "Plage de date et de prix", "Règle", "Loupe"],
            ),
        ];
        assert_eq!(vu.len(), 3);
        for ((titre, noms), (t, n)) in vu.iter().zip(attendu.iter()) {
            assert_eq!(titre, t);
            assert_eq!(noms, n, "{t}");
        }
    }

    #[test]
    fn barre_comme_v1() {
        let l = layout(1000.0);
        assert_eq!(l[0].1, Btn::Tool("cursor"));
        assert_eq!(l[1].1, Btn::Group("lines"));
        assert_eq!(l[7].1, Btn::Tool("measure"));
        assert_eq!(l.last().map(|x| x.1), Some(Btn::Trash));
        let (b, _) = hit(l[2].0.cx(), l[2].0.cy(), 1000.0).unwrap();
        assert_eq!(b, Btn::Group("fib"));
        // centre de l'icône = l'outil ; coin bas droit (la petite flèche) = la liste du groupe
        let r = l[2].0;
        assert!(!on_arrow(&r, r.cx(), r.cy()));
        assert!(on_arrow(&r, r.x + r.w - 3.0, r.y + r.h - 3.0));
        // le groupe montre son dernier outil
        let mut p = Prefs::default();
        assert_eq!(p.current("lines"), "trend");
        p.set_last("lines", "hray");
        assert_eq!(p.current("lines"), "hray");
        assert!(tip(&p, Btn::Group("lines")).contains("Rayon horizontal"));
        // 📱 tooltip of the horizontal line: « name · shortcut » on the PC, the name only on the phone (Jee, 20 Nov)
        crate::actions::set_tactile(false);
        let pc = tip(&p, Btn::Tool("hline"));
        assert!(pc.starts_with("Ligne horizontale · ") && pc.len() > "Ligne horizontale · ".len(), "{pc}");
        crate::actions::set_tactile(true);
        assert_eq!(tip(&p, Btn::Tool("hline")), "Ligne horizontale");
        crate::actions::set_tactile(false);
        // sous-menu : sections en majuscules, icônes, raccourcis, ⭐
        let m = group_menu(&p, "lines", "hray");
        assert_eq!(m[0].title, "LIGNES");
        let hline = m[0].items.iter().find(|i| i.label == "Ligne horizontale").unwrap();
        assert_eq!(hline.hint, "Alt + H");
        assert!(m[0].items.iter().any(|i| i.checked));
        assert!(p.toggle_fav("trend"));
        assert_eq!(group_menu(&p, "lines", "")[0].items[0].star, Some(true));
        // menus du bas
        apply(&mut p, MenuAction::MagnetStrong);
        assert_eq!(p.magnet, Magnet::Strong);
        apply(&mut p, MenuAction::HideAll);
        assert!(p.hide_draw && p.hide_ind && p.hide_pos);
        assert_eq!(button_menu(&p, Btn::Trash, 6, false)[0].items[0].label, "Supprimer 6 dessins");
        assert_eq!(MenuAction::from_id(MenuAction::DelAll as i64), Some(MenuAction::DelAll));
        assert!(draw(&p, 800, "hray", false, None).is_some());
    }

    /// 📸 La barre de gauche en verre (comparaison avec la maquette) : seulement si `ARGOS_CAPTURES`.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        draw(&Prefs::default(), 760, "trend", false, None)
            .unwrap()
            .save_png(format!("{dir}/app-barre-gauche.png"))
            .unwrap();
    }

    /// 🫧 Jee (21 Nov): on the PC the drawing bar is the right column's glass pill (the same function, the same 44 wide
    /// and round ends, 6 from the window's edge), its buttons inside it, and a click on each one falls on it; by the
    /// finger nothing changes.
    #[test]
    fn barre_de_gauche_en_pilule_comme_la_colonne() {
        let h = 760.0;
        crate::actions::set_tactile(false);
        let pil = pilule_rect(h).expect("PC: a pill");
        assert_eq!((pil.x, pil.w), (6.0, 44.0), "the right column's place and width");
        let btns = layout(h);
        assert!(btns.len() >= 15, "{}", btns.len());
        for (r, b) in &btns {
            assert!(
                r.x >= pil.x && r.x + r.w <= pil.x + pil.w && r.y >= pil.y && r.y + r.h <= pil.y + pil.h,
                "{b:?} in the pill"
            );
            assert_eq!(hit(r.cx(), r.cy(), h).map(|x| x.0), Some(*b), "{b:?}: the click falls on it");
        }
        // drawn: the same pixels as the column's pill (same function), the round end shows the background
        let pm = draw(&Prefs::default(), h as u32, "", false, None).unwrap();
        let mut seule = crate::px::new(RAIL_W, h as u32).unwrap();
        crate::fill(&mut seule, 0.0, 0.0, RAIL_W as f64, h, Rgb(0x0a, 0x0b, 0x0f), 1.0);
        let mut p = crate::px::new(pil.w as u32, pil.h as u32).unwrap();
        crate::watchlist::pilule_bande(&mut p, pil.w, pil.h, &crate::Theme::default());
        crate::px::put(&mut seule, pil.x, pil.y, &p);
        let k = crate::px::scale() as f64;
        let y_vide = (btns.last().unwrap().0.y + btns.last().unwrap().0.h + 2.0).min(pil.y + pil.h - 30.0);
        for (x, y) in [(pil.x + 1.0, pil.cy()), (pil.x + 2.0, pil.y + 2.0), (2.0, pil.cy()), (pil.cx(), y_vide)] {
            let (px, py) = ((x * k) as u32, (y * k) as u32);
            assert_eq!(pm.pixel(px, py), seule.pixel(px, py), "({x}, {y})");
        }
        assert_ne!(
            pm.pixel((pil.x * k) as u32 + 2, (pil.cy() * k) as u32),
            pm.pixel(2, (pil.cy() * k) as u32),
            "a pill, not the background"
        );
        // 📱 by the finger: as before
        crate::actions::set_tactile(true);
        assert!(pilule_rect(h).is_none());
        assert_eq!(layout(h)[0].0.y, 8.0);
        crate::actions::set_tactile(false);
    }
}
