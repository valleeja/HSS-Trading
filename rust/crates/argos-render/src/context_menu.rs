//! **Moteur de menus unique** (clic droit, ⚙, ➕). Un menu, c'est des données (`Entry`) fournies par une source
//! (colonne de prix, échelle de temps, graphique, ➕) ; le moteur fait tout le reste une seule fois : position, dessin
//! (✓, ›, raccourcis), sous-menus qui s'ouvrent **à côté** au survol, clics, fermeture.
//! Les actions viennent de la table unique (`actions`) : le raccourci affiché est le vrai.

use crate::actions::{self, ChartAction, Flag};
use crate::palette as pal;
use crate::palette::WHITE;
use crate::settings::{tz_label, ChartSettings, ScaleMode, TIMEZONES};
use crate::text::{Align, Rgb, Text};
use crate::viewport::{format_price, Viewport};
use crate::{fill, Theme};
use std::sync::OnceLock;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap, Stroke};

/// Une ligne de menu.
#[derive(Debug, Clone, PartialEq)]
pub enum Entry {
    /// Ligne cliquable ; `action: None` = en gris (pas encore disponible).
    Item {
        icon: &'static str,
        label: String,
        checked: bool,
        action: Option<ChartAction>,
    },
    /// Sous-menu : s'ouvre à côté au survol.
    Sub {
        label: String,
        entries: Vec<Entry>,
    },
    Sep,
}

fn item(icon: &'static str, label: impl Into<String>, checked: bool, action: ChartAction) -> Entry {
    Entry::Item {
        icon,
        label: label.into(),
        checked,
        action: Some(action),
    }
}

fn soon(icon: &'static str, label: impl Into<String>) -> Entry {
    Entry::Item {
        icon,
        label: label.into(),
        checked: false,
        action: None,
    }
}

fn toggle(label: &str, st: &ChartSettings, f: Flag) -> Entry {
    item("", label, actions::flag(st, f), ChartAction::Toggle(f))
}

/// D'où vient le menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    PriceAxis,
    TimeAxis,
    Chart,
    Plus,
}

/// Où le menu s'ouvre.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Anchor {
    /// Au curseur (clic droit).
    Cursor(f64, f64),
    /// Au-dessus du bouton ⚙, collé à droite.
    Gear,
    /// Sous le bouton ➕ (à la hauteur donnée), collé à droite de la zone des bougies.
    Below(f64),
}

/// Ce que les sources ont besoin de savoir.
pub struct Ctx<'a> {
    pub settings: &'a ChartSettings,
    pub drawings: usize,
    /// Moment présent (secondes UTC), pour les décalages des fuseaux.
    pub now: i64,
    /// Type de chart TPO : ses réglages dans le clic droit (comme LuxAlgo).
    pub tpo: bool,
}

/// Un menu ouvert : prix figé au moment de l'ouverture (clic droit ou ➕), ligne survolée, sous-menu ouvert.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OpenMenu {
    pub source: Source,
    pub anchor: Anchor,
    pub price: f64,
    pub decimals: usize,
    /// (niveau 0 = menu, 1 = sous-menu ; index de la ligne).
    pub hover: Option<(usize, usize)>,
    /// Index (dans le menu) du sous-menu ouvert.
    pub sub: Option<usize>,
    /// Sous-menu trop long pour la fenêtre (ex. fuseaux) : première ligne affichée (molette pour défiler).
    pub sub_scroll: usize,
}

/// Le menu de la chart (un seul ouvert à la fois).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ContextMenu {
    pub open: Option<OpenMenu>,
}

thread_local! {
    /// ⛶ The chart menu's last line: (name of what goes full screen, full screen on). `None`: no line (split, a page).
    static LIGNE_PLEIN_ECRAN: std::cell::RefCell<Option<(String, bool)>> = const { std::cell::RefCell::new(None) };
}

/// ⛶ Sets the chart menu's last line: « ⛶ Plein écran : <name> » or, when on, « Quitter le plein écran »; `None`: none.
pub fn set_plein_ecran(l: Option<(String, bool)>) {
    LIGNE_PLEIN_ECRAN.with(|c| *c.borrow_mut() = l);
}

/// Les lignes d'une source (comme les menus de TradingView, avec les fonctions qui marchent dans Argos).
/// 📱 In phone view (phone, or tablet used by touch) the menu is cut down for the finger: see [`au_doigt`].
pub fn entries(src: Source, ctx: &Ctx, om: &OpenMenu) -> Vec<Entry> {
    let v = toutes(src, ctx, om);
    if actions::tactile() {
        au_doigt(src, v)
    } else {
        v
    }
}

/// Labels of the price-axis rows that go in « Avancé » in phone view (a beginner needs Auto, Normal, Pourcent, Log).
const AVANCE: [&str; 3] = [
    "Verrouiller le rapport prix / barre",
    "Mise à l'échelle des prix du graphique uniquement",
    "Indexé sur 100",
];

