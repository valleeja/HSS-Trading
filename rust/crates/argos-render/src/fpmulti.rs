//! 🏦 Footprint multi-bourses (maquette v1 validée par Jee le 22 nov. 2026,
//! `simulateur-demo/2026-11-22-maquette-footprint-multi-v1-{pc,pc-reglages,cell}.png`) : un seul footprint avec les
//! vrais trades de la pièce sur plusieurs bourses, additionnés prix par prix (`argos_core::orderflow::fusionner`).
//! Ici : ce que l'écran montre en plus du footprint d'aujourd'hui — les points de couleur des bourses dans la légende,
//! la petite barre « part de chaque bourse » sous chaque bougie, la bulle du détail par bourse d'une rangée (🖥 au
//! survol, 📱 à l'appui) et le ⚙ de la légende (🖥 panneau de verre sous la légende, 📱 feuille de verre en bas).
//! Le choix des bourses ([`choisies`]) suit les réglages du groupe « 🏦 Bourses » du Footprint ; la fenêtre branche
//! les flux et additionne (elle seule connaît les bourses).

use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::orderflow::{Bourse, FlowBar};
use argos_script::indicators::flow::{FOOTPRINT_BOURSES, FOOTPRINT_CASES, FOOTPRINT_PARAMS};
use argos_script::inputs::Inputs;
use tiny_skia::Pixmap;

/// Couleur de chaque bourse de `FOOTPRINT_BOURSES` (maquette : Binance jaune, Bybit violet, OKX blanc, Coinbase bleu,
/// Hyperliquid rose ; puis des couleurs bien à part pour les cinq autres).
pub const COULEURS: [Rgb; 10] = [
    Rgb(0xf0, 0xb9, 0x0b),
    Rgb(0xb3, 0x9d, 0xff),
    Rgb(0xe6, 0xe9, 0xf2),
    Rgb(0x3d, 0x8b, 0xff),
    Rgb(0xff, 0x7a, 0xc6),
    Rgb(0x2d, 0xd4, 0xbf),
    Rgb(0xd4, 0xa3, 0x73),
    Rgb(0xa3, 0xe6, 0x35),
    Rgb(0x94, 0xa3, 0xb8),
    Rgb(0x7d, 0xd3, 0xfc),
];
/// Couleur d'une bourse de chart hors de la liste (Binance comptant, Bitunix…).
pub const AUTRE: Rgb = Rgb(0x9a, 0xa4, 0xbf);
/// Bourses prises par « Auto ».
pub const AUTO_N: usize = 5;

// — couleurs de la maquette (fp.css) —
const FOND: (Rgb, f32) = (Rgb(26, 30, 42), 0.72);
const TITRE: Rgb = Rgb(0xe8, 0xec, 0xf7);
const DOUX: Rgb = Rgb(0x93, 0xa0, 0xc2);
const GRIS: Rgb = Rgb(0x8a, 0x93, 0xad);
const TEXTE: Rgb = Rgb(0xc9, 0xd3, 0xee);
const CLAIR: Rgb = Rgb(0xdf, 0xe5, 0xf5);
const ETEINT: Rgb = Rgb(0x7d, 0x87, 0xa5);
const BLEU: Rgb = Rgb(0x4d, 0xa3, 0xff);
const LIEN: Rgb = Rgb(0x9c, 0xc3, 0xff);
const REEL: Rgb = Rgb(0x9f, 0xe8, 0xff);
const ACHAT: Rgb = Rgb(0x26, 0xa6, 0x9a);
const VENTE: Rgb = Rgb(0xef, 0x53, 0x50);
const BLANC: Rgb = Rgb(0xff, 0xff, 0xff);

/// Les bourses fusionnées (indices dans `FOOTPRINT_BOURSES`), celle de la chart d'abord quand elle est dans la liste
/// (`chart`) : « Auto » prend ensuite les plus grosses dans l'ordre de la liste jusqu'à [`AUTO_N`] ; sinon les cases
/// cochées. `possible` : la bourse peut-elle lire les vrais trades de cette pièce (flux branché, taille lisible) ?
/// La bourse de la chart est toujours là (elle est la référence des prix) : décocher toutes les autres donne le
/// footprint d'une seule bourse.
pub fn choisies(vals: &[f64], chart: Option<usize>, possible: &dyn Fn(usize) -> bool) -> Vec<usize> {
    let p = Inputs {
        params: &FOOTPRINT_PARAMS,
        vals,
    };
    let mut v: Vec<usize> = chart.into_iter().collect();
    let auto = p.on("auto");
    for (k, case) in FOOTPRINT_CASES.iter().enumerate() {
        if Some(k) == chart || !possible(k) {
            continue;
        }
        if auto && v.len() + usize::from(chart.is_none()) >= AUTO_N {
            break;
        }
        if auto || p.on(case) {
            v.push(k);
        }
    }
    v
}

