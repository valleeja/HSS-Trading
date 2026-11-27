//! 🔎 Recherche universelle (Ctrl + K) : une palette au centre, en verre, qui filtre en direct les symboles de la
//! watchlist, les indicateurs, les leçons de l'Académie (titre et texte), des réglages et les sections de l'app. ↑ ↓ choisissent,
//! Entrée ouvre, Échap ferme. Le filtre ([`filtrer`]) ignore majuscules et accents ; tous les mots tapés doivent
//! se trouver dans la ligne, celles qui commencent par la recherche passent devant.

use crate::accueil::Section;
use crate::indicators::Ind;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// Réglages trouvables dans la palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reglage {
    StyleVerre,
    AccueilAuLancement,
    RevoirAccueil,
    Theme,
}

/// Ce qu'ouvre une ligne.
#[derive(Debug, Clone, PartialEq)]
pub enum Cible {
    /// Ligne de la watchlist (son identifiant).
    Symbole(String),
    Indicateur(Ind),
    /// Guide de l'Académie (son id).
    Lecon(String),
    Reglage(Reglage),
    Section(Section),
    Accueil,
    Raccourcis,
    /// 🧪 Volet du Simulateur démo.
    Simulateur,
    /// ⟳ Actualiser : direct reconnecté, bougies rechargées (Ctrl + R).
    Actualiser,
    /// 🛟 Support: the tickets panel (also when its button has no room in the top bar).
    Support,
    /// 🧭 📶 A panel of the column (Explorer Hyperliquid, Stats Hyperliquid).
    Outil(crate::watchlist::Tool),
}

/// Une ligne de la palette.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Genre affiché à droite (« Symbole », « Indicateur »…).
    pub genre: &'static str,
    pub nom: String,
    pub detail: String,
    pub cible: Cible,
    /// Mots cachés trouvables (texte d'une leçon, déjà en minuscules sans accents).
    pub cles: String,
}

/// Minuscules sans accents (é → e, ç → c…), pour comparer.
pub fn simple(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            c => c,
        })
        .collect()
}

/// Lignes qui correspondent à `q` (index dans `items`), les meilleures d'abord ; `q` vide = toutes, dans l'ordre.
pub fn filtrer(items: &[Item], q: &str) -> Vec<usize> {
    let q = simple(q.trim());
    // 📱 phone view: no « Raccourcis du clavier » line (there is no keyboard)
    let garder = |it: &Item| crate::actions::montrer_raccourcis() || !matches!(it.cible, Cible::Raccourcis);
    if q.is_empty() {
        return (0..items.len()).filter(|&i| garder(&items[i])).collect();
    }
    let mots: Vec<&str> = q.split_whitespace().collect();
    // 🔎 a symbol also without separators: « btcusdc » finds « BTC-USDC.P »
    let net = |s: &str| {
        s.chars()
            .filter(|c| !matches!(c, '/' | '-' | '_' | '.' | ':') && !c.is_whitespace())
            .collect::<String>()
    };
    let qn = net(&q);
    let mut v: Vec<(u8, usize)> = items
        .iter()
        .enumerate()
        .filter(|(_, it)| garder(it))
        .filter_map(|(i, it)| {
            let nom = simple(&it.nom);
            let tout = format!("{nom} {} {} {}", simple(&it.detail), simple(it.genre), it.cles);
            if !mots.iter().all(|m| tout.contains(m)) {
                return (!qn.is_empty() && net(&nom).contains(&qn)).then_some((3, i));
            }
            let rang = if nom.starts_with(&q) {
                0
            } else if nom.split(|c: char| !c.is_alphanumeric()).any(|w| w.starts_with(mots[0])) {
                1
            } else if nom.contains(&q) {
                2
            } else {
                3
            };
            Some((rang, i))
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, i)| i).collect()
}

