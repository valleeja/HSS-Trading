//! ⚙ Fenêtre « Réglages » du PC, d'après la maquette (croquis/maquette-verre, `#reg`) : en-tête « ⚙ Réglages » et ✕,
//! puis une ligne par catégorie (icône, nom, résumé à droite, ›), et « ⋯ Toutes les options » (les Panneaux d'avant,
//! avec toutes leurs tuiles). Chaque ligne mène à une fonction qui existe déjà (`sheet::Act`) ; ce qui n'existe pas
//! encore dit « bientôt ». Verre arrondi, survol en pilule, sans séparateurs.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::sheet::Act;
use crate::text::{Align, Graisse, Text};
use tiny_skia::Pixmap;

/// 🖥 Largeur (maquette : volet `#reg`, 400).
pub const LARGE: f64 = 400.0;
const X: f64 = 12.0;
const LIGNE_PC: f64 = 50.0;

/// Pitch of a row: 50 at the PC; by the finger, a touch target (pitch − 4) of the finger's minimum (Jee, 21 Nov).
fn ligne_pas() -> f64 {
    if crate::actions::tactile() {
        crate::formfactor::TOUCH_MIN_PT + 4.0
    } else {
        LIGNE_PC
    }
}
const HAUT_LISTE: f64 = 60.0;

/// Où mène une ligne.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Cible {
    Act(Act),
    /// ☰ Appli (le menu de l'appli : mises à jour, installation, affichage).
    Appli,
    /// 🫥 Estompage des barres (oui / non).
    Estomper,
    /// ⌂ 📱 Au doigt : une seule ligne « Accueil », avec ses deux choix (au lancement oui / non, revoir l'accueil).
    Accueil,
    Bientot(&'static str),
}

/// Les catégories, dans l'ordre de la maquette : (icône Phosphor, nom, résumé, cible).
pub const LIGNES: [(&str, &str, &str, Cible); 18] = [
    ("chart-line", "Chart", "bougies, grille, échelle", Cible::Act(Act::ChartSettings)),
    ("palette", "Thème", "couleurs de l'app", Cible::Act(Act::Theme)),
    ("clock", "Unité de temps", "1m, 5m, 1h, 4h, 1D…", Cible::Act(Act::Timeframe)),
    (
        "globe",
        "Sessions et fuseau",
        "heures et sessions",
        Cible::Bientot("Sessions et fuseau : bientôt"),
    ),
    ("key", "Exchanges et clés", "Hyperliquid, OANDA, Binance…", Cible::Act(Act::Exchanges)),
    (
        "squares-four",
        "Panneaux",
        "Trading, gardien, compte, indicateurs…",
        Cible::Act(Act::OpenPanels),
    ),
    ("download-simple", "Importer / exporter", "données d'Argos v1", Cible::Act(Act::ImportV1)),
    ("user-circle", "Compte", "compte Argos, clé, empreinte", Cible::Act(Act::Account)),
    ("sliders-horizontal", "Paramètres", "réglages avancés", Cible::Act(Act::Params)),
    ("layout", "Appli", "mises à jour, installation, affichage", Cible::Appli),
    (
        "arrows-clockwise",
        "Synchro par QR",
        "PC ↔ téléphone, en local",
        Cible::Bientot("Synchro par QR : bientôt"),
    ),
    ("lightning", "Raccourcis", "Ctrl + / · par catégories", Cible::Act(Act::Raccourcis)),
    (
        "eye-slash",
        "Estompage des barres",
        "barres estompées quand tu travailles sur la chart",
        Cible::Estomper,
    ),
    ("bell", "Son des alertes", "son et volume", Cible::Act(Act::SonAlertes)),
    ("compass", "Accueil", "s'ouvre au lancement (oui / non)", Cible::Act(Act::AccueilLancement)),
    (
        "arrow-counter-clockwise",
        "Revoir l'accueil",
        "les 3 écrans du premier lancement",
        Cible::Act(Act::RevoirAccueil),
    ),
    (
        "alarm",
        "À venir",
        "revue de semaine, sauvegarde, accessibilité",
        Cible::Bientot("À venir : bientôt"),
    ),
    ("sparkle", "Argos AI", "assistant local", Cible::Bientot("Réglages d'Argos AI : bientôt")),
];

