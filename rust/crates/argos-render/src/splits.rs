//! 🪟 Split chart (comme TradingView et Argos v1) : plusieurs charts dans la zone de chart. La case en haut à gauche
//! est la chart principale (dessins, indicateurs, replay) ; les autres montrent les bougies de leur symbole et de leur
//! unité de temps. Menu « Splits » : 1 à 16 cases, Trinity, grille à toi (jusqu'à 4 × 4) et synchronisation.
//! Une disposition = des cases en fractions de la zone (x, y, largeur, hauteur) ; la 1re est toujours en haut à gauche.

use crate::palette as pal;
use crate::palette::{accent, GREY, ICON, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use serde::{Deserialize, Serialize};
use tiny_skia::Pixmap;

/// Une case : (x, y, largeur, hauteur) en fractions de la zone.
pub type Cell = [f64; 4];

/// Grille de `c` colonnes × `r` rangées, ligne par ligne.
pub fn grid(c: usize, r: usize) -> Vec<Cell> {
    let (c, r) = (c.max(1), r.max(1));
    (0..r)
        .flat_map(|j| (0..c).map(move |i| [i as f64 / c as f64, j as f64 / r as f64, 1.0 / c as f64, 1.0 / r as f64]))
        .collect()
}

/// Les cases d'une disposition (clé du menu ; « g3x2 » = grille à toi). Inconnue : une seule chart.
pub fn cells(key: &str) -> Vec<Cell> {
    let h = 1.0 / 2.0;
    let t = 1.0 / 3.0;
    match key {
        "2c" => grid(2, 1),
        "2r" => grid(1, 2),
        "3c" => grid(3, 1),
        "3r" => grid(1, 3),
        "3l" => vec![[0.0, 0.0, h, 1.0], [h, 0.0, h, h], [h, h, h, h]],
        "3R" => vec![[0.0, 0.0, h, h], [h, 0.0, h, 1.0], [0.0, h, h, h]],
        "3t" => vec![[0.0, 0.0, 1.0, h], [0.0, h, h, h], [h, h, h, h]],
        "3b" => vec![[0.0, 0.0, h, h], [h, 0.0, h, h], [0.0, h, 1.0, h]],
        "4g" => grid(2, 2),
        "4l" => vec![[0.0, 0.0, h, 1.0], [h, 0.0, h, t], [h, t, h, t], [h, 2.0 * t, h, t]],
        "4R" => vec![[0.0, 0.0, h, t], [h, 0.0, h, 1.0], [0.0, t, h, t], [0.0, 2.0 * t, h, t]],
        "4t" => vec![[0.0, 0.0, 1.0, h], [0.0, h, t, h], [t, h, t, h], [2.0 * t, h, t, h]],
        "4c" => grid(4, 1),
        "5l" => vec![[0.0, 0.0, h, 1.0], [h, 0.0, 0.25, h], [0.75, 0.0, 0.25, h], [h, h, 0.25, h], [0.75, h, 0.25, h]],
        "5t" => vec![[0.0, 0.0, 1.0, h], [0.0, h, 0.25, h], [0.25, h, 0.25, h], [h, h, 0.25, h], [0.75, h, 0.25, h]],
        "6g" => grid(3, 2),
        "6r" => grid(2, 3),
        "6t" => vec![
            [0.0, 0.0, 2.0 * t, 2.0 * t],
            [2.0 * t, 0.0, t, t],
            [2.0 * t, t, t, t],
            [0.0, 2.0 * t, t, t],
            [t, 2.0 * t, t, t],
            [2.0 * t, 2.0 * t, t, t],
        ],
        "8c" => grid(4, 2),
        "8r" => grid(2, 4),
        "9g" => grid(3, 3),
        "12c" => grid(4, 3),
        "12r" => grid(3, 4),
        "16g" => grid(4, 4),
        "tri1" => vec![[0.0, 0.0, 0.25, 1.0], [0.25, 0.0, h, 1.0], [0.75, 0.0, 0.25, 1.0]],
        "tri2" => vec![[0.0, 0.0, h, h], [h, 0.0, h, h], [0.0, h, t, h], [t, h, t, h], [2.0 * t, h, t, h]],
        k if k.starts_with('g') => {
            let (c, r) = k[1..].split_once('x').map_or((1, 1), |(a, b)| (a.parse().unwrap_or(1), b.parse().unwrap_or(1)));
            grid(c.clamp(1, 4), r.clamp(1, 4))
        }
        _ => vec![[0.0, 0.0, 1.0, 1.0]],
    }
}

/// Lignes du menu : (nombre de cases, dispositions), comme TradingView.
const ROWS: [(&str, &[&str]); 10] = [
    ("1", &["1"]),
    ("2", &["2c", "2r"]),
    ("3", &["3c", "3r", "3l", "3R", "3t", "3b"]),
    ("4", &["4g", "4l", "4R", "4t", "4c"]),
    ("5", &["5l", "5t"]),
    ("6", &["6g", "6r", "6t"]),
    ("8", &["8c", "8r"]),
    ("9", &["9g"]),
    ("12", &["12c", "12r"]),
    ("16", &["16g"]),
];
const TRINITY: [&str; 2] = ["tri1", "tri2"];

/// Synchronisations entre les cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sync {
    Symbol,
    Interval,
    Crosshair,
    Time,
}