/// Recherche vide : le petit mélange de la maquette (15 lignes, dans son ordre) plutôt que toute la liste : le premier
/// symbole, 9 leçons prises à intervalles réguliers, le premier indicateur, 2 réglages (Thème, Accueil au lancement)
/// et 2 pages (Accueil, Simulateur démo). Toute la liste revient dès qu'on tape.
pub fn suggestions(items: &[Item]) -> Vec<usize> {
    let premier = |f: &dyn Fn(&Item) -> bool| items.iter().position(f);
    let lecons: Vec<usize> = (0..items.len()).filter(|&i| matches!(items[i].cible, Cible::Lecon(_))).collect();
    let mut v: Vec<usize> = premier(&|i| matches!(i.cible, Cible::Symbole(_))).into_iter().collect();
    v.extend((0..9.min(lecons.len())).map(|k| lecons[k * lecons.len() / 9.min(lecons.len())]));
    v.extend(premier(&|i| matches!(i.cible, Cible::Indicateur(_))));
    for c in [
        Cible::Reglage(Reglage::Theme),
        Cible::Reglage(Reglage::AccueilAuLancement),
        Cible::Accueil,
        Cible::Simulateur,
    ] {
        v.extend(premier(&|i| i.cible == c));
    }
    v
}

/// Les lignes fixes : sections, indicateurs, leçons de l'Académie et réglages (les symboles viennent de la fenêtre).
pub fn items_fixes() -> Vec<Item> {
    let it = |genre, nom: &str, detail: &str, cible| Item {
        genre,
        nom: nom.into(),
        detail: detail.into(),
        cible,
        cles: String::new(),
    };
    let mut v = vec![
        it("Section", "Accueil", "bouton ⌂ de la barre du haut", Cible::Accueil),
        it("Section", "Raccourcis du clavier", "Ctrl + /", Cible::Raccourcis),
        it("Section", "Simulateur démo", "ordres démo, replay, scénarios d'examen", Cible::Simulateur),
        it("Action", "Actualiser", "Ctrl + R · direct reconnecté, bougies rechargées", Cible::Actualiser),
        it("Section", "Support", "aide et retours : problème, idée, question", Cible::Support),
        it(
            "Module",
            "Explorer Hyperliquid",
            "Marchés · une adresse : positions, ordres, trades, funding",
            Cible::Outil(crate::watchlist::Tool::Explorer),
        ),
        it(
            "Module",
            "Stats Hyperliquid",
            "Marchés · volume, open interest, funding, HIP-3",
            Cible::Outil(crate::watchlist::Tool::Stats),
        ),
    ];
    v.extend(Section::ALL.map(|s| it("Section", s.nom(), s.sous_titre(), Cible::Section(s))));
    v.extend(Ind::ALL.map(|i| it("Indicateur", i.name(), "", Cible::Indicateur(i))));
    // 🎓 les leçons : trouvées par leur titre et par leur texte
    v.extend(crate::academy::lecons().iter().map(|l| Item {
        cles: argos_academy::mots_cles(l),
        ..it(
            "Leçon",
            &l.titre,
            &format!(
                "Parcours {} · {} · {} min",
                l.parcours,
                crate::academy::PARCOURS[l.parcours as usize].nom,
                l.minutes
            ),
            Cible::Lecon(l.id.clone()),
        )
    }));
    v.extend([
        it(
            "Réglage",
            "Style verre",
            "Thème · menus et panneaux translucides",
            Cible::Reglage(Reglage::StyleVerre),
        ),
        it(
            "Réglage",
            "Accueil au lancement",
            "Ouvrir l'Accueil au démarrage",
            Cible::Reglage(Reglage::AccueilAuLancement),
        ),
        it(
            "Réglage",
            "Revoir l'accueil",
            "Les 3 écrans du premier lancement",
            Cible::Reglage(Reglage::RevoirAccueil),
        ),
        it("Réglage", "Thème", "Couleurs de l'app", Cible::Reglage(Reglage::Theme)),
    ]);
    v
}

impl Item {
    /// Sorte affichée à droite : le genre ; pour une leçon, avec son parcours (« Leçon · Bien commencer »), comme la
    /// maquette.
    pub fn sorte(&self) -> String {
        match (&self.cible, self.detail.split(" · ").nth(1)) {
            (Cible::Lecon(_), Some(p)) => format!("{} · {p}", self.genre),
            _ => self.genre.to_string(),
        }
    }

