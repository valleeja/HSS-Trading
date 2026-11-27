//! 🎓 Académie Argos (page plein écran) : l'accueil (progression globale, ▶ Reprendre, les parcours 0 à 8), un
//! parcours (ses leçons par groupe) et la fiche d'une leçon (sections emoji, tableaux, blocs repliables, check-list,
//! quiz de 5 questions avec score). Le contenu et la logique viennent du crate `argos-academy` (leçons Markdown
//! embarquées) ; la progression (lues, terminées, scores, fuseau) est gardée par l'app dans `academy.json`.
//! Les horaires des leçons sont à l'heure de New York ; le bouton 🕘 du haut choisit le fuseau de l'élève.
//! C'est de la formation, pas un conseil financier.

use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
pub use argos_academy::markdown::{Bloc, Morceau};
pub use argos_academy::{lecons, Fuseau, Lecon, Progress, PARCOURS};
use tiny_skia::Pixmap;

/// La fiche « News éco → DXY / XAU » (parcours 5) : l'Agenda y renvoie, l'assistant s'en sert comme référence.
pub const NEWS_GUIDE: &str = "5-01-news-dxy-xau";

/// Le titre d'une leçon (par son index).
pub fn titre(i: usize) -> &'static str {
    lecons().get(i).map_or("", |l| l.titre.as_str())
}

/// Ligne du tableau de la fiche news (section 1, sans l'en-tête) qui parle de cette annonce du calendrier
/// (titres Forex Factory : « Core CPI m/m », « Non-Farm Employment Change », « Unemployment Claims »…).
pub fn news_row(title: &str) -> Option<usize> {
    let t = title.to_lowercase();
    let has = |k: &[&str]| k.iter().any(|w| t.contains(w));
    Some(if has(&["inflation expectations"]) {
        11
    } else if has(&["consumer sentiment", "umich", "uom "]) {
        10
    } else if has(&["unemployment claims", "jobless"]) {
        6
    } else if has(&["unemployment rate"]) {
        4
    } else if has(&["non-farm", "nonfarm", "nfp"]) {
        3
    } else if has(&["average hourly earnings"]) {
        5
    } else if has(&["jolts"]) {
        7
    } else if has(&["pce"]) {
        2
    } else if has(&["ppi"]) {
        1
    } else if has(&["cpi"]) {
        0
    } else if has(&["retail sales"]) {
        8
    } else if has(&["ism "]) {
        9
    } else if has(&["gdp"]) {
        12
    } else if has(&["fomc", "federal funds rate"]) {
        13
    } else {
        return None;
    })
}