/// Ce qui est gardé : disposition, cases (symbole « exchange:base », unité de temps en secondes), synchros.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prefs {
    pub layout: String,
    /// Cases 2, 3… (la 1re = la chart principale).
    pub panes: Vec<(String, i64)>,
    pub sync_symbol: bool,
    pub sync_interval: bool,
    /// ✚ Le réticule de la chart principale suit dans les autres cases (heure, et prix sur le même symbole).
    pub sync_crosshair: bool,
    /// ⟷ Les autres cases montrent la même plage de temps (défilement et zoom suivent).
    pub sync_time: bool,
    /// 🔗 Cases 2, 3… liées à la principale : elles suivent son symbole (chacune garde son unité de temps).
    pub liees: Vec<bool>,
}

impl Default for Prefs {
    fn default() -> Self {
        Self {
            layout: "1".into(),
            panes: vec![],
            sync_symbol: false,
            sync_interval: false,
            sync_crosshair: false,
            sync_time: false,
            liees: vec![],
        }
    }
}

impl Prefs {
    pub fn sync(&self, s: Sync) -> bool {
        match s {
            Sync::Symbol => self.sync_symbol,
            Sync::Interval => self.sync_interval,
            Sync::Crosshair => self.sync_crosshair,
            Sync::Time => self.sync_time,
        }
    }
}

/// Ce qu'un clic dans le menu demande.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuClick {
    None,
    Close,
    Layout(String),
    Sync(Sync),
    /// 📱 Open (true) or leave (false) the synchro sub-menu of the touch sheet.
    Synchro(bool),
}

/// Haut des blocs du menu (fenêtre) et pas des interrupteurs.
struct Geo {
    /// Bord gauche du contenu.
    x: f64,
    /// Haut de la 1re rangée de dispositions, et de l'icône dans sa rangée.
    rangs: f64,
    icone_y: f64,
    /// Rangée Trinity, grille à toi, haut de SYNCHRO (libellés).
    tri: f64,
    pz: f64,
    sync: f64,
    /// Interrupteur sous son libellé, écart des 2 colonnes, des 2 rangées.
    sw: f64,
    col: f64,
    rang: f64,
}

#[derive(Debug, Clone, PartialEq)]
enum Zone {
    Layout(String),
    Custom(usize, usize),
    Sync(Sync),
    /// 📱 « Synchro › » (true) and « ‹ Synchro » (false).
    Synchro(bool),
}

