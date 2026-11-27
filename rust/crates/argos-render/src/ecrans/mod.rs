//! 📐 Écrans de la maquette (interface-2). Les zones ne sont pas recopiées à la main : elles sont **mesurées** dans la
//! maquette (`tools/maquette-mesure.js`, navigateur à 390 × 844) et rangées dans `maquette-cell.json`. Ce module les
//! lit, les place pour la taille de l'écran (ce qui est à droite reste à droite, ce qui est en bas reste en bas), les
//! dessine et dit ce qui est sous le doigt. Une seule table pour le dessin et le toucher.

use crate::phonebar::Icone;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

pub mod feuille;
mod maquette_cell;

/// Une zone mesurée dans la maquette (boîte en points CSS, styles calculés).
#[derive(Debug, Clone, Copy)]
pub struct Zone {
    pub id: &'static str,
    pub boite: [f64; 4],
    pub rayon: f64,
    pub fond: &'static str,
    pub bord: Option<&'static str>,
    pub texte: &'static str,
    pub taille: f64,
    pub libelle: &'static str,
    pub enfants: &'static [Zone],
}

/// Les mesures d'une vue de la maquette.
#[derive(Debug)]
pub struct Maquette {
    pub vue: &'static str,
    pub zones: &'static [Zone],
}

impl Maquette {
    /// Taille de la vue mesurée (390 × 844 pour le cell).
    pub fn taille(&self) -> (f64, f64) {
        let mut it = self.vue.split('x').filter_map(|v| v.parse::<f64>().ok());
        (it.next().unwrap_or(390.0), it.next().unwrap_or(844.0))
    }

    /// La zone `id` (« thaut », « pg.0 »…) et son parent.
    pub fn zone(&self, id: &str) -> Option<(&'static Zone, Option<&'static Zone>)> {
        for z in self.zones {
            if z.id == id {
                return Some((z, None));
            }
            if let Some(e) = z.enfants.iter().find(|e| e.id == id) {
                return Some((e, Some(z)));
            }
        }
        None
    }

    /// Boîte de `id` pour un écran de `w` × `h` : ce qui est à droite de la vue mesurée garde sa marge de droite, ce
    /// qui est en bas garde sa marge du bas ; la rangée du haut et la barre d'icônes s'étirent.
    pub fn place(&self, id: &str, w: f64, h: f64) -> Option<Rect> {
        let (z, parent) = self.zone(id)?;
        let (vw, vh) = self.taille();
        let (dx, dy) = (w - vw, h - vh);
        let [x, y, bw, bh] = z.boite;
        let racine = parent.map_or(z.id, |p| p.id);
        let cy = parent.map_or(y + bh / 2.0, |p| p.boite[1] + p.boite[3] / 2.0);
        let y = if cy > vh / 2.0 { y + dy } else { y };
        let r = match (racine, parent.is_some()) {
            // la rangée du haut et la barre d'icônes : toute la largeur
            ("thaut" | "icones", false) => Rect::new(x, y, bw + dx, bh),
            // 🟠 DÉMO : centré ; 📟 assez large (maquette tablette, mesurée 209,1 × 22,8) : sur une seule ligne
            ("demoband", false) => {
                let (bw, bh) = if w >= DEMO_UNE_LIGNE.0 + 2.0 * x {
                    (DEMO_UNE_LIGNE.0, DEMO_UNE_LIGNE.1)
                } else {
                    (bw, bh)
                };
                Rect::new((w - bw) / 2.0, z.boite[1], bw, bh)
            }
            // ⋮⋮ barre des favoris repliée : à droite, à la même proportion de la hauteur (maquette tablette)
            ("favbar", false) => Rect::new(x + dx, z.boite[1] * h / vh, bw, bh),
            ("favbar", true) => {
                let py = parent.map_or(0.0, |p| p.boite[1]);
                Rect::new(x + dx, z.boite[1] - py + py * h / vh, bw, bh)
            }
            // la barre d'icônes : ses boutons restent alignés à gauche
            ("icones", true) => Rect::new(x, y, bw, bh),
            // ailleurs : à droite si le centre est dans la moitié droite
            _ => {
                let cx = x + bw / 2.0;
                let ref_w = parent.filter(|p| p.id == "thaut").map_or(vw, |p| p.boite[0] * 2.0 + p.boite[2]);
                Rect::new(if cx > ref_w / 2.0 { x + dx } else { x }, y, bw, bh)
            }
        };
        Some(r)
    }
}

/// 📟 Taille de DÉMO sur une ligne (maquette tablette 820 × 1180, `maquette-tablette.json`).
const DEMO_UNE_LIGNE: (f64, f64) = (209.1, 22.8);

/// 📟 The tablet's dock (Jee, 21 Nov, list 2 point 21): ⚙ then the 5 most used, at 44 points, and ⋯ for the others.
pub const DOCK: [(Icone, &str); 6] = [
    (Icone::Reglages, "⚙"),
    (Icone::Trading, "📈"),
    (Icone::Demo, "🧪"),
    (Icone::PropFirm, "🛡"),
    (Icone::Script, "✎"),
    (Icone::Macro, "🌐"),
];
/// 📟 The dock's other buttons, in the small sheet of ⋯ (with their names).
pub const DOCK_AUTRES: [(Icone, &str, &str); 5] = [
    (Icone::Baleines, "🐋", "Baleines"),
    (Icone::Modules, "🧩", "Modules"),
    (Icone::Academie, "🎓", "Académie"),
    (Icone::Communaute, "👥", "Communauté"),
    (Icone::Marketplace, "🛒", "Marketplace"),
];
/// 📟 Side of a touch target of the tablet (dock, top icons): 44 real px at the 0.8 touch scale; gap between dock
/// buttons; side of what is drawn in it (the icons keep their size, only the invisible zone grows).
const BOUTON: f64 = crate::formfactor::TOUCH_MIN_PT;
const DESSIN: f64 = 44.0;
const DOCK_PAS: f64 = BOUTON + 1.0;
/// ⚙ alone at the left, then the line, then the others.
const DOCK_X0: f64 = 6.0;
const DOCK_X1: f64 = DOCK_X0 + BOUTON + 9.0;

/// 📟 What is drawn in the touch target `r`: a 44-point square in its centre.
fn dessin(r: Rect) -> Rect {
    Rect::new(r.cx() - DESSIN / 2.0, r.cy() - DESSIN / 2.0, DESSIN, DESSIN)
}

/// 📟 The dock's buttons (from the dock's top-left): ⚙, a line, 📈 🧪 🛡 ✎ 🌐 and ⋯; `BOUTON` square targets, none
/// overlapping. A dock lower than a target: the target ends at the dock's bottom (it reaches a little above it).
pub fn dock_boutons() -> Vec<(Rect, CellHit, &'static str)> {
    let dh = dock_h();
    let y = if dh >= BOUTON { (dh - BOUTON) / 2.0 } else { dh - BOUTON };
    let mut out = vec![];
    for (k, (i, em)) in DOCK.into_iter().enumerate() {
        // ⚙ alone at the left, then the line (`DOCK_TRAIT`), then the others
        let x = if k == 0 { DOCK_X0 } else { DOCK_X1 + (k - 1) as f64 * DOCK_PAS };
        out.push((Rect::new(x, y, BOUTON, BOUTON), CellHit::Icone(i), em));
    }
    out.push((Rect::new(DOCK_X1 + 5.0 * DOCK_PAS, y, BOUTON, BOUTON), CellHit::Autres, "⋯"));
    out
}

/// 📟 The line between ⚙ and the other dock buttons (x from the dock's left).
const DOCK_TRAIT: f64 = DOCK_X1 - 4.5;

