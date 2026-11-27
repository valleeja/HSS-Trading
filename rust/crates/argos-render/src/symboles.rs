//! 🔎 Volet « Symboles » du PC, d'après la maquette (croquis/maquette-verre, `#symboles`) : en-tête et ✕, la recherche
//! « Symbole, nom… », les onglets en pilules (★ Favoris, Récents, Crypto perp, Crypto spot, Forex), la fiche du symbole
//! ouvert, la liste (☆, nom, prix, variation), les récents en pilules et « ⋯ Toutes les options » (la watchlist
//! d'avant, avec ses listes, ses sections et ＋). Carte de verre arrondie (rayon du volet), pilules à bouts ronds,
//! sans séparateurs.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

const X: f64 = 12.0;
const LIGNE: f64 = 30.0;
const PUCE_H: f64 = 26.0;
const PUCE_PAD: f64 = 10.0;
const PUCE_GAP: f64 = 6.0;
const PUCE_FONT: f32 = 12.5;

/// Les onglets, dans l'ordre de la maquette.
pub const ONGLETS: [&str; 5] = ["★ Favoris", "Récents", "Crypto perp", "Crypto spot", "Forex"];
/// Onglet montré à l'ouverture : la watchlist (Crypto perp).
pub const ONGLET_DEPART: usize = 2;

/// Un symbole de la liste.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Ligne {
    /// Nom pour l'API (« BTC »).
    pub coin: String,
    /// Nom affiché (« BTC-USDC »).
    pub label: String,
    /// Prix tel que l'exchange l'écrit (vide : pas encore reçu).
    pub prix: String,
    /// Variation sur 24 h, en %.
    pub chg: Option<f64>,
    /// ★ dans les favoris (drapeau de la watchlist).
    pub fav: bool,
}

/// Ce que le volet montre (vient du moteur).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Donnees {
    /// Fiche du symbole ouvert : nom, source, détail (prix, 24 h…).
    pub titre: String,
    pub source: String,
    pub detail: String,
    /// Exchange of the watchlist shown (header on the right of « WATCHLIST · n »); empty when it mixes exchanges.
    pub bourse: String,
    /// La watchlist ouverte.
    pub lignes: Vec<Ligne>,
    /// Les symboles ouverts récemment (le plus récent d'abord).
    pub recents: Vec<Ligne>,
}

/// Ce que fait un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum SymClick {
    None,
    Changed,
    Fermer,
    /// Ouvrir ce symbole (son `coin`).
    Ouvrir(String),
    /// ☆ : favori (drapeau) de ce symbole.
    Favori(String),
    /// ⋯ Toutes les options : la watchlist d'avant.
    Options,
    Bientot(&'static str),
}

/// Une chose posée dans le volet.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Fermer,
    Champ,
    Onglet(usize),
    Carte,
    Section(String),
    Ligne(Ligne),
    Vide(&'static str),
    Recent(Ligne),
    Options,
}

/// État du volet.
#[derive(Debug, Clone, PartialEq)]
pub struct Symboles {
    pub onglet: usize,
    /// `Some` : la recherche reçoit le clavier.
    pub recherche: Option<String>,
    pub defile: f64,
}

impl Default for Symboles {
    fn default() -> Self {
        Self {
            onglet: ONGLET_DEPART,
            recherche: None,
            defile: 0.0,
        }
    }
}

fn puce_w(text: &Text, t: &str) -> f64 {
    text.width(t, PUCE_FONT) as f64 + 2.0 * PUCE_PAD
}

/// Pilules posées en rangées à partir de `y` (largeur `w`) ; renvoie leurs rectangles et le bas de la dernière rangée.
fn rangees(text: &Text, noms: &[String], y: f64, w: f64) -> (Vec<Rect>, f64) {
    let (mut x, mut y) = (X, y);
    let out: Vec<Rect> = noms
        .iter()
        .map(|n| {
            let pw = puce_w(text, n).min(w - 2.0 * X);
            if x > X && x + pw > w - X {
                (x, y) = (X, y + PUCE_H + PUCE_GAP);
            }
            let r = Rect::new(x, y, pw, PUCE_H);
            x += pw + PUCE_GAP;
            r
        })
        .collect();
    let bas = out.last().map_or(y, |r| r.y + r.h);
    (out, bas)
}