const W: f64 = 330.0;
const PAD: f64 = 14.0;
const ROW: f64 = 34.0;
const ICON_W: f64 = 34.0;
const CUSTOM: f64 = 30.0;

// 📱 The touch sheet ▦ (iPad, and iPhone when opened): 1 to 4 cells as big tiles, then « Synchro › » (Jee, 21 Nov,
// list 2 point 4). Points from the top of the sheet.
const C_X: f64 = 16.0;
/// The 4 dispositions offered by the finger.
const C_KEYS: [&str; 4] = ["1", "2c", "3c", "4g"];
/// Tiles: top, side, gap.
const C_TILE_Y: f64 = 40.0;
const C_TILE: f64 = 72.0;
const C_GAP: f64 = 16.0;
/// Rows (« Synchro › », « ‹ Synchro », each synchro): height, the finger's touch minimum (44 real px at 0.8).
const C_ROW: f64 = crate::formfactor::TOUCH_MIN_PT;
/// Height of the sheet: the synchro view (back row + 4 rows + margins), the tallest of the two views.
const C_FIN: f64 = 8.0 + C_ROW + 4.0 * C_ROW + 24.0;

/// Le menu « Splits », ouvert sous son bouton.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub rect: Rect,
    /// Grille à toi survolée (colonnes, rangées).
    pub hover_grid: Option<(usize, usize)>,
    /// Hauteur de tout le contenu et défilement (plus haut que l'écran du téléphone : il défile).
    pub full: f64,
    pub scroll: f64,
    /// 📱 Feuille du bas du cell et de la tablette (verre, toute la largeur).
    pub cell: bool,
    /// 📱 The sheet shows the synchro sub-menu (« Synchro › »).
    pub synchro: bool,
}

impl Menu {
    pub fn new(anchor: Rect, win_w: f64, win_h: f64) -> Self {
        let h = 30.0 + ROWS.len() as f64 * ROW + 26.0 + ROW + 26.0 + 4.0 * CUSTOM + 26.0 + 2.0 * 50.0 + PAD;
        let x = anchor.x.clamp(8.0, (win_w - W - 8.0).max(8.0));
        let y = (anchor.y + anchor.h + 6.0).min((win_h - h - 6.0).max(6.0));
        let vis = h.min((win_h - y - 6.0).max(120.0));
        Self {
            rect: Rect::new(x, y, W, vis),
            hover_grid: None,
            full: h,
            scroll: 0.0,
            cell: false,
            synchro: false,
        }
    }

    /// 📱 La feuille ▦ du cell et de la tablette : posée en bas, toute la largeur, en verre (coins du haut arrondis à
    /// 26), juste la hauteur de son contenu (au plus 80 % de l'écran).
    pub fn feuille(win_w: f64, win_h: f64) -> Self {
        let haut = C_FIN.min(win_h * 0.8);
        Self {
            rect: Rect::new(0.0, win_h - haut, win_w, haut),
            hover_grid: None,
            full: C_FIN,
            scroll: 0.0,
            cell: true,
            synchro: false,
        }
    }

    /// Défile de `dy` points (positif : vers le bas du menu).
    pub fn wheel(&mut self, dy: f64) {
        self.scroll = (self.scroll + dy).clamp(0.0, (self.full - self.rect.h).max(0.0));
    }

    /// Le contenu entier, décalé du défilement.
    fn content(&self) -> Rect {
        Rect::new(self.rect.x, self.rect.y - self.scroll, self.rect.w, self.full.max(self.rect.h))
    }

    /// Où tombe chaque bloc (fenêtre) du menu du PC.
    fn geo(&self) -> Geo {
        let r = self.content();
        let rangs = r.y + 30.0;
        let tri = rangs + ROWS.len() as f64 * ROW + 26.0;
        let pz = tri + ROW + 26.0;
        Geo {
            x: r.x + PAD,
            rangs,
            icone_y: 4.0,
            tri,
            pz,
            sync: pz + 4.0 * CUSTOM + 26.0,
            sw: 18.0,
            col: (r.w - 2.0 * PAD) / 2.0,
            rang: 50.0,
        }
    }

