//! 🤖 Carte « Argos AI · ton trade » (lot 7 du Simulateur, maquette v1 validée par Jee le 21 nov.) : après un trade
//! démo ou replay fermé, une carte de verre un peu transparente (🖥 en bas à gauche de la chart, 📱 au-dessus de la
//! barre du bas) dit, une ligne chacune, ce que disaient les indicateurs à l'entrée, si le plan a été suivi et une
//! leçon à revoir (avec la leçon de l'Académie qui va avec), puis le rappel « Analyse affichée, pas un ordre. ».
//! ✕ la ferme ; le trade suivant la remplace. Le commentaire vient des règles d'`argos_ai::review` (la fenêtre le
//! traduit en [`Avis`]) ; ici : la mise en page, le dessin et les clics. Jamais un ordre.

use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::simulateur::argent;
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::Money;
use tiny_skia::Pixmap;

/// 🖥 Largeur de la carte sur PC (maquette : 372 de texte, plus 16 de marge intérieure et le liseré de chaque côté).
pub const LARGEUR: f64 = 406.0;
/// Marge de la carte avec les bords de la chart.
const MARGE: f64 = 16.0;
/// Écart entre le bas de la carte et le bas de la chart (au-dessus de l'axe du temps).
const MARGE_BAS: f64 = 12.0;
/// Marges intérieures (celles de la maquette, plus le liseré de 1).
const PAD_X: f64 = 17.0;
const PAD_HAUT: f64 = 15.0;
const PAD_BAS: f64 = 14.0;
const RAYON: f64 = 22.0;
/// Le rond Argos AI de l'en-tête.
const ROND: f64 = 30.0;
/// Rangée d'un petit titre (et de sa pastille).
const RANGEE: f64 = 22.0;
/// Hauteur d'une pastille (12 px gras, 3 en haut et en bas).
const PASTILLE: f64 = 20.5;
/// Le lien vers la leçon de l'Académie (une pilule).
const LIEN: f64 = 32.0;

// — couleurs de la maquette (carte.css) —
const FOND: (Rgb, f32) = (Rgb(22, 30, 58), 0.66);
const TITRE: Rgb = Rgb(0xe8, 0xec, 0xf7);
const DOUX: Rgb = Rgb(0x93, 0xa0, 0xc2);
const TEXTE: Rgb = Rgb(0xc9, 0xd3, 0xee);
const HAUSSE: Rgb = Rgb(0x3c, 0xcf, 0x8e);
const BAISSE: Rgb = Rgb(0xff, 0x5c, 0x66);

/// Le ton d'une partie : la couleur de sa pastille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ton {
    /// Vert : dans ton sens, plan suivi.
    Bon,
    /// Orange : autant pour que contre, plan suivi en partie.
    Moyen,
    /// Rouge : contre toi, plan pas suivi.
    Mauvais,
    /// Gris, sans jugement : pas lus, Gardien PropFirm, fin du replay.
    Neutre,
}

impl Ton {
    /// (texte, fond et son opacité) de la pastille.
    fn couleurs(self) -> (Rgb, Rgb, f32) {
        match self {
            Ton::Bon => (HAUSSE, Rgb(60, 207, 142), 0.17),
            Ton::Moyen => (Rgb(0xff, 0xb7, 0x4d), Rgb(255, 183, 77), 0.17),
            Ton::Mauvais => (Rgb(0xff, 0x6b, 0x73), Rgb(234, 57, 67), 0.2),
            Ton::Neutre => (DOUX, Rgb(255, 255, 255), 0.08),
        }
    }
}

/// Une partie de la carte : sa pastille et sa phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct Partie {
    pub ton: Ton,
    pub pastille: String,
    pub texte: String,
}

/// Ce que la carte montre d'un trade fermé.
#[derive(Debug, Clone, PartialEq)]
pub struct Avis {
    /// ▲ à l'achat, ▼ à la vente.
    pub long: bool,
    pub base: String,
    /// « −1,62 R » (`None` : sans stop, pas de R).
    pub r: Option<String>,
    /// Le résultat net du trade entier (coûts et funding déduits).
    pub net: Money,
    /// « stop touché », « objectif atteint »…
    pub sortie: String,
    /// « À l'entrée » et « Ton plan ».
    pub entree: Partie,
    pub plan: Partie,
    /// « Une leçon à revoir » et la leçon de l'Académie qui va avec (son identifiant).
    pub lecon: String,
    pub academie: Option<&'static str>,
    pub rappel: String,
}