    /// Icône de la ligne (colonne de 26 de la maquette).
    pub fn icone(&self) -> &'static str {
        match &self.cible {
            Cible::Symbole(_) => "📈",
            Cible::Indicateur(_) => "🧪",
            Cible::Lecon(_) => "🎓",
            Cible::Reglage(_) => "⚙️",
            Cible::Section(s) => s.emoji(),
            Cible::Accueil => "🏠",
            Cible::Raccourcis => "⌨️",
            Cible::Simulateur => "🧪",
            Cible::Actualiser => "🔄",
            Cible::Support => "🛟",
            Cible::Outil(t) => t.emoji(),
        }
    }
}

// 🫧 Mesures de la maquette (croquis/maquette-verre, `.pal`) : largeur min(640, 94 %), 12 % du haut, coins de 20 ;
// en-tête 50 (`.pal-h`), liste au plus 52 % de la hauteur avec 6 de marge (`.pal-l`), lignes de 33,5 (`.pal-i` :
// 8 + 17,55 + 8), pied 32 (`.pal-p`). Sur un cell de 390 × 844 : 366 × 521.
/// Hauteur d'une ligne.
const LIGNE: f64 = 33.5;
/// En-tête (loupe, champ, Échap).
const TETE: f64 = 50.0;
/// Pied (« N résultats », « ↑ ↓ choisir · Entrée ouvrir »).
const PIED: f64 = 32.0;
/// Marge intérieure de la liste.
const MARGE: f64 = 6.0;
/// Coins de la boîte.
const RAYON: f64 = 20.0;

/// La palette ouverte.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    pub q: String,
    pub items: Vec<Item>,
    /// Résultats (index dans `items`) et ligne choisie (rang dans les résultats).
    pub res: Vec<usize>,
    pub sel: usize,
    debut: usize,
    /// Lignes entières visibles (mises à jour au dessin, selon la hauteur de la fenêtre) : ↑ ↓ gardent la ligne choisie
    /// dedans.
    vues: std::cell::Cell<usize>,
}

impl Palette {
    /// Palette avec les symboles donnés (identifiant, nom affiché) en tête, puis les lignes fixes.
    pub fn new(symboles: &[(String, String)]) -> Self {
        let mut items: Vec<Item> = symboles
            .iter()
            .map(|(id, nom)| Item {
                genre: "Symbole",
                nom: nom.clone(),
                detail: String::new(),
                cible: Cible::Symbole(id.clone()),
                cles: String::new(),
            })
            .collect();
        items.extend(items_fixes());
        let res = suggestions(&items);
        Self {
            q: String::new(),
            items,
            res,
            sel: 0,
            debut: 0,
            vues: std::cell::Cell::new(9),
        }
    }

    fn refiltrer(&mut self) {
        self.res = if self.q.trim().is_empty() {
            suggestions(&self.items)
        } else {
            filtrer(&self.items, &self.q)
        };
        (self.sel, self.debut) = (0, 0);
    }

    pub fn tape(&mut self, t: &str) {
        self.q.extend(t.chars().filter(|c| !c.is_control()));
        self.refiltrer();
    }

    pub fn efface(&mut self) {
        self.q.pop();
        self.refiltrer();
    }

    fn vues(&self) -> usize {
        self.vues.get().max(1)
    }

    /// ↑ (−1) / ↓ (+1), en restant dans la liste ; la fenêtre visible suit.
    pub fn bouge(&mut self, d: i32) {
        if self.res.is_empty() {
            return;
        }
        self.sel = (self.sel as i64 + d as i64).clamp(0, self.res.len() as i64 - 1) as usize;
        if self.sel < self.debut {
            self.debut = self.sel;
        } else if self.sel >= self.debut + self.vues() {
            self.debut = self.sel + 1 - self.vues();
        }
    }

    /// Entrée : la cible de la ligne choisie.
    pub fn choisie(&self) -> Option<Cible> {
        self.res.get(self.sel).map(|&i| self.items[i].cible.clone())
    }

    /// Haut et largeur de la boîte dans une fenêtre `w` × `h`, et la hauteur la plus grande de la liste.
    fn cadre(w: f64, h: f64) -> (f64, f64, f64, f64) {
        let bw = (w * 0.94).min(640.0).floor();
        let y = (h * 0.12).round();
        // la liste prend au plus 52 % de la hauteur, et la boîte reste dans la fenêtre (cell couché)
        let liste = (h * 0.52).min(h - y - 8.0 - TETE - PIED).max(MARGE * 2.0 + LIGNE);
        ((w - bw) / 2.0, y, bw, liste)
    }