    fn zones(&self) -> Vec<(Rect, Zone)> {
        if self.cell {
            return self.zones_cell();
        }
        let g = self.geo();
        let mut out = vec![];
        let mut y = g.rangs;
        for (_, keys) in ROWS {
            for (k, key) in keys.iter().enumerate() {
                out.push((
                    Rect::new(g.x + 34.0 + k as f64 * (ICON_W + 8.0), y + g.icone_y, ICON_W, ROW - 8.0),
                    Zone::Layout(key.to_string()),
                ));
            }
            y += ROW;
        }
        for (k, key) in TRINITY.iter().enumerate() {
            out.push((
                Rect::new(g.x + k as f64 * (ICON_W + 8.0), g.tri + 4.0, ICON_W, ROW - 8.0),
                Zone::Layout(key.to_string()),
            ));
        }
        for j in 0..4 {
            for i in 0..4 {
                out.push((
                    Rect::new(g.x + i as f64 * CUSTOM, g.pz + j as f64 * CUSTOM, CUSTOM - 3.0, CUSTOM - 3.0),
                    Zone::Custom(i + 1, j + 1),
                ));
            }
        }
        for (k, s) in [Sync::Symbol, Sync::Interval, Sync::Crosshair, Sync::Time].into_iter().enumerate() {
            let x = g.x + (k % 2) as f64 * g.col;
            out.push((Rect::new(x, g.sync + (k / 2) as f64 * g.rang + g.sw, 80.0, 26.0), Zone::Sync(s)));
        }
        out
    }

    /// La case d'une disposition (clé du menu), en coordonnées de la fenêtre.
    pub fn layout_rect(&self, key: &str) -> Option<Rect> {
        self.zones().into_iter().find(|(_, z)| matches!(z, Zone::Layout(k) if k == key)).map(|(q, _)| q)
    }

    /// L'interrupteur d'une synchro, en coordonnées de la fenêtre.
    pub fn sync_rect(&self, s: Sync) -> Option<Rect> {
        self.zones().into_iter().find(|(_, z)| *z == Zone::Sync(s)).map(|(q, _)| q)
    }

    /// 📱 « Synchro › » (`open`) or « ‹ Synchro » of the touch sheet, en coordonnées de la fenêtre.
    pub fn synchro_rect(&self, open: bool) -> Option<Rect> {
        self.zones().into_iter().find(|(_, z)| *z == Zone::Synchro(open)).map(|(q, _)| q)
    }

    /// Survol (pour la grille à toi).
    pub fn hover(&mut self, x: f64, y: f64) {
        self.hover_grid = self.zones().into_iter().find_map(|(q, z)| match z {
            Zone::Custom(c, r) if q.contains(x, y) => Some((c, r)),
            _ => None,
        });
    }

    pub fn click(&self, x: f64, y: f64) -> MenuClick {
        if !self.rect.contains(x, y) {
            return MenuClick::Close;
        }
        match self.zones().into_iter().find(|(q, _)| q.contains(x, y)).map(|(_, z)| z) {
            Some(Zone::Layout(k)) => MenuClick::Layout(k),
            Some(Zone::Custom(c, r)) => MenuClick::Layout(if c * r == 1 { "1".into() } else { format!("g{c}x{r}") }),
            Some(Zone::Sync(s)) => MenuClick::Sync(s),
            Some(Zone::Synchro(on)) => MenuClick::Synchro(on),
            None => MenuClick::None,
        }
    }