/// 📱 The menu for the finger (Jee, 21 Nov, list 2): no grey row (« bientôt », « pas en crypto »), no empty
/// sub-menu, no doubled or dangling separator; on the price axis the technical rows go in an « Avancé » sub-menu.
fn au_doigt(src: Source, v: Vec<Entry>) -> Vec<Entry> {
    fn garder(v: Vec<Entry>) -> Vec<Entry> {
        let v = v.into_iter().filter_map(|e| match e {
            Entry::Item { action: None, .. } => None,
            Entry::Sub { label, entries } => {
                let entries = garder(entries);
                (!entries.is_empty()).then_some(Entry::Sub { label, entries })
            }
            e => Some(e),
        });
        let mut out: Vec<Entry> = Vec::new();
        for e in v {
            if e == Entry::Sep && out.last().is_none_or(|l| *l == Entry::Sep) {
                continue;
            }
            out.push(e);
        }
        if out.last() == Some(&Entry::Sep) {
            out.pop();
        }
        out
    }
    let mut v = garder(v);
    if src == Source::PriceAxis {
        let est_avance = |e: &Entry| matches!(e, Entry::Item { label, .. } if AVANCE.contains(&label.as_str()));
        let avance: Vec<Entry> = v.iter().filter(|&e| est_avance(e)).cloned().collect();
        v.retain(|e| !est_avance(e));
        v.push(Entry::Sep);
        v.push(Entry::Sub {
            label: "Avancé".into(),
            entries: avance,
        });
    }
    v
}

/// Every row of a source (the PC menu).
fn toutes(src: Source, ctx: &Ctx, om: &OpenMenu) -> Vec<Entry> {
    let st = ctx.settings;
    let o = st.scale;
    let p = format_price(om.price, om.decimals);
    let mode = |m, label| item("", label, o.mode == m, ChartAction::Mode(m));
    match src {
        Source::PriceAxis => vec![
            item("↺", "Réinitialiser l'échelle de prix", false, ChartAction::ResetPrice),
            Entry::Sep,
            item("", "Auto (adapte les données à l'écran)", o.auto(), ChartAction::AutoScale),
            item("", AVANCE[0], o.lock_ratio.is_some(), ChartAction::LockRatio),
            toggle(AVANCE[1], st, Flag::MainOnly),
            item("", "Inverser l'échelle", o.invert, ChartAction::Invert),
            Entry::Sep,
            mode(ScaleMode::Normal, "Normal"),
            mode(ScaleMode::Percent, "Pourcent"),
            mode(ScaleMode::Indexed, AVANCE[2]),
            mode(ScaleMode::Log, "Logarithmique"),
            Entry::Sep,
            soon("", "Déplacer l'échelle vers la gauche  (bientôt)"),
            Entry::Sep,
            Entry::Sub {
                label: "Étiquettes".into(),
                entries: vec![
                    toggle("Étiquette de nom du symbole", st, Flag::SymbolName),
                    toggle("Étiquette de la dernière valeur du symbole", st, Flag::LastPriceLabel),
                    toggle("Étiquette de clôture du jour précédent", st, Flag::PrevCloseLabel),
                    soon("", "Étiquette de prix pré / post-marché  (pas en crypto)"),
                    toggle("Étiquettes de prix haut et bas", st, Flag::HighLowLabel),
                    toggle("Prix du plus haut et du plus bas (sur la chart)", st, Flag::HighLowChart),
                    toggle("Étiquettes Bid & Ask", st, Flag::BidAskLabel),
                    soon("", "Libellés des indicateurs  (bientôt)"),
                    soon("", "Étiquettes de valeur des indicateurs  (bientôt)"),
                    toggle("Compte à rebours jusqu'à la fermeture de la barre", st, Flag::Countdown),
                    Entry::Sep,
                    soon("", "Pas d'étiquettes superposées  (bientôt)"),
                ],
            },
            Entry::Sub {
                label: "Lignes".into(),
                entries: vec![
                    toggle("Ligne de prix", st, Flag::LastPriceLine),
                    toggle("Ligne de prix de clôture du jour précédent", st, Flag::PrevCloseLine),
                    soon("", "Ligne de prix pré / post-marché  (pas en crypto)"),
                    toggle("Lignes de prix haut et bas", st, Flag::HighLowLine),
                    toggle("Lignes Bid & Ask", st, Flag::BidAskLine),
                    Entry::Sep,
                    toggle("Grille", st, Flag::Grid),
                ],
            },
            toggle("Bouton +", st, Flag::PlusButton),
            Entry::Sep,
            soon("⚙", "Plus de paramètres…  (bientôt)"),
        ],
        Source::TimeAxis => {
            let mut zones = vec![item("", "Ordinateur (heure locale)", st.timezone.is_empty(), ChartAction::SetTimezone(""))];
            let first = TIMEZONES.iter().position(|(_, z)| *z == st.timezone);
            zones.extend(
                TIMEZONES
                    .iter()
                    .enumerate()
                    .map(|(k, (city, z))| item("", tz_label(city, z, ctx.now), first == Some(k), ChartAction::SetTimezone(z))),
            );
            vec![
                item("↺", "Réinitialiser l'échelle de temps", false, ChartAction::ResetTime),
                Entry::Sep,
                Entry::Sub {
                    label: "Fuseau horaire".into(),
                    entries: zones,
                },
                toggle("Arrêts de session", st, Flag::SessionBreaks),
                toggle("📅 Annonces économiques", st, Flag::EconEvents),
                toggle("📰 Nouvelles sur la chart", st, Flag::EconNews),
                toggle("🛡️ Fenêtres PropFirm ±2 min", st, Flag::PropFirmWindows),
                Entry::Sep,
                soon("⚙", "Plus de paramètres…  (bientôt)"),
            ]
        }
        Source::Chart => {
            let mut v = vec![
                item("↺", "Réinitialiser l'affichage du graphique", false, ChartAction::ResetView),
                Entry::Sep,
                item("", format!("Copier le prix {p}"), false, ChartAction::CopyPrice(om.price)),
                soon("", "Coller  (bientôt)"),
                Entry::Sep,
                item("◷", format!("Ajouter une alerte à {p}"), false, ChartAction::AddAlert(om.price)),
                item("—", format!("Tracer une ligne horizontale à {p}"), false, ChartAction::HLine(om.price)),
                item("▲", format!("Préparer un achat démo à {p}"), false, ChartAction::Brouillon(om.price, true)),
                item("▼", format!("Préparer une vente démo à {p}"), false, ChartAction::Brouillon(om.price, false)),
                Entry::Sep,
                soon("", "Verrouiller la ligne verticale du curseur  (bientôt)"),
                Entry::Sep,
                soon("", "Vue du tableau  (bientôt)"),
                item("", "Arborescence des objets", false, ChartAction::ObjectTree),
                Entry::Sub {
                    label: "Modèle de graphique".into(),
                    entries: vec![soon("", "Sauvegarder sous…  (bientôt)")],
                },
            ];
            if ctx.drawings > 0 {
                v.push(item(
                    "✕",
                    format!("Effacer tous les dessins ({})", ctx.drawings),
                    false,
                    ChartAction::ClearDrawings,
                ));
            }
            if ctx.tpo {
                let t = st.tpo;
                v.push(Entry::Sep);
                v.push(Entry::Sub {
                    label: "Réglages TPO".into(),
                    entries: [(300, "5 min"), (900, "15 min"), (1800, "30 min"), (3600, "1 h"), (14_400, "4 h")]
                        .iter()
                        .map(|&(s, n)| item("", format!("Lettre : {n}"), t.block == s, ChartAction::TpoBlock(s)))
                        .chain([Entry::Sep])
                        .chain(
                            [60u8, 68, 70, 80]
                                .iter()
                                .map(|&v| item("", format!("Zone de valeur : {v} %"), t.va == v, ChartAction::TpoVa(v))),
                        )
                        .chain([Entry::Sep])
                        .chain(
                            [30i64, 60, 120]
                                .iter()
                                .map(|&m| item("", format!("Initial balance : {m} min"), t.ib == m, ChartAction::TpoIb(m))),
                        )
                        .chain([Entry::Sep, toggle("Lettres", st, Flag::TpoLetters), toggle("Blocs", st, Flag::TpoBlocks)])
                        .collect(),
                });
            }
            v.push(Entry::Sep);
            v.push(soon("⚙", "Configurations…  (bientôt)"));
            // ⛶ the last line (Jee, 17 Nov): full screen of what is under the right-click, or back
            if let Some((nom, on)) = LIGNE_PLEIN_ECRAN.with(|c| c.borrow().clone()) {
                v.push(Entry::Sep);
                let l = if on {
                    "Quitter le plein écran".to_string()
                } else {
                    format!("Plein écran : {nom}")
                };
                v.push(item("⤢", l, false, ChartAction::PleinEcran));
            }
            v
        }
        Source::Plus => {
            let mut v = vec![
                item("◷", format!("Ajouter une alerte à {p}"), false, ChartAction::AddAlert(om.price)),
                Entry::Sep,
                item("▲", format!("Préparer un achat démo à {p}"), false, ChartAction::Brouillon(om.price, true)),
                item("▼", format!("Préparer une vente démo à {p}"), false, ChartAction::Brouillon(om.price, false)),
                Entry::Sep,
                item("—", format!("Tracer une ligne horizontale à {p}"), false, ChartAction::HLine(om.price)),
            ];
            if ctx.drawings > 0 {
                v.push(item(
                    "✕",
                    format!("Effacer tous les dessins ({})", ctx.drawings),
                    false,
                    ChartAction::ClearDrawings,
                ));
            }
            v
        }
    }
}

