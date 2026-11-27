//! 🗺 Plan de l'app (bouton 🗺 du cell) : une page pleine à la mise en page de la page « plan » de la maquette
//! (‹ Retour, titre, ✕ ; un texte, des blocs « endroit » avec leur nombre à droite, des tuiles 2 × 2, des lignes avec
//! une puce « Voir » ; tout défile au doigt). Le contenu est le plan de l'app (choix de Jee, 15 nov.) : un bloc par
//! endroit, et chaque bloc, tuile ou « Voir » ouvre vraiment sa partie de l'app.

use crate::accueil::Section;
use crate::assistant::wrap;
use crate::jetons as j;
use crate::shapes::{round_fill, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

/// Une partie de l'app qu'on peut ouvrir depuis le plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lieu {
    /// Fermer le plan : la chart.
    Chart,
    Outils,
    Indicateurs,
    Alertes,
    Trading,
    Watchlist,
    Recherche,
    Reglages,
    Argos,
    /// ☰ Appli : mises à jour, installer, copier l'URL, plein écran, zoom de l'interface.
    Appli,
    Splits,
    Accueil,
    Section(Section),
    Raccourcis,
    Simulateur,
}

/// Ce que fait un toucher dans le plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanClick {
    Rien,
    Fermer,
    Ouvrir(Lieu),
}

/// Un endroit de l'app (un bloc du plan).
struct Endroit {
    nom: &'static str,
    texte: &'static str,
    nombre: Option<String>,
    lieu: Lieu,
}

fn endroits() -> Vec<Endroit> {
    let e = |nom, texte, nombre: Option<String>, lieu| Endroit { nom, texte, nombre, lieu };
    vec![
        e(
            "📈 Chart",
            "Les bougies de ton symbole. Glisse pour défiler, pince pour zoomer, appui long pour le menu de ce qui est sous ton doigt.",
            None,
            Lieu::Chart,
        ),
        e(
            "✏️ Outils de dessin",
            "Lignes, Fibonacci, formes, textes et émojis : la barre ⋮⋮ sur la chart (elle se replie après ton choix) et le crayon en bas.",
            Some(format!("{} outils", crate::tools::TOOLS.len())),
            Lieu::Outils,
        ),
        e(
            "🧪 Indicateurs",
            "Ajoute, règle et range tes indicateurs ; tes favoris d'abord.",
            Some(format!("{} indicateurs", crate::indicators::Ind::ALL.len())),
            Lieu::Indicateurs,
        ),
        e(
            "🔔 Alertes",
            "Prix, lignes et indicateurs : Argos te prévient quand ça bouge.",
            None,
            Lieu::Alertes,
        ),
        e(
            "🛡️ Trading",
            "Ordres démo, positions et le Gardien du risque, sans argent réel.",
            None,
            Lieu::Trading,
        ),
        e(
            "📋 Watchlist",
            "Tes symboles et leur variation du jour ; touche-en un pour l'ouvrir.",
            None,
            Lieu::Watchlist,
        ),
        e(
            "🔎 Recherche",
            "Symboles, leçons, indicateurs, réglages et pages : un seul champ.",
            Some(format!("{} lignes", crate::recherche::items_fixes().len())),
            Lieu::Recherche,
        ),
        e("▦ Mises en page", "Une, deux ou quatre charts à la fois.", None, Lieu::Splits),
        e("⚙️ Réglages", "Thème, style verre et panneaux de l'écran.", None, Lieu::Reglages),
        e(
            "☰ Menu ARGOS",
            "Ton compte, l'Académie, le Journal, le Portefeuille et tout le reste.",
            None,
            Lieu::Argos,
        ),
        e(
            "📲 Appli",
            // (📱 phone view: no full screen nor interface size in ☰ Appli)
            if crate::actions::montrer_raccourcis() {
                "Mises à jour, installer l'appli, copier le lien, plein écran, taille de l'interface."
            } else {
                "Mises à jour, installer l'appli, copier le lien, langue."
            },
            None,
            Lieu::Appli,
        ),
    ]
}

