//! 🅘 Page « Indicateurs » du cell et de la tablette (ƒx), d'après la maquette (croquis/maquette-verre, vue Cell,
//! `.fen` plein écran mesurée le 9 nov. 2026) : 🔎 Chercher ; PERSONNEL (Favoris, Mes indicateurs (Argos Script),
//! Mes modèles) ; INTÉGRÉS (Classiques, Tendance, Oscillateurs, Volume, Volatilité, SMC / Price action, Order flow /
//! carnet, Multi-UT, Macro / CMC) en accordéon : toucher une catégorie déplie ses indicateurs (☆ favori, nom, ＋ sur la
//! chart). Les listes sont celles de la maquette, qui reprend les vrais indicateurs de l'app.

use crate::indicators::Ind;
use crate::panels::ScriptOpen;
use crate::shapes::{round_fill, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

const TEXTE: Rgb = crate::jetons::TEXTE;
const DIM: Rgb = crate::jetons::DIM;
const OR: Rgb = crate::jetons::TIER2;
const X: f64 = 17.0;
const LIGNE: f64 = 55.0;
const SOUS_PC: f64 = 39.5;

/// Height of an indicator's row (📱 phone view: 46, so ☆ and + are 44 for the finger; Jee, 21 Nov, list 2).
fn sous() -> f64 {
    if crate::actions::tactile() {
        46.0
    } else {
        SOUS_PC
    }
}

/// 📱 Width of the ☆ zone at the start of a row (44 for the finger).
fn etoile_w() -> f64 {
    if crate::actions::tactile() {
        44.0
    } else {
        28.0
    }
}

/// 📱 A category kept: in phone view not the ones that are not ready (« Mes modèles », « Macro / CMC »: « bientôt »).
fn montree(c: Cat) -> bool {
    !(crate::actions::tactile() && matches!(c, Cat::Modeles | Cat::Macro))
}
/// Haut du contenu qui défile (sous la recherche : la section PERSONNEL).
const HAUT_LISTE: f64 = 121.7;

/// Une catégorie de la page.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Cat {
    Favoris,
    Scripts,
    Modeles,
    /// Une catégorie du catalogue unique (`argos_script::catalogue::CATEGORIES_FX`, son rang).
    Liste(usize),
    Macro,
}

const CATS: [Cat; 12] = [
    Cat::Favoris,
    Cat::Scripts,
    Cat::Modeles,
    Cat::Liste(0),
    Cat::Liste(1),
    Cat::Liste(2),
    Cat::Liste(3),
    Cat::Liste(4),
    Cat::Liste(5),
    Cat::Liste(6),
    Cat::Liste(7),
    Cat::Macro,
];

/// Une catégorie du catalogue : (icône, nom, indicateurs dans l'ordre affiché).
fn categorie(k: usize) -> (&'static str, &'static str, Vec<Ind>) {
    let (ico, nom, ids) = argos_script::catalogue::CATEGORIES_FX[k];
    (ico, nom, ids.iter().filter_map(|id| crate::catalogue::ind(id)).collect())
}

/// Les lignes « Mes indicateurs (Argos Script) ».
const SCRIPTS: [(&str, ScriptOpen); 3] = [
    ("Créer un indicateur (éditeur </>)", ScriptOpen::New),
    ("Modifier mes indicateurs", ScriptOpen::Mine),
    ("Importer du Pine Script", ScriptOpen::Import),
];

/// Ce que fait un toucher.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FxClick {
    None,
    Fermer,
    Changed,
    /// ＋ : mettre (ou retirer) l'indicateur sur la chart.
    Ajouter(Ind),
    /// ☆ : favori.
    Favori(Ind),
    Script(ScriptOpen),
    Bientot(&'static str),
}

/// Ce que la page montre (vient du moteur).
#[derive(Debug, Clone, Default)]
pub struct Donnees {
    pub actifs: Vec<Ind>,
    pub favoris: Vec<Ind>,
}

/// Une ligne posée à l'écran.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ligne {
    Section(&'static str),
    Cat(usize),
    Ind(Ind),
    Script(usize),
    Texte(&'static str),
}

/// État de la page.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FxPage {
    /// La catégorie dépliée.
    pub ouverte: Option<usize>,
    /// `Some` : la recherche reçoit le clavier.
    pub recherche: Option<String>,
    pub defile: f64,
    /// 🖥 Dans le volet flottant (PC) : carte arrondie en verre, sans la poignée du cell.
    pub volet: bool,
}