const ROW: f64 = 30.0;
const SEP: f64 = 9.0;
const PAD: f64 = 6.0;
const SIZE: f32 = 13.5;

/// Polices partagées (chargées une seule fois) pour mesurer et écrire les menus.
fn font() -> &'static Text {
    static T: OnceLock<Text> = OnceLock::new();
    T.get_or_init(Text::default)
}

/// Un panneau placé : position, taille, et pour chaque ligne (y, hauteur, index de l'entrée ou `None` = séparateur).
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub rows: Vec<(f64, f64, Option<usize>)>,
}

fn width(es: &[Entry]) -> f64 {
    let t = font();
    let w = es
        .iter()
        .map(|e| match e {
            Entry::Item { label, action, .. } => {
                let sc = action.and_then(actions::shortcut_bulle).map_or(0.0, |s| t.width(&s, 12.0) as f64 + 28.0);
                t.width(label, SIZE) as f64 + sc
            }
            Entry::Sub { label, .. } => t.width(label, SIZE) as f64 + 40.0,
            Entry::Sep => 0.0,
        })
        .fold(0.0, f64::max);
    (w + 64.0).max(220.0)
}

fn rows_of(es: &[Entry], y0: f64) -> (Vec<(f64, f64, Option<usize>)>, f64) {
    rows_from(es, y0, 0, f64::INFINITY)
}