/// 📟 The top pill's icons 🔔 ▦ 📤 🔎 🗺 (window): 44 × 44 touch targets, side by side against the pill's right end,
/// centred on its height (Jee, 21 Nov, list 2 point 6).
pub fn haut_icones(w: f64) -> Vec<(Rect, CellHit, usize)> {
    let m = cell();
    let Some(p) = m.place("thaut", w, m.taille().1) else { return vec![] };
    let y = p.cy() - BOUTON / 2.0;
    let droite = p.x + p.w - 2.0;
    [CellHit::Alertes, CellHit::Disposition, CellHit::Partager, CellHit::Recherche, CellHit::Plan]
        .into_iter()
        .enumerate()
        .map(|(k, hit)| (Rect::new(droite - (5 - k) as f64 * BOUTON, y, BOUTON, BOUTON), hit, k + 3))
        .collect()
}

/// La vue cell de la maquette.
pub fn cell() -> &'static Maquette {
    &maquette_cell::MAQUETTE
}

/// Couleur CSS « rgb(r, g, b) » ou « rgba(r, g, b, a) » → (couleur, opacité).
pub fn couleur(css: &str) -> (Rgb, f32) {
    let n: Vec<f32> = css
        .trim_start_matches("rgba(")
        .trim_start_matches("rgb(")
        .trim_end_matches(')')
        .split(',')
        .filter_map(|v| v.trim().parse().ok())
        .collect();
    match n.as_slice() {
        [r, g, b] => (Rgb(*r as u8, *g as u8, *b as u8), 1.0),
        [r, g, b, a] => (Rgb(*r as u8, *g as u8, *b as u8), *a),
        _ => (Rgb(0, 0, 0), 0.0),
    }
}

/// Ce qu'on touche sur l'écran principal du cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellHit {
    Accueil,
    Argos,
    MiseEnPage,
    Alertes,
    Disposition,
    Partager,
    Recherche,
    Plan,
    Demo,
    Flux,
    Symbole,
    SymbolePrec,
    SymboleSuiv,
    Unite,
    UnitePrec,
    UniteSuiv,
    Fx,
    Ai,
    Icone(Icone),
    /// 📟 ⋯ of the tablet's dock: the small sheet of the other buttons (`DOCK_AUTRES`).
    Autres,
    /// 📱 The single pill (phone): ✎ drawings, Trading, ⋯ (the « Argos » sheet), its fold bar and the ⌃ tab.
    Dessins,
    Trading,
    Plus,
    Replier,
    Rouvrir,
}

/// Ce que montre l'écran (vient du moteur).
#[derive(Debug, Clone, Default)]
pub struct Etat {
    /// Nom de la mise en page (« Argos »), et sauvegarde en cours.
    pub mise_en_page: String,
    pub sauvegarde: bool,
    /// Nombre d'alertes actives (pastille rouge de 🔔 ; 0 = pas de pastille).
    pub alertes: usize,
    /// Nom du compte démo (« Argos 100K »).
    pub compte: String,
    /// Flux de marché : texte et en retard (orange) ou à l'heure (vert).
    pub flux: (String, bool),
    /// Symbole (« BTC-USDC »), voisins de la molette ; unité (« 4h ») et voisines.
    pub symbole: String,
    pub sym_voisins: (Option<String>, Option<String>),
    pub unite: String,
    pub unite_voisines: (Option<String>, Option<String>),
    /// Icône dont le volet est ouvert.
    pub allume: Option<Icone>,
    /// Barres cachées (poignées).
    pub haut_cache: bool,
    pub bas_cache: bool,
    /// Une feuille du bas est ouverte (watchlist, volet) : la pilule du symbole, ƒx et Argos AI passent dessous.
    pub feuille: bool,
    /// 📱 The single pill of the phone (Jee's mock-up, 17 Nov): instead of the top pill, the icon bar and the
    /// separate pills. `repliee`: folded away (the ⌃ tab brings it back); `logo`: the symbol's logo; `pastille`:
    /// the red badge of ⋯; `demo`: the gold dot of Trading; `titre`: where the chart's title ends and its middle
    /// height (window), for the flux dot and the DÉMO label beside it.
    pub pilule: bool,
    pub repliee: bool,
    pub logo: Option<Pixmap>,
    pub coin: String,
    pub pastille: usize,
    pub demo: bool,
    pub titre: (f64, f64),
}

/// 📱 Height of the single pill, its margin to the screen's edges, and the room above it for its fold bar.
pub const PILULE_H: f64 = 56.0;
pub const PILULE_MARGE: f64 = 10.0;
const PILULE_BARRE: f64 = 40.0;
/// Width of each icon of the pill (✎ ƒx Trading ⋯) and of Argos AI at its right end.
const PILULE_BOUTON: f64 = feuille::TOUCHER;
const PILULE_AI: f64 = 56.0;

/// 📱 Room the single pill takes at the bottom of the screen (the chart ends above it): none when folded.
pub fn pilule_reserve(repliee: bool) -> f64 {
    if repliee {
        0.0
    } else {
        PILULE_H + PILULE_MARGE + PILULE_BARRE
    }
}

/// 📱 The single pill in the window `w` × `h`.
pub fn pilule_rect(w: f64, h: f64) -> Rect {
    Rect::new(PILULE_MARGE, h - PILULE_MARGE - PILULE_H, w - 2.0 * PILULE_MARGE, PILULE_H)
}

/// 📱 The parts of the pill, left to right: symbol, interval, ✎, ƒx, Trading, ⋯, Argos AI (window).
pub fn pilule_parts(w: f64, h: f64) -> [(Rect, CellHit); 7] {
    let p = pilule_rect(w, h);
    let reste = (p.w - 8.0 - 4.0 * PILULE_BOUTON - PILULE_AI).max(80.0);
    // (the interval needs « 30m »: the symbol gets the rest, its name and logo)
    let uw = feuille::TOUCHER;
    let sw = reste - uw;
    let x0 = p.x + 4.0;
    let b = |k: f64| Rect::new(x0 + reste + k * PILULE_BOUTON, p.y, PILULE_BOUTON, p.h);
    [
        (Rect::new(x0, p.y, sw, p.h), CellHit::Symbole),
        (Rect::new(x0 + sw, p.y, uw, p.h), CellHit::Unite),
        (b(0.0), CellHit::Dessins),
        (b(1.0), CellHit::Fx),
        (b(2.0), CellHit::Trading),
        (b(3.0), CellHit::Plus),
        (Rect::new(p.x + p.w - PILULE_AI, p.y, PILULE_AI, p.h), CellHit::Ai),
    ]
}

/// 📱 The fold bar's touch zone (a 36 × 4 bar drawn in its middle), just above the pill.
pub fn pilule_barre(w: f64, h: f64) -> Rect {
    let p = pilule_rect(w, h);
    Rect::new(w / 2.0 - 40.0, p.y - feuille::TOUCHER, 80.0, feuille::TOUCHER)
}

/// 📱 The glass ⌃ tab that brings the folded pill back (bottom middle).
pub fn pilule_onglet(w: f64, h: f64) -> Rect {
    Rect::new(w / 2.0 - 36.0, h - PILULE_MARGE - feuille::TOUCHER, 72.0, feuille::TOUCHER)
}

/// 📱 The flux dot and the DÉMO label beside the chart's title (window): their touch zones.
pub fn pilule_haut(e: &Etat) -> [(Rect, CellHit); 2] {
    let (x, cy) = e.titre;
    [
        (
            Rect::new(x + 2.0, cy - feuille::TOUCHER / 2.0, feuille::TOUCHER, feuille::TOUCHER),
            CellHit::Flux,
        ),
        (
            Rect::new(x + 2.0 + feuille::TOUCHER, cy - feuille::TOUCHER / 2.0, 58.0, feuille::TOUCHER),
            CellHit::Demo,
        ),
    ]
}