/// Ce qu'un clic sur la carte demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvisClick {
    /// Sur la carte, rien à faire (le clic ne passe pas à la chart dessous).
    Rien,
    Fermer,
    /// 🎓 Ouvrir cette leçon de l'Académie.
    Lecon(&'static str),
}

/// Un morceau de la carte, placé (coordonnées de la carte, `y` : ligne de base des textes).
#[derive(Debug, Clone, PartialEq)]
enum Morceau {
    Resultat(f64, String),
    Etiquette(f64, String),
    Pastille(Rect, String, Ton),
    Ligne(f64, String, f32, Rgb),
    Lien(Rect, String),
    Rappel(f64, String),
}

/// La carte mise en page pour une largeur : ses morceaux, sa hauteur.
struct Mise {
    morceaux: Vec<Morceau>,
    h: f64,
}

/// Le titre de la leçon de l'Académie `id` (vide si elle n'existe pas).
pub(crate) fn titre_lecon(id: &str) -> &'static str {
    crate::academy::lecons().iter().find(|l| l.id == id).map_or("", |l| l.titre.as_str())
}

/// Coupe `s` en lignes de `w` au plus (`size` points) ; « : », « ; », « ? », « ! », « » » et « · » restent avec le
/// mot d'avant, « « » avec le mot d'après (jamais une ligne qui commence par deux-points).
pub(crate) fn couper(text: &Text, s: &str, size: f32, w: f64) -> Vec<String> {
    let mut mots: Vec<String> = vec![];
    let mut ouvre = false;
    for m in s.split(' ').filter(|m| !m.is_empty()) {
        match mots.last_mut() {
            Some(d) if ouvre || [":", ";", "?", "!", "»", "·"].contains(&m) => {
                d.push(' ');
                d.push_str(m);
            }
            _ => mots.push(m.to_string()),
        }
        ouvre = m == "«";
    }
    let mut out: Vec<String> = vec![];
    let mut ligne = String::new();
    for m in mots {
        let essai = if ligne.is_empty() { m.clone() } else { format!("{ligne} {m}") };
        // (1 point de tolérance : la mesure des lettres diffère d'un poil de celle du navigateur de la maquette)
        if !ligne.is_empty() && text.width(&essai, size) as f64 > w + 1.0 {
            out.push(std::mem::replace(&mut ligne, m));
        } else {
            ligne = essai;
        }
    }
    if !ligne.is_empty() || out.is_empty() {
        out.push(ligne);
    }
    out
}

/// Hauteur d'une ligne de texte (`line-height: normal` d'Inter, 1,21 fois la taille) et place de sa ligne de base.
fn ligne_normale(size: f64) -> (f64, f64) {
    (size * 1.21, size * 0.97)
}

/// A chip `p`: its tone's colours, its text in bold 12 (also on the month report's card).
pub(crate) fn pastille(pm: &mut Pixmap, gras: &Text, p: Rect, t: &str, ton: Ton) {
    let (c, fond, a) = ton.couleurs();
    round_fill(pm, p, p.h / 2.0, fond, a);
    gras.draw(pm, t, (p.cx() as f32, (p.cy() + 4.3) as f32), 12.0, c, Align::Center);
}

/// 🎓 The link `p` to an Academy lesson (a pill): 🎓, « Leçon : … » cut to fit, › (also on the month report's card).
pub(crate) fn lien(pm: &mut Pixmap, text: &Text, p: Rect, t: &str, survol: bool) {
    round_fill(pm, p, p.h / 2.0, crate::palette::WHITE, if survol { 0.11 } else { 0.06 });
    let base = (p.cy() + 4.6) as f32;
    text.draw(pm, "🎓", ((p.x + 12.0) as f32, (p.cy() + 5.2) as f32), 13.5, TEXTE, Align::Left);
    let place = p.w - 12.0 - 21.0 - 30.0;
    text.draw(pm, &text.fit(t, 13.0, place), ((p.x + 33.0) as f32, base), 13.0, TEXTE, Align::Left);
    text.draw(pm, "›", ((p.x + p.w - 14.0) as f32, base), 15.0, TEXTE, Align::Right);
}

impl Avis {
    /// « ▲ BTC · −1,62 R · −1 618,40 $ · stop touché ».
    pub fn resultat(&self) -> String {
        let r = self.r.as_ref().map_or(String::new(), |r| format!("{r} · "));
        format!(
            "{} {} · {r}{} · {}",
            if self.long { "▲" } else { "▼" },
            self.base,
            argent(self.net, true),
            self.sortie
        )
    }