/// Lignes à partir de l'entrée `from`, tant qu'elles tiennent dans `max_h` (panneau qui défile).
fn rows_from(es: &[Entry], y0: f64, from: usize, max_h: f64) -> (Vec<(f64, f64, Option<usize>)>, f64) {
    let mut y = y0 + PAD;
    let mut rows = vec![];
    for (k, e) in es.iter().enumerate().skip(from) {
        let (h, idx) = if *e == Entry::Sep { (SEP, None) } else { (ROW, Some(k)) };
        if y + h + PAD - y0 > max_h {
            break;
        }
        rows.push((y, h, idx));
        y += h;
    }
    (rows, y + PAD - y0)
}

/// Nombre de lignes d'un sous-menu qui tiennent dans la fenêtre (toutes si elles tiennent).
fn fits(es: &[Entry], max_h: f64) -> usize {
    rows_from(es, 0.0, 0, max_h).0.len()
}

/// Un sous-menu placé : ses lignes et son panneau.
pub type SubPanel = (Vec<Entry>, Panel);

/// Place le menu et, s'il y en a un d'ouvert, son sous-menu. Tout reste dans la fenêtre.
pub fn panels(vp: &Viewport, ctx: &Ctx, om: &OpenMenu) -> (Vec<Entry>, Panel, Option<SubPanel>) {
    let es = entries(om.source, ctx, om);
    let (ww, wh) = (vp.width as f64, vp.height as f64);
    let w = width(&es);
    let (_, h) = rows_of(&es, 0.0);
    let (mut x, mut y) = match om.anchor {
        Anchor::Cursor(x, y) => (x, y),
        Anchor::Gear => (ww - w - 4.0, vp.plot_h() - h - 4.0),
        Anchor::Below(by) => {
            let below = by + 14.0;
            (vp.plot_w() - w - 4.0, if below + h <= vp.plot_h() { below } else { by - 14.0 - h })
        }
    };
    if x + w > ww - 2.0 {
        x = (x - w).max(2.0).min(ww - w - 2.0).max(2.0);
    }
    if y + h > wh - 2.0 {
        y = (wh - h - 2.0).max(2.0);
    }
    y = y.max(2.0);
    let (rows, _) = rows_of(&es, y);
    let main = Panel { x, y, w, h, rows };
    let sub = om.sub.and_then(|k| match es.get(k) {
        Some(Entry::Sub { entries, .. }) => {
            let sw = width(entries);
            let max_h = wh - 4.0;
            let scroll = om.sub_scroll.min(entries.len().saturating_sub(fits(entries, max_h)));
            let (_, sh) = rows_from(entries, 0.0, scroll, max_h);
            let row_y = main.rows.iter().find(|r| r.2 == Some(k)).map_or(y, |r| r.0);
            let sx = if main.x + main.w + sw <= ww - 2.0 {
                main.x + main.w - 2.0
            } else {
                (main.x - sw + 2.0).max(2.0)
            };
            let sy = (row_y - PAD).min(wh - sh - 2.0).max(2.0);
            let (srows, _) = rows_from(entries, sy, scroll, max_h);
            Some((
                entries.clone(),
                Panel {
                    x: sx,
                    y: sy,
                    w: sw,
                    h: sh,
                    rows: srows,
                },
            ))
        }
        _ => None,
    });
    (es, main, sub)
}

fn inside(p: &Panel, x: f64, y: f64) -> bool {
    x >= p.x && x < p.x + p.w && y >= p.y && y < p.y + p.h
}

fn row_at(p: &Panel, x: f64, y: f64) -> Option<usize> {
    if !inside(p, x, y) {
        return None;
    }
    p.rows.iter().find(|r| y >= r.0 && y < r.0 + r.1).and_then(|r| r.2)
}

/// Résultat d'un clic quand un menu est ouvert.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Click {
    /// Une ligne active : le menu se ferme, l'action est à faire.
    Action(ChartAction),
    /// Dans le menu, sans action (ligne en gris, sous-menu, séparateur) : rien.
    Inside,
    /// À côté : le menu se ferme.
    Outside,
}

impl ContextMenu {
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Ouvre un menu (le prix est figé pour les lignes « à {prix} »).
    pub fn show(&mut self, source: Source, anchor: Anchor, price: f64, decimals: usize) {
        self.open = Some(OpenMenu {
            source,
            anchor,
            price,
            decimals,
            hover: None,
            sub: None,
            sub_scroll: 0,
        });
    }

    /// Survol : surligne la ligne et ouvre le sous-menu sous la souris (il reste ouvert pendant qu'on y va).
    pub fn on_move(&mut self, vp: &Viewport, ctx: &Ctx, x: f64, y: f64) {
        let Some(om) = self.open.as_mut() else { return };
        let (es, main, sub) = panels(vp, ctx, om);
        if let Some(k) = sub.as_ref().and_then(|(_, sp)| row_at(sp, x, y)) {
            om.hover = Some((1, k));
        } else if let Some(k) = row_at(&main, x, y) {
            om.hover = Some((0, k));
            let sub = matches!(es[k], Entry::Sub { .. }).then_some(k);
            if sub != om.sub {
                // nouveau sous-menu : il s'ouvre sur la ligne cochée (ex. le fuseau choisi)
                om.sub_scroll = match sub.map(|k| &es[k]) {
                    Some(Entry::Sub { entries, .. }) => entries
                        .iter()
                        .position(|e| matches!(e, Entry::Item { checked: true, .. }))
                        .map_or(0, |c| c.saturating_sub(3)),
                    _ => 0,
                };
            }
            om.sub = sub;
        } else {
            om.hover = None;
        }
    }