    /// La boîte la plus grande (liste pleine), centrée en haut d'une fenêtre `w` × `h`.
    pub fn rect(w: f64, h: f64) -> Rect {
        let (x, y, bw, liste) = Self::cadre(w, h);
        Rect::new(x, y, bw, TETE + liste + PIED)
    }

    /// Hauteur de la liste : elle suit le nombre de résultats (une ligne « Rien trouvé » s'il n'y en a pas).
    fn liste_h(&self, w: f64, h: f64) -> f64 {
        let (.., max) = Self::cadre(w, h);
        (MARGE * 2.0 + self.res.len().max(1) as f64 * LIGNE).min(max)
    }

    /// La boîte telle qu'elle est dessinée (plus courte s'il y a peu de résultats).
    pub fn boite(&self, w: f64, h: f64) -> Rect {
        let r = Self::rect(w, h);
        Rect::new(r.x, r.y, r.w, TETE + self.liste_h(w, h) + PIED)
    }

    /// 📱 Le champ où on tape (haut de la boîte) : y toucher ouvre le clavier du cell.
    pub fn champ(w: f64, h: f64) -> Rect {
        let r = Self::rect(w, h);
        Rect::new(r.x, r.y, r.w, TETE)
    }

    /// Clic dans la fenêtre : `Some(cible)` sur une ligne ; `None` ailleurs (à côté de la boîte = fermer, à la fenêtre).
    pub fn clic(&mut self, w: f64, h: f64, x: f64, y: f64) -> Option<Cible> {
        let r = self.boite(w, h);
        let haut = r.y + TETE + MARGE;
        if !r.contains(x, y) || y < haut || y >= r.y + TETE + self.liste_h(w, h) - MARGE {
            return None;
        }
        let k = self.debut + ((y - haut) / LIGNE).floor() as usize;
        if k < self.res.len() {
            self.sel = k;
            return self.choisie();
        }
        None
    }

    /// Molette : la liste défile.
    pub fn molette(&mut self, d: i32) {
        let vues = self.vues();
        let max = self.res.len().saturating_sub(vues);
        self.debut = (self.debut as i64 + d as i64).clamp(0, max as i64) as usize;
        self.sel = self
            .sel
            .clamp(self.debut, (self.debut + vues).min(self.res.len()).saturating_sub(1).max(self.debut));
    }