/// 🏦 Ce que le footprint multi-bourses montre en plus, pour l'écran.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FpMulti {
    /// Les bourses additionnées qui ont des vrais trades (celle de la chart d'abord) : nom et couleur.
    pub bourses: Vec<(String, Rgb)>,
    /// Les bougies de chaque bourse à l'unité de la chart (tailles en pièces, triées par heure).
    pub barres: Vec<Vec<FlowBar>>,
    /// L'écart ajouté aux prix de chaque bourse (0 : prix tels quels).
    pub ecarts: Vec<f64>,
    /// Pas commun des rangées additionnées ; tailles en $ ?
    pub pas: f64,
    pub dollars: bool,
    /// Par heure de bougie : le volume de chaque bourse (même ordre que `bourses`, même unité).
    pub parts: Vec<(i64, Vec<f64>)>,
    /// L'unité écrite : la pièce (« BTC ») ou « $ ».
    pub unite: String,
    /// La bourse de la chart (référence des prix), pour le ⚙.
    pub reference: String,
}

impl FpMulti {
    /// Additionne les bourses (`barres` et `ecarts` dans l'ordre de `bourses`) au plus petit de leurs pas ; rend le
    /// footprint multi-bourses et l'orderflow additionné (à passer au footprint). Une bourse sans aucune bougie est
    /// retirée (elle ne compte pas dans « N bourses »).
    pub fn de(bourses: Vec<(String, Rgb)>, barres: Vec<Vec<FlowBar>>, ecarts: Vec<f64>, dollars: bool, unite: &str, reference: &str) -> (Self, Vec<FlowBar>) {
        let garder: Vec<usize> = (0..bourses.len()).filter(|&k| barres.get(k).is_some_and(|b| !b.is_empty())).collect();
        let bourses: Vec<(String, Rgb)> = garder.iter().map(|&k| bourses[k].clone()).collect();
        let ecarts: Vec<f64> = garder.iter().map(|&k| ecarts.get(k).copied().unwrap_or(0.0)).collect();
        let barres: Vec<Vec<FlowBar>> = garder.iter().map(|&k| barres[k].clone()).collect();
        let pas = barres.iter().flatten().map(|b| b.pas).filter(|p| *p > 0.0).fold(f64::INFINITY, f64::min);
        let pas = if pas.is_finite() { pas } else { 0.0 };
        let (fusion, parts) = argos_core::orderflow::fusionner(&Self::sources(&barres, &ecarts), pas, dollars);
        let m = FpMulti {
            bourses,
            barres,
            ecarts,
            pas,
            dollars,
            parts,
            unite: if dollars { "$".into() } else { unite.into() },
            reference: reference.into(),
        };
        (m, fusion)
    }

    fn sources<'a>(barres: &'a [Vec<FlowBar>], ecarts: &[f64]) -> Vec<Bourse<'a>> {
        barres.iter().zip(ecarts).map(|(b, &e)| Bourse { bars: b, ecart: e }).collect()
    }

    /// (achats, ventes) de chaque bourse dans la rangée `j` (pas de l'écran `step`) de la bougie de `time`.
    pub fn detail(&self, time: i64, step: f64, j: i64) -> Vec<(f64, f64)> {
        argos_core::orderflow::detail(&Self::sources(&self.barres, &self.ecarts), self.pas, self.dollars, time, step, j)
    }

    /// Le volume de chaque bourse dans la bougie de `time`.
    pub fn parts_de(&self, time: i64) -> Option<&[f64]> {
        self.parts.binary_search_by_key(&time, |p| p.0).ok().map(|k| self.parts[k].1.as_slice())
    }

    /// Les bourses dont la bougie de `time` a manqué un bout (coupure, démarrage).
    pub fn incompletes(&self, time: i64) -> Vec<&str> {
        self.barres
            .iter()
            .zip(&self.bourses)
            .filter(|(b, _)| b.binary_search_by_key(&time, |x| x.time).is_ok_and(|k| !b[k].complete))
            .map(|(_, (n, _))| n.as_str())
            .collect()
    }

    /// « 5 bourses » (« 1 bourse »).
    pub fn nombre(&self) -> String {
        let n = self.bourses.len();
        format!("{n} bourse{}", if n > 1 { "s" } else { "" })
    }
}

/// Un volume écrit dans la bulle : pièces avec une décimale (deux sous 1), dollars en court (« 1,2 M »).
pub fn volume(x: f64, dollars: bool) -> String {
    if dollars {
        return argos_core::nombres::kf(x);
    }
    let d = if x.abs() >= 100.0 {
        0
    } else if x.abs() >= 1.0 {
        1
    } else {
        2
    };
    argos_core::nombres::format_number(x, d)
}