    /// Molette sur un sous-menu trop long : le fait défiler. `true` si la molette a servi au menu.
    pub fn scroll(&mut self, vp: &Viewport, ctx: &Ctx, x: f64, y: f64, steps: f64) -> bool {
        let Some(om) = self.open.as_mut() else { return false };
        let (_, _, sub) = panels(vp, ctx, om);
        let Some((ses, sp)) = sub.filter(|(_, sp)| inside(sp, x, y)) else {
            return false;
        };
        let room = ses.len().saturating_sub(fits(&ses, vp.height as f64 - 4.0));
        let lines = (steps.abs().ceil() as usize).max(1) * 3;
        om.sub_scroll = if steps > 0.0 {
            om.sub_scroll.saturating_sub(lines)
        } else {
            (om.sub_scroll + lines).min(room)
        };
        let _ = sp;
        om.hover = None;
        true
    }

    /// Clic gauche (menu ouvert).
    pub fn click(&mut self, vp: &Viewport, ctx: &Ctx, x: f64, y: f64) -> Click {
        let Some(om) = self.open else { return Click::Outside };
        let (es, main, sub) = panels(vp, ctx, &om);
        let picked = if let Some((ses, sp)) = sub.as_ref().filter(|(_, sp)| inside(sp, x, y)) {
            row_at(sp, x, y).map(|k| ses[k].clone())
        } else if inside(&main, x, y) {
            row_at(&main, x, y).map(|k| es[k].clone())
        } else {
            self.open = None;
            return Click::Outside;
        };
        match picked {
            Some(Entry::Item { action: Some(a), .. }) => {
                self.open = None;
                Click::Action(a)
            }
            Some(Entry::Sub { .. }) => {
                if let Some(o) = self.open.as_mut() {
                    o.sub = main.rows.iter().find(|r| y >= r.0 && y < r.0 + r.1).and_then(|r| r.2);
                }
                Click::Inside
            }
            _ => Click::Inside,
        }
    }
}

// ---------- boutons qui ouvrent des menus ----------

/// Bouton ⚙ : le coin en bas à droite (sous l'échelle de prix, à droite de l'échelle de temps).
pub fn gear_rect(vp: &Viewport) -> (f64, f64, f64, f64) {
    (vp.plot_w(), vp.plot_h(), vp.price_axis as f64, vp.time_axis as f64)
}

/// La souris est-elle sur le bouton ⚙ ?
pub fn on_gear(vp: &Viewport, x: f64, y: f64) -> bool {
    let (gx, gy, gw, gh) = gear_rect(vp);
    x >= gx && x <= gx + gw && y >= gy && y <= gy + gh
}

const PLUS_R: f64 = 8.0;
/// Le bouton ➕ du réticule n'est plus là (ni au doigt ni à la souris).
const PLUS_REMOVED: bool = true;

/// Centre du bouton ➕ pour un réticule à la hauteur `y`.
pub fn plus_center(vp: &Viewport, y: f64) -> (f64, f64) {
    (vp.plot_w() - 14.0, y)
}

/// La souris (réticule en `cross`) est-elle sur le bouton ➕ ? (jamais s'il est caché dans les réglages)
pub fn on_plus(vp: &Viewport, st: &ChartSettings, cross: Option<(f64, f64)>, x: f64, y: f64) -> bool {
    // ➕ retiré (demande de Jee) : le clic droit / l'appui long ouvrent le même menu, ligne horizontale comprise
    if PLUS_REMOVED {
        return false;
    }
    let Some((_, cy)) = cross.filter(|(cx, cy)| *cx >= 0.0 && *cx <= vp.plot_w() && *cy >= 0.0 && *cy <= vp.plot_h()) else {
        return false;
    };
    let (bx, by) = plus_center(vp, cy);
    st.plus_button && (x - bx).hypot(y - by) <= PLUS_R + 3.0
}

/// Dessine le bouton ⚙ (surligné si son menu est ouvert).
pub fn draw_gear(pm: &mut Pixmap, vp: &Viewport, open: bool, th: &Theme) {
    let (gx, gy, gw, gh) = gear_rect(vp);
    if open {
        fill(pm, gx + 1.0, gy + 1.0, gw - 1.0, gh - 1.0, pal::hover(), 1.0);
    }
    font().draw(pm, "⚙", ((gx + gw / 2.0) as f32, (gy + gh / 2.0 + 6.0) as f32), 17.0, th.text, Align::Center);
}

/// Dessine le bouton ➕ (rond avec un +) à la hauteur `y`.
pub fn draw_plus(pm: &mut Pixmap, vp: &Viewport, y: f64, hover: bool) {
    let (bx, by) = plus_center(vp, y.round());
    let bg = if hover { pal::knob() } else { pal::label() };
    let mut p = Paint::default();
    p.set_color_rgba8(bg.0, bg.1, bg.2, 255);
    if let Some(c) = PathBuilder::from_circle(bx as f32, by as f32, PLUS_R as f32) {
        pm.fill_path(&c, &p, FillRule::Winding, crate::px::t(), None);
        p.set_color_rgba8(0xe6, 0xe8, 0xee, 255);
        let s = Stroke {
            width: 1.2,
            ..Stroke::default()
        };
        pm.stroke_path(&c, &p, &s, crate::px::t(), None);
    }
    fill(pm, bx - 4.0, by - 0.5, 9.0, 1.0, WHITE, 1.0);
    fill(pm, bx - 0.5, by - 4.0, 1.0, 9.0, WHITE, 1.0);
}