    pub fn draw(&self, text: &Text, prefs: &Prefs, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        if self.cell {
            return self.draw_cell(text, prefs, crate::glass::on());
        }
        let r = self.content();
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        // (`.pop` de la maquette : rayon du menu, sans traits entre les rangées)
        crate::jetons::fond_fenetre(&mut pm, loc(r), crate::jetons::R_MENU);
        round_stroke(&mut pm, loc(r), crate::jetons::R_MENU, pal::edge(), 1.0);
        let mut y = 30.0;
        text.draw(&mut pm, "SPLITS", (PAD as f32, 20.0), 10.5, GREY, Align::Left);
        for (n, _) in ROWS {
            text.draw(&mut pm, n, (PAD as f32, (y + ROW / 2.0 + 4.0) as f32), 13.0, WHITE, Align::Left);
            y += ROW;
        }
        text.draw(&mut pm, "TRINITY", (PAD as f32, (y + 18.0) as f32), 10.5, GREY, Align::Left);
        y += 26.0 + ROW;
        text.draw(&mut pm, "PERSONNALISÉ", (PAD as f32, (y + 18.0) as f32), 10.5, GREY, Align::Left);
        y += 26.0 + 4.0 * CUSTOM;
        text.draw(&mut pm, "SYNCHRO", (PAD as f32, (y + 18.0) as f32), 10.5, GREY, Align::Left);
        for (q, z) in self.zones() {
            let lq = loc(q);
            let over = q.contains(mouse.0, mouse.1);
            match z {
                Zone::Layout(k) => {
                    let on = prefs.layout == k;
                    if on || over {
                        round_fill(
                            &mut pm,
                            Rect::new(lq.x - 3.0, lq.y - 2.0, lq.w + 6.0, lq.h + 4.0),
                            6.0,
                            if on { accent() } else { pal::hover() },
                            if on { 0.3 } else { 1.0 },
                        );
                    }
                    for c in cells(&k) {
                        let cell = Rect::new(lq.x + c[0] * lq.w + 1.0, lq.y + c[1] * lq.h + 1.0, c[2] * lq.w - 2.0, c[3] * lq.h - 2.0);
                        round_stroke(&mut pm, cell, 2.0, if on { WHITE } else { ICON }, 1.2);
                    }
                }
                Zone::Synchro(_) => {}
                Zone::Custom(c, rr) => {
                    let lit = self.hover_grid.is_some_and(|(hc, hr)| c <= hc && rr <= hr);
                    round_fill(&mut pm, lq, 4.0, if lit { accent() } else { pal::tile() }, if lit { 0.6 } else { 1.0 });
                    round_stroke(&mut pm, lq, 4.0, pal::edge(), 1.0);
                }
                Zone::Sync(s) => {
                    let name = match s {
                        Sync::Symbol => "Symbole",
                        Sync::Interval => "Unité de temps",
                        Sync::Crosshair => "✚ Réticule",
                        Sync::Time => "⟷ Temps",
                    };
                    let ready = true;
                    text.draw(
                        &mut pm,
                        name,
                        (lq.x as f32, (lq.y - 6.0) as f32),
                        12.0,
                        if ready { WHITE } else { GREY },
                        Align::Left,
                    );
                    let on = prefs.sync(s);
                    round_fill(&mut pm, lq, lq.h / 2.0, if on { accent() } else { pal::tile() }, 1.0);
                    round_stroke(&mut pm, lq, lq.h / 2.0, pal::edge(), 1.0);
                    let label = if !ready {
                        "bientôt"
                    } else if on {
                        "ON"
                    } else {
                        "OFF"
                    };
                    text.draw(
                        &mut pm,
                        label,
                        ((lq.x + 14.0) as f32, (lq.cy() + 4.5) as f32),
                        11.5,
                        if on { WHITE } else { GREY },
                        Align::Left,
                    );
                    round_fill(
                        &mut pm,
                        Rect::new(lq.x + lq.w - 18.0, lq.cy() - 3.0, 6.0, 6.0),
                        3.0,
                        if on { WHITE } else { GREY },
                        1.0,
                    );
                }
            }
        }
        if let Some((c, rr)) = self.hover_grid {
            text.draw(
                &mut pm,
                &format!("{c} × {rr}"),
                ((PAD + 4.0 * CUSTOM + 14.0) as f32, (r.h - 2.0 * 50.0 - 26.0 - 2.0 * CUSTOM) as f32),
                13.0,
                WHITE,
                Align::Left,
            );
        }
        let v = self.rect;
        if self.full > v.h {
            let part = crate::px::int_rect(0.0, self.scroll, v.w, v.h).and_then(|ir| pm.clone_rect(ir))?;
            return Some((part, v));
        }
        Some((pm, v))
    }
}