    /// Dessine la boîte (sa place : [`Palette::boite`]).
    pub fn draw(&self, text: &Text, w: f64, h: f64, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.boite(w, h);
        let lh = self.liste_h(w, h);
        self.vues.set((((lh - MARGE * 2.0) / LIGNE).floor() as usize).max(1));
        let mut pm = crate::px::new(r.w.max(1.0).round() as u32, r.h.max(1.0).round() as u32)?;
        let b = Rect::new(0.0, 0.0, r.w, r.h);
        // 🫧 verre de la maquette : fond, verre, liseré clair, coins de 20
        use crate::jetons as j;
        // sans le style verre, un fond presque plein ; avec, le verre de la maquette laisse voir la chart floutée
        if !crate::glass::on() {
            round_fill(&mut pm, b, RAYON, j::FOND, 0.94);
        }
        round_fill(&mut pm, b, RAYON, j::VERRE.0, j::VERRE.1);
        round_stroke(&mut pm, Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0), RAYON, j::sur_fond(j::LISERE), 1.0);
        // en-tête : 🔎, le champ (16), Échap dans sa touche
        let mi = (TETE / 2.0) as f32;
        text.draw(&mut pm, "🔎", (14.0, mi + 5.0), 13.0, j::TEXTE, Align::Left);
        let kw = text.width("Échap", 11.0) as f64 + 12.0;
        let kbd = Rect::new(r.w - 14.0 - kw, TETE / 2.0 - 10.0, kw, 20.0);
        // (📱 phone view: no « Échap » key cap)
        if crate::actions::montrer_raccourcis() {
            round_fill(&mut pm, kbd, 6.0, j::BLANC, 0.1);
            round_stroke(&mut pm, kbd, 6.0, j::sur_fond(j::LISERE), 1.0);
            text.draw(&mut pm, "Échap", (kbd.cx() as f32, mi + 4.0), 11.0, j::TEXTE, Align::Center);
        }
        let (q, c) = if self.q.is_empty() {
            ("Symbole, leçon, indicateur, réglage, section…".to_string(), j::DIM)
        } else {
            (format!("{}│", self.q), j::TEXTE)
        };
        text.draw(&mut pm, &text.fit(&q, 16.0, kbd.x - 8.0 - 39.0), (39.0, mi + 5.5), 16.0, c, Align::Left);
        crate::fill(&mut pm, 0.0, TETE - 1.0, r.w, 1.0, j::sur_fond(j::LISERE), 1.0);
        // la liste, dans sa propre image : la dernière ligne est coupée net au bord, comme dans la maquette
        let mut lp = crate::px::new(r.w.round() as u32, lh.round() as u32)?;
        if self.res.is_empty() {
            let t = text.fit(&format!("Rien trouvé pour « {} ».", self.q.trim()), 11.0, r.w - 36.0);
            text.draw(&mut lp, &t, (18.0, (MARGE + LIGNE / 2.0 + 4.0) as f32), 11.0, j::DIM, Align::Left);
        }
        let (mx, my) = (mouse.0 - r.x, mouse.1 - r.y - TETE);
        let large = r.w >= 560.0;
        let moyen = Text::graisse(crate::text::Graisse::Medium);
        let pleines = ((lh - MARGE * 2.0) / LIGNE).ceil() as usize;
        for (k, &i) in self.res.iter().enumerate().skip(self.debut).take(pleines) {
            let l = Rect::new(MARGE, MARGE + (k - self.debut) as f64 * LIGNE, r.w - MARGE * 2.0, LIGNE);
            if k == self.sel {
                round_fill(&mut lp, l, j::pilule(l), j::ACCENT, 0.28);
            } else if l.contains(mx, my) && my < lh {
                round_fill(&mut lp, l, j::pilule(l), j::VERRE2.0, j::VERRE2.1);
            }
            let it = &self.items[i];
            let base = (l.cy() + 4.5) as f32;
            // icône (colonne de 26), titre (13, 500), sorte à droite (11, pâle)
            text.draw(&mut lp, it.icone(), ((l.x + 10.0 + 13.0) as f32, base + 0.5), 14.0, j::TEXTE, Align::Center);
            let sorte = it.sorte();
            let gw = text.width(&sorte, 11.0) as f64;
            let x0 = l.x + 10.0 + 26.0 + 8.0;
            let place = l.x + l.w - 10.0 - gw - 8.0 - x0;
            let nom = moyen.fit(&it.nom, 13.0, place);
            moyen.draw(&mut lp, &nom, (x0 as f32, base), 13.0, j::TEXTE, Align::Left);
            // (📱 phone view: « Ctrl + R · direct reconnecté » → « direct reconnecté »)
            let detail = crate::actions::sans_touches(&it.detail);
            if large && !detail.is_empty() {
                let nw = moyen.width(&nom, 13.0) as f64;
                let room = place - nw - 12.0;
                if room > 40.0 {
                    let d = text.fit(&detail, 11.0, room);
                    text.draw(&mut lp, &d, ((x0 + nw + 12.0) as f32, base), 11.0, j::DIM, Align::Left);
                }
            }
            text.draw(&mut lp, &sorte, ((l.x + l.w - 10.0) as f32, base), 11.0, j::DIM, Align::Right);
        }
        crate::px::put(&mut pm, 0.0, TETE, &lp);
        // pied : « N résultats » à gauche, les touches à droite
        let yp = r.h - PIED;
        crate::fill(&mut pm, 0.0, yp, r.w, 1.0, j::sur_fond(j::LISERE), 1.0);
        let n = self.res.len();
        let bp = (yp + PIED / 2.0 + 4.0) as f32;
        let pied = format!("{n} résultat{}", if n > 1 { "s" } else { "" });
        text.draw(&mut pm, &pied, (14.0, bp), 11.0, j::DIM, Align::Left);
        // (📱 phone view: no « ↑ ↓ choisir · Entrée ouvrir », keyboard only)
        if crate::actions::montrer_raccourcis() {
            text.draw(&mut pm, "↑ ↓ choisir · Entrée ouvrir", ((r.w - 14.0) as f32, bp), 11.0, j::DIM, Align::Right);
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pal() -> Palette {
        Palette::new(&[("hl:BTC".into(), "BTC-USDC.P".into()), ("oanda:XAU_USD".into(), "XAU/USD".into())])
    }

    #[test]
    fn filtre_sans_accents_ni_majuscules() {
        assert_eq!(simple("Académie Réglé ÇA"), "academie regle ca");
        let p = pal();
        let noms = |q: &str| filtrer(&p.items, q).into_iter().map(|i| p.items[i].nom.clone()).collect::<Vec<_>>();
        assert_eq!(noms("xau")[0], "XAU/USD");
        assert_eq!(noms("btc")[0], "BTC-USDC.P");
        assert!(noms("academie").contains(&"Académie".to_string()), "sans accent");
        assert!(noms("ACCUEIL").contains(&"Accueil".to_string()), "majuscules");
        assert!(noms("zzzz").is_empty());
        // tous les mots doivent y être
        assert!(noms("style verre").contains(&"Style verre".to_string()));
        assert!(!noms("style zzz").contains(&"Style verre".to_string()));
        // vide : tout, dans l'ordre (symboles d'abord)
        assert_eq!(filtrer(&p.items, "  ").len(), p.items.len());
        assert_eq!(p.items[0].cible, Cible::Symbole("hl:BTC".into()));
    }

    /// 🔎 « Où est… ? » finds a symbol without its separators (« btcusdc », « BTC/USDC » → « BTC-USDC.P »); what was
    /// found before still is, in the same order.
    #[test]
    fn symbole_sans_separateurs() {
        let p = pal();
        let noms = |q: &str| filtrer(&p.items, q).into_iter().map(|i| p.items[i].nom.clone()).collect::<Vec<_>>();
        for q in ["btcusdc", "BTC/USDC", "btc usdc", "btc_usdc"] {
            assert!(noms(q).contains(&"BTC-USDC.P".to_string()), "{q}");
        }
        assert!(noms("xauusd").contains(&"XAU/USD".to_string()));
        assert_eq!(noms("btc")[0], "BTC-USDC.P");
        assert_eq!(noms("xau")[0], "XAU/USD");
        assert!(noms("academie").contains(&"Académie".to_string()));
        assert!(noms("zzzz").is_empty());
    }

    #[test]
    fn le_debut_du_nom_passe_devant() {
        let it = |n: &str| Item {
            genre: "Indicateur",
            nom: n.into(),
            detail: String::new(),
            cible: Cible::Accueil,
            cles: String::new(),
        };
        let items = vec![it("Moyenne VWAP"), it("VWAP ancré"), it("Bandes de VWAP"), it("Session")];
        let v = filtrer(&items, "vwap");
        assert_eq!(v[0], 1, "commence par « vwap »");
        assert_eq!(v.len(), 3);
    }

    #[test]
    fn toutes_les_sources_sont_la() {
        let p = pal();
        // 🎓 une leçon se trouve par son titre et par un mot de son texte
        let lecon = |q: &str| filtrer(&p.items, q).into_iter().map(|i| p.items[i].nom.clone()).find(|_| true);
        assert_eq!(lecon("break of structure").as_deref(), Some("BOS : Break of Structure"));
        assert!(filtrer(&p.items, "jobless claims")
            .iter()
            .any(|&i| p.items[i].cible == Cible::Lecon(crate::academy::NEWS_GUIDE.into())));
        for g in ["Symbole", "Indicateur", "Leçon", "Réglage", "Section"] {
            assert!(p.items.iter().any(|i| i.genre == g), "{g}");
        }
        assert_eq!(
            p.items.iter().filter(|i| i.genre == "Section").count(),
            14,
            "Accueil, Raccourcis, Simulateur démo, Support et les 10 cartes"
        );
        assert!(filtrer(&p.items, "simulateur").iter().any(|&i| p.items[i].cible == Cible::Simulateur));
        assert!(filtrer(&p.items, "support").iter().any(|&i| p.items[i].cible == Cible::Support));
    }

    #[test]
    fn mesures_de_la_maquette_sur_le_cell() {
        // cell 390 × 844 : 366 × 521, en haut à 12 %, comme `.pal` de la maquette
        let mut p = pal();
        let r = p.boite(390.0, 844.0);
        assert_eq!((r.w, r.h.round()), (366.0, 521.0));
        assert_eq!((r.x, r.y), (12.0, 101.0));
        let (pm, d) = p.draw(&Text::default(), 390.0, 844.0, (0.0, 0.0)).unwrap();
        assert_eq!(d, r);
        assert_eq!((pm.width(), pm.height()), (366, 521));
        assert_eq!(p.vues(), 12, "12 lignes entières, la 13e coupée");
        // peu de résultats : la boîte rapetisse, toucher dessous ferme
        p.tape("xau/usd");
        let b = p.boite(390.0, 844.0);
        let n = p.res.len();
        assert!((1..12).contains(&n), "{n}");
        assert_eq!(b.h, TETE + MARGE * 2.0 + n as f64 * LIGNE + PIED);
        assert_eq!(p.clic(390.0, 844.0, 100.0, b.y + b.h + 20.0), None);
        assert_eq!(
            p.clic(390.0, 844.0, 100.0, b.y + TETE + MARGE + 10.0),
            Some(Cible::Symbole("oanda:XAU_USD".into()))
        );
        // chaque ligne a son icône
        assert!(p.items.iter().all(|i| crate::emoji::at_start(i.icone()).is_some()));
        // la sorte d'une leçon dit son parcours, comme la maquette (« Leçon · Bien commencer »)
        let l = p.items.iter().find(|i| i.genre == "Leçon").unwrap();
        assert!(l.sorte().starts_with("Leçon · ") && l.sorte().len() > "Leçon · ".len(), "{}", l.sorte());
        assert_eq!(p.items[0].sorte(), "Symbole");
    }

    #[test]
    fn recherche_vide_le_melange_de_la_maquette() {
        let mut p = pal();
        let sortes: Vec<&str> = p.res.iter().map(|&i| p.items[i].genre).collect();
        let mut attendu = vec!["Symbole"];
        attendu.extend(["Leçon"; 9]);
        attendu.extend(["Indicateur", "Réglage", "Réglage", "Section", "Section"]);
        assert_eq!(sortes, attendu, "15 lignes dans l'ordre de la maquette");
        assert_eq!(p.choisie(), Some(Cible::Symbole("hl:BTC".into())));
        let lecons: std::collections::HashSet<usize> = p.res[1..10].iter().copied().collect();
        assert_eq!(lecons.len(), 9, "9 leçons différentes");
        // on tape : toute la liste ; on efface tout : le mélange revient
        p.tape("e");
        assert!(p.res.len() > 15);
        p.efface();
        assert_eq!(p.res.len(), 15);
    }

    #[test]
    fn fleches_entree_et_clic() {
        let mut p = pal();
        p.tape("xa");
        p.tape("u");
        assert_eq!(p.choisie(), Some(Cible::Symbole("oanda:XAU_USD".into())));
        p.efface();
        p.efface();
        p.efface();
        assert_eq!(p.q, "");
        p.bouge(1);
        assert_eq!(p.sel, 1);
        p.bouge(-5);
        assert_eq!(p.sel, 0, "reste en haut");
        for _ in 0..30 {
            p.bouge(1);
        }
        assert!(p.debut > 0 && p.sel >= p.debut && p.sel < p.debut + p.vues(), "la fenêtre suit");
        let (w, h) = (1280.0, 800.0);
        let r = Palette::rect(w, h);
        let mut p = pal();
        p.tape("xau");
        assert_eq!(p.clic(w, h, r.x + 50.0, r.y + TETE + LIGNE * 0.5), Some(Cible::Symbole("oanda:XAU_USD".into())));
        assert_eq!(p.clic(w, h, 5.0, 5.0), None, "à côté");
        assert!(p.draw(&Text::default(), w, h, (0.0, 0.0)).is_some());
        p.tape("rien ne correspond");
        assert_eq!(p.choisie(), None);
        assert!(p.draw(&Text::default(), 390.0, 700.0, (0.0, 0.0)).is_some());
    }
}