// ───────────────────────────── légende ─────────────────────────────

/// Taille des points des bourses dans la légende, et leur écart.
const POINT: f64 = 8.0;
const POINT_PAS: f64 = 11.0;

/// 🖥 Largeur de ce qui suit le titre dans la légende : les points des bourses, puis « ■ réel ».
pub fn legende_suite(text: &Text, m: &FpMulti, avec_reel: bool) -> f64 {
    let n = m.bourses.len() as f64;
    let points = if n > 0.0 { 6.0 + n * POINT_PAS } else { 0.0 };
    points + if avec_reel { 8.0 + 11.0 + text.width("réel", 12.0) as f64 } else { 0.0 }
}

/// Dessine les points des bourses (et « ■ réel ») à partir de `x`, centrés sur `cy`.
pub fn legende_dessin(pm: &mut Pixmap, text: &Text, m: &FpMulti, x: f64, cy: f64, avec_reel: bool) {
    let mut x = x + 6.0;
    for (_, c) in &m.bourses {
        round_fill(pm, Rect::new(x, cy - POINT / 2.0, POINT, POINT), POINT / 2.0, *c, 1.0);
        x += POINT_PAS;
    }
    if avec_reel {
        x += 8.0 - (POINT_PAS - POINT);
        crate::fill(pm, x, cy - 3.0, 6.0, 6.0, Rgb(0x3f, 0xd0, 0xf0), 1.0);
        text.draw(pm, "réel", ((x + 11.0) as f32, (cy + 4.2) as f32), 12.0, REEL, Align::Left);
    }
}

// ───────────────────────────── petite barre ─────────────────────────────

/// La petite barre « part de chaque bourse » (largeur `w`, centrée sur `xc`, en haut à `y`) : un morceau par bourse,
/// à sa couleur, de la longueur de sa part du volume.
pub fn bande(pm: &mut Pixmap, m: &FpMulti, parts: &[f64], xc: f64, y: f64, w: f64) {
    let tot: f64 = parts.iter().filter(|v| **v > 0.0).sum();
    if !(tot > 0.0) || w < 4.0 {
        return;
    }
    let mut x = xc - w / 2.0;
    for (k, v) in parts.iter().enumerate() {
        let l = w * v.max(0.0) / tot;
        if l > 0.0 {
            let c = m.bourses.get(k).map_or(AUTRE, |b| b.1);
            crate::fill(pm, x, y, l, 4.0, c, 1.0);
        }
        x += l;
    }
}

// ───────────────────────────── bulle du détail ─────────────────────────────

/// La bulle du détail d'une rangée : son titre (« 62 040 – 62 050 »), son heure (« 1 min · 10:44 »), les totaux et
/// une ligne par bourse (ventes à gauche, achats à droite, comme la cellule).
#[derive(Debug, Clone, PartialEq)]
pub struct Bulle {
    pub rangee: String,
    pub heure: String,
    pub unite: String,
    pub dollars: bool,
    /// (nom, couleur, ventes, achats)
    pub lignes: Vec<(String, Rgb, f64, f64)>,
    /// Les bourses qui ont manqué un bout de cette bougie.
    pub incompletes: Vec<String>,
}

const BULLE_W: f64 = 236.0;
const BULLE_LIGNE: f64 = 19.0;

/// « 1 min », « 15 min », « 4 h », « 1 j » : l'unité de la chart dans la bulle.
pub fn unite_temps(tf_s: i64) -> String {
    match tf_s {
        s if s < 3600 => format!("{} min", (s / 60).max(1)),
        s if s < 86_400 => format!("{} h", s / 3600),
        s => format!("{} j", s / 86_400),
    }
}

impl Bulle {
    /// La bulle de la rangée `j` (pas `step`) de la bougie de `time` ; `None` si aucune bourse n'y a de volume.
    /// `prix` écrit un prix, `heure` l'heure de la bougie.
    pub fn de(m: &FpMulti, time: i64, step: f64, j: i64, prix: &dyn Fn(f64) -> String, heure: String) -> Option<Bulle> {
        let d = m.detail(time, step, j);
        if d.iter().all(|(a, v)| *a + *v <= 0.0) {
            return None;
        }
        Some(Bulle {
            rangee: format!("{} – {}", prix(j as f64 * step), prix((j + 1) as f64 * step)),
            heure,
            unite: m.unite.clone(),
            dollars: m.dollars,
            lignes: m.bourses.iter().zip(d).map(|((n, c), (a, v))| (n.clone(), *c, v, a)).collect(),
            incompletes: m.incompletes(time).into_iter().map(String::from).collect(),
        })
    }