impl FxPage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Le champ de recherche (coordonnées de la page de largeur `w`).
    pub fn champ(w: f64) -> Rect {
        Rect::new(X, 77.7, w - 2.0 * X, 36.0)
    }

    fn fermer(w: f64) -> Rect {
        Rect::new(w - 47.0, 37.7, 30.0, 30.0)
    }

    fn q(&self) -> String {
        self.recherche.as_deref().map(|q| crate::i18n::plain(q.trim())).unwrap_or_default()
    }

    /// Les lignes dans l'ordre, avec leur hauteur (avant défilement, depuis `HAUT_LISTE`).
    fn lignes(&self, d: &Donnees) -> Vec<(Ligne, f64)> {
        let q = self.q();
        if !q.is_empty() {
            return Ind::ALL
                .iter()
                .filter(|i| crate::i18n::plain(&crate::catalogue::texte_recherche(**i)).contains(&q))
                .map(|i| (Ligne::Ind(*i), sous()))
                .collect();
        }
        let mut out = vec![(Ligne::Section("PERSONNEL"), 25.0)];
        for (k, c) in CATS.iter().enumerate().filter(|(_, c)| montree(**c)) {
            if k == 3 {
                // (maquette : la ligne Classiques 32,9 points sous Mes modèles, le titre 12 points sous elle)
                out.push((Ligne::Section("INTÉGRÉS"), 7.9 + 25.0));
            }
            out.push((Ligne::Cat(k), LIGNE));
            if self.ouverte != Some(k) {
                continue;
            }
            let avant = out.len();
            match c {
                Cat::Favoris => out.extend(d.favoris.iter().map(|i| (Ligne::Ind(*i), sous()))),
                Cat::Scripts => out.extend((0..SCRIPTS.len()).map(|s| (Ligne::Script(s), sous()))),
                Cat::Modeles => out.push((Ligne::Texte("Modèles d'indicateurs · bientôt"), sous())),
                Cat::Liste(c) => out.extend(categorie(*c).2.into_iter().map(|i| (Ligne::Ind(i), sous()))),
                Cat::Macro => out.push((Ligne::Texte("Macro / CMC · bientôt"), sous())),
            }
            if out.len() == avant {
                out.push((Ligne::Texte("Aucun favori : touche ☆ devant un indicateur"), sous()));
            }
            // (marge sous la liste dépliée, comme la maquette)
            if let Some(l) = out.last_mut() {
                l.1 += 12.4;
            }
        }
        out
    }

    /// Lignes posées : (ligne, rectangle à l'écran).
    fn places(&self, w: f64, d: &Donnees) -> Vec<(Ligne, Rect)> {
        let mut y = HAUT_LISTE - self.defile;
        let mut out = vec![];
        for (l, h) in self.lignes(d) {
            let r = match l {
                Ligne::Section(_) => Rect::new(X + 2.0, y + h - 25.0 + 4.0, w - 2.0 * X, 14.8),
                Ligne::Cat(_) => Rect::new(X, y, w - 2.0 * X, LIGNE),
                // (sous-lignes : décalées comme dans la maquette, après l'icône de la catégorie)
                _ => Rect::new(
                    59.0,
                    y + if matches!(l, Ligne::Ind(_)) && !self.q().is_empty() { 0.0 } else { 4.0 },
                    w - 59.0 - X,
                    sous(),
                ),
            };
            out.push((l, r));
            y += h;
        }
        out
    }

    fn haut_contenu(&self, d: &Donnees) -> f64 {
        // (📱 in phone view, room under the last row for the round Argos AI button, which sits over the + column)
        self.lignes(d).iter().map(|(_, h)| h).sum::<f64>() + if crate::actions::tactile() { 140.0 } else { 90.0 }
    }

    /// Molette / doigt (`dy` > 0 : vers le bas de la liste) dans une page de hauteur `h`.
    pub fn wheel(&mut self, dy: f64, h: f64, d: &Donnees) {
        let max = (self.haut_contenu(d) - (h - HAUT_LISTE)).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// Clavier de la recherche : un caractère, ou `None` = effacer.
    pub fn key(&mut self, c: Option<char>) {
        if let Some(q) = self.recherche.as_mut() {
            match c {
                Some(c) if !c.is_control() => q.push(c),
                Some(_) => {}
                None => _ = q.pop(),
            }
            self.defile = 0.0;
        }
    }

    /// Toucher (coordonnées de la page `w` × `h`).
    pub fn click(&mut self, x: f64, y: f64, w: f64, d: &Donnees) -> FxClick {
        if Self::fermer(w).contains(x, y) {
            return FxClick::Fermer;
        }
        if Self::champ(w).contains(x, y) {
            self.recherche.get_or_insert_with(String::new);
            return FxClick::Changed;
        }
        if y < HAUT_LISTE {
            return FxClick::None;
        }
        for (l, r) in self.places(w, d) {
            if !r.contains(x, y) {
                continue;
            }
            return match l {
                Ligne::Cat(k) => {
                    if matches!(CATS[k], Cat::Macro) {
                        return FxClick::Bientot("Macro / CMC : bientôt");
                    }
                    self.ouverte = if self.ouverte == Some(k) { None } else { Some(k) };
                    FxClick::Changed
                }
                Ligne::Ind(i) => {
                    if x < r.x + etoile_w() {
                        FxClick::Favori(i)
                    } else {
                        FxClick::Ajouter(i)
                    }
                }
                Ligne::Script(s) => FxClick::Script(SCRIPTS[s].1),
                Ligne::Texte(t) if t.starts_with("Modèles") => FxClick::Bientot("Modèles d'indicateurs : bientôt"),
                _ => FxClick::None,
            };
        }
        FxClick::None
    }

    /// Dessin de toute la page (`w` × `h`), fond de verre ; `mouse` en coordonnées de la page.
    pub fn draw(&self, text: &Text, w: f64, h: f64, d: &Donnees, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let mut pm = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        // fond : la couleur du verre posée sur le fond sombre (opaque : la chart ne se voit pas nette dessous)
        round_fill(&mut pm, Rect::new(0.0, 0.0, w, h), 0.0, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        let gras = Text::graisse(Graisse::Bold);
        let moyen = Text::graisse(Graisse::Medium);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let blanc = |a: f32| (crate::jetons::BLANC, a);
        // la liste qui défile d'abord (coupée sous l'en-tête et la recherche)
        let mut liste = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        for (l, r) in self.places(w, d) {
            if r.y + r.h < HAUT_LISTE - 4.0 || r.y > h {
                continue;
            }
            match l {
                Ligne::Section(s) => text.draw(&mut liste, s, (r.x as f32, (r.y + 11.5) as f32), 11.0, DIM, Align::Left),
                Ligne::Cat(k) => {
                    let (ico, nom, n) = match CATS[k] {
                        Cat::Favoris => ("☆", "Favoris", Some(d.favoris.len())),
                        // (📱 phone view: no keyboard icon)
                        Cat::Scripts => (
                            if crate::actions::tactile() { "</>" } else { "⌨" },
                            "Mes indicateurs (Argos Script)",
                            Some(SCRIPTS.len()),
                        ),
                        Cat::Modeles => ("▦", "Mes modèles", None),
                        Cat::Liste(c) => {
                            let (i, n, l) = categorie(c);
                            (i, n, Some(l.len()))
                        }
                        Cat::Macro => ("◍", "Macro / CMC", None),
                    };
                    if survol(&r) {
                        round_fill(&mut liste, r, 12.0, blanc(0.04).0, 0.04);
                    }
                    let t = Rect::new(r.x + 6.0, r.y + 12.0, 30.0, 30.0);
                    round_fill(&mut liste, t, 9.0, blanc(0.07).0, 0.07);
                    text.draw(&mut liste, ico, (t.cx() as f32, (t.cy() + 4.6) as f32), 13.0, TEXTE, Align::Center);
                    moyen.draw(&mut liste, nom, ((r.x + 48.0) as f32, (r.y + 32.0) as f32), 13.0, TEXTE, Align::Left);
                    let ouvert = self.ouverte == Some(k);
                    let droite = r.x + r.w - 9.0;
                    if matches!(CATS[k], Cat::Macro | Cat::Modeles) {
                        let lx = r.x + 48.0 + moyen.width(nom, 13.0) as f64 + 6.0;
                        let b = Rect::new(lx, r.y + 21.6, 40.5, 12.0);
                        round_fill(&mut liste, b, 6.0, Rgb(255, 209, 102), 1.0);
                        moyen.draw(&mut liste, "bientôt", (b.cx() as f32, (b.y + 9.0) as f32), 9.0, Rgb(17, 17, 17), Align::Center);
                    } else if let Some(n) = n {
                        let nx = if ouvert { droite - 26.0 } else { droite - 16.0 };
                        text.draw(&mut liste, &n.to_string(), (nx as f32, (r.y + 32.0) as f32), 10.8, DIM, Align::Right);
                    }
                    text.draw(
                        &mut liste,
                        if ouvert { "⌄" } else { "›" },
                        (droite as f32, (r.y + 33.0) as f32),
                        13.0,
                        DIM,
                        Align::Right,
                    );
                    crate::fill(&mut liste, r.x, r.y + r.h - 1.0, r.w, 1.0, crate::jetons::sur_fond(blanc(0.08)), 1.0);
                }
                Ligne::Ind(i) => {
                    if survol(&r) {
                        round_fill(&mut liste, r, 9.0, blanc(0.05).0, 0.05);
                    }
                    let fav = d.favoris.contains(&i);
                    // 📱 phone view: ☆ and + of 44 for the finger
                    let doigt = crate::actions::tactile();
                    let (et, base) = (etoile_w(), r.y + r.h / 2.0 + 5.5);
                    let st = if doigt { 18.0 } else { 13.0 };
                    let sx = if doigt { r.x + et / 2.0 } else { r.x + 8.0 };
                    let sa = if doigt { Align::Center } else { Align::Left };
                    text.draw(&mut liste, if fav { "★" } else { "☆" }, (sx as f32, (base + 0.5) as f32), st, OR, sa);
                    let pw = if doigt { 44.0 } else { 33.0 };
                    let nom = text.fit(i.name(), 13.0, r.w - et - pw - 17.0);
                    text.draw(&mut liste, &nom, ((r.x + et - 0.3) as f32, base as f32), 13.0, TEXTE, Align::Left);
                    let p = if doigt {
                        Rect::new(r.x + r.w - 46.0, r.y + 4.0, 44.0, 38.0)
                    } else {
                        Rect::new(r.x + r.w - 41.0, r.y + 6.0, 33.0, 27.5)
                    };
                    let on = d.actifs.contains(&i);
                    let (pc, pa) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
                    round_fill(&mut liste, p, 12.0, pc, pa);
                    // (« ＋ » pleine largeur n'est pas dans la police : le « + » simple, un peu plus grand)
                    let (t, taille) = if on { ("✓", 13.0) } else { ("+", 17.0) };
                    text.draw(
                        &mut liste,
                        t,
                        (p.cx() as f32, (p.cy() + taille as f64 * 0.36) as f32),
                        taille,
                        TEXTE,
                        Align::Center,
                    );
                }
                Ligne::Script(s) => {
                    if survol(&r) {
                        round_fill(&mut liste, r, 9.0, blanc(0.05).0, 0.05);
                    }
                    text.draw(&mut liste, SCRIPTS[s].0, ((r.x + 8.0) as f32, (r.y + 25.0) as f32), 13.0, TEXTE, Align::Left);
                    text.draw(&mut liste, "›", ((r.x + r.w - 9.0) as f32, (r.y + 25.0) as f32), 13.0, DIM, Align::Right);
                }
                Ligne::Texte(t) => text.draw(&mut liste, t, ((r.x + 8.0) as f32, (r.y + 25.0) as f32), 13.0, DIM, Align::Left),
            }
        }
        if self.ouverte.is_none() && !self.q().is_empty() && self.lignes(d).is_empty() {
            text.draw(
                &mut liste,
                "Aucun indicateur trouvé",
                ((w / 2.0) as f32, (HAUT_LISTE + 30.0) as f32),
                13.0,
                DIM,
                Align::Center,
            );
        }
        // (seulement sous la recherche)
        let px = crate::px::dev(1.0);
        let coupe = (HAUT_LISTE * px).round() as usize;
        let (lw, data) = (liste.width() as usize, liste.data_mut());
        let n = (coupe * lw * 4).min(data.len());
        data[..n].fill(0);
        crate::px::put(&mut pm, 0.0, 0.0, &liste);
        // barre de défilement (pilule) quand la liste dépasse
        let vue = h - HAUT_LISTE;
        let total = self.haut_contenu(d);
        if total > vue {
            let th = (vue * vue / total).max(30.0);
            let max = (total - vue).max(1.0);
            let ty = HAUT_LISTE + self.defile.min(max) / max * (vue - th);
            crate::jetons::barre_defilement(&mut pm, w - 3.0, ty, th, false);
        }
        // poignée, en-tête, ✕, recherche
        if !self.volet {
            round_fill(&mut pm, Rect::new(w / 2.0 - 20.0, 9.0, 40.0, 5.0), 2.5, crate::jetons::BLANC, 0.16);
        }
        let ic = Rect::new(X, 42.5, 20.0, 20.0);
        round_fill(&mut pm, ic, 10.0, TEXTE, 1.0);
        gras.draw(&mut pm, "I", (ic.cx() as f32, (ic.cy() + 4.5) as f32), 12.0, Rgb(26, 30, 42), Align::Center);
        gras.draw(&mut pm, "Indicateurs", ((X + 26.0) as f32, 59.0), 17.0, TEXTE, Align::Left);
        let f = Self::fermer(w);
        round_fill(&mut pm, f, 15.0, crate::jetons::BLANC, if survol(&f) { 0.12 } else { 0.07 });
        text.draw(&mut pm, "✕", (f.cx() as f32, (f.cy() + 4.7) as f32), 13.0, TEXTE, Align::Center);
        let c = Self::champ(w);
        round_fill(&mut pm, c, 14.0, crate::jetons::BLANC, 0.07);
        if self.recherche.is_some() {
            crate::shapes::round_stroke(&mut pm, c, 14.0, crate::jetons::ACCENT, 1.0);
        }
        text.draw(&mut pm, "🔎", ((c.x + 12.0) as f32, (c.cy() + 4.7) as f32), 13.0, TEXTE, Align::Left);
        let (t, col) = match self.recherche.as_deref() {
            Some(q) => (format!("{q}▏"), TEXTE),
            None => ("Chercher".to_string(), DIM),
        };
        text.draw(&mut pm, &t, ((c.x + 36.2) as f32, (c.cy() + 5.0) as f32), 14.0, col, Align::Left);
        // 🖥 volet flottant : la carte arrondie des volets (rayon 20, liseré du verre), comme la maquette
        if self.volet {
            let (tout, carte) = (Rect::new(0.0, 0.0, w, h), Rect::new(4.0, 4.0, w - 6.0, (h - 8.0).max(1.0)));
            crate::shapes::autour(&mut pm, tout, carte, crate::jetons::R_VOLET, crate::jetons::FOND);
            crate::shapes::round_stroke(&mut pm, carte, crate::jetons::R_VOLET, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🅘 Chaque indicateur de l'app est dans une catégorie de la maquette (aucun d'oublié, aucun en double), chaque
    /// catégorie se déplie au toucher, ＋ et ☆ répondent, la recherche trouve, ✕ ferme.
    #[test]
    fn page_indicateurs() {
        let mut tous: Vec<Ind> = CATS
            .iter()
            .flat_map(|c| match c {
                Cat::Liste(c) => categorie(*c).2,
                _ => vec![],
            })
            .collect();
        let n = tous.len();
        tous.sort_by_key(|i| *i as usize);
        tous.dedup();
        assert_eq!(n, tous.len(), "un indicateur en double");
        assert_eq!(tous.len(), Ind::ALL.len(), "chaque indicateur a sa catégorie");
        let d = Donnees {
            actifs: vec![Ind::Rsi],
            favoris: vec![Ind::Macd],
        };
        let mut p = FxPage::new();
        let w = 390.0;
        assert_eq!(p.click(358.0, 52.0, w, &d), FxClick::Fermer);
        let classiques = p.places(w, &d).into_iter().find(|(l, _)| *l == Ligne::Cat(3)).unwrap().1;
        assert_eq!(p.click(classiques.cx(), classiques.cy(), w, &d), FxClick::Changed);
        assert_eq!(p.ouverte, Some(3));
        let rsi = p.places(w, &d).into_iter().find(|(l, _)| *l == Ligne::Ind(Ind::Rsi)).unwrap().1;
        assert_eq!(p.click(rsi.x + rsi.w - 20.0, rsi.cy(), w, &d), FxClick::Ajouter(Ind::Rsi));
        assert_eq!(p.click(rsi.x + 10.0, rsi.cy(), w, &d), FxClick::Favori(Ind::Rsi));
        let c = FxPage::champ(w);
        assert_eq!(p.click(c.cx(), c.cy(), w, &d), FxClick::Changed);
        for ch in "ichi".chars() {
            p.key(Some(ch));
        }
        assert_eq!(p.lignes(&d), vec![(Ligne::Ind(Ind::Ichimoku), sous())]);
        p.recherche = None;
        p.ouverte = Some(1);
        let s = p.places(w, &d).into_iter().find(|(l, _)| *l == Ligne::Script(0)).unwrap().1;
        assert_eq!(p.click(s.cx(), s.cy(), w, &d), FxClick::Script(ScriptOpen::New));
        // places mesurées dans la maquette (390 × 844) : PERSONNEL 125,7 ; Favoris 146,5 ; INTÉGRÉS 323,5 ;
        // Classiques 344,4 ; RSI 14 403,4 (au demi-point près)
        let mut m = FxPage::new();
        m.ouverte = Some(3);
        let pl = m.places(w, &d);
        let y = |l: Ligne| pl.iter().find(|(x, _)| *x == l).unwrap().1.y;
        for (l, attendu) in [
            (Ligne::Section("PERSONNEL"), 125.7),
            (Ligne::Cat(0), 146.5),
            (Ligne::Section("INTÉGRÉS"), 323.5),
            (Ligne::Cat(3), 344.4),
            (Ligne::Ind(Ind::Rsi), 403.4),
        ] {
            assert!((y(l) - attendu).abs() <= 0.5, "{l:?} : {} au lieu de {attendu}", y(l));
        }
        p.wheel(10_000.0, 844.0, &d);
        assert!(p.defile >= 0.0);
    }

    /// 📱 Jee (21 Nov, list 2): by the finger, ☆ and + are 44 wide and tall, « Mes modèles » and « Macro / CMC »
    /// (« bientôt ») are gone, and the end of the list goes above the round Argos AI button; the PC keeps everything.
    #[test]
    fn indicateurs_au_doigt() {
        let d = Donnees {
            actifs: vec![],
            favoris: vec![],
        };
        let w = 402.0;
        let cats = |p: &FxPage| p.lignes(&d).iter().filter(|(l, _)| matches!(l, Ligne::Cat(_))).count();
        let p = FxPage::new();
        crate::actions::set_tactile(false);
        assert_eq!(cats(&p), CATS.len(), "PC: every category");
        crate::actions::set_tactile(true);
        assert_eq!(cats(&p), CATS.len() - 2, "phone view: without the two « bientôt »");
        assert!(p
            .lignes(&d)
            .iter()
            .all(|(l, _)| !matches!(l, Ligne::Cat(k) if matches!(CATS[*k], Cat::Modeles | Cat::Macro))));
        let mut p = FxPage::new();
        p.ouverte = Some(3);
        let (_, r) = p.places(w, &d).into_iter().find(|(l, _)| matches!(l, Ligne::Ind(_))).unwrap();
        let Ligne::Ind(i) = p.places(w, &d).into_iter().find(|(l, _)| matches!(l, Ligne::Ind(_))).unwrap().0 else {
            unreachable!()
        };
        assert!(r.h >= 44.0, "{r:?}");
        for x in [r.x + 1.0, r.x + 43.0] {
            assert_eq!(p.click(x, r.cy(), w, &d), FxClick::Favori(i), "☆ 44 wide");
        }
        for x in [r.x + r.w - 44.0, r.x + r.w - 1.0] {
            assert_eq!(p.click(x, r.cy(), w, &d), FxClick::Ajouter(i), "+ 44 wide");
        }
        assert!(
            p.haut_contenu(&d) - p.lignes(&d).iter().map(|(_, h)| h).sum::<f64>() >= 130.0,
            "room for Argos AI"
        );
        assert!(p.draw(&Text::default(), w, 874.0, &d, None).is_some());
        crate::actions::set_tactile(false);
    }
}