/// Tuiles 2 × 2 : 4 pages de l'Accueil.
const TUILES: [Section; 4] = [Section::Academie, Section::Journal, Section::Portefeuille, Section::Calendrier];

/// Lignes avec « Voir » : (titre, détail, où).
fn raccourcis() -> Vec<(&'static str, &'static str, Lieu)> {
    // 📱 phone view: no keyboard, so no « Ctrl + K » and no « Raccourcis du clavier » line (Jee, 20 Nov)
    let pc = crate::actions::montrer_raccourcis();
    let mut v = vec![
        ("Accueil", "Les 10 cartes : marchés, académie, journal, calendrier…", Lieu::Accueil),
        (
            "Rechercher partout",
            if pc {
                "Ctrl + K sur un clavier"
            } else {
                "Symboles, leçons, indicateurs, réglages"
            },
            Lieu::Recherche,
        ),
        ("Simulateur démo", "Ordres démo, replay, scénarios d'examen", Lieu::Simulateur),
    ];
    if pc {
        v.push(("Raccourcis du clavier", "Ctrl + / sur un clavier", Lieu::Raccourcis));
    }
    v.push(("Académie", Section::Academie.sous_titre(), Lieu::Section(Section::Academie)));
    v
}

// 🫧 Mesures de la page « plan » de la maquette sur un cell de 390 × 844 (#page, #pagecorps…).
/// Haut du contenu qui défile (sous ‹ Retour, le titre et ✕).
const CORPS: f64 = 86.0;
/// Marge de côté.
const MARGE: f64 = 20.0;
/// Texte (13 px, interligne 1,35).
const T: f32 = 13.0;
const INTER: f64 = 17.55;
/// Titres de partie (h3 : 15,2 px gras, interligne 20,6).
const H3: f32 = 15.2;
const H3_H: f64 = 20.6;
/// Marge sous un paragraphe (1 em).
const APRES_P: f64 = 13.0;
/// Bloc « endroit » : rayon 16, marge intérieure 12, 10 entre deux.
const BLOC_R: f64 = 16.0;
const BLOC_PAD: f64 = 12.0;
/// Tuile : rayon 14, marge 9, 8 entre deux, texte 12.
const TUILE_R: f64 = 14.0;
const TUILE_PAD: f64 = 9.0;
/// Ligne avec « Voir » : rayon 14, marge 10, 8 entre deux ; puce « Voir » 44 × 27,6, rayon 12.
const PROP_R: f64 = 14.0;
const PROP_PAD: f64 = 10.0;
const VOIR: (f64, f64) = (44.1, 27.6);

#[derive(Debug, Clone, PartialEq)]
enum Quoi {
    /// Lignes de texte, en gras ou pâle.
    Texte(Vec<String>, bool, bool),
    H3(String),
    Bloc {
        nom: String,
        nombre: Option<String>,
        lignes: Vec<String>,
    },
    Tuile {
        nom: String,
        lignes: Vec<String>,
    },
    Prop {
        n: usize,
        titre: String,
        lignes: Vec<String>,
    },
}

/// Un élément placé (y : depuis le haut du contenu, sans le défilement).
#[derive(Debug, Clone, PartialEq)]
struct Elem {
    r: Rect,
    quoi: Quoi,
    lieu: Option<Lieu>,
}

/// La page ouverte.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    pub scroll: f64,
}

/// 🫧 Le fond de la page en style verre : la teinte de la maquette (rgba(8, 10, 16, 0,82)), seule, à poser flouée sous
/// le contenu (le flou du verre abîme le texte s'il est dans la même image).
pub fn fond(w: f64, h: f64) -> Option<Pixmap> {
    let mut pm = crate::px::new(w.max(1.0) as u32, h.max(1.0) as u32)?;
    crate::fill(&mut pm, 0.0, 0.0, w, h, Rgb(8, 10, 16), 0.82);
    Some(pm)
}

/// ‹ Retour (coordonnées de la page).
pub fn retour() -> Rect {
    Rect::new(20.0, 29.2, 68.8, 27.6)
}