impl Symboles {
    fn q(&self) -> String {
        self.recherche.as_deref().map(|q| crate::i18n::plain(q.trim())).unwrap_or_default()
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

    /// Haut de la partie qui défile (sous les onglets), pour une largeur `w`.
    fn haut_liste(&self, text: &Text, w: f64) -> f64 {
        let noms: Vec<String> = ONGLETS.iter().map(|s| s.to_string()).collect();
        rangees(text, &noms, 96.0, w).1 + 10.0
    }

    /// Les lignes de la liste selon l'onglet et la recherche, avec le titre de leur section.
    fn liste(&self, d: &Donnees) -> (String, Vec<Ligne>, Option<&'static str>) {
        let q = self.q();
        if !q.is_empty() {
            let l: Vec<Ligne> = d
                .lignes
                .iter()
                .filter(|l| crate::i18n::plain(&l.label).contains(&q) || crate::i18n::plain(&l.coin).contains(&q))
                .cloned()
                .collect();
            return (
                format!("RECHERCHE · {}", l.len()),
                l,
                Some("Aucun symbole de la watchlist : ⋯ Toutes les options pour en ajouter"),
            );
        }
        match self.onglet {
            0 => {
                let l: Vec<Ligne> = d.lignes.iter().filter(|l| l.fav).cloned().collect();
                (format!("FAVORIS · {}", l.len()), l, Some("Aucun favori : touche ☆ devant un symbole"))
            }
            1 => (
                format!("RÉCENTS · {}", d.recents.len()),
                d.recents.clone(),
                Some("Aucun symbole ouvert récemment"),
            ),
            2 => (
                format!("WATCHLIST · {}", d.lignes.len()),
                d.lignes.clone(),
                Some("Watchlist vide : ⋯ Toutes les options"),
            ),
            3 => ("CRYPTO SPOT".into(), vec![], Some("Crypto spot : bientôt")),
            _ => ("FOREX".into(), vec![], Some("Forex : bientôt")),
        }
    }

    /// Tout ce qui est posé, coordonnées du volet `w` (la liste décalée par le défilement).
    fn places(&self, text: &Text, w: f64, d: &Donnees) -> Vec<(Item, Rect)> {
        let mut out = vec![
            (Item::Fermer, Rect::new(w - 44.0, 12.0, 30.0, 30.0)),
            (Item::Champ, Rect::new(X, 52.0, w - 2.0 * X, 34.0)),
        ];
        let noms: Vec<String> = ONGLETS.iter().map(|s| s.to_string()).collect();
        for (k, r) in rangees(text, &noms, 96.0, w).0.into_iter().enumerate() {
            out.push((Item::Onglet(k), r));
        }
        let mut y = self.haut_liste(text, w) - self.defile;
        if !d.titre.is_empty() {
            out.push((Item::Carte, Rect::new(X, y, w - 2.0 * X, 56.0)));
            y += 56.0 + 12.0;
        }
        let (titre, lignes, vide) = self.liste(d);
        out.push((Item::Section(titre), Rect::new(X + 2.0, y, w - 2.0 * X, 22.0)));
        y += 22.0;
        if lignes.is_empty() {
            if let Some(v) = vide {
                out.push((Item::Vide(v), Rect::new(X, y, w - 2.0 * X, LIGNE)));
                y += LIGNE;
            }
        }
        for l in lignes {
            out.push((Item::Ligne(l), Rect::new(X - 4.0, y, w - 2.0 * X + 8.0, LIGNE)));
            y += LIGNE;
        }
        // récents en pilules (sauf sur l'onglet Récents, qui les montre déjà en liste)
        if self.onglet != 1 && self.q().is_empty() && !d.recents.is_empty() {
            y += 12.0;
            out.push((Item::Section("RÉCENTS".into()), Rect::new(X + 2.0, y, w - 2.0 * X, 22.0)));
            y += 24.0;
            let noms: Vec<String> = d.recents.iter().map(|l| l.label.clone()).collect();
            let (rs, bas) = rangees(text, &noms, y, w);
            for (l, r) in d.recents.iter().zip(rs) {
                out.push((Item::Recent(l.clone()), r));
            }
            y = bas;
        }
        y += 14.0;
        out.push((Item::Options, Rect::new(X, y, 180.0, 20.0)));
        out
    }

    fn haut_contenu(&self, text: &Text, w: f64, d: &Donnees) -> f64 {
        self.places(text, w, d).last().map_or(0.0, |(_, r)| r.y + r.h + self.defile + 16.0)
    }

    /// Molette (`dy` > 0 : vers le bas) dans un volet `w` × `h`.
    pub fn wheel(&mut self, dy: f64, w: f64, h: f64, d: &Donnees) {
        let t = Text::default();
        let max = (self.haut_contenu(&t, w, d) - h).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// Clic (coordonnées du volet `w` × `h`).
    pub fn click(&mut self, x: f64, y: f64, w: f64, h: f64, d: &Donnees) -> SymClick {
        let t = Text::default();
        let haut = self.haut_liste(&t, w);
        for (it, r) in self.places(&t, w, d) {
            if !r.contains(x, y) {
                continue;
            }
            // la liste qui défile : seulement sous les onglets et dans le volet
            let fixe = matches!(it, Item::Fermer | Item::Champ | Item::Onglet(_));
            if !fixe && (y < haut || y > h) {
                continue;
            }
            return match it {
                Item::Fermer => SymClick::Fermer,
                Item::Champ => {
                    self.recherche.get_or_insert_with(String::new);
                    SymClick::Changed
                }
                Item::Onglet(k) => {
                    (self.onglet, self.defile, self.recherche) = (k, 0.0, None);
                    SymClick::Changed
                }
                Item::Ligne(l) if x < r.x + 30.0 => SymClick::Favori(l.coin),
                Item::Ligne(l) | Item::Recent(l) => SymClick::Ouvrir(l.coin),
                Item::Options => SymClick::Options,
                Item::Vide(v) if v.ends_with("bientôt") => SymClick::Bientot(v),
                Item::Vide(v) if v.contains("Toutes les options") => SymClick::Options,
                _ => SymClick::None,
            };
        }
        SymClick::None
    }

    /// Dessin du volet (`w` × `h`) : carte de verre arrondie et liseré ; `mouse` en coordonnées du volet.
    pub fn draw(&self, text: &Text, w: f64, h: f64, d: &Donnees, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        use crate::jetons as j;
        let mut pm = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        let carte = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
        round_fill(&mut pm, carte, j::R_VOLET, j::sur_fond(j::VERRE), 1.0);
        round_stroke(&mut pm, carte, j::R_VOLET, j::sur_fond(j::LISERE), 1.0);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let gras = Text::graisse(Graisse::Bold);
        gras.draw(&mut pm, "Symboles", (X as f32 + 4.0, 33.0), 16.0, j::TEXTE, Align::Left);
        let haut = self.haut_liste(text, w);
        let (vert, rouge) = (crate::palette::up(), crate::palette::down());
        for (it, r) in self.places(text, w, d) {
            let fixe = matches!(it, Item::Fermer | Item::Champ | Item::Onglet(_));
            if !fixe && (r.y + r.h < haut || r.y > h) {
                continue;
            }
            match &it {
                Item::Fermer => {
                    if survol(&r) {
                        round_fill(&mut pm, r, r.h / 2.0, j::VERRE2.0, j::VERRE2.1);
                    }
                    text.draw(&mut pm, "✕", (r.cx() as f32, (r.cy() + 5.0) as f32), 14.0, j::DIM, Align::Center);
                }
                Item::Champ => {
                    let c = j::CHAMP;
                    round_fill(&mut pm, r, r.h / 2.0, c.0, if survol(&r) { c.1 + 0.04 } else { c.1 });
                    round_stroke(&mut pm, r, r.h / 2.0, j::sur_fond(j::LISERE), 1.0);
                    crate::phosphor::draw(&mut pm, "magnifying-glass", r.x + 18.0, r.cy(), 15, j::DIM);
                    let (t, col) = match &self.recherche {
                        Some(q) => (format!("{q}|"), j::TEXTE),
                        None => ("Symbole, nom… (ex. BTC, XAU, NVDA)".to_string(), j::DIM),
                    };
                    let t = text.fit(&t, 13.0, r.w - 46.0);
                    text.draw(&mut pm, &t, ((r.x + 34.0) as f32, (r.cy() + 4.5) as f32), 13.0, col, Align::Left);
                }
                Item::Onglet(k) => {
                    crate::shapes::pilule(&mut pm, r, *k == self.onglet && self.recherche.is_none(), survol(&r));
                    text.draw(&mut pm, ONGLETS[*k], (r.cx() as f32, (r.cy() + 4.5) as f32), PUCE_FONT, j::TEXTE, Align::Center);
                }
                Item::Carte => {
                    round_fill(&mut pm, r, j::R_TUILE, j::VERRE2.0, 0.09);
                    round_stroke(&mut pm, r, j::R_TUILE, j::sur_fond(j::LISERE), 1.0);
                    let tw = gras.width(&d.titre, 13.5) as f64;
                    gras.draw(&mut pm, &d.titre, ((r.x + 12.0) as f32, (r.y + 23.0) as f32), 13.5, j::TEXTE, Align::Left);
                    let s = text.fit(&d.source, 12.0, r.w - tw - 34.0);
                    text.draw(&mut pm, &s, ((r.x + 20.0 + tw) as f32, (r.y + 23.0) as f32), 12.0, j::DIM, Align::Left);
                    let det = text.fit(&d.detail, 12.0, r.w - 24.0);
                    text.draw(&mut pm, &det, ((r.x + 12.0) as f32, (r.y + 43.0) as f32), 12.0, j::DIM, Align::Left);
                }
                Item::Section(t) => {
                    text.draw(&mut pm, t, (r.x as f32, (r.y + 15.0) as f32), 11.0, j::DIM, Align::Left);
                    if t.starts_with("WATCHLIST") && !d.bourse.is_empty() {
                        let s = d.bourse.to_uppercase();
                        text.draw(&mut pm, &s, ((r.x + r.w - 4.0) as f32, (r.y + 15.0) as f32), 11.0, j::DIM, Align::Right);
                    }
                }
                Item::Vide(v) => {
                    text.draw(
                        &mut pm,
                        &text.fit(v, 12.5, r.w - 8.0),
                        ((r.x + 4.0) as f32, (r.y + 19.0) as f32),
                        12.5,
                        j::DIM,
                        Align::Left,
                    );
                }
                Item::Ligne(l) => {
                    if survol(&r) {
                        round_fill(&mut pm, r, r.h / 2.0, j::BLANC, 0.06);
                    }
                    let y = (r.y + 20.0) as f32;
                    let etoile = if l.fav { "★" } else { "☆" };
                    text.draw(
                        &mut pm,
                        etoile,
                        ((r.x + 14.0) as f32, y),
                        13.0,
                        if l.fav { Rgb(0xf7, 0xa5, 0x31) } else { j::DIM },
                        Align::Center,
                    );
                    let (xc, xp) = (r.x + r.w - 10.0, r.x + r.w - 82.0);
                    let lab = text.fit(&l.label, 13.0, xp - 70.0 - (r.x + 30.0));
                    text.draw(&mut pm, &lab, ((r.x + 30.0) as f32, y), 13.0, j::TEXTE, Align::Left);
                    text.draw(&mut pm, &l.prix, (xp as f32, y), 13.0, j::TEXTE, Align::Right);
                    if let Some(c) = l.chg {
                        let s = format!("{}{} %", if c > 0.0 { "+" } else { "" }, crate::viewport::format_number(c, 2));
                        text.draw(&mut pm, &s, (xc as f32, y), 12.5, if c >= 0.0 { vert } else { rouge }, Align::Right);
                    }
                }
                Item::Recent(l) => {
                    crate::shapes::pilule(&mut pm, r, false, survol(&r));
                    let t = text.fit(&l.label, PUCE_FONT, r.w - 2.0 * PUCE_PAD);
                    text.draw(&mut pm, &t, (r.cx() as f32, (r.cy() + 4.5) as f32), PUCE_FONT, j::TEXTE, Align::Center);
                }
                Item::Options => {
                    let c = if survol(&r) { j::TEXTE } else { j::DIM };
                    text.draw(&mut pm, "▸ ⋯ Toutes les options", (r.x as f32, (r.y + 14.0) as f32), 13.0, c, Align::Left);
                }
            }
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn donnees() -> Donnees {
        let l = |coin: &str, prix: &str, chg: f64, fav: bool| Ligne {
            coin: coin.into(),
            label: format!("{coin}-USDC"),
            prix: prix.into(),
            chg: Some(chg),
            fav,
        };
        Donnees {
            titre: "BTC-USDC.P".into(),
            source: "Hyperliquid".into(),
            detail: "Prix 75 853 · 24 h −0,18 %".into(),
            bourse: "Hyperliquid".into(),
            lignes: vec![l("BTC", "75 853", -0.18, true), l("ETH", "2 236", 0.41, false), l("SOL", "89,17", 1.02, false)],
            recents: vec![l("ETH", "2 236", 0.41, false)],
        }
    }

    /// 🔎 Le volet Symboles : onglets en pilules qui tiennent dans le volet, la watchlist à l'ouverture, Favoris ne
    /// montre que les ★, la recherche filtre, un symbole s'ouvre, ☆ change le favori, un récent s'ouvre, ⋯ Toutes les
    /// options ouvre la watchlist d'avant, ✕ ferme ; coins arrondis.
    #[test]
    fn volet_symboles() {
        let t = Text::default();
        let d = donnees();
        let (w, h) = (360.0, 800.0);
        let mut s = Symboles::default();
        let p = s.places(&t, w, &d);
        let onglets: Vec<Rect> = p.iter().filter(|(i, _)| matches!(i, Item::Onglet(_))).map(|(_, r)| *r).collect();
        assert_eq!(onglets.len(), ONGLETS.len());
        assert!(onglets.iter().all(|r| r.x + r.w <= w - X + 0.01), "{onglets:?}");
        for (k, a) in onglets.iter().enumerate() {
            for b in onglets.iter().skip(k + 1) {
                assert!(a.y != b.y || b.x >= a.x + a.w + PUCE_GAP - 0.01, "onglets sans chevauchement");
            }
        }
        let lignes = |s: &Symboles| {
            s.places(&t, w, &d)
                .into_iter()
                .filter_map(|(i, r)| if let Item::Ligne(l) = i { Some((l, r)) } else { None })
                .collect::<Vec<_>>()
        };
        assert_eq!(lignes(&s).len(), 3, "la watchlist à l'ouverture");
        let (eth, r) = lignes(&s)[1].clone();
        assert_eq!(s.click(r.x + 100.0, r.cy(), w, h, &d), SymClick::Ouvrir(eth.coin.clone()));
        assert_eq!(s.click(r.x + 10.0, r.cy(), w, h, &d), SymClick::Favori(eth.coin));
        assert_eq!(s.click(onglets[0].cx(), onglets[0].cy(), w, h, &d), SymClick::Changed);
        assert_eq!(lignes(&s).len(), 1, "Favoris : seulement BTC");
        let champ = s.places(&t, w, &d).into_iter().find(|(i, _)| *i == Item::Champ).unwrap().1;
        s.click(champ.cx(), champ.cy(), w, h, &d);
        "sol".chars().for_each(|c| s.key(Some(c)));
        assert_eq!(lignes(&s).iter().map(|(l, _)| l.coin.as_str()).collect::<Vec<_>>(), ["SOL"]);
        s = Symboles::default();
        let rec = s.places(&t, w, &d).into_iter().find(|(i, _)| matches!(i, Item::Recent(_))).unwrap().1;
        assert_eq!(s.click(rec.cx(), rec.cy(), w, h, &d), SymClick::Ouvrir("ETH".into()));
        let opt = s.places(&t, w, &d).into_iter().find(|(i, _)| *i == Item::Options).unwrap().1;
        assert_eq!(s.click(opt.x + 20.0, opt.cy(), w, h, &d), SymClick::Options);
        assert_eq!(s.click(w - 29.0, 27.0, w, h, &d), SymClick::Fermer);
        let pm = s.draw(&t, w, h, &d, None).unwrap();
        assert_eq!(pm.pixel(1, 1).unwrap().alpha(), 0, "coins arrondis");
    }
}