    fn total(&self, ventes: bool) -> String {
        let s: f64 = self.lignes.iter().map(|l| if ventes { l.2 } else { l.3 }).sum();
        let v = volume(s, self.dollars);
        if self.dollars {
            format!("{} {v} $", if ventes { "Ventes" } else { "Achats" })
        } else {
            format!("{} {v} {}", if ventes { "Ventes" } else { "Achats" }, self.unite)
        }
    }

    fn manque(&self) -> Option<String> {
        (!self.incompletes.is_empty()).then(|| format!("● {} : un bout manque", self.incompletes.join(", ")))
    }

    /// Taille de la bulle.
    pub fn taille(&self) -> (f64, f64) {
        let h = 10.0 + 17.0 + 5.0 + 15.0 + 7.0 + self.lignes.len() as f64 * BULLE_LIGNE + 9.0 + if self.manque().is_some() { 18.0 } else { 0.0 };
        (BULLE_W, h)
    }

    /// Place de la bulle près de la souris (`mx`, `my`) dans la zone des prix `pw` × `ph` : au-dessus à droite, sinon
    /// à gauche ou dessous ; jamais hors de la chart.
    pub fn place(&self, mx: f64, my: f64, pw: f64, ph: f64) -> Rect {
        let (w, h) = self.taille();
        let x = if mx + 18.0 + w <= pw - 4.0 { mx + 18.0 } else { (mx - 18.0 - w).max(4.0) };
        let y = if my - 18.0 - h >= 4.0 {
            my - 18.0 - h
        } else {
            (my + 18.0).min((ph - h - 4.0).max(4.0))
        };
        Rect::new(x, y, w, h)
    }

    /// Dessine la bulle en `r`.
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, r: Rect) {
        round_fill(pm, r, 16.0, crate::jetons::FOND, 0.9);
        round_fill(pm, r, 16.0, FOND.0, FOND.1);
        round_stroke(pm, r, 16.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let gras = Text::graisse(Graisse::Bold);
        let (x0, x1) = (r.x + 12.0, r.x + r.w - 12.0);
        let mut y = r.y + 10.0 + 12.5;
        gras.draw(pm, &self.rangee, (x0 as f32, y as f32), 12.5, TITRE, Align::Left);
        text.draw(pm, &self.heure, (x1 as f32, y as f32), 12.0, DOUX, Align::Right);
        y += 5.0 + 15.0;
        let v = self.total(true);
        text.draw(pm, &v, (x0 as f32, y as f32), 12.0, VENTE, Align::Left);
        text.draw(
            pm,
            &self.total(false),
            ((x0 + text.width(&v, 12.0) as f64 + 12.0) as f32, y as f32),
            12.0,
            ACHAT,
            Align::Left,
        );
        y += 7.0;
        let col = (x1 - x0 - 90.0) / 2.0;
        for (n, c, ven, ach) in &self.lignes {
            let cy = y + BULLE_LIGNE / 2.0;
            round_fill(pm, Rect::new(x0, cy - 4.0, 8.0, 8.0), 4.0, *c, 1.0);
            text.draw(pm, n, ((x0 + 12.0) as f32, (cy + 4.2) as f32), 12.0, TEXTE, Align::Left);
            text.draw(
                pm,
                &volume(*ven, self.dollars),
                ((x0 + 90.0 + col) as f32, (cy + 4.2) as f32),
                12.0,
                VENTE,
                Align::Right,
            );
            text.draw(pm, &volume(*ach, self.dollars), (x1 as f32, (cy + 4.2) as f32), 12.0, ACHAT, Align::Right);
            y += BULLE_LIGNE;
        }
        if let Some(t) = self.manque() {
            text.draw(pm, &t, (x0 as f32, (y + 14.0) as f32), 11.5, crate::palette::ORANGE, Align::Left);
        }
    }
}

// ───────────────────────────── ⚙ de la légende ─────────────────────────────

/// Ce que le ⚙ montre (les réglages du groupe « 🏦 Bourses » du Footprint).
#[derive(Debug, Clone, PartialEq)]
pub struct Reglages {
    pub auto: bool,
    /// Les bourses fusionnées (indices de `FOOTPRINT_BOURSES`).
    pub choisies: Vec<usize>,
    /// La bourse de la chart dans la liste (toujours fusionnée, sa pastille ne se décoche pas).
    pub chart: Option<usize>,
    pub dollars: bool,
    pub aligne: bool,
    pub parts: bool,
    /// La pièce (« BTC ») et la bourse de la chart (« Binance Perpétuel »).
    pub piece: String,
    pub reference: String,
}

/// Un clic dans le ⚙.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clic {
    Fermer,
    Auto,
    Bourse(usize),
    Unite(bool),
    Aligne,
    Parts,
    Tous,
    /// Sur le panneau, rien dessous.
    Rien,
}