/// Hauteur de la rangée du haut (sous elle commence la chart : la légende y est à 58 comme dans la maquette).
pub fn haut_h() -> f64 {
    cell().zone("demoband").map_or(58.0, |(z, _)| z.boite[1])
}

/// Haut de la chart sur cell : la légende de la maquette est à 58 points (titre de 16 points, première ligne à 70) ;
/// la nôtre écrit son titre 24 points sous le haut de la chart.
pub fn chart_y() -> f64 {
    haut_h() + 12.0 - 24.0
}

/// Boîte mesurée `id` (telle quelle, sans placement).
pub fn boite_de(id: &str) -> Option<[f64; 4]> {
    cell().zone(id).map(|(z, _)| z.boite)
}

/// Hauteur de la barre d'icônes (la chart s'arrête au-dessus).
pub fn dock_h() -> f64 {
    cell().zone("icones").map_or(54.0, |(z, _)| z.boite[3])
}

/// Zones touchables de l'écran `w` × `h` (coordonnées de la fenêtre).
pub fn zones(w: f64, h: f64, e: &Etat) -> Vec<(Rect, CellHit)> {
    let m = cell();
    let p = |id: &str| m.place(id, w, h);
    let mut out = vec![];
    if e.pilule {
        out.extend(pilule_haut(e));
        if e.repliee {
            out.push((pilule_onglet(w, h), CellHit::Rouvrir));
        } else if !e.feuille {
            out.push((pilule_barre(w, h), CellHit::Replier));
            out.extend(pilule_parts(w, h));
        }
        return out;
    }
    if !e.haut_cache {
        for (k, hit) in [CellHit::Accueil, CellHit::Argos, CellHit::MiseEnPage].into_iter().enumerate() {
            if let Some(r) = p(&format!("thaut.{k}")) {
                out.push((r, hit));
            }
        }
        out.extend(haut_icones(w).into_iter().map(|(r, hit, _)| (r, hit)));
        out.extend(p("demoband").map(|r| (r, CellHit::Demo)));
        out.extend(p("flux").map(|r| (r, CellHit::Flux)));
    }
    if !e.bas_cache && !e.feuille {
        // la molette : la bande du haut et du bas de chaque moitié de la pilule (sous les petits voisins)
        if let (Some(s), Some(u)) = (p("pg.0"), p("pg.3")) {
            let b = 14.0;
            out.push((Rect::new(s.x, s.y, s.w, b), CellHit::SymbolePrec));
            out.push((Rect::new(s.x, s.y + s.h - b, s.w, b), CellHit::SymboleSuiv));
            out.push((Rect::new(s.x, s.y + b, s.w, s.h - 2.0 * b), CellHit::Symbole));
            out.push((Rect::new(u.x, u.y, u.w, b), CellHit::UnitePrec));
            out.push((Rect::new(u.x, u.y + u.h - b, u.w, b), CellHit::UniteSuiv));
            out.push((Rect::new(u.x, u.y + b, u.w, u.h - 2.0 * b), CellHit::Unite));
        }
        out.extend(p("pd").map(|r| (r, CellHit::Fx)));
    }
    if !e.bas_cache {
        let y0 = h - dock_h();
        out.extend(dock_boutons().into_iter().map(|(r, hit, _)| (Rect::new(r.x, r.y + y0, r.w, r.h), hit)));
    }
    if !e.feuille {
        out.push((ai_rect(w, h, e.bas_cache), CellHit::Ai));
    }
    out
}

/// Bouton rond Argos AI (fenêtre) : collé au coin bas droit, au-dessus de la barre d'icônes (barres du bas cachées :
/// tout en bas).
pub fn ai_rect(w: f64, h: f64, bas_cache: bool) -> Rect {
    let r = cell().place("aipill", w, h).unwrap_or(Rect::new(w - 68.0, h - 122.0, 56.0, 56.0));
    if bas_cache {
        Rect::new(r.x, h - 12.0 - r.h, r.w, r.h)
    } else {
        r
    }
}

/// Ce qui est sous le doigt (fenêtre).
pub fn hit(w: f64, h: f64, e: &Etat, x: f64, y: f64) -> Option<(CellHit, Rect)> {
    zones(w, h, e).into_iter().find(|(r, _)| r.contains(x, y)).map(|(r, k)| (k, r))
}

/// Une boîte de la maquette avec son fond, son bord et son rayon (le style verre central pour les pilules).
fn boite(pm: &mut Pixmap, z: &Zone, r: Rect) {
    let rayon = z.rayon.min(r.h / 2.0).min(r.w / 2.0);
    // fond posé plein (sa couleur sur le fond sombre) : à l'écran, glass::blit le remplace par la chart floutée,
    // comme le backdrop-filter de la maquette
    let (f, a) = couleur(z.fond);
    if a > 0.0 {
        round_fill(pm, r, rayon, crate::jetons::sur_fond((f, a)), 1.0);
    }
    if let Some(b) = &z.bord {
        let (c, a) = couleur(b);
        round_stroke(pm, r, rayon, crate::jetons::sur_fond((c, a)), 1.0);
    }
}

fn texte_c(z: &Zone) -> Rgb {
    couleur(z.texte).0
}