/// ✕ (coordonnées de la page de largeur `w`).
pub fn croix(w: f64) -> Rect {
    Rect::new(w - 50.0, 28.0, 30.0, 30.0)
}

fn mise_en_page(text: &Text, w: f64) -> (Vec<Elem>, f64) {
    let gras = Text::graisse(Graisse::Bold);
    let cw = (w - MARGE * 2.0 - 6.0).max(120.0);
    let mut v = vec![];
    let mut y = APRES_P;
    let texte = |v: &mut Vec<Elem>, y: &mut f64, s: &str, g: bool, pale: bool| {
        let l = wrap(if g { &gras } else { text }, s, T, cw);
        let h = l.len() as f64 * INTER;
        v.push(Elem {
            r: Rect::new(MARGE, *y, cw, h),
            quoi: Quoi::Texte(l, g, pale),
            lieu: None,
        });
        *y += h;
    };
    let h3 = |v: &mut Vec<Elem>, y: &mut f64, s: &str| {
        *y += H3 as f64;
        v.push(Elem {
            r: Rect::new(MARGE, *y, cw, H3_H),
            quoi: Quoi::H3(s.into()),
            lieu: None,
        });
        *y += H3_H + H3 as f64;
    };
    texte(&mut v, &mut y, "Tout Argos sur une page.", true, false);
    texte(
        &mut v,
        &mut y,
        "Touche un bloc pour y aller ; ‹ Retour ou ✕ te ramène à la chart.",
        false,
        false,
    );
    y += APRES_P;
    let e = endroits();
    h3(&mut v, &mut y, &format!("{} endroits dans l'app", e.len()));
    let noms: Vec<&str> = e.iter().map(|x| x.nom.split_once(' ').map_or(x.nom, |(_, n)| n)).collect();
    texte(&mut v, &mut y, &noms.join(" · "), false, true);
    y += APRES_P;
    for x in &e {
        let lignes = wrap(text, x.texte, T, cw - BLOC_PAD * 2.0);
        let h = BLOC_PAD * 2.0 + 24.0 + lignes.len() as f64 * INTER;
        v.push(Elem {
            r: Rect::new(MARGE, y, cw, h),
            quoi: Quoi::Bloc {
                nom: x.nom.into(),
                nombre: x.nombre.clone(),
                lignes,
            },
            lieu: Some(x.lieu),
        });
        y += h + 10.0;
    }
    h3(&mut v, &mut y, "Pages de l'Accueil");
    let tw = (cw - 8.0) / 2.0;
    for rang in TUILES.chunks(2) {
        let cases: Vec<(Section, Vec<String>)> = rang.iter().map(|&s| (s, wrap(text, s.sous_titre(), 12.0, tw - TUILE_PAD * 2.0))).collect();
        let h = TUILE_PAD * 2.0 + 17.0 + cases.iter().map(|c| c.1.len()).max().unwrap_or(1) as f64 * 16.2;
        for (k, (s, lignes)) in cases.into_iter().enumerate() {
            v.push(Elem {
                r: Rect::new(MARGE + k as f64 * (tw + 8.0), y, tw, h),
                quoi: Quoi::Tuile {
                    nom: format!("{} {}", s.emoji(), s.nom()),
                    lignes,
                },
                lieu: Some(Lieu::Section(s)),
            });
        }
        y += h + 8.0;
    }
    y += 6.0;
    h3(&mut v, &mut y, "Toutes les cartes de l'Accueil");
    let cartes: Vec<&str> = Section::ALL.iter().map(|s| s.nom()).collect();
    texte(&mut v, &mut y, &cartes.join(" · "), false, false);
    y += APRES_P;
    h3(&mut v, &mut y, "Raccourcis");
    for (n, (titre, detail, lieu)) in raccourcis().into_iter().enumerate() {
        let lignes = wrap(text, detail, 12.0, cw - PROP_PAD * 2.0 - 30.0 - VOIR.0 - 10.0);
        let h = (PROP_PAD * 2.0 + 18.0 + lignes.len() as f64 * 16.2).max(PROP_PAD * 2.0 + VOIR.1);
        v.push(Elem {
            r: Rect::new(MARGE, y, cw, h),
            quoi: Quoi::Prop {
                n: n + 1,
                titre: titre.into(),
                lignes,
            },
            lieu: Some(lieu),
        });
        y += h + 8.0;
    }
    // en bas : la place du bouton rond Argos AI (posé par-dessus tout), pour que la dernière ligne remonte au-dessus
    (v, y + 110.0)
}