impl Menu {
    /// 📱 The touch sheet's zones: the 4 big tiles and « Synchro › », or « ‹ Synchro » and the 4 synchro rows.
    fn zones_cell(&self) -> Vec<(Rect, Zone)> {
        let r = self.content();
        let w = r.w - 2.0 * C_X;
        if self.synchro {
            let mut out = vec![(Rect::new(r.x + C_X, r.y + 8.0, w, C_ROW), Zone::Synchro(false))];
            for (k, s) in [Sync::Symbol, Sync::Interval, Sync::Crosshair, Sync::Time].into_iter().enumerate() {
                out.push((Rect::new(r.x + C_X, r.y + 8.0 + (k + 1) as f64 * C_ROW, w, C_ROW), Zone::Sync(s)));
            }
            return out;
        }
        let mut out: Vec<(Rect, Zone)> = C_KEYS
            .iter()
            .enumerate()
            .map(|(k, key)| {
                let x = r.x + C_X + k as f64 * (C_TILE + C_GAP);
                (Rect::new(x, r.y + C_TILE_Y, C_TILE, C_TILE), Zone::Layout(key.to_string()))
            })
            .collect();
        out.push((Rect::new(r.x + C_X, r.y + C_TILE_Y + C_TILE + C_GAP, w, C_ROW), Zone::Synchro(true)));
        out
    }