/// Une leçon en texte simple (tableaux en lignes « | a | b | »), pour la référence de l'assistant.
pub fn guide_text(id: &str) -> Option<String> {
    Some(lecons()[argos_academy::index(id)?].texte())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum View {
    Home,
    /// Un parcours (son numéro).
    Parcours(u8),
    /// Une leçon (index dans `lecons()`).
    Lecon(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Back,
    Parcours(u8),
    Lecon(usize),
    Choice(usize, usize),
    /// Une ligne de tableau (section, ligne sans l'en-tête) : repère pour y aller, pas un bouton.
    Row(usize, usize),
    /// Ouvrir / refermer un bloc repliable (section, bloc).
    Details(usize, usize),
    Recommencer,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum AcClick {
    None,
    Close,
    /// La progression a changé (à garder).
    Changed,
}

/// La page.
#[derive(Debug, Clone, PartialEq)]
pub struct AcademyPage {
    pub view: View,
    pub progress: Progress,
    /// Réponses du quiz de la leçon ouverte (une par question, `None` = pas encore répondu).
    answers: Vec<Option<usize>>,
    scroll: f64,
    /// Ligne de tableau mise en avant (section, ligne), ouverte depuis l'Agenda.
    focus: Option<(usize, usize)>,
    /// Blocs repliables ouverts (section, bloc).
    ouverts: Vec<(usize, usize)>,
}

const HEAD: f64 = 60.0;
const PAD: f64 = 18.0;

/// Le bouton 🕘 du fuseau, dans l'en-tête (à gauche du compteur).
fn bouton_fuseau(w: f64) -> Rect {
    let bw = if w < 520.0 { 44.0 } else { 132.0 };
    Rect::new(w - 64.0 - (if w < 520.0 { 52.0 } else { 170.0 }) - bw, 14.0, bw, 32.0)
}

enum Op {
    Text(String, f64, f64, f32, Rgb, Align),
    Fill(Rect, Rgb, f32),
    Stroke(Rect, Rgb),
}

/// Dessine un tableau à `y` : colonnes sur grand écran (largeur selon le texte), une carte par ligne sur téléphone
/// (première colonne en titre, puis « en-tête : valeur »). `focus` : ligne mise en avant.
#[allow(clippy::too_many_arguments)]
fn table(ops: &mut Vec<Op>, zones: &mut Vec<(Rect, Hit)>, y: &mut f64, text: &Text, inner: f64, si: usize, rows: &[Vec<String>], focus: Option<usize>) {
    const S: f32 = 13.0;
    const LH: f64 = 18.5;
    let Some((head, body)) = rows.split_first() else { return };
    let n = head.len().max(1);
    let wrap = |s: &str, w: f64| crate::assistant::wrap(text, s, S, w.max(30.0));
    let mark = |ops: &mut Vec<Op>, r: Rect, k: usize| {
        if focus == Some(k) {
            ops.push(Op::Fill(r, pal::accent(), 0.22));
            ops.push(Op::Stroke(r, pal::accent()));
        }
    };
    if inner >= 640.0 {
        // largeur des colonnes selon la longueur du texte (bornée)
        let len = |c: usize| rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count().clamp(5, 38)).max().unwrap_or(5) as f64;
        let total: f64 = (0..n).map(len).sum();
        let ws: Vec<f64> = (0..n).map(|c| inner * len(c) / total).collect();
        let line = |ops: &mut Vec<Op>, y: f64, cells: &[String], col: Rgb| -> f64 {
            let mut x = PAD;
            let mut h: f64 = 1.0;
            for (c, w) in ws.iter().enumerate() {
                let ls = wrap(cells.get(c).map_or("", |s| s.as_str()), w - 16.0);
                for (k, l) in ls.iter().enumerate() {
                    ops.push(Op::Text(l.clone(), x + 8.0, y + 19.0 + k as f64 * LH, S, col, Align::Left));
                }
                h = h.max(ls.len() as f64);
                x += w;
            }
            h * LH + 10.0
        };
        let top = *y;
        let start = ops.len();
        let hh = line(ops, *y, head, GREY);
        ops.insert(start, Op::Fill(Rect::new(PAD, *y, inner, hh), pal::field(), 1.0));
        *y += hh;
        for (k, r) in body.iter().enumerate() {
            let start = ops.len();
            let h = line(ops, *y, r, WHITE);
            let rect = Rect::new(PAD, *y, inner, h);
            let mut deco = vec![];
            if k % 2 == 1 {
                deco.push(Op::Fill(rect, pal::card(), 1.0));
            }
            mark(&mut deco, rect, k);
            ops.splice(start..start, deco);
            zones.push((rect, Hit::Row(si, k)));
            *y += h;
        }
        ops.push(Op::Stroke(Rect::new(PAD, top, inner, *y - top), pal::edge_soft()));
    } else {
        for (k, r) in body.iter().enumerate() {
            let mut ls: Vec<(String, f32, Rgb)> = wrap(r.first().map_or("", |s| s.as_str()), inner - 24.0)
                .into_iter()
                .map(|l| (l, 14.0, WHITE))
                .collect();
            for (h, v) in head.iter().zip(r).skip(1).filter(|(_, v)| !v.is_empty()) {
                ls.extend(wrap(&format!("{h} : {v}"), inner - 24.0).into_iter().map(|l| (l, S, GREY)));
            }
            let rect = Rect::new(PAD, *y, inner, ls.len() as f64 * LH + 16.0);
            ops.push(Op::Fill(rect, pal::card(), 1.0));
            ops.push(Op::Stroke(rect, pal::edge_soft()));
            mark(ops, rect, k);
            for (i, (l, size, col)) in ls.into_iter().enumerate() {
                ops.push(Op::Text(l, PAD + 12.0, *y + 21.0 + i as f64 * LH, size, col, Align::Left));
            }
            zones.push((rect, Hit::Row(si, k)));
            *y += rect.h + 8.0;
        }
    }
}

/// Dessine une ligne de texte riche (`m`) enroulée sur `w`, avec un préfixe (« • », « 2. ») : le gras du début
/// (« **Entrée :** … ») passe en couleur d'accent ; le reste en blanc.
#[allow(clippy::too_many_arguments)]
fn riche(ops: &mut Vec<Op>, y: &mut f64, text: &Text, prefixe: &str, m: &[Morceau], x: f64, w: f64, size: f32) {
    let plain = format!("{prefixe}{}", argos_academy::markdown::simple(m));
    let lignes = crate::assistant::wrap(text, &plain, size, w);
    let tete = match m.first() {
        Some(Morceau { texte, gras: true }) => format!("{prefixe}{texte}"),
        _ => String::new(),
    };
    for (k, l) in lignes.into_iter().enumerate() {
        *y += size as f64 * 1.45;
        if k == 0 && !tete.is_empty() && l.starts_with(&tete) {
            let dx = text.width(&tete, size) as f64;
            ops.push(Op::Text(tete.clone(), x, *y, size, pal::accent(), Align::Left));
            ops.push(Op::Text(l[tete.len()..].to_string(), x + dx, *y, size, WHITE, Align::Left));
        } else {
            ops.push(Op::Text(l, x, *y, size, WHITE, Align::Left));
        }
    }
}

/// Une barre de progression (`k` sur `n`).
fn jauge(ops: &mut Vec<Op>, r: Rect, k: usize, n: usize) {
    ops.push(Op::Fill(r, pal::field(), 1.0));
    if n > 0 && k > 0 {
        ops.push(Op::Fill(Rect::new(r.x, r.y, r.w * k as f64 / n as f64, r.h), pal::up(), 1.0));
    }
}

impl AcademyPage {
    pub fn new(progress: Progress) -> Self {
        Self {
            view: View::Home,
            progress,
            answers: vec![],
            scroll: 0.0,
            focus: None,
            ouverts: vec![],
        }
    }

    /// Leçons terminées / total.
    pub fn done_count(&self) -> (usize, usize) {
        self.progress.globale()
    }

    /// Ouvre la leçon `id` à la ligne `row` du premier tableau de sa section `section` (mise en avant, défilée en vue).
    pub fn open_at(&mut self, id: &str, section: usize, row: usize, text: &Text, w: f64) -> AcClick {
        let Some(i) = argos_academy::index(id) else {
            return AcClick::None;
        };
        let changed = self.open_guide(i);
        self.focus = Some((section, row));
        let (_, zones, _) = self.build(text, w);
        if let Some((r, _)) = zones.iter().find(|(_, z)| *z == Hit::Row(section, row)) {
            self.scroll = (r.y - 120.0).max(0.0);
        }
        changed
    }

    /// ⌂ La prochaine leçon du parcours pas encore terminée (pour « ▶ Reprendre » de l'Accueil).
    pub fn next_guide(&self) -> Option<usize> {
        self.progress.prochaine()
    }

    /// ▶ Reprendre : ouvre la prochaine leçon pas encore terminée.
    pub fn resume(&mut self) -> AcClick {
        match self.next_guide() {
            Some(i) => self.open_guide(i),
            None => AcClick::None,
        }
    }

    fn open_guide(&mut self, i: usize) -> AcClick {
        let l = &lecons()[i];
        self.view = View::Lecon(i);
        (self.focus, self.scroll) = (None, 0.0);
        self.ouverts.clear();
        self.answers = vec![None; l.quiz.len()];
        if self.progress.lire(l) {
            AcClick::Changed
        } else {
            AcClick::None
        }
    }

    /// Échap : retour d'une leçon à son parcours, d'un parcours à l'accueil ; `true` = fermer la page.
    pub fn escape(&mut self) -> bool {
        self.scroll = 0.0;
        match self.view {
            View::Lecon(i) => {
                self.view = View::Parcours(lecons()[i].parcours);
                false
            }
            View::Parcours(_) => {
                self.view = View::Home;
                false
            }
            View::Home => true,
        }
    }

    pub fn wheel(&mut self, dy: f64, text: &Text, w: f64, h: f64) {
        let (_, _, full) = self.build(text, w);
        self.scroll = (self.scroll + dy).clamp(0.0, (full - (h - HEAD)).max(0.0));
    }

    pub fn at_top(&self) -> bool {
        self.scroll <= 0.0
    }

    pub fn click(&mut self, text: &Text, w: f64, _h: f64, x: f64, y: f64) -> AcClick {
        if Rect::new(w - 52.0, 12.0, 36.0, 36.0).contains(x, y) {
            return AcClick::Close;
        }
        if bouton_fuseau(w).contains(x, y) {
            self.progress.fuseau = self.progress.fuseau.suivant();
            return AcClick::Changed;
        }
        if y < HEAD {
            return AcClick::None;
        }
        let (_, zones, _) = self.build(text, w);
        let cy = y - HEAD + self.scroll;
        let Some(hit) = zones.iter().rev().find(|(r, _)| r.contains(x, cy)).map(|(_, h)| *h) else {
            return AcClick::None;
        };
        match hit {
            Hit::Row(..) => AcClick::None,
            Hit::Back => {
                self.escape();
                AcClick::None
            }
            Hit::Parcours(p) => {
                (self.view, self.scroll) = (View::Parcours(p), 0.0);
                AcClick::None
            }
            Hit::Lecon(i) => self.open_guide(i),
            Hit::Details(s, b) => {
                match self.ouverts.iter().position(|o| *o == (s, b)) {
                    Some(k) => {
                        self.ouverts.remove(k);
                    }
                    None => self.ouverts.push((s, b)),
                }
                AcClick::None
            }
            Hit::Recommencer => {
                self.answers.iter_mut().for_each(|a| *a = None);
                AcClick::None
            }
            Hit::Choice(q, c) => {
                let View::Lecon(i) = self.view else { return AcClick::None };
                if self.answers.get(q).is_some_and(|a| a.is_none()) {
                    self.answers[q] = Some(c);
                }
                let l = &lecons()[i];
                if self.answers.iter().all(Option::is_some) && self.progress.noter(l, argos_academy::score(l, &self.answers)) {
                    return AcClick::Changed;
                }
                AcClick::None
            }
        }
    }

    /// Tout le contenu de la vue (coordonnées sous l'en-tête, avant défilement) : dessin, zones, hauteur.
    fn build(&self, text: &Text, w: f64) -> (Vec<Op>, Vec<(Rect, Hit)>, f64) {
        let (mut ops, mut zones) = (vec![], vec![]);
        let inner = (w - 2.0 * PAD).max(200.0);
        let mut y = 14.0;
        let wrap = |s: &str, size: f32| crate::assistant::wrap(text, s, size, inner);
        let para = |ops: &mut Vec<Op>, y: &mut f64, s: &str, size: f32, col: Rgb| {
            for l in wrap(s, size) {
                *y += size as f64 * 1.45;
                ops.push(Op::Text(l, PAD, *y, size, col, Align::Left));
            }
        };
        let bouton = |ops: &mut Vec<Op>, zones: &mut Vec<(Rect, Hit)>, r: Rect, s: &str, hit: Hit| {
            ops.push(Op::Fill(r, pal::field(), 1.0));
            ops.push(Op::Text(
                text.fit(s, 13.0, r.w - 16.0),
                r.cx(),
                r.y + r.h / 2.0 + 5.0,
                13.0,
                WHITE,
                Align::Center,
            ));
            zones.push((r, hit));
        };
        match self.view {
            View::Home => {
                // progression globale et ▶ Reprendre
                let (d, n) = self.progress.globale();
                let half = if inner >= 560.0 { (inner - 10.0) / 2.0 } else { inner };
                let r = Rect::new(PAD, y, half, 84.0);
                ops.push(Op::Fill(r, pal::card(), 1.0));
                ops.push(Op::Stroke(r, pal::edge_soft()));
                ops.push(Op::Text("Progression globale".into(), r.x + 12.0, r.y + 24.0, 13.0, GREY, Align::Left));
                let pct = (d * 100).checked_div(n).unwrap_or(0);
                ops.push(Op::Text(format!("{pct} %"), r.x + r.w - 12.0, r.y + 26.0, 20.0, WHITE, Align::Right));
                jauge(&mut ops, Rect::new(r.x + 12.0, r.y + 40.0, r.w - 24.0, 8.0), d, n);
                ops.push(Op::Text(
                    format!("{d} leçons terminées sur {n}"),
                    r.x + 12.0,
                    r.y + 70.0,
                    12.0,
                    GREY,
                    Align::Left,
                ));
                let r2 = if inner >= 560.0 {
                    Rect::new(PAD + half + 10.0, y, half, 84.0)
                } else {
                    Rect::new(PAD, y + 94.0, inner, 84.0)
                };
                ops.push(Op::Fill(r2, pal::card(), 1.0));
                ops.push(Op::Stroke(r2, pal::edge_soft()));
                match self.next_guide() {
                    Some(i) => {
                        let l = &lecons()[i];
                        let p = &PARCOURS[l.parcours as usize];
                        ops.push(Op::Text(
                            "▶ Reprendre où j'étais".into(),
                            r2.x + 12.0,
                            r2.y + 24.0,
                            14.0,
                            pal::accent(),
                            Align::Left,
                        ));
                        ops.push(Op::Text(
                            text.fit(&l.titre, 14.0, r2.w - 24.0),
                            r2.x + 12.0,
                            r2.y + 48.0,
                            14.0,
                            WHITE,
                            Align::Left,
                        ));
                        let sous = format!("{} {} · {}", p.icone, p.num, p.nom);
                        ops.push(Op::Text(text.fit(&sous, 12.0, r2.w - 24.0), r2.x + 12.0, r2.y + 70.0, 12.0, GREY, Align::Left));
                        zones.push((r2, Hit::Lecon(i)));
                    }
                    None => ops.push(Op::Text(
                        "🏆 Toutes les leçons sont terminées".into(),
                        r2.x + 12.0,
                        r2.y + 48.0,
                        14.0,
                        WHITE,
                        Align::Left,
                    )),
                }
                y = r2.y + r2.h + 18.0;
                ops.push(Op::Text("PARCOURS".into(), PAD, y + 14.0, 12.0, GREY, Align::Left));
                y += 26.0;
                let cols = ((inner / 300.0).floor() as usize).clamp(1, 3);
                let cw = (inner - (cols as f64 - 1.0) * 10.0) / cols as f64;
                // 📱 au doigt : seulement les parcours ouverts (pas de carte « bientôt »)
                let montres: Vec<_> = PARCOURS.iter().filter(|p| p.ouvert || !crate::actions::tactile()).collect();
                for (k, p) in montres.iter().enumerate() {
                    let r = Rect::new(PAD + (k % cols) as f64 * (cw + 10.0), y + (k / cols) as f64 * 86.0, cw, 78.0);
                    let (fait, total) = self.progress.du_parcours(p.num);
                    ops.push(Op::Fill(r, pal::card(), if p.ouvert { 1.0 } else { 0.5 }));
                    ops.push(Op::Stroke(r, pal::edge_soft()));
                    let col = if p.ouvert { WHITE } else { GREY };
                    let name = text.fit(&format!("{} · {}{}", p.num, icon(p.icone), p.nom), 15.0, r.w - 24.0);
                    ops.push(Op::Text(name, r.x + 12.0, r.y + 26.0, 15.0, col, Align::Left));
                    if p.ouvert {
                        let s = if total > 1 { "s" } else { "" };
                        ops.push(Op::Text(format!("{total} leçon{s}"), r.x + 12.0, r.y + 48.0, 12.0, GREY, Align::Left));
                        let c = if fait == total { pal::up() } else { GREY };
                        ops.push(Op::Text(format!("{fait} / {total}"), r.x + r.w - 12.0, r.y + 48.0, 12.0, c, Align::Right));
                        jauge(&mut ops, Rect::new(r.x + 12.0, r.y + 60.0, r.w - 24.0, 6.0), fait, total);
                        zones.push((r, Hit::Parcours(p.num)));
                    } else {
                        ops.push(Op::Text("bientôt".into(), r.x + 12.0, r.y + 48.0, 12.0, GREY, Align::Left));
                    }
                }
                y += montres.len().div_ceil(cols) as f64 * 86.0 + 10.0;
                para(
                    &mut ops,
                    &mut y,
                    "Formation seulement ; ce n'est pas un conseil financier. Teste toujours en démo avant le réel.",
                    12.0,
                    GREY,
                );
            }
            View::Parcours(num) => {
                let p = &PARCOURS[num as usize];
                bouton(&mut ops, &mut zones, Rect::new(PAD, y, 120.0, 32.0), "‹ Parcours", Hit::Back);
                y += 44.0;
                para(&mut ops, &mut y, &format!("{}{} · {}", icon(p.icone), p.num, p.nom), 20.0, WHITE);
                let (fait, total) = self.progress.du_parcours(num);
                y += 10.0;
                jauge(&mut ops, Rect::new(PAD, y, inner, 6.0), fait, total);
                y += 6.0;
                para(&mut ops, &mut y, &format!("{fait} / {total} leçons terminées"), 12.5, GREY);
                y += 8.0;
                let mut groupe = "";
                for li in argos_academy::lecons_du(num) {
                    let l = &lecons()[li];
                    if l.groupe != groupe && !l.groupe.is_empty() {
                        groupe = &l.groupe;
                        y += 8.0;
                        ops.push(Op::Text(groupe.to_uppercase(), PAD, y + 14.0, 11.5, GREY, Align::Left));
                        y += 24.0;
                    }
                    let r = Rect::new(PAD, y, inner, 58.0);
                    ops.push(Op::Fill(r, pal::card(), 1.0));
                    ops.push(Op::Stroke(r, pal::edge_soft()));
                    let mark = if self.progress.is_done(l) {
                        "✅ "
                    } else if self.progress.read.contains(&l.id) {
                        "📖 "
                    } else {
                        "○ "
                    };
                    ops.push(Op::Text(
                        text.fit(&format!("{mark}{}", l.titre), 15.0, r.w - 24.0),
                        r.x + 12.0,
                        r.y + 24.0,
                        15.0,
                        WHITE,
                        Align::Left,
                    ));
                    let quiz = match (l.quiz.is_empty(), self.progress.scores.get(&l.id)) {
                        (true, _) => " · Fiche".to_string(),
                        (false, Some(s)) => format!(" · Quiz {s} / {}", l.quiz.len()),
                        (false, None) => " · Quiz".to_string(),
                    };
                    ops.push(Op::Text(format!("⏱️ {} min{quiz}", l.minutes), r.x + 12.0, r.y + 46.0, 12.0, GREY, Align::Left));
                    zones.push((r, Hit::Lecon(li)));
                    y += 66.0;
                }
            }
            View::Lecon(li) => {
                let l = &lecons()[li];
                let p = &PARCOURS[l.parcours as usize];
                bouton(
                    &mut ops,
                    &mut zones,
                    Rect::new(PAD, y, 200.0_f64.min(inner), 32.0),
                    &format!("‹ {}", p.nom),
                    Hit::Back,
                );
                y += 40.0;
                para(&mut ops, &mut y, &l.titre, 21.0, WHITE);
                let mut meta = format!("Parcours {}", p.num);
                if !l.groupe.is_empty() {
                    meta += &format!(" · {}", l.groupe);
                }
                meta += &format!(" · ⏱️ {} min", l.minutes);
                if !l.note.is_empty() {
                    meta += &format!(" · {}", l.note);
                }
                para(&mut ops, &mut y, &meta, 12.5, GREY);
                if l.horaires {
                    y += 10.0;
                    let s = self.progress.fuseau.phrase(crate::journal::now_ms() / 1000);
                    let ls = crate::assistant::wrap(text, &s, 13.0, inner - 24.0);
                    let r = Rect::new(PAD, y, inner, ls.len() as f64 * 19.0 + 14.0);
                    ops.push(Op::Fill(r, pal::accent(), 0.14));
                    for (k, t) in ls.into_iter().enumerate() {
                        ops.push(Op::Text(t, r.x + 12.0, r.y + 22.0 + k as f64 * 19.0, 13.0, WHITE, Align::Left));
                    }
                    y += r.h;
                }
                for (si, s) in l.sections.iter().enumerate() {
                    y += 16.0;
                    para(&mut ops, &mut y, &s.titre, 17.0, pal::accent());
                    let mut premier_tableau = true;
                    for (bi, b) in s.blocs.iter().enumerate() {
                        y += 4.0;
                        match b {
                            Bloc::Para(m) => riche(&mut ops, &mut y, text, "", m, PAD, inner, 14.0),
                            // (the lessons have none today; the editor's 📖 Doc draws them itself)
                            Bloc::Titre(_, t) => para(&mut ops, &mut y, t, 15.0, WHITE),
                            Bloc::Code(lignes) => {
                                for l in lignes {
                                    para(&mut ops, &mut y, l, 13.0, GREY);
                                }
                            }
                            Bloc::Liste { numerotee, elements } => {
                                for (k, e) in elements.iter().enumerate() {
                                    let pre = if *numerotee { format!("{}. ", k + 1) } else { "• ".into() };
                                    riche(&mut ops, &mut y, text, &pre, e, PAD + 6.0, inner - 6.0, 14.0);
                                }
                            }
                            Bloc::Tableau(rows) => {
                                y += 6.0;
                                let focus = if premier_tableau {
                                    self.focus.filter(|f| f.0 == si).map(|f| f.1)
                                } else {
                                    None
                                };
                                table(&mut ops, &mut zones, &mut y, text, inner, si, rows, focus);
                                premier_tableau = false;
                            }
                            Bloc::Details { resume, blocs } => {
                                let ouvert = self.ouverts.contains(&(si, bi));
                                let r = Rect::new(PAD, y + 4.0, inner, 34.0);
                                let s = format!("{} {resume}", if ouvert { "▾" } else { "▸" });
                                ops.push(Op::Fill(r, pal::field(), 1.0));
                                ops.push(Op::Text(text.fit(&s, 13.0, r.w - 24.0), r.x + 12.0, r.y + 22.0, 13.0, WHITE, Align::Left));
                                zones.push((r, Hit::Details(si, bi)));
                                y += 42.0;
                                if ouvert {
                                    for b in blocs {
                                        if let Bloc::Liste { elements, .. } = b {
                                            for e in elements {
                                                riche(&mut ops, &mut y, text, "• ", e, PAD + 12.0, inner - 12.0, 13.5);
                                            }
                                        } else if let Bloc::Para(m) = b {
                                            riche(&mut ops, &mut y, text, "", m, PAD + 12.0, inner - 12.0, 13.5);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                if !l.checklist.is_empty() {
                    y += 16.0;
                    para(&mut ops, &mut y, "✅ Check-list", 17.0, pal::accent());
                    for c in &l.checklist {
                        para(&mut ops, &mut y, &format!("☐ {c}"), 14.0, WHITE);
                    }
                }
                if !l.quiz.is_empty() {
                    y += 20.0;
                    let got = argos_academy::score(l, &self.answers);
                    let faites = self.answers.iter().filter(|a| a.is_some()).count();
                    para(
                        &mut ops,
                        &mut y,
                        &format!("❓ Quiz · {got} / {} bonne(s) réponse(s)", l.quiz.len()),
                        17.0,
                        pal::accent(),
                    );
                    for (qi, q) in l.quiz.iter().enumerate() {
                        y += 8.0;
                        para(&mut ops, &mut y, &format!("{}. {}", qi + 1, q.q), 14.5, WHITE);
                        y += 6.0;
                        let ans = self.answers.get(qi).copied().flatten();
                        for (ci, ch) in q.choix.iter().enumerate() {
                            let lines = wrap(ch, 13.5);
                            let r = Rect::new(PAD, y, inner, lines.len() as f64 * 19.0 + 14.0);
                            let (bg, a) = match ans {
                                Some(_) if ci == q.bonne => (pal::up(), 0.35),
                                Some(a) if a == ci => (pal::down(), 0.35),
                                _ => (pal::field(), 1.0),
                            };
                            ops.push(Op::Fill(r, bg, a));
                            ops.push(Op::Stroke(r, pal::edge_soft()));
                            for (k, t) in lines.iter().enumerate() {
                                ops.push(Op::Text(t.clone(), r.x + 12.0, r.y + 21.0 + k as f64 * 19.0, 13.5, WHITE, Align::Left));
                            }
                            zones.push((r, Hit::Choice(qi, ci)));
                            y += r.h + 6.0;
                        }
                        if let Some(a) = ans {
                            let verdict = if a == q.bonne {
                                "✅ Bonne réponse.".to_string()
                            } else {
                                format!("❌ La bonne réponse : {}.", q.choix[q.bonne])
                            };
                            para(&mut ops, &mut y, &verdict, 13.0, GREY);
                        }
                    }
                    if faites == l.quiz.len() {
                        y += 12.0;
                        let fin = if got >= argos_academy::REUSSITE {
                            format!("🏆 Score : {got} / {} · leçon terminée", l.quiz.len())
                        } else {
                            format!(
                                "Score : {got} / {} · il faut {} bonnes réponses pour terminer la leçon",
                                l.quiz.len(),
                                argos_academy::REUSSITE
                            )
                        };
                        para(&mut ops, &mut y, &fin, 15.0, WHITE);
                        y += 10.0;
                        bouton(
                            &mut ops,
                            &mut zones,
                            Rect::new(PAD, y, 220.0_f64.min(inner), 36.0),
                            "🔁 Recommencer le quiz",
                            Hit::Recommencer,
                        );
                        y += 36.0;
                    }
                }
                // leçon précédente / suivante (ordre des parcours)
                let c = argos_academy::chemin();
                if let Some(k) = c.iter().position(|&i| i == li) {
                    y += 20.0;
                    let half = (inner - 10.0) / 2.0;
                    if k > 0 {
                        let r = Rect::new(PAD, y, half, 40.0);
                        ops.push(Op::Fill(r, pal::field(), 1.0));
                        let s = text.fit(&format!("◀ {}", titre(c[k - 1])), 13.0, half - 20.0);
                        ops.push(Op::Text(s, r.x + 12.0, r.y + 25.0, 13.0, WHITE, Align::Left));
                        zones.push((r, Hit::Lecon(c[k - 1])));
                    }
                    if k + 1 < c.len() {
                        let r = Rect::new(PAD + half + 10.0, y, half, 40.0);
                        ops.push(Op::Fill(r, pal::field(), 1.0));
                        let s = text.fit(&format!("{} ▶", titre(c[k + 1])), 13.0, half - 20.0);
                        ops.push(Op::Text(s, r.x + r.w - 12.0, r.y + 25.0, 13.0, WHITE, Align::Right));
                        zones.push((r, Hit::Lecon(c[k + 1])));
                    }
                    y += 40.0;
                }
                y += 10.0;
                para(&mut ops, &mut y, "Formation seulement ; ce n'est pas un conseil financier.", 12.0, GREY);
            }
        }
        // 📱 au doigt : de la place en bas pour que le rond Argos AI ne cache pas la dernière ligne
        (ops, zones, y + if crate::actions::tactile() { 90.0 } else { 30.0 })
    }

    /// Dessine la page (`w` × `h`).
    pub fn draw(&self, text: &Text, w: u32, h: u32, mouse: (f64, f64)) -> Option<Pixmap> {
        let (wf, hf) = (w as f64, h as f64);
        let mut pm = crate::px::new(w.max(1), h.max(1))?;
        crate::fill(&mut pm, 0.0, 0.0, wf, hf, pal::panel(), 1.0);
        let (ops, zones, _) = self.build(text, wf);
        let mut body = crate::px::new(w.max(1), (hf - HEAD).max(1.0) as u32)?;
        let dy = -self.scroll;
        let hover = |r: &Rect| r.contains(mouse.0, mouse.1 - HEAD + self.scroll);
        for op in &ops {
            match op {
                Op::Fill(r, c, a) => round_fill(&mut body, Rect::new(r.x, r.y + dy, r.w, r.h), 8.0, *c, *a),
                Op::Stroke(r, c) => round_stroke(&mut body, Rect::new(r.x, r.y + dy, r.w, r.h), 8.0, *c, 1.0),
                Op::Text(s, x, y, size, c, al) => {
                    if *y + dy > -30.0 && *y + dy < hf {
                        text.draw(&mut body, s, (*x as f32, (*y + dy) as f32), *size, *c, *al);
                    }
                }
            }
        }
        for (r, _) in zones.iter().filter(|(r, z)| hover(r) && !matches!(z, Hit::Row(..))) {
            round_stroke(&mut body, Rect::new(r.x, r.y + dy, r.w, r.h), 8.0, pal::accent(), 1.5);
        }
        crate::px::put(&mut pm, 0.0, HEAD, &body);
        // en-tête : titre, fuseau, progression, ✕
        crate::fill(&mut pm, 0.0, HEAD - 1.0, wf, 1.0, pal::edge(), 1.0);
        crate::phosphor::draw(&mut pm, "graduation-cap", PAD + 12.0, 30.0, 24, pal::accent());
        text.draw(&mut pm, "Académie Argos", ((PAD + 32.0) as f32, 36.0), 19.0, WHITE, Align::Left);
        let f = bouton_fuseau(wf);
        round_fill(&mut pm, f, 8.0, if f.contains(mouse.0, mouse.1) { pal::hover() } else { pal::field() }, 1.0);
        let fs = if wf < 520.0 {
            "🕘".to_string()
        } else {
            format!("🕘 {}", self.progress.fuseau.nom())
        };
        text.draw(&mut pm, &fs, (f.cx() as f32, (f.y + 21.0) as f32), 13.0, WHITE, Align::Center);
        let (d, n) = self.done_count();
        let compte = if wf < 520.0 {
            format!("{d}/{n}")
        } else {
            format!("{d} / {n} leçons terminées")
        };
        text.draw(&mut pm, &compte, ((wf - 64.0) as f32, 35.0), 12.5, GREY, Align::Right);
        let x = Rect::new(wf - 52.0, 12.0, 36.0, 36.0);
        if x.contains(mouse.0, mouse.1) {
            round_fill(&mut pm, x, 8.0, pal::hover(), 1.0);
        }
        crate::shapes::lines(&mut pm, x.cx(), x.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.6);
        Some(pm)
    }
}

/// L'emoji d'un parcours suivi d'une espace, seulement s'il peut s'afficher (sinon rien plutôt qu'un carré vide).
fn icon(e: &str) -> String {
    if crate::emoji::image(e, 16).is_some() {
        format!("{e} ")
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests;