/// La puce « Voir » d'une ligne (mêmes coordonnées que la ligne).
fn voir(r: Rect) -> Rect {
    Rect::new(r.x + r.w - PROP_PAD - VOIR.0, r.y + PROP_PAD, VOIR.0, VOIR.1)
}

impl Plan {
    /// Défilement le plus grand (fenêtre `w` × `h`).
    fn max(&self, w: f64, h: f64) -> f64 {
        let (_, total) = mise_en_page(&Text::default(), w);
        (total - (h - CORPS)).max(0.0)
    }

    /// Molette / doigt : `dy` positif = le contenu monte.
    pub fn wheel(&mut self, dy: f64, w: f64, h: f64) {
        self.scroll = (self.scroll + dy).clamp(0.0, self.max(w, h));
    }

    /// En haut de la page (un coup de doigt vers le bas la ferme alors).
    pub fn at_top(&self) -> bool {
        self.scroll <= 0.5
    }

    /// Ce que touche le doigt en (`x`, `y`) (coordonnées de la page `w` × `h`).
    pub fn click(&self, x: f64, y: f64, w: f64, h: f64) -> PlanClick {
        if retour().contains(x, y) || croix(w).contains(x, y) {
            return PlanClick::Fermer;
        }
        if y < CORPS || y > h {
            return PlanClick::Rien;
        }
        let cy = y - CORPS + self.scroll;
        let (v, _) = mise_en_page(&Text::default(), w);
        v.iter()
            .find(|e| e.lieu.is_some() && e.r.contains(x, cy))
            .and_then(|e| e.lieu)
            .map_or(PlanClick::Rien, PlanClick::Ouvrir)
    }

    /// Les zones touchables (pour les essais) : (rectangle de la page, ce qu'il ouvre), y compris les « Voir ».
    pub fn zones(&self, w: f64) -> Vec<(Rect, Lieu)> {
        let (v, _) = mise_en_page(&Text::default(), w);
        let mut z = vec![];
        for e in v {
            let Some(l) = e.lieu else { continue };
            let r = Rect::new(e.r.x, e.r.y + CORPS - self.scroll, e.r.w, e.r.h);
            if matches!(e.quoi, Quoi::Prop { .. }) {
                z.push((voir(r), l));
            }
            z.push((r, l));
        }
        z
    }