/// La rangée du haut : la pilule de verre et ses boutons (image `w` × `haut_h()`, transparente autour : elle se
/// pose sur la chart). `fond` : un fond plein dessous (captures).
pub fn haut(text: &Text, w: f64, e: &Etat, fond: Option<Rgb>, survol: Option<(f64, f64)>) -> Option<Pixmap> {
    let m = cell();
    let hh = haut_h();
    let mut pm = crate::px::new(w as u32, hh as u32)?;
    if let Some(f) = fond {
        crate::fill(&mut pm, 0.0, 0.0, w, hh, f, 1.0);
    }
    let (z, _) = m.zone("thaut")?;
    let r = m.place("thaut", w, 844.0)?;
    boite(&mut pm, z, r);
    let gras = Text::graisse(Graisse::Bold);
    let icones = haut_icones(w);
    for k in 0..8 {
        let id = format!("thaut.{k}");
        let (Some((c, _)), Some(b)) = (m.zone(&id), m.place(&id, w, 844.0)) else {
            continue;
        };
        // 🔔 ▦ 📤 🔎 🗺 : drawn at 44 points in the middle of their bigger touch targets
        let cible = icones.iter().find(|(_, _, j)| *j == k).map(|(r, _, _)| *r);
        let b = cible.map_or(b, dessin);
        if survol.is_some_and(|(x, y)| cible.unwrap_or(b).contains(x, y)) {
            round_fill(&mut pm, b, c.rayon, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
        }
        let col = texte_c(c);
        let base = (b.cy() + c.taille * 0.36) as f32;
        let t = c.taille as f32;
        match k {
            // ⌂
            0 => text.draw(&mut pm, "⌂", (b.cx() as f32, base + 1.0), t + 4.0, col, Align::Center),
            // « ARGOS ⌄ » (en gras dans la maquette)
            1 => {
                gras.draw(&mut pm, "ARGOS", ((b.x + 6.0) as f32, base), t, col, Align::Left);
                text.draw(&mut pm, "⌄", ((b.x + b.w - 6.0) as f32, base - 2.0), t, col, Align::Right);
            }
            // la mise en page, en petit et gris, avec ✓ (« Argos ✓ »)
            2 => {
                let s = format!("{} {}", e.mise_en_page, if e.sauvegarde { "…" } else { "✓" });
                let room = (w / 2.0 - b.x - 8.0).max(b.w);
                let s = text.fit(&s, (t - 1.0).max(10.0), room);
                text.draw(&mut pm, &s, ((b.x + 6.0) as f32, base), (t - 1.0).max(10.0), crate::jetons::DIM, Align::Left);
            }
            // 🗺 : pas dans les images d'émojis de l'app, dessiné (carte pliée)
            7 => carte(&mut pm, b.cx(), b.cy()),
            _ => text.draw(&mut pm, c.libelle, (b.cx() as f32, base + 2.0), t + 6.0, col, Align::Center),
        }
        // pastille rouge du nombre d'alertes
        if k == 3 && e.alertes > 0 {
            let n = e.alertes.to_string();
            let d = Rect::new(b.x + b.w - 13.0, b.y + 4.0, 13.0f64.max(text.width(&n, 9.0) as f64 + 6.0), 13.0);
            round_fill(&mut pm, d, 6.5, crate::jetons::BAS, 1.0);
            text.draw(&mut pm, &n, (d.cx() as f32, (d.cy() + 3.2) as f32), 9.0, crate::jetons::BLANC, Align::Center);
        }
    }
    Some(pm)
}

/// 🗺 Petite carte pliée (trois volets) aux couleurs de l'émoji.
fn carte(pm: &mut Pixmap, cx: f64, cy: f64) {
    let (w, h) = (16.0, 13.0);
    let x0 = cx - w / 2.0;
    let y0 = cy - h / 2.0;
    for k in 0..3 {
        let x = x0 + k as f64 * w / 3.0;
        let dy = if k % 2 == 0 { 0.0 } else { 1.5 };
        round_fill(
            pm,
            Rect::new(x, y0 + dy, w / 3.0, h - 1.5),
            1.0,
            if k == 1 { Rgb(0x55, 0xac, 0xee) } else { Rgb(0x77, 0xb2, 0x55) },
            1.0,
        );
    }
    round_fill(pm, Rect::new(x0 + 3.0, cy - 1.0, 5.0, 3.0), 1.0, Rgb(0xf4, 0xab, 0xba), 1.0);
}

/// Les pilules qui flottent sur la chart : DÉMO, Flux, symbole · unité, ƒx, Argos AI — (image, place).
pub fn flottants(text: &Text, w: f64, h: f64, e: &Etat, survol: Option<(f64, f64)>) -> Vec<(Pixmap, Rect)> {
    if e.pilule {
        return pilule(text, w, h, e, survol);
    }
    let m = cell();
    let mut out = vec![];
    let gras = Text::graisse(Graisse::Bold);
    let image = |r: Rect| crate::px::new(r.w.ceil() as u32 + 1, r.h.ceil() as u32 + 1);
    if !e.haut_cache {
        // 🟠 DÉMO · Argos 100K · aucun ordre réel (le petit texte va à la ligne comme dans la maquette)
        if let (Some((z, _)), Some(r)) = (m.zone("demoband"), m.place("demoband", w, h)) {
            if let Some(mut pm) = image(r) {
                let q = Rect::new(0.0, 0.0, r.w, r.h);
                let mut une_ligne = false;
                boite(&mut pm, z, q);
                let or = crate::jetons::TIER2;
                round_fill(&mut pm, Rect::new(12.0, q.cy() - 3.5, 7.0, 7.0), 3.5, or, 1.0);
                gras.draw(&mut pm, "DÉMO", (25.0, (q.cy() + 4.0) as f32), z.taille as f32, texte_c(z), Align::Left);
                let x0 = 25.0 + gras.width("DÉMO", z.taille as f32) as f64 + 10.0;
                let petit = format!("· {} · aucun ordre réel", e.compte);
                // 📟 une seule ligne (tablette) : comme la maquette (`<small>` ≈ 0,83 de la taille, 6 px après DÉMO)
                if r.h < z.boite[3] - 4.0 {
                    let x1 = x0 - 4.0;
                    let place = q.w - x1 - 12.0;
                    let mut t = z.taille as f32 * 0.83;
                    let tw = text.width(&petit, t) as f64;
                    if tw > place {
                        t *= (place / tw) as f32;
                    }
                    text.draw(&mut pm, &petit, (x1 as f32, (q.cy() + 3.5) as f32), t, crate::jetons::DIM, Align::Left);
                    une_ligne = true;
                }
                let lignes = if une_ligne {
                    vec![]
                } else {
                    couper(text, &petit, z.taille as f32, q.w - x0 - 12.0)
                };
                let n = lignes.len().max(1) as f64;
                for (k, l) in lignes.iter().enumerate() {
                    let y = q.cy() - (n - 1.0) * 6.5 + k as f64 * 13.0 + 4.0;
                    text.draw(
                        &mut pm,
                        l,
                        (((x0 + q.w - 12.0) / 2.0) as f32, y as f32),
                        z.taille as f32,
                        crate::jetons::DIM,
                        Align::Center,
                    );
                }
                out.push((pm, r));
            }
        }
        // ● Flux
        if let (Some((z, _)), Some(r)) = (m.zone("flux"), m.place("flux", w, h)) {
            let tw = text.width(&e.flux.0, z.taille as f32) as f64;
            let r = Rect::new(r.x + r.w - (tw + 30.0).max(40.0), r.y, (tw + 30.0).max(40.0), r.h);
            if let Some(mut pm) = image(r) {
                let q = Rect::new(0.0, 0.0, r.w, r.h);
                boite(&mut pm, z, q);
                let c = if e.flux.1 { crate::jetons::TIER2 } else { crate::palette::GREEN };
                round_fill(&mut pm, Rect::new(10.0, q.cy() - 3.5, 7.0, 7.0), 3.5, c, 1.0);
                text.draw(&mut pm, &e.flux.0, (22.0, (q.cy() + 4.0) as f32), z.taille as f32, texte_c(z), Align::Left);
                out.push((pm, r));
            }
        }
    }
    if !e.bas_cache && !e.feuille {
        // la pilule « symbole · unité » et sa molette
        if let (Some((z, _)), Some(r)) = (m.zone("pg"), m.place("pg", w, h)) {
            let (s0, u0) = (
                m.zone("pg.0").map(|(z, _)| z.boite[2]).unwrap_or(92.6),
                m.zone("pg.3").map(|(z, _)| z.boite[2]).unwrap_or(43.8),
            );
            // la largeur suit le nom (mesurée pour « BTC-USDC »)
            let marge_s = s0 - gras.width("BTC-USDC", 13.0) as f64;
            let sw = (gras.width(&e.symbole, 13.0) as f64 + marge_s).clamp(60.0, w * 0.5);
            let uw = (text.width(&e.unite, 13.0) as f64 + (u0 - text.width("4h", 13.0) as f64)).max(u0);
            let r = Rect::new(r.x, r.y, sw + uw + 2.0, r.h);
            if let Some(mut pm) = image(r) {
                let q = Rect::new(0.0, 0.0, r.w, r.h);
                boite(&mut pm, z, q);
                let sep = 1.0 + sw;
                round_fill(&mut pm, Rect::new(sep, 0.0, 1.0, q.h), 0.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                let dim = Rgb(255, 255, 255);
                let roue = |pm: &mut Pixmap, t: &Option<String>, cx: f64, y: f64, bold: bool| {
                    if let Some(t) = t {
                        let f = if bold { &gras } else { text };
                        f.draw(pm, t, (cx as f32, y as f32), 8.5, crate::jetons::sur_fond((dim, 0.32)), Align::Center);
                    }
                };
                let (cs, cu) = (1.0 + sw / 2.0, sep + uw / 2.0);
                roue(&mut pm, &e.sym_voisins.0, cs, 4.0 + 7.5, true);
                roue(&mut pm, &e.sym_voisins.1, cs, q.h - 4.0 - 1.0, true);
                roue(&mut pm, &e.unite_voisines.0, cu, 4.0 + 7.5, false);
                roue(&mut pm, &e.unite_voisines.1, cu, q.h - 4.0 - 1.0, false);
                gras.draw(
                    &mut pm,
                    &e.symbole,
                    (cs as f32, (q.cy() + 4.6) as f32),
                    13.0,
                    crate::jetons::TEXTE,
                    Align::Center,
                );
                text.draw(&mut pm, &e.unite, (cu as f32, (q.cy() + 4.6) as f32), 13.0, crate::jetons::TEXTE, Align::Center);
                out.push((pm, r));
            }
        }
        // ƒx
        if let (Some((z, _)), Some(r)) = (m.zone("pd"), m.place("pd", w, h)) {
            if let Some(mut pm) = image(r) {
                let q = Rect::new(0.0, 0.0, r.w, r.h);
                boite(&mut pm, z, q);
                if survol.is_some_and(|(x, y)| r.contains(x, y)) {
                    round_fill(&mut pm, q, q.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                }
                text.draw(&mut pm, "ƒx", (q.cx() as f32, (q.cy() + 4.6) as f32), 13.0, texte_c(z), Align::Center);
                out.push((pm, r));
            }
        }
    }
    // ✨ Argos AI : la pilule de verre et le logo couleur (le même bouton partout)
    if m.zone("aipill").is_some() && !e.feuille {
        let r = ai_rect(w, h, e.bas_cache);
        if let Some(pm) = crate::aibtn::image(r.w, survol.is_some_and(|(x, y)| r.contains(x, y))) {
            out.push((pm, r));
        }
    }
    out
}

/// 📱 ⚡ Height of the Funding pill of a perpetual: the height of « ⌄ N », on its line (Jee, 23 Nov 2026, choice A).
pub const FUNDING_H: f64 = 22.0;
/// 📱 ⚡ Size of the Funding pill's text and of its card's.
const FUNDING_PX: f32 = 12.5;
/// 📱 ⚡ One row of the Funding card, and its title's.
const FUNDING_LIGNE: f64 = 22.0;
const FUNDING_TITRE: f64 = 30.0;

/// 📱 ⚡ The width of the Funding pill for `libelle` (« Funding 0.0100 % 05:49:44 »), at most `max` points.
pub fn funding_largeur(text: &Text, libelle: &str, max: f64) -> f64 {
    (text.width(libelle, FUNDING_PX) as f64 + 20.0).min(max)
}

/// 📱 ⚡ The Funding pill of a perpetual (`w` points wide): its own glass pill, the text cut to fit, lighter while
/// its card is open.
pub fn funding_pilule(text: &Text, libelle: &str, w: f64, ouverte: bool) -> Option<Pixmap> {
    let mut pm = crate::px::new(w.ceil() as u32 + 1, FUNDING_H.ceil() as u32 + 1)?;
    let q = Rect::new(0.5, 0.5, w - 1.0, FUNDING_H - 1.0);
    let k = crate::jetons::pilule(q);
    round_fill(&mut pm, q, k, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
    if ouverte {
        round_fill(&mut pm, q, k, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
    }
    round_stroke(&mut pm, q, k, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    let t = text.fit(libelle, FUNDING_PX, w - 20.0);
    text.draw(&mut pm, &t, (10.0, (q.cy() + 4.5) as f32), FUNDING_PX, crate::jetons::TEXTE, Align::Left);
    Some(pm)
}

/// 📱 ⚡ The height of the Funding card for `n` rows.
pub fn funding_carte_h(n: usize) -> f64 {
    FUNDING_TITRE + n as f64 * FUNDING_LIGNE + 10.0
}

/// 📱 ⚡ The Funding card (`w` points wide): its title (« XRPUSDT Perpétuel · Bitfinex »), then one row per (name,
/// value) as the PC's row writes them; a value in `couleur` when it has one (the funding's sign).
pub fn funding_carte(text: &Text, titre: &str, lignes: &[(String, String, Option<Rgb>)], w: f64) -> Option<Pixmap> {
    let h = funding_carte_h(lignes.len());
    let mut pm = crate::px::new(w.ceil() as u32 + 1, h.ceil() as u32 + 1)?;
    let q = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
    round_fill(&mut pm, q, 14.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
    round_stroke(&mut pm, q, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    let gras = Text::graisse(Graisse::Bold);
    gras.draw(&mut pm, &gras.fit(titre, 13.0, w - 24.0), (12.0, 21.0), 13.0, crate::jetons::TEXTE, Align::Left);
    for (k, (nom, valeur, couleur)) in lignes.iter().enumerate() {
        let y = (FUNDING_TITRE + k as f64 * FUNDING_LIGNE + 15.0) as f32;
        let vw = text.width(valeur, FUNDING_PX) as f64;
        let nom = text.fit(nom, FUNDING_PX, (w - 36.0 - vw).max(30.0));
        text.draw(&mut pm, &nom, (12.0, y), FUNDING_PX, crate::jetons::DIM, Align::Left);
        let v = text.fit(valeur, FUNDING_PX, w - 24.0);
        text.draw(
            &mut pm,
            &v,
            ((w - 12.0) as f32, y),
            FUNDING_PX,
            couleur.unwrap_or(crate::jetons::TEXTE),
            Align::Right,
        );
    }
    Some(pm)
}

/// 👆 The round glass « ⋯ » button that goes with the finger's crosshair (`taille` points across).
pub fn bouton_points(taille: f64, survol: bool) -> Option<Pixmap> {
    let mut pm = crate::px::new(taille.ceil() as u32 + 1, taille.ceil() as u32 + 1)?;
    let q = Rect::new(0.5, 0.5, taille - 1.0, taille - 1.0);
    round_fill(&mut pm, q, taille / 2.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
    if survol {
        round_fill(&mut pm, q, taille / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
    }
    round_stroke(&mut pm, q, taille / 2.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    let c = taille / 2.0;
    for k in [-7.0, 0.0, 7.0] {
        round_fill(&mut pm, Rect::new(c + k - 2.2, c - 2.2, 4.4, 4.4), 2.2, crate::jetons::TEXTE, 1.0);
    }
    Some(pm)
}

/// 📱 The single pill and what goes with it (images and places, window): the « ● DÉMO » chip beside the title,
/// then the pill (or, folded, the ⌃ tab) and its fold bar. Drawn from the same zones as the touches.
pub fn pilule(text: &Text, w: f64, h: f64, e: &Etat, survol: Option<(f64, f64)>) -> Vec<(Pixmap, Rect)> {
    let gras = Text::graisse(Graisse::Bold);
    let sur = |r: Rect| survol.is_some_and(|(x, y)| r.contains(x, y));
    let verre = |pm: &mut Pixmap, q: Rect, rayon: f64| {
        round_fill(pm, q, rayon, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        round_stroke(pm, q, rayon, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    };
    let mut out = vec![];
    // ● DÉMO beside the title: one small glass chip, the dot (flux: green on time, orange late) and the gold label
    let [(f, _), (d, _)] = pilule_haut(e);
    let chip = Rect::new(f.x + 6.0, f.cy() - 13.0, d.x + d.w - f.x - 10.0, 26.0);
    if let Some(mut pm) = crate::px::new(chip.w.ceil() as u32 + 1, chip.h.ceil() as u32 + 1) {
        verre(&mut pm, Rect::new(0.0, 0.0, chip.w, chip.h), 13.0);
        let c = if e.flux.1 { crate::jetons::TIER2 } else { crate::palette::GREEN };
        round_fill(&mut pm, Rect::new(f.cx() - chip.x - 4.0, chip.h / 2.0 - 4.0, 8.0, 8.0), 4.0, c, 1.0);
        let l = Rect::new(d.x - chip.x + 4.0, 4.0, d.w - 14.0, chip.h - 8.0);
        round_fill(&mut pm, l, 9.0, crate::jetons::TIER2, 0.18);
        gras.draw(
            &mut pm,
            "DÉMO",
            (l.cx() as f32, (l.cy() + 3.6) as f32),
            10.0,
            crate::jetons::TIER2,
            Align::Center,
        );
        out.push((pm, chip));
    }
    if e.repliee {
        // ⌃ the tab that brings the pill back
        let z = pilule_onglet(w, h);
        let t = Rect::new(z.cx() - 28.0, z.y + z.h - 30.0, 56.0, 30.0);
        if let Some(mut pm) = crate::px::new(t.w as u32 + 1, t.h as u32 + 1) {
            verre(&mut pm, Rect::new(0.0, 0.0, t.w, t.h), 15.0);
            if sur(z) {
                round_fill(&mut pm, Rect::new(0.0, 0.0, t.w, t.h), 15.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            gras.draw(
                &mut pm,
                "⌃",
                ((t.w / 2.0) as f32, (t.h / 2.0 + 7.0) as f32),
                18.0,
                crate::jetons::TEXTE,
                Align::Center,
            );
            out.push((pm, t));
        }
        return out;
    }
    if e.feuille {
        return out;
    }
    // the fold bar (36 × 4)
    let b = pilule_barre(w, h);
    let bar = Rect::new(b.cx() - 18.0, b.y + b.h - 14.0, 36.0, 4.0);
    if let Some(mut pm) = crate::px::new(36, 4) {
        round_fill(
            &mut pm,
            Rect::new(0.0, 0.0, 36.0, 4.0),
            2.0,
            crate::jetons::BLANC,
            if sur(b) { 0.7 } else { 0.4 },
        );
        out.push((pm, bar));
    }
    let p = pilule_rect(w, h);
    let Some(mut pm) = crate::px::new(p.w.ceil() as u32 + 1, p.h.ceil() as u32 + 1) else {
        return out;
    };
    let q = Rect::new(0.0, 0.0, p.w, p.h);
    verre(&mut pm, q, p.h / 2.0);
    let local = |r: Rect| Rect::new(r.x - p.x, r.y - p.y, r.w, r.h);
    let pale = crate::jetons::sur_fond((crate::jetons::BLANC, 0.32));
    for (r, k) in pilule_parts(w, h) {
        let z = local(r);
        if sur(r) && k != CellHit::Ai {
            round_fill(
                &mut pm,
                Rect::new(z.x + 2.0, z.y + 6.0, z.w - 4.0, z.h - 12.0),
                14.0,
                crate::jetons::VERRE2.0,
                crate::jetons::VERRE2.1,
            );
        }
        let (cx, cy) = (z.cx(), z.cy());
        match k {
            CellHit::Symbole => {
                // logo, then the name; the wheel's neighbours, pale, above and below
                crate::watchlist::coin_badge(&mut pm, text, &e.coin, e.logo.as_ref(), z.x + 18.0, cy, 10.0);
                let x = z.x + 31.0;
                // the name: a little smaller before it is cut (« BTC-USDC » whole on a 375-point phone)
                let place = z.x + z.w - x - 4.0;
                let t = (13.0 * 0.96 * (place / gras.width(&e.symbole, 13.0).max(1.0) as f64)).clamp(10.5, 13.0) as f32;
                let nom = gras.fit(&e.symbole, t, place);
                let mx = (x + z.x + z.w - 4.0) / 2.0;
                gras.draw(&mut pm, &nom, (mx as f32, (cy + 4.6) as f32), t, crate::jetons::TEXTE, Align::Center);
                for (v, y) in [(&e.sym_voisins.0, 12.5), (&e.sym_voisins.1, z.h - 5.0)] {
                    if let Some(v) = v {
                        let v = gras.fit(v, 8.5, z.x + z.w - x - 4.0);
                        gras.draw(&mut pm, &v, (mx as f32, y as f32), 8.5, pale, Align::Center);
                    }
                }
                round_fill(
                    &mut pm,
                    Rect::new(z.x + z.w, 12.0, 1.0, z.h - 24.0),
                    0.0,
                    crate::jetons::sur_fond(crate::jetons::LISERE),
                    1.0,
                );
            }
            CellHit::Unite => {
                text.draw(&mut pm, &e.unite, (cx as f32, (cy + 4.6) as f32), 13.0, crate::jetons::TEXTE, Align::Center);
                for (v, y) in [(&e.unite_voisines.0, 12.5), (&e.unite_voisines.1, z.h - 5.0)] {
                    if let Some(v) = v {
                        text.draw(&mut pm, v, (cx as f32, y as f32), 8.5, pale, Align::Center);
                    }
                }
                round_fill(
                    &mut pm,
                    Rect::new(z.x + z.w, 12.0, 1.0, z.h - 24.0),
                    0.0,
                    crate::jetons::sur_fond(crate::jetons::LISERE),
                    1.0,
                );
            }
            CellHit::Dessins => feuille::icone(&mut pm, "pencil-simple", cx, cy, 22.0),
            CellHit::Fx => {
                text.draw(&mut pm, "ƒx", (cx as f32, (cy + 6.0) as f32), 17.0, feuille::GRIS, Align::Center);
            }
            CellHit::Trading => {
                feuille::icone(&mut pm, "chart-line-up", cx, cy, 22.0);
                if e.demo {
                    round_fill(&mut pm, Rect::new(cx + 8.0, cy - 15.0, 8.0, 8.0), 4.0, crate::jetons::TIER2, 1.0);
                }
            }
            CellHit::Plus => {
                for k in [-7.0, 0.0, 7.0] {
                    round_fill(&mut pm, Rect::new(cx + k - 2.0, cy - 2.0, 4.0, 4.0), 2.0, feuille::GRIS, 1.0);
                }
                if e.pastille > 0 {
                    let n = if e.pastille > 9 { "9+".to_string() } else { e.pastille.to_string() };
                    let b = Rect::new(cx + 5.0, cy - 19.0, 16.0, 16.0);
                    round_fill(&mut pm, b, 8.0, crate::jetons::BAS, 1.0);
                    gras.draw(&mut pm, &n, (b.cx() as f32, (b.cy() + 3.5) as f32), 9.5, crate::jetons::BLANC, Align::Center);
                }
            }
            CellHit::Ai => {
                if let Some(ai) = crate::aibtn::image(z.w, sur(r)) {
                    crate::px::put(&mut pm, z.x, z.y, &ai);
                }
            }
            _ => {}
        }
    }
    out.push((pm, p));
    out
}

/// La barre d'icônes (`w` × `dock_h()`) : verre, coins du haut arrondis, 7 icônes | ⚙ (émojis de la maquette).
pub fn dock(text: &Text, w: f64, e: &Etat, fond: Rgb, survol: Option<(f64, f64)>) -> Option<Pixmap> {
    let m = cell();
    let (z, _) = m.zone("icones")?;
    let dh = dock_h();
    let mut pm = crate::px::new(w as u32, dh as u32)?;
    crate::fill(&mut pm, 0.0, 0.0, w, dh, fond, 1.0);
    let r = Rect::new(0.0, 0.0, w, dh + z.rayon + 2.0);
    boite(&mut pm, z, r);
    let col = m.zone("icones.0").map_or(crate::jetons::TEXTE, |(c, _)| texte_c(c));
    for (cible, hit, em) in dock_boutons() {
        // (drawn at 44 points, centred in the dock: only the invisible target is bigger)
        let b = Rect::new(cible.cx() - DESSIN / 2.0, (dh - DESSIN) / 2.0, DESSIN, DESSIN);
        let on = matches!(hit, CellHit::Icone(i) if e.allume == Some(i))
            || (hit == CellHit::Autres && e.allume.is_some_and(|i| DOCK_AUTRES.iter().any(|(j, _, _)| *j == i)));
        if on || survol.is_some_and(|(sx, sy)| cible.contains(sx, sy)) {
            let (c, a) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
            round_fill(&mut pm, b, 12.0, c, a);
        }
        let t = 22.0;
        text.draw(&mut pm, em, (b.cx() as f32, (b.cy() + t as f64 * 0.36) as f32), t, col, Align::Center);
    }
    // le trait avant les autres boutons
    crate::fill(
        &mut pm,
        DOCK_TRAIT,
        (dh - 24.0) / 2.0,
        1.0,
        24.0,
        crate::jetons::sur_fond(crate::jetons::LISERE),
        1.0,
    );
    Some(pm)
}

/// 🎡 La roue de la pilule (vidéo de Jee) : verre sombre, `lignes` qui défilent, la ligne `k` au milieu en
/// surbrillance (pilule claire, texte blanc gras) ; `fraction` (−0,5 … 0,5) fait suivre le doigt au point près ; les
/// lignes du haut et du bas s'estompent. Image de `w` × `h` points.
pub fn roue(text: &Text, lignes: &[String], k: usize, fraction: f64, w: f64, h: f64, ligne_h: f64) -> Option<Pixmap> {
    let mut pm = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
    let r = Rect::new(0.0, 0.0, w, h);
    let rayon = cell().zone("roue").map_or(16.0, |(z, _)| z.rayon);
    round_fill(&mut pm, r, rayon, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
    round_stroke(&mut pm, r, rayon, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
    let gras = Text::graisse(Graisse::Bold);
    let milieu = h / 2.0;
    // la ligne du milieu : une pilule plus claire
    round_fill(
        &mut pm,
        Rect::new(6.0, milieu - ligne_h / 2.0, w - 12.0, ligne_h),
        ligne_h / 2.0,
        crate::jetons::BLANC,
        0.14,
    );
    let n = (h / ligne_h / 2.0).ceil() as i64 + 1;
    for d in -n..=n {
        let i = k as i64 + d;
        if i < 0 || i >= lignes.len() as i64 {
            continue;
        }
        let cy = milieu + (d as f64 - fraction) * ligne_h;
        if cy < -ligne_h || cy > h + ligne_h {
            continue;
        }
        // fondu vers le haut et le bas
        let a = (1.0 - ((cy - milieu).abs() / (h / 2.0))).clamp(0.0, 1.0) as f32;
        let c = crate::jetons::sur_fond((crate::jetons::BLANC, 0.25 + 0.75 * a * a));
        let t = text.fit(&lignes[i as usize], 15.0, w - 20.0);
        if d == 0 && fraction.abs() < 0.5 {
            gras.draw(&mut pm, &t, ((w / 2.0) as f32, (cy + 5.4) as f32), 15.0, crate::jetons::BLANC, Align::Center);
        } else {
            text.draw(&mut pm, &t, ((w / 2.0) as f32, (cy + 5.4) as f32), 15.0, c, Align::Center);
        }
    }
    Some(pm)
}

/// Coupe `s` en lignes d'au plus `max` points (par mots).
fn couper(t: &Text, s: &str, size: f32, max: f64) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    let mut cur = String::new();
    for mot in s.split(' ') {
        let essai = if cur.is_empty() { mot.to_string() } else { format!("{cur} {mot}") };
        if !cur.is_empty() && t.width(&essai, size) as f64 > max {
            out.push(std::mem::take(&mut cur));
            cur = mot.to_string();
        } else {
            cur = essai;
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Les mesures se lisent ; chaque zone de la maquette a son équivalent dans l'app (et l'inverse).
    #[test]
    fn chaque_zone_de_la_maquette_a_son_equivalent() {
        let m = cell();
        assert_eq!(m.taille(), (390.0, 844.0));
        let ids: Vec<&str> = m
            .zones
            .iter()
            .flat_map(|z| std::iter::once(z.id).chain(z.enfants.iter().map(|e| e.id)))
            .collect();
        // ce que l'app sait dessiner et toucher
        let connus = |id: &str| {
            matches!(
                id,
                "thaut" | "demoband" | "flux" | "favbar" | "favbar.0" | "pg" | "pd" | "pd.0" | "aipill" | "icones" | "icones.10"
            ) || id.starts_with("thaut.")
                || id.starts_with("pg.")
                // la watchlist du cell (mesures des photos de Jee) : dessinée par watchlist.rs
                || id.starts_with("wl-")
                || id == "roue"
                // 📟 the dock's buttons: redrawn at 44 points for the finger (`dock_boutons`)
                || id.starts_with("icones.")
        };
        let manquent: Vec<&&str> = ids.iter().filter(|i| !connus(i)).collect();
        assert!(manquent.is_empty(), "zones de la maquette sans équivalent : {manquent:?}");
        // à 390 × 844, les zones de l'app sont exactement celles mesurées
        let e = Etat::default();
        let z = zones(390.0, 844.0, &e);
        let r = |k: CellHit| z.iter().find(|(_, h)| *h == k).unwrap().0;
        assert_eq!(r(CellHit::Fx), Rect::new(264.0, 728.0, 50.0, 50.0));
        assert_eq!(r(CellHit::Ai), Rect::new(322.0, 776.0, 56.0, 56.0), "collé au coin (choix B)");
        assert_eq!(
            r(CellHit::Icone(Icone::Reglages)),
            Rect::new(6.0, 789.0, 55.0, 55.0),
            "⚙ à gauche (choix B), 55 points (44 vrais px)"
        );
        assert_eq!(r(CellHit::Demo), Rect::new(97.5, 58.0, 195.0, 32.8));
        // un autre cell (430 × 932) : ce qui est à droite reste collé à droite, en bas reste en bas
        let z = zones(430.0, 932.0, &e);
        let ai = z.iter().find(|(_, h)| *h == CellHit::Ai).unwrap().0;
        assert_eq!((ai.x + ai.w, ai.y + ai.h), (378.0 + 40.0, 832.0 + 88.0));
    }

    /// La table Rust est à jour avec les mesures (sinon : relancer argos-outils maquette).
    #[test]
    fn table_a_jour_avec_les_mesures() {
        let v: serde_json::Value = serde_json::from_str(include_str!("maquette-cell.json")).unwrap();
        let mut n = 0;
        for z in v["zones"].as_array().unwrap() {
            for e in std::iter::once(z).chain(z["enfants"].as_array().unwrap().iter()) {
                let id = e["id"].as_str().unwrap();
                let (t, _) = cell().zone(id).unwrap_or_else(|| panic!("{id} absent de la table"));
                let b: Vec<f64> = e["boite"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
                assert_eq!(t.boite.to_vec(), b, "{id}");
                n += 1;
            }
        }
        assert!(n >= 30);
    }

    /// 📟 Chaque zone de la maquette TABLETTE (820 × 1180, mesurée : `maquette-tablette.json`) a son équivalent dans
    /// l'app : les zones du cell placées à cette taille (même place au demi-point près), les icônes de plus (👥 🌐 🛒)
    /// et l'écran entier (`zones`) pour le toucher.
    #[test]
    fn chaque_zone_de_la_maquette_tablette_a_son_equivalent() {
        let v: serde_json::Value = serde_json::from_str(include_str!("maquette-tablette.json")).unwrap();
        let (w, h) = (820.0, 1180.0);
        let mut n = 0;
        for z in v["zones"].as_array().unwrap() {
            for e in std::iter::once(z).chain(z["enfants"].as_array().unwrap().iter()) {
                let id = e["id"].as_str().unwrap();
                let b: Vec<f64> = e["boite"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
                // 📟 the dock's buttons and the top icons: redrawn at 44 points for the finger (tested below)
                if id.starts_with("icones.") || ["thaut.3", "thaut.4", "thaut.5", "thaut.6", "thaut.7"].contains(&id) {
                    continue;
                }
                let r = cell().place(id, w, h).unwrap_or_else(|| panic!("{id} absent"));
                for (k, (a, m)) in [r.x, r.y, r.w, r.h].iter().zip(&b).enumerate() {
                    assert!((a - m).abs() <= 0.5, "{id} [{k}] : app {a} ≠ maquette {m} ({r:?})");
                }
                n += 1;
            }
        }
        assert!(n >= 15, "{n} zones");
        let z = zones(w, h, &Etat::default());
        for (i, _) in DOCK {
            assert!(z.iter().any(|(_, c)| *c == CellHit::Icone(i)), "{i:?} se touche");
        }
        assert!(z.iter().any(|(_, c)| *c == CellHit::Autres), "⋯ se touche");
        for (i, _, _) in DOCK_AUTRES {
            assert!(!z.iter().any(|(_, c)| *c == CellHit::Icone(i)), "{i:?} : sous ⋯");
        }
    }

    /// 📟 iPad (liste 2, points 6 et 21) : 🔔 ▦ 📤 🔎 🗺 et le dock (⚙ 📈 🧪 🛡 ✎ 🌐 ⋯) en cibles de 44 × 44, sans
    /// chevauchement, dans l'écran, portrait et paysage ; chaque bouton du dock compte une seule fois.
    #[test]
    fn tablette_cibles_de_44_sans_chevauchement() {
        for (w, h) in [(820.0, 1180.0), (1180.0, 820.0), (768.0, 1024.0)] {
            let z = zones(w, h, &Etat::default());
            let cibles: Vec<Rect> = z
                .iter()
                .filter(|(_, c)| {
                    matches!(
                        c,
                        CellHit::Alertes | CellHit::Disposition | CellHit::Partager | CellHit::Recherche | CellHit::Plan | CellHit::Icone(_) | CellHit::Autres
                    )
                })
                .map(|(r, _)| *r)
                .collect();
            assert_eq!(cibles.len(), 5 + 7, "{w}×{h}");
            for (k, a) in cibles.iter().enumerate() {
                assert!(a.w >= BOUTON && a.h >= BOUTON, "{a:?}");
                assert!(a.x >= 0.0 && a.x + a.w <= w && a.y >= 0.0 && a.y + a.h <= h, "{a:?}");
                for b in &cibles[k + 1..] {
                    assert!(!(a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h), "{a:?} {b:?}");
                }
            }
            // nothing else of the screen under them (⌂ ARGOS, mise en page, Argos AI)
            for (r, c) in &z {
                if matches!(c, CellHit::Accueil | CellHit::Argos | CellHit::MiseEnPage | CellHit::Ai) {
                    for a in &cibles {
                        assert!(
                            !(a.x < r.x + r.w && r.x < a.x + a.w && a.y < r.y + r.h && r.y < a.y + a.h),
                            "{c:?} {r:?} sous {a:?}"
                        );
                    }
                }
            }
        }
        assert!(dock(&Text::default(), 820.0, &Etat::default(), Rgb(0, 0, 0), None).is_some());
        assert!(haut(&Text::default(), 820.0, &Etat::default(), None, None).is_some());
    }

    #[test]
    fn couleurs_css() {
        assert_eq!(couleur("rgba(26, 30, 42, 0.6)"), (Rgb(26, 30, 42), 0.6));
        assert_eq!(couleur("rgb(232, 234, 240)"), (Rgb(232, 234, 240), 1.0));
    }

    /// 📸 L'écran cell (rangée du haut, pilules, barre d'icônes) : seulement si `ARGOS_CAPTURES`.
    #[test]
    fn capture_cell() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let e = Etat {
            mise_en_page: "Argos".into(),
            alertes: 3,
            compte: "Argos 100K".into(),
            flux: ("Flux 1 en retard".into(), true),
            symbole: "BTC-USDC".into(),
            sym_voisins: (Some("ETH".into()), Some("SOL".into())),
            unite: "4h".into(),
            unite_voisines: (Some("1h".into()), Some("1D".into())),
            ..Default::default()
        };
        let (w, h) = (390.0, 844.0);
        let mut pm = crate::px::new(w as u32, h as u32).unwrap();
        crate::fill(&mut pm, 0.0, 0.0, w, h, Rgb(0x0b, 0x0e, 0x16), 1.0);
        let fond = Rgb(0x0b, 0x0e, 0x16);
        crate::px::put(&mut pm, 0.0, 0.0, &haut(&t, w, &e, Some(fond), None).unwrap());
        let d = dock(&t, w, &e, fond, None).unwrap();
        crate::px::put(&mut pm, 0.0, h - dock_h(), &d);
        for (p, r) in flottants(&t, w, h, &e, None) {
            crate::px::put(&mut pm, r.x, r.y, &p);
        }
        pm.save_png(format!("{dir}/ecran-cell.png")).unwrap();
    }

    /// 📱 The single pill: every zone at least 40 points, inside the screen, none overlapping; the parts in the
    /// order of the mock-up; folded, only the ⌃ tab (and the DÉMO chip); a sheet open hides the pill.
    #[test]
    fn pilule_unique_zones() {
        // (layout points: the phone's screen at the compact factor 0.8 — 402 × 874 is 502 × 1092)
        let k = crate::formfactor::COMPACTE_CELL;
        for (w, h) in [(402.0 / k, 874.0 / k), (390.0 / k, 844.0 / k), (375.0 / k, 667.0 / k)] {
            let mut e = Etat {
                pilule: true,
                titre: (150.0, 22.0),
                symbole: "BTC-USDC".into(),
                unite: "30m".into(),
                ..Default::default()
            };
            let z = zones(w, h, &e);
            let kinds: Vec<CellHit> = z.iter().map(|(_, k)| *k).collect();
            assert_eq!(
                kinds,
                [
                    CellHit::Flux,
                    CellHit::Demo,
                    CellHit::Replier,
                    CellHit::Symbole,
                    CellHit::Unite,
                    CellHit::Dessins,
                    CellHit::Fx,
                    CellHit::Trading,
                    CellHit::Plus,
                    CellHit::Ai
                ]
            );
            for (k, (a, ka)) in z.iter().enumerate() {
                assert!(a.w >= feuille::TOUCHER && a.h >= feuille::TOUCHER, "{ka:?} {a:?} at {w}: 40 screen points");
                assert!(a.x >= 0.0 && a.x + a.w <= w && a.y + a.h <= h, "{ka:?} inside");
                for (b, kb) in &z[k + 1..] {
                    let inter = a.x < b.x + b.w - 0.01 && b.x < a.x + a.w - 0.01 && a.y < b.y + b.h - 0.01 && b.y < a.y + a.h - 0.01;
                    assert!(!inter, "{ka:?} and {kb:?} overlap at {w}");
                }
                assert_eq!(hit(w, h, &e, a.cx(), a.cy()).map(|x| x.0), Some(*ka));
            }
            assert_eq!(pilule(&Text::default(), w, h, &e, None).len(), 3, "chip, bar, pill");
            e.repliee = true;
            let kinds: Vec<CellHit> = zones(w, h, &e).iter().map(|(_, k)| *k).collect();
            assert_eq!(kinds, [CellHit::Flux, CellHit::Demo, CellHit::Rouvrir]);
            assert_eq!(pilule_reserve(true), 0.0);
            e.repliee = false;
            e.feuille = true;
            assert!(!zones(w, h, &e).iter().any(|(_, k)| *k == CellHit::Plus), "a sheet open: no pill");
        }
    }
}