/// ⌂ 📱 The single « Accueil » row of the finger (Jee, 21 Nov, list 2 point 15).
static ACCUEIL_DOIGT: (&str, &str, &str, Cible) = ("compass", "Accueil", "au lancement, revoir l'accueil", Cible::Accueil);

/// The rows shown (📱 phone view: without « Raccourcis », the keyboard shortcuts' list: there is no keyboard; by the
/// finger: one « Accueil » row holding its two choices).
pub fn lignes() -> Vec<&'static (&'static str, &'static str, &'static str, Cible)> {
    let doigt = crate::actions::tactile();
    LIGNES
        .iter()
        .filter(|l| crate::actions::montrer_raccourcis() || l.3 != Cible::Act(crate::sheet::Act::Raccourcis))
        .filter(|l| !doigt || l.3 != Cible::Act(Act::RevoirAccueil))
        .map(|l| {
            if doigt && l.3 == Cible::Act(Act::AccueilLancement) {
                &ACCUEIL_DOIGT
            } else {
                l
            }
        })
        .collect()
}

/// Ce que fait un clic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegClick {
    None,
    Fermer,
    /// 🖥 ⤢ : volet large ⇄ normal (en volet seulement).
    Agrandir,
    Ligne(Cible),
    /// ⋯ Toutes les options : les Panneaux d'avant.
    Options,
}

/// État de la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub struct Reglages {
    /// Place maximale (fenêtre) : (x, y, largeur, hauteur maximale).
    pub cadre: Rect,
    pub defile: f64,
    /// 🖥 Dans le volet flottant (comme les autres volets) : toute la hauteur du volet, ⤢ et ✕, un clic à côté va à
    /// la chart ; sinon fenêtre flottante (hauteur au contenu, un clic à côté la ferme).
    pub volet: bool,
    /// Résumés en direct (nom de la ligne → texte : thème courant, unité de temps…), donnés par l'app ; sinon le
    /// résumé fixe de `LIGNES`.
    pub resumes: Vec<(&'static str, String)>,
}

impl Reglages {
    /// À la place du volet de la maquette : contre le bord `droite` (le début de la colonne d'icônes), sous la 1re
    /// rangée (`haut`), dans une fenêtre de hauteur `h`.
    pub fn new(droite: f64, haut: f64, h: f64) -> Self {
        let x = (droite - 8.0 - LARGE).max(8.0);
        Self {
            cadre: Rect::new(x, haut, LARGE, (h - haut - 10.0).max(160.0)),
            defile: 0.0,
            volet: false,
            resumes: vec![],
        }
    }

    /// 🖥 Dans le volet flottant, à sa place `v` (fenêtre).
    pub fn dans_volet(v: Rect) -> Self {
        Self {
            cadre: v,
            defile: 0.0,
            volet: true,
            resumes: vec![],
        }
    }

    /// Hauteur du contenu ; s'il défile (plus haut que la place), la marge de fin pour passer au-dessus du rond Argos AI.
    fn haut_contenu(&self) -> f64 {
        let h = HAUT_LISTE + lignes().len() as f64 * ligne_pas() + 14.0 + 20.0 + 16.0;
        if h > self.cadre.h {
            h + crate::aibtn::FIN_DE_LISTE
        } else {
            h
        }
    }

    /// La dernière ligne (« ⋯ Toutes les options »), coordonnées de la fenêtre de l'app.
    pub fn derniere_ligne(&self) -> Rect {
        let (r, q) = (self.rect(), self.places().last().map_or(Rect::new(0.0, 0.0, 0.0, 0.0), |p| p.1));
        Rect::new(r.x + q.x, r.y + q.y, q.w, q.h)
    }

    /// Place réelle (hauteur ajustée au contenu).
    pub fn rect(&self) -> Rect {
        if self.volet {
            return self.cadre;
        }
        Rect::new(self.cadre.x, self.cadre.y, self.cadre.w, self.haut_contenu().min(self.cadre.h))
    }