fn draw_panel(pm: &mut Pixmap, es: &[Entry], p: &Panel, hover: Option<usize>, open_sub: Option<usize>, th: &Theme) {
    let t = font();
    let line = pal::label();
    fill(pm, p.x - 1.0, p.y - 1.0, p.w + 2.0, p.h + 2.0, line, 1.0);
    fill(pm, p.x, p.y, p.w, p.h, Rgb(0x1e, 0x22, 0x2d), 1.0);
    let white = WHITE;
    let grey = Rgb(0x6a, 0x6e, 0x78);
    for &(ry, rh, idx) in &p.rows {
        let Some(k) = idx else {
            fill(pm, p.x, ry + rh / 2.0 - 0.5, p.w, 1.0, line, 1.0);
            continue;
        };
        let base = (ry + rh / 2.0 + 5.0) as f32;
        match &es[k] {
            Entry::Item { icon, label, checked, action } => {
                let on = action.is_some();
                if on && hover == Some(k) {
                    fill(pm, p.x, ry, p.w, rh, pal::hover(), 1.0);
                }
                let col = if on { white } else { grey };
                if *checked {
                    t.draw(pm, "✓", ((p.x + 14.0) as f32, base), 14.0, col, Align::Left);
                } else if let Some(name) = crate::phosphor::from_emoji(icon) {
                    crate::phosphor::draw(pm, name, p.x + 20.0, ry + rh / 2.0, 16, col);
                } else if *icon == "⤢" {
                    // (⤢ is not in the fonts: the two diagonal arrows of the panels' ⤢)
                    crate::shapes::lines(
                        pm,
                        p.x + 20.0,
                        ry + rh / 2.0,
                        &[
                            &[(-5.0, 5.0), (5.0, -5.0)],
                            &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                            &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                        ],
                        col,
                        1.3,
                    );
                } else if !icon.is_empty() {
                    t.draw(pm, icon, ((p.x + 20.0) as f32, base), 15.0, col, Align::Center);
                }
                t.draw(pm, label, ((p.x + 38.0) as f32, base), SIZE, col, Align::Left);
                // (📱 none in phone view: no keyboard)
                if let Some(sc) = action.and_then(actions::shortcut_bulle) {
                    t.draw(pm, &sc, ((p.x + p.w - 12.0) as f32, base), 12.0, th.text, Align::Right);
                }
            }
            Entry::Sub { label, .. } => {
                if hover == Some(k) || open_sub == Some(k) {
                    fill(pm, p.x, ry, p.w, rh, pal::hover(), 1.0);
                }
                t.draw(pm, label, ((p.x + 38.0) as f32, base), SIZE, white, Align::Left);
                t.draw(pm, "›", ((p.x + p.w - 14.0) as f32, base), 16.0, th.text, Align::Right);
            }
            Entry::Sep => {}
        }
    }
}