/// 🖥 Largeur du panneau (maquette : 392).
pub const LARGEUR: f64 = 392.0;
const PAD: f64 = 14.0;
const PUCE_H: f64 = 26.0;
const LIGNE_H: f64 = 40.0;
/// 📱 Haut de la feuille : la poignée.
const POIGNEE: f64 = 13.0;

/// Les parties du panneau, placées (coordonnées du panneau).
struct Mise {
    fermer: Rect,
    titre_y: f64,
    lb_bourses: f64,
    auto: Rect,
    puces: Vec<(Rect, usize)>,
    lb_unite: f64,
    seg: Rect,
    aligne: Rect,
    parts: Rect,
    tous: Rect,
    h: f64,
}

impl Reglages {
    fn puce_texte(&self, k: usize) -> String {
        if self.choisies.contains(&k) {
            format!("{} ✓", FOOTPRINT_BOURSES[k])
        } else {
            format!("+ {}", FOOTPRINT_BOURSES[k])
        }
    }

    fn puce_w(&self, text: &Text, k: usize) -> f64 {
        let dot = if self.choisies.contains(&k) { 14.0 } else { 0.0 };
        20.0 + dot + text.width(&self.puce_texte(k), 12.5) as f64
    }

    fn mise(&self, text: &Text, w: f64, cell: bool) -> Mise {
        let haut = if cell { 8.0 + POIGNEE } else { 12.0 };
        let fermer = Rect::new(w - PAD - 28.0, haut, 28.0, 28.0);
        let titre_y = haut + 19.0;
        let mut y = haut + 28.0 + 13.0;
        let lb_bourses = y + 15.0;
        let auto = Rect::new(w - PAD - 2.0 - 38.0, y, 38.0, 22.0);
        y += 22.0 + 7.0;
        let mut puces = vec![];
        let mut x = PAD;
        // les bourses fusionnées d'abord (dans l'ordre de la liste), puis les autres
        let ordre = (0..FOOTPRINT_BOURSES.len())
            .filter(|k| self.choisies.contains(k))
            .chain((0..FOOTPRINT_BOURSES.len()).filter(|k| !self.choisies.contains(k)));
        for k in ordre {
            let pw = self.puce_w(text, k);
            if x > PAD && x + pw > w - PAD {
                x = PAD;
                y += PUCE_H + 6.0;
            }
            puces.push((Rect::new(x, y, pw, PUCE_H), k));
            x += pw + 6.0;
        }
        y += PUCE_H + 13.0;
        let lb_unite = y + 12.0;
        y += 16.0 + 7.0;
        let seg = Rect::new(PAD, y, w - 2.0 * PAD, 31.0);
        y += 31.0 + 6.0;
        let aligne = Rect::new(PAD, y, w - 2.0 * PAD, LIGNE_H);
        y += LIGNE_H + 6.0;
        let parts = Rect::new(PAD, y, w - 2.0 * PAD, LIGNE_H);
        y += LIGNE_H + 11.0;
        let tw = text.width(crate::assistant::TOUS_REGLAGES, 12.5) as f64 + 24.0;
        let tous = Rect::new((w - tw) / 2.0, y - 4.0, tw, 24.0);
        y += 20.0 + if cell { 26.0 } else { 12.0 };
        Mise {
            fermer,
            titre_y,
            lb_bourses,
            auto,
            puces,
            lb_unite,
            seg,
            aligne,
            parts,
            tous,
            h: y,
        }
    }

    /// Hauteur du panneau sur la largeur `w` (📱 `cell` : la feuille, avec sa poignée et la marge du bas).
    pub fn hauteur(&self, text: &Text, w: f64, cell: bool) -> f64 {
        self.mise(text, w, cell).h
    }

    /// 🖥 Place du panneau : sous la ligne du Footprint dans la légende (`sous` : son bas, fenêtre), à `x` ;
    /// 📱 la feuille : toute la largeur, en bas de la fenêtre.
    pub fn place(&self, text: &Text, x: f64, sous: f64, win_w: f64, win_h: f64, cell: bool) -> Rect {
        if cell {
            let h = self.hauteur(text, win_w, true);
            return Rect::new(0.0, (win_h - h).max(0.0), win_w, h);
        }
        let h = self.hauteur(text, LARGEUR, false);
        Rect::new(x, sous.min(win_h - h - 8.0).max(0.0), LARGEUR, h)
    }

    /// Ce qui se clique, avec sa zone (coordonnées du panneau de largeur `w`), dans l'ordre de l'écran.
    pub fn zones(&self, text: &Text, w: f64, cell: bool) -> Vec<(Rect, Clic)> {
        let m = self.mise(text, w, cell);
        let mut z = vec![(m.fermer, Clic::Fermer), (m.auto, Clic::Auto)];
        z.extend(m.puces.iter().map(|(r, k)| (*r, Clic::Bourse(*k))));
        let s = m.seg;
        z.push((Rect::new(s.x, s.y, s.w / 2.0, s.h), Clic::Unite(false)));
        z.push((Rect::new(s.cx(), s.y, s.w / 2.0, s.h), Clic::Unite(true)));
        z.extend([(m.aligne, Clic::Aligne), (m.parts, Clic::Parts), (m.tous, Clic::Tous)]);
        z
    }