    /// 📱 La feuille ▦ au doigt : verre, coins du haut arrondis à 26 ; « SPLITS », 4 grandes tuiles (1 à 4 cases) et
    /// « Synchro › » ; ou le sous-menu « ‹ Synchro » et ses 4 interrupteurs. `verre` : la chart floutée est déjà dessous
    /// (teinte), sinon opaque.
    fn draw_cell(&self, text: &Text, prefs: &Prefs, verre: bool) -> Option<(Pixmap, Rect)> {
        use crate::jetons::{sur_fond, ACCENT, BLANC, DIM, LISERE, TEXTE, VERRE};
        let r = self.content();
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        // la feuille : coins du haut arrondis, le bas déborde (caché)
        let carte = Rect::new(0.0, 0.0, r.w, r.h + 30.0);
        if verre {
            round_fill(&mut pm, carte, 26.0, VERRE.0, VERRE.1);
        } else {
            round_fill(&mut pm, carte, 26.0, sur_fond(VERRE), 1.0);
        }
        round_stroke(&mut pm, carte, 26.0, sur_fond(LISERE), 1.0);
        crate::fill(&mut pm, 18.0, 1.0, r.w - 36.0, 1.0, BLANC, 0.12);
        if !self.synchro {
            text.draw(&mut pm, "SPLITS", (C_X as f32, 26.0), 11.0, DIM, Align::Left);
        }
        let ligne = |q: Rect, t: f32| (q.cy() as f32) + t * 0.36;
        for (q, z) in self.zones() {
            let lq = loc(q);
            match z {
                Zone::Layout(k) => {
                    let on = prefs.layout == k;
                    round_fill(&mut pm, lq, 14.0, if on { ACCENT } else { BLANC }, if on { 0.3 } else { 0.07 });
                    round_stroke(&mut pm, lq, 14.0, sur_fond((BLANC, 0.12)), 1.0);
                    // the icon (44 × 32) above the number of cells
                    let ic = Rect::new(lq.cx() - 22.0, lq.y + 12.0, 44.0, 32.0);
                    for c in cells(&k) {
                        let cell = Rect::new(ic.x + c[0] * ic.w + 1.0, ic.y + c[1] * ic.h + 1.0, c[2] * ic.w - 2.0, c[3] * ic.h - 2.0);
                        round_stroke(&mut pm, cell, 2.0, if on { BLANC } else { DIM }, 1.4);
                    }
                    let n = cells(&k).len().to_string();
                    text.draw(&mut pm, &n, (lq.cx() as f32, (lq.y + 62.0) as f32), 13.0, TEXTE, Align::Center);
                }
                Zone::Synchro(open) => {
                    round_fill(&mut pm, Rect::new(lq.x, lq.y + 4.0, lq.w, lq.h - 8.0), 12.0, BLANC, 0.07);
                    let label = if open { "Synchro" } else { "‹ Synchro" };
                    text.draw(&mut pm, label, ((lq.x + 14.0) as f32, ligne(lq, 14.0)), 14.0, TEXTE, Align::Left);
                    if open {
                        text.draw(&mut pm, "›", ((lq.x + lq.w - 14.0) as f32, ligne(lq, 16.0)), 16.0, DIM, Align::Right);
                    }
                }
                Zone::Sync(s) => {
                    let name = match s {
                        Sync::Symbol => "Symbole",
                        Sync::Interval => "Unité de temps",
                        Sync::Crosshair => "✚ Réticule",
                        Sync::Time => "⟷ Temps",
                    };
                    text.draw(&mut pm, name, ((lq.x + 14.0) as f32, ligne(lq, 14.0)), 14.0, TEXTE, Align::Left);
                    let on = prefs.sync(s);
                    let sw = Rect::new(lq.x + lq.w - 66.0, lq.cy() - 16.0, 56.0, 32.0);
                    round_fill(&mut pm, sw, 16.0, if on { ACCENT } else { BLANC }, if on { 1.0 } else { 0.12 });
                    let knob = if on { sw.x + sw.w - 29.0 } else { sw.x + 3.0 };
                    round_fill(&mut pm, Rect::new(knob, sw.y + 3.0, 26.0, 26.0), 13.0, BLANC, 1.0);
                }
                Zone::Custom(..) => {}
            }
        }
        Some((pm, self.rect))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispositions() {
        // chaque disposition couvre toute la zone, sans chevauchement, la 1re case en haut à gauche
        let all: Vec<&str> = ROWS
            .iter()
            .flat_map(|(_, k)| k.iter().copied())
            .chain(TRINITY)
            .chain(["g3x2", "g4x4"])
            .collect();
        for k in all {
            let c = cells(k);
            let area: f64 = c.iter().map(|c| c[2] * c[3]).sum();
            assert!((area - 1.0).abs() < 1e-9, "{k} : aire {area}");
            assert_eq!((c[0][0], c[0][1]), (0.0, 0.0), "{k} : 1re case en haut à gauche");
            for (i, a) in c.iter().enumerate() {
                for b in &c[i + 1..] {
                    let over = a[0] < b[0] + b[2] - 1e-9 && b[0] < a[0] + a[2] - 1e-9 && a[1] < b[1] + b[3] - 1e-9 && b[1] < a[1] + a[3] - 1e-9;
                    assert!(!over, "{k} : cases qui se chevauchent");
                }
            }
        }
        assert_eq!(cells("16g").len(), 16);
        assert_eq!(cells("g3x2").len(), 6);
        let mut m = Menu::new(Rect::new(300.0, 10.0, 40.0, 30.0), 1600.0, 1000.0);
        let (q, _) = m.zones().into_iter().find(|(_, z)| *z == Zone::Layout("4g".into())).unwrap();
        assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Layout("4g".into()));
        let (q, _) = m.zones().into_iter().find(|(_, z)| *z == Zone::Custom(3, 2)).unwrap();
        m.hover(q.cx(), q.cy());
        assert_eq!(m.hover_grid, Some((3, 2)));
        assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Layout("g3x2".into()));
        let (q, _) = m.zones().into_iter().find(|(_, z)| *z == Zone::Sync(Sync::Interval)).unwrap();
        assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Sync(Sync::Interval));
        for s in [Sync::Crosshair, Sync::Time] {
            let (q, _) = m.zones().into_iter().find(|(_, z)| *z == Zone::Sync(s)).unwrap();
            assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Sync(s), "réticule et temps : prêts");
        }
        assert!(m.draw(&Text::default(), &Prefs::default(), (0.0, 0.0)).is_some());
    }

    /// 📱 La feuille ▦ au doigt (Jee, 21 nov., liste 2 point 4) : 1 à 4 cases en tuiles ≥ 64, « Synchro › » ouvre le
    /// sous-menu (cibles ≥ 55 points = 44 vrais px, rien ne se chevauche) ; le menu du PC garde tout (16, Trinity, grille à toi).
    #[test]
    fn feuille_au_doigt_1_a_4_et_synchro() {
        for (w, h) in [(820.0, 1180.0), (1180.0, 820.0), (402.0, 874.0)] {
            let mut m = Menu::feuille(w, h);
            assert!(m.cell && m.rect.x == 0.0 && m.rect.w == w && m.rect.y + m.rect.h == h);
            let pas_de_chevauchement = |z: &[(Rect, Zone)]| {
                for (i, (a, _)) in z.iter().enumerate() {
                    assert!(a.w >= crate::formfactor::TOUCH_MIN_PT && a.h >= crate::formfactor::TOUCH_MIN_PT, "{a:?}");
                    assert!(
                        a.x >= 0.0 && a.x + a.w <= w && a.y >= m.rect.y - 0.01 && a.y + a.h <= h,
                        "{a:?} hors de la feuille"
                    );
                    for (b, _) in &z[i + 1..] {
                        assert!(!(a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h), "{a:?} {b:?}");
                    }
                }
            };
            let z = m.zones();
            let tuiles: Vec<_> = z.iter().filter_map(|(q, z)| matches!(z, Zone::Layout(_)).then_some(*q)).collect();
            assert_eq!(tuiles.len(), 4);
            assert!(tuiles.iter().all(|q| q.w >= 64.0 && q.h >= 64.0));
            assert!(
                z.iter().all(|(_, z)| !matches!(z, Zone::Custom(..) | Zone::Sync(_))),
                "ni grille ni synchro à plat"
            );
            pas_de_chevauchement(&z);
            assert_eq!(m.click(tuiles[3].cx(), tuiles[3].cy()), MenuClick::Layout("4g".into()));
            assert_eq!(cells("4g").len(), 4);
            let (q, _) = z.iter().find(|(_, z)| *z == Zone::Synchro(true)).unwrap();
            assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Synchro(true));
            m.synchro = true;
            let z = m.zones();
            pas_de_chevauchement(&z);
            for s in [Sync::Symbol, Sync::Interval, Sync::Crosshair, Sync::Time] {
                let q = m.sync_rect(s).unwrap();
                assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Sync(s));
            }
            let (q, _) = z.iter().find(|(_, z)| *z == Zone::Synchro(false)).unwrap();
            assert_eq!(m.click(q.cx(), q.cy()), MenuClick::Synchro(false));
            assert_eq!(m.click(w / 2.0, 5.0), MenuClick::Close, "au-dessus de la feuille : fermer");
            assert!(m.draw(&Text::default(), &Prefs::default(), (0.0, 0.0)).is_some());
        }
    }
}