/// Dessine le menu ouvert (et son sous-menu).
pub fn draw(pm: &mut Pixmap, vp: &Viewport, ctx: &Ctx, cm: &ContextMenu, th: &Theme) {
    let Some(om) = &cm.open else { return };
    let (es, main, sub) = panels(vp, ctx, om);
    let hov = |lvl| om.hover.filter(|h| h.0 == lvl).map(|h| h.1);
    draw_panel(pm, &es, &main, hov(0), om.sub, th);
    if let Some((ses, sp)) = &sub {
        draw_panel(pm, ses, sp, hov(1), None, th);
        // flèches : il y a d'autres lignes au-dessus / en dessous (molette)
        let first = sp.rows.first().and_then(|r| r.2).unwrap_or(0);
        let last = sp.rows.iter().rev().find_map(|r| r.2).unwrap_or(0);
        if first > 0 {
            font().draw(pm, "▲", ((sp.x + sp.w - 14.0) as f32, (sp.y + 14.0) as f32), 10.0, th.text, Align::Center);
        }
        if last + 1 < ses.len() {
            font().draw(pm, "▼", ((sp.x + sp.w - 14.0) as f32, (sp.y + sp.h - 6.0) as f32), 10.0, th.text, Align::Center);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📱 Jee (21 Nov, list 2): in phone view the chart menus write no shortcut (« Alt + R » was still drawn on
    /// the price axis), show no grey row, and put the technical rows of the price axis in « Avancé ».
    #[test]
    fn au_doigt_ni_raccourci_ni_ligne_grise() {
        let st = ChartSettings::default();
        let ctx = Ctx {
            settings: &st,
            drawings: 0,
            now: 0,
            tpo: false,
        };
        let mut cm = ContextMenu::default();
        cm.show(Source::PriceAxis, Anchor::Cursor(700.0, 100.0), 80000.0, 1);
        let mut pm = Pixmap::new(1400, 800).unwrap();
        let th = Theme::default();
        // 🖥 PC: everything, shortcuts written
        actions::set_tactile(false);
        let _ = crate::text::ecrits();
        draw(&mut pm, &vp(), &ctx, &cm, &th);
        assert!(crate::text::ecrits().iter().any(|s| s.contains("Alt")), "PC: « Alt + R » written");
        let pc = entries(Source::PriceAxis, &ctx, &cm.open.unwrap());
        assert!(pc.iter().any(|e| matches!(e, Entry::Item { action: None, .. })), "PC: the grey rows stay");
        // 📱 phone view
        actions::set_tactile(true);
        draw(&mut pm, &vp(), &ctx, &cm, &th);
        let ecrits = crate::text::ecrits();
        assert!(!ecrits.iter().any(|s| s.contains("Alt")), "phone view: no shortcut: {ecrits:?}");
        fn gris(es: &[Entry]) -> bool {
            es.iter().any(|e| match e {
                Entry::Item { action: None, .. } => true,
                Entry::Sub { entries, .. } => entries.is_empty() || gris(entries),
                _ => false,
            })
        }
        for src in [Source::PriceAxis, Source::TimeAxis, Source::Chart, Source::Plus] {
            let es = entries(src, &ctx, &cm.open.unwrap());
            assert!(!gris(&es), "{src:?}: no grey row nor empty sub-menu");
            assert!(es.first() != Some(&Entry::Sep) && es.last() != Some(&Entry::Sep), "{src:?}: no dangling line");
            assert!(es.windows(2).all(|w| !(w[0] == Entry::Sep && w[1] == Entry::Sep)), "{src:?}: no double line");
        }
        let es = entries(Source::PriceAxis, &ctx, &cm.open.unwrap());
        let avance = es.iter().find_map(|e| match e {
            Entry::Sub { label, entries } if label == "Avancé" => Some(entries.len()),
            _ => None,
        });
        assert_eq!(avance, Some(3), "the 3 technical rows in « Avancé »");
        assert!(
            !es.iter().any(|e| matches!(e, Entry::Item { label, .. } if AVANCE.contains(&label.as_str()))),
            "and not twice"
        );
        actions::set_tactile(false);
    }

    fn vp() -> Viewport {
        Viewport {
            width: 1400,
            height: 800,
            ..Viewport::default()
        }
    }

    fn center(p: &Panel, k: usize) -> (f64, f64) {
        let r = p.rows.iter().find(|r| r.2 == Some(k)).unwrap();
        (p.x + 60.0, r.0 + r.1 / 2.0)
    }

    #[test]
    fn reglages_tpo_au_clic_droit() {
        let mut st = ChartSettings::default();
        let om = OpenMenu {
            source: Source::Chart,
            anchor: Anchor::Cursor(0.0, 0.0),
            price: 1.0,
            decimals: 2,
            hover: None,
            sub: None,
            sub_scroll: 0,
        };
        let sub = |st: &ChartSettings, tpo: bool| {
            entries(
                Source::Chart,
                &Ctx {
                    settings: st,
                    drawings: 0,
                    now: 0,
                    tpo,
                },
                &om,
            )
            .into_iter()
            .find_map(|e| match e {
                Entry::Sub { label, entries } if label == "Réglages TPO" => Some(entries),
                _ => None,
            })
        };
        assert!(sub(&st, false).is_none(), "seulement en TPO");
        let e = sub(&st, true).expect("réglages TPO");
        assert!(
            e.contains(&item("", "Lettre : 30 min", true, ChartAction::TpoBlock(1800))),
            "30 min coché par défaut"
        );
        assert!(actions::apply_settings(ChartAction::TpoBlock(3600), &mut st, 7.0, None));
        assert!(actions::apply_settings(ChartAction::Toggle(Flag::TpoLetters), &mut st, 7.0, None));
        assert_eq!((st.tpo.block, st.tpo.letters), (3600, false));
    }

    #[test]
    fn sous_menu_s_ouvre_a_cote_au_survol() {
        let st = ChartSettings::default();
        let ctx = Ctx {
            settings: &st,
            drawings: 0,
            now: 1_792_929_600,
            tpo: false,
        };
        let v = vp();
        let mut cm = ContextMenu::default();
        cm.show(Source::PriceAxis, Anchor::Cursor(1300.0, 200.0), 80000.0, 1);
        let (es, main, sub) = panels(&v, &ctx, &cm.open.unwrap());
        assert!(sub.is_none());
        assert!(main.x + main.w <= v.width as f64, "le menu reste dans la fenêtre (ouvert vers la gauche)");
        let k = es.iter().position(|e| matches!(e, Entry::Sub { label, .. } if label == "Étiquettes")).unwrap();
        let (x, y) = center(&main, k);
        cm.on_move(&v, &ctx, x, y);
        let (_, main2, sub) = panels(&v, &ctx, &cm.open.unwrap());
        let (ses, sp) = sub.expect("le sous-menu Étiquettes s'ouvre");
        assert!(sp.x + sp.w <= main2.x + 2.0 || sp.x >= main2.x + main2.w - 2.0, "à côté du menu, pas dessus");
        // cliquer « Étiquettes de prix haut et bas » dans le sous-menu
        let j = ses
            .iter()
            .position(|e| matches!(e, Entry::Item { label, .. } if label.contains("haut et bas")))
            .unwrap();
        let (sx, sy) = (sp.x + 60.0, sp.rows.iter().find(|r| r.2 == Some(j)).unwrap().0 + 10.0);
        cm.on_move(&v, &ctx, sx, sy);
        assert_eq!(cm.open.unwrap().sub, Some(k), "le sous-menu reste ouvert quand on y va");
        assert_eq!(cm.click(&v, &ctx, sx, sy), Click::Action(ChartAction::Toggle(Flag::HighLowLabel)));
        assert!(!cm.is_open(), "le menu se ferme après une action");
    }

    #[test]
    fn lignes_en_gris_et_clic_a_cote() {
        let st = ChartSettings::default();
        let ctx = Ctx {
            settings: &st,
            drawings: 2,
            now: 0,
            tpo: false,
        };
        let v = vp();
        let mut cm = ContextMenu::default();
        cm.show(Source::Chart, Anchor::Cursor(200.0, 100.0), 83805.8, 1);
        let (es, main, _) = panels(&v, &ctx, &cm.open.unwrap());
        assert!(matches!(&es[2], Entry::Item { label, action: Some(ChartAction::CopyPrice(p)), .. } if label.contains("83\u{202f}805,8") && *p == 83805.8));
        let grey = es
            .iter()
            .position(|e| matches!(e, Entry::Item { label, .. } if label.contains("Coller")))
            .unwrap();
        // 💼 préparer un achat / une vente au prix du clic : de vraies actions (plus de « bientôt »)
        for buy in [true, false] {
            assert!(es
                .iter()
                .any(|e| matches!(e, Entry::Item { action: Some(ChartAction::Brouillon(p, b)), .. } if *p == 83805.8 && *b == buy)));
        }
        let (x, y) = center(&main, grey);
        assert_eq!(cm.click(&v, &ctx, x, y), Click::Inside, "ligne en gris : rien");
        assert!(cm.is_open());
        // 🔔 l'alerte au prix du clic est une vraie action
        assert!(es
            .iter()
            .any(|e| matches!(e, Entry::Item { action: Some(ChartAction::AddAlert(p)), .. } if *p == 83805.8)));
        assert!(es.iter().any(|e| matches!(
            e,
            Entry::Item {
                action: Some(ChartAction::ClearDrawings),
                ..
            }
        )));
        assert_eq!(cm.click(&v, &ctx, 5.0, 790.0), Click::Outside);
        assert!(!cm.is_open());
    }

    #[test]
    fn long_sous_menu_defile_et_s_ouvre_sur_le_choix() {
        let st = ChartSettings {
            timezone: "Asia/Tokyo".into(),
            ..Default::default()
        };
        let ctx = Ctx {
            settings: &st,
            drawings: 0,
            now: 1_792_929_600,
            tpo: false,
        };
        let v = vp();
        let mut cm = ContextMenu::default();
        cm.show(Source::TimeAxis, Anchor::Cursor(500.0, 760.0), 0.0, 0);
        let (es, main, _) = panels(&v, &ctx, &cm.open.unwrap());
        let (x, y) = center(&main, 2);
        cm.on_move(&v, &ctx, x, y);
        let (_, _, sub) = panels(&v, &ctx, &cm.open.unwrap());
        let (ses, sp) = sub.unwrap();
        assert!(sp.y >= 0.0 && sp.y + sp.h <= v.height as f64, "le sous-menu tient dans la fenêtre");
        assert!(sp.rows.len() < ses.len(), "trop long : une partie seulement");
        let tokyo = ses.iter().position(|e| matches!(e, Entry::Item { checked: true, .. })).unwrap();
        assert!(sp.rows.iter().any(|r| r.2 == Some(tokyo)), "s'ouvre sur le fuseau choisi (Tokyo)");
        // la molette fait défiler jusqu'au bout, sans dépasser
        let (sx, sy) = (sp.x + 30.0, sp.y + 40.0);
        for _ in 0..30 {
            assert!(cm.scroll(&v, &ctx, sx, sy, -1.0));
        }
        let (_, _, sub) = panels(&v, &ctx, &cm.open.unwrap());
        let (ses, sp) = sub.unwrap();
        assert_eq!(sp.rows.last().unwrap().2, Some(ses.len() - 1), "dernière ligne atteinte");
        let _ = es;
        assert!(!cm.scroll(&v, &ctx, 5.0, 5.0, -1.0), "molette hors du sous-menu : pas pour le menu");
    }

    #[test]
    fn fuseaux_et_boutons() {
        let st = ChartSettings {
            timezone: "America/New_York".into(),
            ..Default::default()
        };
        let ctx = Ctx {
            settings: &st,
            drawings: 0,
            now: 1_792_929_600,
            tpo: false,
        };
        let om = OpenMenu {
            source: Source::TimeAxis,
            anchor: Anchor::Cursor(500.0, 760.0),
            price: 0.0,
            decimals: 0,
            hover: None,
            sub: None,
            sub_scroll: 0,
        };
        let es = entries(Source::TimeAxis, &ctx, &om);
        let Entry::Sub { entries: zones, .. } = &es[2] else {
            panic!("Fuseau horaire")
        };
        let checked: Vec<_> = zones
            .iter()
            .filter_map(|e| match e {
                Entry::Item { label, checked: true, .. } => Some(label.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(checked, vec!["(UTC-4) New York".to_string()], "un seul fuseau coché");
        // boutons
        let v = vp();
        assert!(on_gear(&v, v.plot_w() + 10.0, v.plot_h() + 10.0));
        // bouton ➕ du réticule retiré : jamais touché, même réglage allumé
        let (bx, by) = plus_center(&v, 300.0);
        let mut on = st.clone();
        on.plus_button = true;
        assert!(!on_plus(&v, &on, Some((500.0, 300.0)), bx, by), "plus de bouton +");
        // la ligne horizontale est dans le menu du clic droit / de l'appui long
        let m = entries(
            Source::Chart,
            &ctx,
            &OpenMenu {
                source: Source::Chart,
                price: 100.0,
                ..om
            },
        );
        assert!(m.iter().any(|e| matches!(
            e,
            Entry::Item {
                action: Some(ChartAction::HLine(_)),
                ..
            }
        )));
    }
}