    fn couleur_resultat(&self) -> Rgb {
        match self.net.cmp(&Money::ZERO) {
            std::cmp::Ordering::Greater => HAUSSE,
            std::cmp::Ordering::Less => BAISSE,
            std::cmp::Ordering::Equal => TITRE,
        }
    }

    /// La carte mise en page sur la largeur `w`.
    fn mise(&self, text: &Text, w: f64) -> Mise {
        let gras = Text::graisse(Graisse::Bold);
        let moyen = Text::graisse(Graisse::Medium);
        let large = (w - 2.0 * PAD_X).max(40.0);
        let mut m = vec![];
        // en-tête (le rond et le titre), puis le résultat (13, demi-gras), à 8 dessous
        let mut y = PAD_HAUT + ROND + 8.0;
        let (lh, base) = ligne_normale(13.0);
        for l in couper(&moyen, &self.resultat(), 13.0, large - 2.0) {
            m.push(Morceau::Resultat(y + base, l));
            y += lh;
        }
        // une partie : son petit titre et sa pastille (à 13 au-dessus, 5 dessous), puis sa phrase (13, interligne 1,4)
        let partie = |m: &mut Vec<Morceau>, y: &mut f64, titre: &str, p: Option<&Partie>| {
            *y += 13.0;
            m.push(Morceau::Etiquette(*y + RANGEE / 2.0 + 4.0, crate::i18n::tr(titre).to_uppercase()));
            if let Some(p) = p {
                let pw = gras.width(&p.pastille, 12.0) as f64 + 20.0;
                let r = Rect::new(w - PAD_X - pw, *y + (RANGEE - PASTILLE) / 2.0, pw, PASTILLE);
                m.push(Morceau::Pastille(r, p.pastille.clone(), p.ton));
            }
            *y += RANGEE + 5.0;
        };
        for (titre, p) in [("À l'entrée", &self.entree), ("Ton plan", &self.plan)] {
            partie(&mut m, &mut y, titre, Some(p));
            for l in couper(text, &p.texte, 13.0, large) {
                m.push(Morceau::Ligne(y + 1.2 + 12.6, l, 13.0, TEXTE));
                y += 13.0 * 1.4;
            }
        }
        partie(&mut m, &mut y, "Une leçon à revoir", None);
        for l in couper(text, &self.lecon, 13.5, large) {
            m.push(Morceau::Ligne(y + 1.3 + 13.1, l, 13.5, TITRE));
            y += 13.5 * 1.4;
        }
        if let Some(id) = self.academie {
            y += 9.0;
            m.push(Morceau::Lien(Rect::new(PAD_X, y, large, LIEN), format!("Leçon : {}", titre_lecon(id))));
            y += LIEN;
        }
        y += 11.0;
        let (lh, base) = ligne_normale(11.5);
        for l in couper(text, &self.rappel, 11.5, large) {
            m.push(Morceau::Rappel(y + base, l));
            y += lh;
        }
        Mise {
            morceaux: m,
            h: (y + PAD_BAS).ceil(),
        }
    }

    /// Hauteur de la carte sur la largeur `w`.
    pub fn hauteur(&self, text: &Text, w: f64) -> f64 {
        self.mise(text, w).h
    }

    /// Place de la carte dans la fenêtre. `chart` : la chart sans ses axes (fenêtre) ; 🖥 406 de large (372 de texte)
    /// en bas à gauche, à 16 du bord ; 📱 (`cell`) toute la largeur de la fenêtre `win_w` moins 16 de chaque côté. Le bas de la
    /// carte est à 12 au-dessus du bas de la chart (l'axe du temps reste visible). `eviter` : une barre qui doit rester
    /// visible et à portée (celle du replay) ; si la carte la couvre, elle se pose à 8 au-dessus. Jamais plus haut que
    /// la chart.
    pub fn place(&self, text: &Text, chart: Rect, win_w: f64, cell: bool, eviter: Option<Rect>) -> Rect {
        let (x, w) = if cell {
            (MARGE, (win_w - 2.0 * MARGE).max(200.0))
        } else {
            (chart.x + MARGE, LARGEUR.min(chart.w - 2.0 * MARGE).max(240.0))
        };
        let h = self.hauteur(text, w);
        let mut y = chart.y + chart.h - MARGE_BAS - h;
        if let Some(e) = eviter.filter(|e| x < e.x + e.w && e.x < x + w && y < e.y + e.h && e.y < y + h) {
            y = e.y - 8.0 - h;
        }
        Rect::new(x, y.max(chart.y + 8.0), w, h)
    }