    /// Dessine la page plein écran (`w` × `h`).
    pub fn draw(&self, text: &Text, w: f64, h: f64, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let mut pm = crate::px::new(w.max(1.0) as u32, h.max(1.0) as u32)?;
        // style verre : le fond est posé à part ([`fond`], flouté) et le contenu net par-dessus ; sinon, presque plein
        if !crate::glass::on() {
            crate::fill(&mut pm, 0.0, 0.0, w, h, Rgb(8, 10, 16), 0.97);
        }
        let gras = Text::graisse(Graisse::Bold);
        let moyen = Text::graisse(Graisse::Medium);
        let carte = j::VERRE2;
        // le contenu qui défile, coupé sous l'en-tête
        let (v, _) = mise_en_page(text, w);
        let mut corps = crate::px::new(w as u32, (h - CORPS).max(1.0) as u32)?;
        let survol = |r: Rect| mouse.is_some_and(|(mx, my)| my > CORPS && r.contains(mx, my - CORPS + self.scroll));
        for e in &v {
            let r = Rect::new(e.r.x, e.r.y - self.scroll, e.r.w, e.r.h);
            if r.y + r.h < 0.0 || r.y > h - CORPS {
                continue;
            }
            let base = |k: usize, y0: f64, pas: f64| (y0 + k as f64 * pas + pas * 0.72) as f32;
            match &e.quoi {
                Quoi::Texte(l, g, pale) => {
                    let (t, c) = (if *g { &gras } else { text }, if *pale { j::DIM } else { j::TEXTE });
                    for (k, s) in l.iter().enumerate() {
                        t.draw(&mut corps, s, (r.x as f32, base(k, r.y, INTER)), T, c, Align::Left);
                    }
                }
                Quoi::H3(s) => gras.draw(&mut corps, s, (r.x as f32, base(0, r.y, H3_H)), H3, j::TEXTE, Align::Left),
                Quoi::Bloc { nom, nombre, lignes } => {
                    round_fill(&mut corps, r, BLOC_R, carte.0, carte.1 + if survol(e.r) { 0.05 } else { 0.0 });
                    let y0 = r.y + BLOC_PAD;
                    gras.draw(&mut corps, nom, ((r.x + BLOC_PAD) as f32, (y0 + 17.0) as f32), 15.0, j::TEXTE, Align::Left);
                    if let Some(n) = nombre {
                        text.draw(&mut corps, n, ((r.x + r.w - BLOC_PAD) as f32, (y0 + 16.0) as f32), T, j::DIM, Align::Right);
                    }
                    for (k, s) in lignes.iter().enumerate() {
                        text.draw(&mut corps, s, ((r.x + BLOC_PAD) as f32, base(k, y0 + 24.0, INTER)), T, j::TEXTE, Align::Left);
                    }
                }
                Quoi::Tuile { nom, lignes } => {
                    round_fill(&mut corps, r, TUILE_R, carte.0, carte.1 + if survol(e.r) { 0.05 } else { 0.0 });
                    let x0 = (r.x + TUILE_PAD) as f32;
                    gras.draw(
                        &mut corps,
                        &gras.fit(nom, 12.0, r.w - TUILE_PAD * 2.0),
                        (x0, (r.y + TUILE_PAD + 12.5) as f32),
                        12.0,
                        j::TEXTE,
                        Align::Left,
                    );
                    for (k, s) in lignes.iter().enumerate() {
                        text.draw(&mut corps, s, (x0, base(k, r.y + TUILE_PAD + 17.0, 16.2)), 12.0, j::DIM, Align::Left);
                    }
                }
                Quoi::Prop { n, titre, lignes } => {
                    round_fill(&mut corps, r, PROP_R, carte.0, carte.1);
                    let x0 = r.x + PROP_PAD;
                    gras.draw(
                        &mut corps,
                        &n.to_string(),
                        ((x0 + 8.0) as f32, (r.y + PROP_PAD + 14.0) as f32),
                        15.0,
                        j::ACCENT,
                        Align::Center,
                    );
                    let xt = (x0 + 30.0) as f32;
                    moyen.draw(&mut corps, titre, (xt, (r.y + PROP_PAD + 13.5) as f32), T, j::TEXTE, Align::Left);
                    for (k, s) in lignes.iter().enumerate() {
                        text.draw(&mut corps, s, (xt, base(k, r.y + PROP_PAD + 18.0, 16.2)), 12.0, j::DIM, Align::Left);
                    }
                    let vr = voir(r);
                    let on = survol(voir(e.r));
                    round_fill(&mut corps, vr, 12.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.35 } else { 0.07 });
                    text.draw(&mut corps, "Voir", (vr.cx() as f32, (vr.cy() + 4.5) as f32), T, j::TEXTE, Align::Center);
                }
            }
        }
        crate::px::put(&mut pm, 0.0, CORPS, &corps);
        // en-tête fixe : ‹ Retour, le titre, ✕
        let rr = retour();
        round_fill(&mut pm, rr, 12.0, j::BLANC, 0.07);
        text.draw(&mut pm, "‹ Retour", (rr.cx() as f32, (rr.cy() + 4.5) as f32), T, j::TEXTE, Align::Center);
        let cr = croix(w);
        round_fill(&mut pm, cr, 15.0, j::BLANC, 0.07);
        text.draw(&mut pm, "✕", (cr.cx() as f32, (cr.cy() + 4.5) as f32), T, j::TEXTE, Align::Center);
        let titre = gras.fit("🗺 Plan de l'app", 20.0, cr.x - 8.0 - (rr.x + rr.w + 8.0));
        gras.draw(&mut pm, &titre, ((w / 2.0) as f32, 50.0), 20.0, j::TEXTE, Align::Center);
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mesures_et_contenu() {
        let p = Plan::default();
        let (v, total) = mise_en_page(&Text::default(), 390.0);
        assert_eq!(retour(), Rect::new(20.0, 29.2, 68.8, 27.6));
        assert_eq!(croix(390.0), Rect::new(340.0, 28.0, 30.0, 30.0));
        // blocs « endroit » : un par endroit, 344 de large comme la maquette, nombre réel à droite
        let blocs: Vec<&Elem> = v.iter().filter(|e| matches!(e.quoi, Quoi::Bloc { .. })).collect();
        assert_eq!(blocs.len(), endroits().len());
        assert!(blocs.iter().all(|b| b.r.w == 344.0 && b.r.x == 20.0));
        assert!(v
            .iter()
            .any(|e| matches!(&e.quoi, Quoi::Bloc { nombre: Some(n), .. } if *n == format!("{} indicateurs", crate::indicators::Ind::ALL.len()))));
        // tuiles 2 × 2 de 168 de large
        let tuiles: Vec<&Elem> = v.iter().filter(|e| matches!(e.quoi, Quoi::Tuile { .. })).collect();
        assert_eq!(tuiles.len(), 4);
        assert!(tuiles.iter().all(|t| t.r.w == 168.0));
        assert_eq!(tuiles[0].r.y, tuiles[1].r.y);
        // aucun nom de fichier ni texte technique
        let tout = format!("{v:?}");
        assert!(!tout.contains(".rs") && !tout.contains("WASM") && !tout.contains("Tauri"));
        // ça défile, jusqu'en bas seulement
        let mut p2 = p.clone();
        p2.wheel(1e9, 390.0, 844.0);
        assert_eq!(p2.scroll, total - (844.0 - CORPS));
        p2.wheel(-1e9, 390.0, 844.0);
        assert!(p2.at_top());
        assert!(p.draw(&Text::default(), 390.0, 844.0, None).is_some());
    }