    /// Choses posées (coordonnées de la fenêtre Réglages).
    fn places(&self) -> Vec<(RegClick, Rect)> {
        let w = self.cadre.w;
        let mut out = vec![(RegClick::Fermer, Rect::new(w - 44.0, 14.0, 30.0, 30.0))];
        if self.volet {
            out.push((RegClick::Agrandir, Rect::new(w - 78.0, 14.0, 30.0, 30.0)));
        }
        let mut y = HAUT_LISTE - self.defile;
        for l in lignes() {
            out.push((RegClick::Ligne(l.3), Rect::new(X - 4.0, y, w - 2.0 * X + 8.0, ligne_pas() - 4.0)));
            y += ligne_pas();
        }
        out.push((RegClick::Options, Rect::new(X, y + 14.0, 190.0, 20.0)));
        out
    }

    pub fn wheel(&mut self, dy: f64) {
        let max = (self.haut_contenu() - self.rect().h).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// Clic (fenêtre de l'app) ; à côté : fermer.
    pub fn click(&self, x: f64, y: f64) -> RegClick {
        let r = self.rect();
        if !r.contains(x, y) {
            return RegClick::Fermer;
        }
        let (lx, ly) = (x - r.x, y - r.y);
        for (c, q) in self.places() {
            let fixe = matches!(c, RegClick::Fermer | RegClick::Agrandir);
            if q.contains(lx, ly) && (fixe || ly >= HAUT_LISTE - 4.0) {
                return c;
            }
        }
        RegClick::None
    }

    /// Image de la fenêtre et sa place ; `mouse` : fenêtre de l'app.
    pub fn draw(&self, text: &Text, mouse: Option<(f64, f64)>) -> Option<(Pixmap, Rect)> {
        use crate::jetons as j;
        let r = self.rect();
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let carte = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0);
        round_fill(&mut pm, carte, j::R_VOLET, j::sur_fond(j::VERRE), 1.0);
        round_stroke(&mut pm, carte, j::R_VOLET, j::sur_fond(j::LISERE), 1.0);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x - r.x, y - r.y));
        Text::graisse(Graisse::Bold).draw(&mut pm, "⚙ Réglages", ((X + 4.0) as f32, 36.0), 17.0, j::TEXTE, Align::Left);
        let mut ligne = 0;
        for (c, q) in self.places() {
            if !matches!(c, RegClick::Fermer | RegClick::Agrandir) && (q.y + q.h < HAUT_LISTE - 4.0 || q.y > r.h) {
                if matches!(c, RegClick::Ligne(_)) {
                    ligne += 1;
                }
                continue;
            }
            match c {
                RegClick::Fermer => {
                    if survol(&q) {
                        round_fill(&mut pm, q, q.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    text.draw(&mut pm, "✕", (q.cx() as f32, (q.cy() + 5.0) as f32), 14.0, j::DIM, Align::Center);
                }
                RegClick::Agrandir => {
                    if survol(&q) {
                        round_fill(&mut pm, q, q.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    // (⤢ n'est pas dans la police : deux flèches en diagonale, comme le ⤢ des autres volets)
                    crate::shapes::lines(
                        &mut pm,
                        q.cx(),
                        q.cy(),
                        &[
                            &[(-5.0, 5.0), (5.0, -5.0)],
                            &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                            &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                        ],
                        j::DIM,
                        1.3,
                    );
                }
                RegClick::Ligne(cible) => {
                    let (ic, nom, resume, _) = *lignes()[ligne];
                    ligne += 1;
                    if survol(&q) {
                        round_fill(&mut pm, q, q.h / 2.0, j::BLANC, 0.06);
                    }
                    let tuile = Rect::new(q.x + 10.0, q.cy() - 15.0, 30.0, 30.0);
                    round_fill(&mut pm, tuile, tuile.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    crate::phosphor::draw(&mut pm, ic, tuile.cx(), tuile.cy(), 16, j::TEXTE);
                    let y = (q.cy() + 5.0) as f32;
                    text.draw(&mut pm, nom, ((q.x + 52.0) as f32, y), 14.0, j::TEXTE, Align::Left);
                    let nw = text.width(nom, 14.0) as f64;
                    // (résumé en direct donné par l'app, sinon le texte fixe)
                    let resume = self.resumes.iter().find(|(n, _)| *n == nom).map_or(resume.to_string(), |(_, t)| t.clone());
                    let res = if matches!(cible, Cible::Bientot(_)) {
                        format!("{resume} · bientôt")
                    } else {
                        resume
                    };
                    let res = text.fit(&res, 12.0, q.w - 52.0 - nw - 44.0);
                    text.draw(&mut pm, &res, ((q.x + q.w - 30.0) as f32, y), 12.0, j::DIM, Align::Right);
                    text.draw(&mut pm, "›", ((q.x + q.w - 14.0) as f32, y), 15.0, j::DIM, Align::Center);
                }
                RegClick::Options => {
                    let c = if survol(&q) { j::TEXTE } else { j::DIM };
                    text.draw(&mut pm, "▸ ⋯ Toutes les options", (q.x as f32, (q.y + 14.0) as f32), 13.0, c, Align::Left);
                }
                RegClick::None => {}
            }
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ⌂ 📱 Au doigt, une seule ligne « Accueil » qui porte ses deux choix ; le PC garde ses deux lignes (liste 2,
    /// point 15).
    #[test]
    fn un_seul_accueil_au_doigt() {
        let pc = lignes();
        assert!(pc.iter().any(|l| l.3 == Cible::Act(Act::AccueilLancement)));
        assert!(pc.iter().any(|l| l.3 == Cible::Act(Act::RevoirAccueil)));
        crate::actions::set_tactile(true);
        let doigt = lignes();
        crate::actions::set_tactile(false);
        let accueil: Vec<_> = doigt.iter().filter(|l| l.1.contains("ccueil")).collect();
        assert_eq!(accueil.len(), 1, "{accueil:?}");
        assert_eq!((accueil[0].1, accueil[0].3), ("Accueil", Cible::Accueil));
        assert_eq!(doigt.len(), pc.len() - 2, "sans Raccourcis ni la 2e ligne d'accueil");
        // 👆 by the finger every row (« Accueil » included) is a 55-point target (44 real px); the PC keeps 46
        assert_eq!(ligne_pas() - 4.0, 46.0);
        crate::actions::set_tactile(true);
        let pas = ligne_pas();
        crate::actions::set_tactile(false);
        assert!(pas - 4.0 >= crate::formfactor::TOUCH_MIN_PT);
    }

    /// ⚙ La fenêtre Réglages : contre la colonne d'icônes, dans la fenêtre ; chaque ligne mène à sa cible ; ⋯ Toutes
    /// les options et ✕ répondent ; à côté : fermer ; elle défile ; coins arrondis.
    #[test]
    fn fenetre_reglages() {
        let mut g = Reglages::new(1390.0, 66.0, 900.0);
        let r = g.rect();
        assert!(r.x + r.w <= 1390.0 - 8.0 + 0.01 && r.y == 66.0 && r.y + r.h <= 890.0, "{r:?}");
        let ligne = |g: &Reglages, k: usize| g.places()[k + 1].1;
        let q = ligne(&g, 0);
        assert_eq!(g.click(r.x + q.cx(), r.y + q.cy()), RegClick::Ligne(Cible::Act(Act::ChartSettings)));
        let q = ligne(&g, 1);
        assert_eq!(g.click(r.x + q.cx(), r.y + q.cy()), RegClick::Ligne(Cible::Act(Act::Theme)));
        assert_eq!(g.click(r.x + r.w - 29.0, r.y + 29.0), RegClick::Fermer);
        assert_eq!(g.click(r.x - 50.0, r.y + 100.0), RegClick::Fermer, "à côté");
        g.wheel(10_000.0);
        let opt = g.places().last().unwrap().1;
        assert!(opt.y + opt.h <= g.rect().h, "⋯ Toutes les options visible en bas");
        assert_eq!(g.click(r.x + opt.x + 20.0, r.y + opt.cy()), RegClick::Options);
        let (pm, _) = g.draw(&Text::default(), None).unwrap();
        assert_eq!(pm.pixel(1, 1).unwrap().alpha(), 0, "coins arrondis");
    }
}