    /// ✕ (coordonnées de la carte).
    fn fermer(w: f64) -> Rect {
        Rect::new(w - PAD_X - 26.0, PAD_HAUT - 2.0, 34.0, 34.0)
    }

    /// Clic aux coordonnées de la carte (largeur `w`).
    pub fn click(&self, text: &Text, w: f64, x: f64, y: f64) -> AvisClick {
        if Self::fermer(w).contains(x, y) {
            return AvisClick::Fermer;
        }
        let lien = self.mise(text, w).morceaux.into_iter().find_map(|m| match m {
            Morceau::Lien(r, _) => Some(r),
            _ => None,
        });
        match (lien, self.academie) {
            (Some(r), Some(id)) if r.contains(x, y) => AvisClick::Lecon(id),
            _ => AvisClick::Rien,
        }
    }

    /// La carte sur la largeur `w` ; `survol` : la souris (coordonnées de la carte).
    pub fn draw(&self, text: &Text, w: f64, survol: Option<(f64, f64)>) -> Option<Pixmap> {
        let mise = self.mise(text, w);
        let h = mise.h;
        let mut pm = crate::px::new(w.ceil() as u32, h as u32)?;
        let r = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
        // 🫧 le verre de la maquette (sans le style verre : un fond presque plein dessous, lisible sur la chart)
        if !crate::glass::on() {
            round_fill(&mut pm, r, RAYON, crate::jetons::FOND, 0.94);
        }
        round_fill(&mut pm, r, RAYON, FOND.0, FOND.1);
        round_stroke(&mut pm, r, RAYON, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let dessus = |z: Rect| survol.is_some_and(|(x, y)| z.contains(x, y));
        // en-tête : le rond Argos AI, le titre, ✕
        crate::aibtn::bouton_verre(&mut pm, Rect::new(PAD_X, PAD_HAUT, ROND, ROND), false);
        let cy = PAD_HAUT + ROND / 2.0;
        let gras = Text::graisse(Graisse::Bold);
        gras.draw(
            &mut pm,
            "Argos AI · ton trade",
            ((PAD_X + ROND + 10.0) as f32, (cy + 5.8) as f32),
            16.0,
            TITRE,
            Align::Left,
        );
        let x = Self::fermer(w);
        if dessus(x) {
            round_fill(&mut pm, x, 17.0, crate::palette::WHITE, 0.1);
        }
        let d = 5.0;
        lines(&mut pm, x.cx(), x.cy(), &[&[(-d, -d), (d, d)], &[(d, -d), (-d, d)]], DOUX, 1.6);
        let moyen = Text::graisse(Graisse::Medium);
        let couleur = self.couleur_resultat();
        for m in mise.morceaux {
            match m {
                Morceau::Resultat(y, t) => moyen.draw(&mut pm, &t, ((PAD_X + 2.0) as f32, y as f32), 13.0, couleur, Align::Left),
                Morceau::Etiquette(y, t) => text.draw_espace(&mut pm, &t, ((PAD_X + 2.0) as f32, y as f32), 11.0, DOUX, 1.1),
                Morceau::Pastille(p, t, ton) => pastille(&mut pm, &gras, p, &t, ton),
                Morceau::Ligne(y, t, size, c) => text.draw(&mut pm, &t, (PAD_X as f32, y as f32), size, c, Align::Left),
                Morceau::Lien(p, t) => lien(&mut pm, text, p, &t, dessus(p)),
                Morceau::Rappel(y, t) => text.draw(&mut pm, &t, (PAD_X as f32, y as f32), 11.5, DOUX, Align::Left),
            }
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::ecrits;

    fn partie(ton: Ton, pastille: &str, texte: &str) -> Partie {
        Partie {
            ton,
            pastille: pastille.into(),
            texte: texte.into(),
        }
    }

    /// Le trade de la maquette PC : stop éloigné à la main, perte au stop à −1,62 R.
    fn maquette_pc() -> Avis {
        Avis {
            long: true,
            base: "BTC".into(),
            r: Some("−1,62 R".into()),
            net: Money::from_cents(-161_840),
            sortie: "stop touché".into(),
            entree: partie(Ton::Mauvais, "1 sur 3 dans ton sens", "Contre toi : Supertrend et MACD (sous son signal)."),
            plan: partie(Ton::Mauvais, "Pas suivi", "Stop éloigné à la main : la perte a fait −1,62 R au lieu de −1 R."),
            lecon: "Garde le stop où ton plan l'a mis : c'est lui qui limite ta perte.".into(),
            academie: Some("0-02-savoir-dans-quoi-tu-t-embarques"),
            rappel: "Analyse affichée, pas un ordre.".into(),
        }
    }

    /// Ce que la carte écrit, dans l'ordre.
    fn ecrit(a: &Avis, w: f64) -> Vec<String> {
        let _ = ecrits();
        a.draw(&Text::default(), w, None).unwrap();
        ecrits()
    }

    #[test]
    fn la_carte_dit_ses_trois_choses_et_le_rappel() {
        let a = maquette_pc();
        assert_eq!(a.resultat(), "▲ BTC · −1,62 R · −1 618,40 $ · stop touché");
        let e = ecrit(&a, LARGEUR);
        let attendu = [
            "Argos AI · ton trade",
            "▲ BTC · −1,62 R · −1 618,40 $ · stop touché",
            "À L'ENTRÉE",
            "1 sur 3 dans ton sens",
            "Contre toi : Supertrend et MACD (sous son signal).",
            "TON PLAN",
            "Pas suivi",
            "UNE LEÇON À REVOIR",
            "🎓",
            "Leçon : Savoir dans quoi tu t'embarques",
            "Analyse affichée, pas un ordre.",
        ];
        for t in attendu {
            assert!(e.iter().any(|x| x == t), "« {t} » manque : {e:?}");
        }
        // les phrases longues passent à la ligne, jamais coupées par « … »
        assert!(e.iter().all(|x| !x.ends_with('…')), "{e:?}");
        let pos = |t: &str| e.iter().position(|x| x == t).unwrap();
        assert!(pos("À L'ENTRÉE") < pos("TON PLAN") && pos("TON PLAN") < pos("UNE LEÇON À REVOIR"));
        assert!(pos("UNE LEÇON À REVOIR") < pos("Analyse affichée, pas un ordre."), "le rappel en bas");
    }

    /// La maquette PC : 406 de large (372 de texte), 343 de haut ; sans leçon de l'Académie, la pilule du lien n'y est
    /// pas.
    #[test]
    fn hauteur_de_la_maquette() {
        let t = Text::default();
        let a = maquette_pc();
        let h = a.hauteur(&t, LARGEUR);
        assert_eq!(h, 343.0);
        let sans = Avis {
            academie: None,
            ..maquette_pc()
        };
        assert!((a.hauteur(&t, LARGEUR) - sans.hauteur(&t, LARGEUR) - 41.0).abs() < 0.5);
        assert!(!ecrit(&sans, LARGEUR).iter().any(|x| x.starts_with("Leçon")));
        // 📱 plus étroite : les phrases passent à la ligne, la carte grandit
        assert!(a.hauteur(&t, 300.0) > h);
    }

    #[test]
    fn clics_fermer_et_lecon() {
        let t = Text::default();
        let a = maquette_pc();
        let w = LARGEUR;
        let x = Avis::fermer(w);
        assert_eq!(a.click(&t, w, x.cx(), x.cy()), AvisClick::Fermer);
        let lien = a
            .mise(&t, w)
            .morceaux
            .into_iter()
            .find_map(|m| match m {
                Morceau::Lien(r, _) => Some(r),
                _ => None,
            })
            .unwrap();
        assert_eq!(a.click(&t, w, lien.cx(), lien.cy()), AvisClick::Lecon("0-02-savoir-dans-quoi-tu-t-embarques"));
        // ailleurs sur la carte : rien, mais le clic est pris (il ne passe pas à la chart)
        assert_eq!(a.click(&t, w, 60.0, 80.0), AvisClick::Rien);
    }

    /// 🖥 en bas à gauche de la chart, 406 de large ; 📱 toute la largeur moins 16 de chaque côté. Toujours au-dessus
    /// du bas de la chart.
    #[test]
    fn place_pc_et_cell() {
        let t = Text::default();
        let a = maquette_pc();
        let chart = Rect::new(52.0, 60.0, 1000.0, 700.0);
        let p = a.place(&t, chart, 1500.0, false, None);
        assert_eq!((p.x, p.w), (68.0, LARGEUR));
        assert!((p.y + p.h - (chart.y + chart.h - 12.0)).abs() < 1e-9, "{p:?}");
        let c = a.place(&t, Rect::new(0.0, 96.0, 332.0, 640.0), 390.0, true, None);
        assert_eq!((c.x, c.w), (16.0, 358.0));
        assert!(c.y + c.h <= 96.0 + 640.0 - 12.0 + 1e-9 && c.y >= 96.0, "{c:?}");
        // ⏪ la barre du replay reste visible : la carte qui la couvrirait se pose à 8 au-dessus ; à côté, rien ne bouge
        let barre = Rect::new(20.0, 600.0, 394.0, 46.0);
        let c2 = a.place(&t, Rect::new(0.0, 96.0, 332.0, 640.0), 390.0, true, Some(barre));
        assert!((c2.y + c2.h - (barre.y - 8.0)).abs() < 1e-9 && (c2.x, c2.w) == (c.x, c.w), "{c2:?}");
        let loin = Rect::new(chart.x + 600.0, p.y + 40.0, 394.0, 46.0);
        assert_eq!(a.place(&t, chart, 1500.0, false, Some(loin)), p);
    }

    /// Les sorties sans stop n'ont pas de R ; une ligne trop longue passe à la ligne sans couper « : ».
    #[test]
    fn sans_r_et_deux_points_jamais_en_debut_de_ligne() {
        let a = Avis {
            r: None,
            net: Money::from_cents(1_230),
            sortie: "fermé par le Gardien PropFirm".into(),
            ..maquette_pc()
        };
        assert_eq!(a.resultat(), "▲ BTC · +12,30 $ · fermé par le Gardien PropFirm");
        let t = Text::default();
        for w in [120.0, 160.0, 200.0, 260.0] {
            for l in couper(&t, "Stop éloigné à la main : la perte a fait −1,62 R au lieu de −1 R.", 13.0, w) {
                assert!(!l.starts_with(':') && !l.starts_with('·'), "« {l} » ({w})");
            }
        }
        assert_eq!(couper(&t, "court", 13.0, 300.0), vec!["court"]);
        assert_eq!(couper(&t, "", 13.0, 300.0), vec![String::new()]);
    }

    /// 📸 Captures de la carte pour Jee (côte à côte avec la maquette) : seulement si `ARGOS_CAPTURES` donne un
    /// dossier. La carte est posée sur un fond sombre comme la chart (le flou du verre est celui de la fenêtre).
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        let t = Text::default();
        let gagne = Avis {
            r: Some("+2,00 R".into()),
            net: Money::from_cents(193_810),
            sortie: "objectif atteint".into(),
            entree: partie(Ton::Bon, "3 sur 3 dans ton sens", "Dans ton sens : EMA 9/21, MACD et RSI 14."),
            plan: partie(Ton::Bon, "Suivi", "Objectif atteint : +2,00 R comme prévu."),
            lecon: "Plan suivi jusqu'au bout : note au journal ce qui rendait l'entrée claire.".into(),
            academie: None,
            ..maquette_pc()
        };
        // (à l'échelle 2, comme les maquettes)
        crate::px::set_scale(2.0);
        for (nom, a, w) in [("lot7-carte-pc", maquette_pc(), LARGEUR), ("lot7-carte-cell", gagne, 358.0)] {
            let pm = a.draw(&t, w, None).unwrap();
            let h = a.hauteur(&t, w);
            let mut fond = crate::px::new((w + 40.0) as u32, (h + 40.0) as u32).unwrap();
            crate::fill(&mut fond, 0.0, 0.0, w + 40.0, h + 40.0, Rgb(0x0b, 0x0e, 0x1a), 1.0);
            crate::px::put(&mut fond, 20.0, 20.0, &pm);
            fond.save_png(format!("{dir}/{nom}.png")).unwrap();
        }
        crate::px::set_scale(1.0);
    }

    /// La couleur des pastilles : vert, orange, rouge, gris.
    #[test]
    fn couleurs_des_pastilles() {
        assert_eq!(Ton::Bon.couleurs().0, HAUSSE);
        assert_eq!(Ton::Moyen.couleurs().0, Rgb(0xff, 0xb7, 0x4d));
        assert_eq!(Ton::Mauvais.couleurs().0, Rgb(0xff, 0x6b, 0x73));
        assert_eq!(Ton::Neutre.couleurs().0, DOUX);
        let a = maquette_pc();
        assert_eq!(a.couleur_resultat(), BAISSE);
        assert_eq!(
            Avis {
                net: Money::from_cents(5),
                ..maquette_pc()
            }
            .couleur_resultat(),
            HAUSSE
        );
        assert_eq!(
            Avis {
                net: Money::ZERO,
                ..maquette_pc()
            }
            .couleur_resultat(),
            TITRE
        );
    }
}