    #[test]
    fn chaque_toucher_agit() {
        let mut p = Plan::default();
        let (w, h) = (390.0, 844.0);
        assert_eq!(p.click(40.0, 40.0, w, h), PlanClick::Fermer, "‹ Retour");
        assert_eq!(p.click(355.0, 43.0, w, h), PlanClick::Fermer, "✕");
        // chaque zone touchable (visible ou après défilement) ouvre son endroit
        let mut vus = std::collections::HashSet::new();
        for _ in 0..200 {
            for (r, l) in p.zones(w) {
                if r.y >= CORPS && r.y + r.h <= h {
                    assert_eq!(p.click(r.cx(), r.cy(), w, h), PlanClick::Ouvrir(l), "{l:?} à {r:?}");
                    vus.insert(format!("{l:?}"));
                }
            }
            p.wheel(60.0, w, h);
        }
        for l in [
            Lieu::Chart,
            Lieu::Outils,
            Lieu::Indicateurs,
            Lieu::Alertes,
            Lieu::Trading,
            Lieu::Watchlist,
            Lieu::Recherche,
            Lieu::Splits,
            Lieu::Reglages,
            Lieu::Argos,
            Lieu::Appli,
            Lieu::Accueil,
            Lieu::Simulateur,
            Lieu::Raccourcis,
        ] {
            assert!(vus.contains(&format!("{l:?}")), "{l:?} touchable");
        }
        assert!(vus.contains("Section(Academie)") && vus.contains("Section(Journal)"));
    }
}