    /// Clic aux coordonnées du panneau (largeur `w`).
    pub fn click(&self, text: &Text, w: f64, cell: bool, x: f64, y: f64) -> Clic {
        let m = self.mise(text, w, cell);
        if m.fermer.contains(x, y) {
            return Clic::Fermer;
        }
        // l'interrupteur « Auto » et son texte
        let auto_zone = Rect::new(w / 2.0 - 30.0, m.auto.y - 4.0, w / 2.0 + 30.0 - PAD, m.auto.h + 8.0);
        if auto_zone.contains(x, y) {
            return Clic::Auto;
        }
        if let Some((_, k)) = m.puces.iter().find(|(r, _)| r.contains(x, y)) {
            return Clic::Bourse(*k);
        }
        if m.seg.contains(x, y) {
            return Clic::Unite(x >= m.seg.cx());
        }
        if m.aligne.contains(x, y) {
            return Clic::Aligne;
        }
        if m.parts.contains(x, y) {
            return Clic::Parts;
        }
        if m.tous.contains(x, y) {
            return Clic::Tous;
        }
        Clic::Rien
    }

    /// Le panneau (🖥 largeur [`LARGEUR`], 📱 `cell` : la feuille sur toute la largeur `w`) ; `survol` : la souris
    /// (coordonnées du panneau).
    pub fn draw(&self, text: &Text, w: f64, cell: bool, survol: Option<(f64, f64)>) -> Option<Pixmap> {
        let m = self.mise(text, w, cell);
        let mut pm = crate::px::new(w.ceil() as u32, m.h.ceil() as u32)?;
        // 🫧 le verre de la maquette (📱 la feuille : coins ronds en haut seulement)
        let (r, rayon) = if cell {
            (Rect::new(0.5, 0.5, w - 1.0, m.h + 40.0), 26.0)
        } else {
            (Rect::new(0.5, 0.5, w - 1.0, m.h - 1.0), 22.0)
        };
        // (style verre : un fond plein de la teinte du verre, que la pose en verre remplace par la chart floutée ;
        // les pastilles et les lignes, plus claires, restent par-dessus sans laisser voir la chart nette)
        if crate::glass::on() {
            round_fill(&mut pm, r, rayon, FOND.0, 1.0);
        } else {
            round_fill(&mut pm, r, rayon, crate::jetons::FOND, 0.94);
            round_fill(&mut pm, r, rayon, FOND.0, FOND.1);
        }
        round_stroke(&mut pm, r, rayon, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let dessus = |z: Rect| survol.is_some_and(|(x, y)| z.contains(x, y));
        if cell {
            round_fill(&mut pm, Rect::new(w / 2.0 - 20.0, 8.0, 40.0, 5.0), 2.5, BLANC, 0.3);
        }
        let gras = Text::graisse(Graisse::Bold);
        gras.draw(&mut pm, "👣 Footprint · bourses", (PAD as f32, m.titre_y as f32), 14.5, TITRE, Align::Left);
        let f = m.fermer;
        round_fill(&mut pm, f, 14.0, BLANC, if dessus(f) { 0.14 } else { 0.07 });
        let d = 4.5;
        lines(&mut pm, f.cx(), f.cy(), &[&[(-d, -d), (d, d)], &[(d, -d), (-d, d)]], TEXTE, 1.4);
        // Bourses fusionnées · Auto
        text.draw(
            &mut pm,
            "Bourses fusionnées",
            ((PAD + 2.0) as f32, m.lb_bourses as f32),
            12.0,
            GRIS,
            Align::Left,
        );
        text.draw(
            &mut pm,
            "Auto (les 5 plus gros volumes)",
            ((m.auto.x - 7.0) as f32, m.lb_bourses as f32),
            12.0,
            TEXTE,
            Align::Right,
        );
        interrupteur(&mut pm, m.auto, self.auto);
        for (q, k) in &m.puces {
            let on = self.choisies.contains(k);
            let (fond, a) = if on { (BLEU, 0.28) } else { (BLANC, if dessus(*q) { 0.12 } else { 0.07 }) };
            round_fill(&mut pm, *q, 12.0, fond, a);
            let mut x = q.x + 10.0;
            if on {
                round_fill(&mut pm, Rect::new(x, q.cy() - 4.0, 8.0, 8.0), 4.0, COULEURS[*k], 1.0);
                x += 14.0;
            }
            let c = if on { BLANC } else { ETEINT };
            text.draw(&mut pm, &self.puce_texte(*k), (x as f32, (q.cy() + 4.4) as f32), 12.5, c, Align::Left);
        }
        // Volume en : pièce | $
        text.draw(&mut pm, "Volume en", ((PAD + 2.0) as f32, m.lb_unite as f32), 12.0, GRIS, Align::Left);
        let s = m.seg;
        round_fill(&mut pm, s, 12.0, BLANC, 0.06);
        let moitie = |droite: bool| Rect::new(if droite { s.cx() } else { s.x + 3.0 }, s.y + 3.0, s.w / 2.0 - 3.0, s.h - 6.0);
        round_fill(&mut pm, moitie(self.dollars), 9.0, BLEU, 0.35);
        for (droite, t) in [(false, self.piece.as_str()), (true, "$")] {
            let q = moitie(droite);
            let c = if droite == self.dollars { BLANC } else { TEXTE };
            text.draw(&mut pm, t, (q.cx() as f32, (q.cy() + 4.4) as f32), 12.5, c, Align::Center);
        }
        let sous = format!("chaque bourse recalée sur {} (écart du moment)", self.reference);
        ligne(&mut pm, text, m.aligne, "Aligner les prix sur la chart", &sous, self.aligne);
        ligne(&mut pm, text, m.parts, "Part de chaque bourse", "la petite barre sous la bougie", self.parts);
        let t = m.tous;
        if dessus(t) {
            round_fill(&mut pm, t, 12.0, BLANC, 0.08);
        }
        text.draw(
            &mut pm,
            crate::assistant::TOUS_REGLAGES,
            (t.cx() as f32, (t.cy() + 4.4) as f32),
            12.5,
            LIEN,
            Align::Center,
        );
        Some(pm)
    }
}

/// Un interrupteur de la maquette (38 × 22).
fn interrupteur(pm: &mut Pixmap, r: Rect, on: bool) {
    round_fill(pm, r, 11.0, if on { BLEU } else { BLANC }, if on { 1.0 } else { 0.14 });
    let x = if on { r.x + 19.0 } else { r.x + 3.0 };
    round_fill(pm, Rect::new(x, r.y + 3.0, 16.0, 16.0), 8.0, if on { BLANC } else { TEXTE }, 1.0);
}

/// Une ligne à interrupteur : son titre, sa petite phrase (raccourcie si elle déborde), l'interrupteur à droite.
fn ligne(pm: &mut Pixmap, text: &Text, r: Rect, titre: &str, sous: &str, on: bool) {
    round_fill(pm, r, 14.0, BLANC, 0.05);
    text.draw(pm, titre, ((r.x + 12.0) as f32, (r.y + 17.0) as f32), 13.0, CLAIR, Align::Left);
    let place = r.w - 12.0 - 38.0 - 6.0 - 10.0;
    let (mut c, mut s): (Vec<char>, String) = (sous.chars().collect(), sous.to_string());
    while c.len() > 4 && text.width(&s, 11.0) as f64 > place {
        c.pop();
        s = format!("{}…", c.iter().collect::<String>().trim_end());
    }
    text.draw(pm, &s, ((r.x + 12.0) as f32, (r.y + 31.0) as f32), 11.0, GRIS, Align::Left);
    interrupteur(pm, Rect::new(r.x + r.w - 6.0 - 38.0, r.cy() - 11.0, 38.0, 22.0), on);
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_script::inputs::defaults;

    fn fb(time: i64, pas: f64, rangees: &[(i64, f64, f64)], complete: bool) -> FlowBar {
        FlowBar {
            time,
            pas,
            rangees: rangees.to_vec(),
            complete,
        }
    }

    fn regle(v: &mut [f64], cle: &str, x: f64) {
        let k = FOOTPRINT_PARAMS.iter().position(|p| p.key == cle).unwrap();
        v[k] = x;
    }

    /// « Auto » : la bourse de la chart d'abord, puis les plus grosses jusqu'à 5 ; une bourse impossible pour cette
    /// pièce est sautée ; sans « Auto » : la bourse de la chart et les cases cochées.
    #[test]
    fn bourses_choisies() {
        let mut v = defaults(&FOOTPRINT_PARAMS);
        let tout = |_: usize| true;
        assert_eq!(
            choisies(&v, Some(0), &tout),
            vec![0, 1, 2, 3, 4],
            "maquette : Binance, Bybit, OKX, Coinbase, Hyperliquid"
        );
        assert_eq!(choisies(&v, Some(4), &tout), vec![4, 0, 1, 2, 3], "la chart sur Hyperliquid : elle d'abord");
        assert_eq!(choisies(&v, None, &tout), vec![0, 1, 2, 3], "une bourse hors liste (comptant…) : elle + 4");
        assert_eq!(
            choisies(&v, Some(0), &|k| k != 2),
            vec![0, 1, 3, 4, 5],
            "OKX impossible : Bitget prend sa place"
        );
        regle(&mut v, "auto", 0.0);
        regle(&mut v, "exBybit", 0.0);
        regle(&mut v, "exKraken", 1.0);
        assert_eq!(choisies(&v, Some(3), &tout), vec![3, 0, 2, 4, 6], "cases cochées");
        for k in FOOTPRINT_CASES {
            regle(&mut v, k, 0.0);
        }
        assert_eq!(choisies(&v, Some(3), &tout), vec![3], "rien de coché : la bourse de la chart seule");
    }

    /// Deux bourses : les parts et le détail d'une rangée par bourse ; une bourse sans bougie ne compte pas.
    #[test]
    fn parts_et_detail() {
        let bn = vec![fb(60, 10.0, &[(6204, 5.5, 7.6)], true)];
        let cb = vec![fb(60, 5.0, &[(12408, 1.2, 1.8)], false)];
        let noms = vec![
            ("Binance".to_string(), COULEURS[0]),
            ("Bybit".to_string(), COULEURS[1]),
            ("Coinbase".to_string(), COULEURS[3]),
        ];
        let (m, fusion) = FpMulti::de(noms, vec![bn, vec![], cb], vec![0.0, 0.0, 0.0], false, "BTC", "Binance Perpétuel");
        assert_eq!(m.nombre(), "2 bourses");
        assert_eq!(m.pas, 5.0);
        assert_eq!(fusion.len(), 1);
        assert!(!fusion[0].complete, "Coinbase a manqué un bout");
        assert_eq!(m.incompletes(60), vec!["Coinbase"]);
        let p = m.parts_de(60).unwrap();
        assert!((p[0] - 13.1).abs() < 1e-9 && (p[1] - 3.0).abs() < 1e-9, "{p:?}");
        let b = Bulle::de(&m, 60, 10.0, 6204, &|x| format!("{x:.0}"), "1 min · 10:44".into()).unwrap();
        assert_eq!(b.rangee, "62040 – 62050");
        assert_eq!(b.lignes.len(), 2);
        assert!((b.lignes[0].2 - 7.6).abs() < 1e-9 && (b.lignes[0].3 - 5.5).abs() < 1e-9, "ventes puis achats");
        assert!((b.lignes[1].2 - 1.8).abs() < 1e-9);
        assert!(
            Bulle::de(&m, 60, 10.0, 6300, &|x| format!("{x}"), String::new()).is_none(),
            "rangée vide : pas de bulle"
        );
    }

    /// Le ⚙ : chaque clic tombe sur sa partie (🖥 et 📱), et le panneau garde la hauteur de la maquette (≈ 370).
    #[test]
    fn reglages_clics() {
        let t = Text::default();
        let r = Reglages {
            auto: true,
            choisies: vec![0, 1, 2, 3, 4],
            chart: Some(0),
            dollars: false,
            aligne: true,
            parts: true,
            piece: "BTC".into(),
            reference: "Binance Perpétuel".into(),
        };
        for (w, cell) in [(LARGEUR, false), (402.0, true)] {
            let m = r.mise(&t, w, cell);
            assert!(m.h > 300.0 && m.h < 440.0, "{w} {}", m.h);
            assert_eq!(r.click(&t, w, cell, m.fermer.cx(), m.fermer.cy()), Clic::Fermer);
            assert_eq!(r.click(&t, w, cell, m.auto.cx(), m.auto.cy()), Clic::Auto);
            for (q, k) in &m.puces {
                assert_eq!(r.click(&t, w, cell, q.cx(), q.cy()), Clic::Bourse(*k));
                assert!(q.x + q.w <= w - PAD + 0.5, "la pastille {k} déborde");
            }
            assert_eq!(m.puces.len(), 10);
            assert_eq!(r.click(&t, w, cell, m.seg.x + 10.0, m.seg.cy()), Clic::Unite(false));
            assert_eq!(r.click(&t, w, cell, m.seg.x + m.seg.w - 10.0, m.seg.cy()), Clic::Unite(true));
            assert_eq!(r.click(&t, w, cell, m.aligne.cx(), m.aligne.cy()), Clic::Aligne);
            assert_eq!(r.click(&t, w, cell, m.parts.cx(), m.parts.cy()), Clic::Parts);
            assert_eq!(r.click(&t, w, cell, m.tous.cx(), m.tous.cy()), Clic::Tous);
            assert!(r.draw(&t, w, cell, None).is_some());
        }
        let m = r.mise(&t, LARGEUR, false);
        assert!((m.h - 370.0).abs() < 30.0, "maquette : 370 de haut, {}", m.h);
    }
}
