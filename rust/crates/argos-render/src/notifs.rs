//! 🔔 Feuille des notifications du cell et de la tablette, d'après la maquette (croquis/maquette-verre, vue Cell,
//! `#alertes` mesuré le 9 nov. 2026) : filtres Tout · Alertes · News · Baleines · Liquidations · PropFirm ; le fil des
//! derniers événements (texte, depuis quand), une ligne sur deux un peu plus claire ; « 🎓 Alertes de leçon » ;
//! « + Nouvelle alerte » ; « ⋯ Toutes les options ». Feuille de verre (rayon 26) posée en bas, sur 584 points.
//! Les événements viennent du moteur : alertes déclenchées, baleines, news, avertissements du gardien PropFirm.

use crate::shapes::{round_fill, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

const TEXTE: Rgb = crate::jetons::TEXTE;
const DIM: Rgb = crate::jetons::DIM;
/// Hauteur de la feuille (maquette : 584,3 sur 844).
pub const HAUT: f64 = 584.3;
const LIGNE: f64 = 31.5;
const X: f64 = 9.0;
/// Haut de « + Nouvelle alerte » et de « ⋯ Toutes les options » depuis le haut de la feuille (maquette : 518,2 et
/// 557,8), quand le fil est court.
const NOUVELLE: f64 = 518.2;
const OPTIONS: f64 = 557.8;
/// 🖥 En-tête du volet 🔔 (titre, ⤢, ✕).
const TETE: f64 = 44.0;
/// 🖥 Écart entre deux rangées de filtres (fenêtre flottante).
const PUCES_RANG: f64 = 33.0;

/// 🧪 Les 5 exemples de la maquette (mode essai seulement, tant qu'aucun vrai événement n'est arrivé).
pub fn exemples(now: i64) -> Vec<Evenement> {
    [
        (1, "🔔 BTC a franchi 76 000", 120),
        (3, "🐋 412 BTC vers Binance", 540),
        (4, "💥 38 M$ de longs liquidés en 1 h", 1_260),
        (2, "📰 Décision de taux à 20:00", 3_600),
        (5, "🛡 50 % de la perte du jour", 90_000),
    ]
    .into_iter()
    .map(|(sorte, t, il_y_a)| Evenement {
        sorte,
        texte: t.into(),
        ts: Some(now - il_y_a),
    })
    .collect()
}

/// Les filtres, dans l'ordre de la maquette (0 = Tout).
pub const FILTRES: [&str; 6] = ["Tout", "Alertes", "News", "Baleines", "Liquidations", "PropFirm"];

/// Un événement du fil.
#[derive(Debug, Clone, PartialEq)]
pub struct Evenement {
    /// Filtre (1 Alertes, 2 News, 3 Baleines, 4 Liquidations, 5 PropFirm).
    pub sorte: usize,
    pub texte: String,
    /// Heure (secondes Unix) ; `None` : maintenant (avertissement en cours).
    pub ts: Option<i64>,
}

/// Ce que fait un toucher.
#[derive(Debug, Clone, PartialEq)]
pub enum NotifClick {
    None,
    Changed,
    Fermer,
    /// 🖥 ⤢ du volet : volet large ⇄ normal.
    Agrandir,
    NouvelleAlerte,
    Options,
    /// Un événement : ouvrir ce qui va avec (le volet News, Baleines…).
    Evenement(usize),
    Bientot(&'static str),
}

/// État de la feuille.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Notifs {
    pub filtre: usize,
    pub defile: f64,
    /// 🖥 PC : la fenêtre flottante sous 🔔 (comme `.pop` de la maquette, 380 de large) : (x, y, largeur, hauteur
    /// maximale), coordonnées de la fenêtre. `None` : la feuille du bas (cell, tablette).
    pub pop: Option<Rect>,
    /// 🖥 Dans le volet flottant (comme les autres volets) : toute la hauteur de `pop`, en-tête « 🔔 Alertes » avec ⤢
    /// et ✕, coins du volet ; un clic à côté va à la chart (la fenêtre de l'app ne lui envoie que ses clics).
    pub volet: bool,
    /// 🔔 The feed of notifications (news, whales, liquidations, PropFirm) shown, from ⚙ of the alerts panel; else
    /// (by default, Jee's mock-up « alertes v1 ») the alerts panel itself, `alertes`.
    pub fil: bool,
    pub alertes: crate::alertes::Etat,
}

impl Notifs {
    /// 🔔 Where the alerts panel is (window `w` × `h`): the volet, else the phone's bottom sheet.
    pub fn alertes_place(&self, w: f64, h: f64) -> Rect {
        if let Some(r) = self.alertes.dessine.get() {
            return r;
        }
        match self.pop {
            Some(p) => p,
            None => Self::rect(w, h),
        }
    }
}

/// « 2 min », « 1 h », « hier »…
pub fn depuis(ts: Option<i64>, now: i64) -> String {
    let Some(ts) = ts else { return "maintenant".into() };
    let s = (now - ts).max(0);
    match s {
        0..=59 => "à l'instant".into(),
        60..=3_599 => format!("{} min", s / 60),
        3_600..=86_399 => format!("{} h", s / 3_600),
        86_400..=172_799 => "hier".into(),
        _ => format!("{} j", s / 86_400),
    }
}

/// 🎓 La rubrique « Alertes de leçon » (bientôt) : sur PC seulement, pas au doigt.
fn lecons_montrees() -> bool {
    !crate::actions::tactile()
}

/// Hauteur de la rubrique « Alertes de leçon » avec ses marges (0 quand elle est cachée).
fn haut_lecons() -> f64 {
    if lecons_montrees() {
        12.0 + 48.8 + 12.0
    } else {
        0.0
    }
}

/// 🖥 Largeur de la fenêtre flottante (maquette : `#alertes`, 380).
pub const POP_W: f64 = 380.0;
/// 🖥 Rayon de la fenêtre flottante (maquette : `.pop`, 16).
const POP_R: f64 = 16.0;

impl Notifs {
    /// La feuille dans une fenêtre de `w` × `h` : en bas, sur toute la largeur.
    pub fn rect(w: f64, h: f64) -> Rect {
        Rect::new(0.0, (h - HAUT).max(0.0), w, HAUT.min(h))
    }

    /// 🖥 PC : la fenêtre flottante sous le bouton `b` (fenêtre `w` × `h`), dans la fenêtre.
    pub fn sous(b: Rect, w: f64, h: f64) -> Rect {
        let x = (b.x + b.w - POP_W).max(8.0).min((w - POP_W - 8.0).max(0.0));
        let y = b.y + b.h + 6.0;
        Rect::new(x, y, POP_W.min(w), (h - y - 10.0).max(120.0))
    }

    /// Place réelle : la fenêtre flottante (hauteur ajustée au contenu) ou la feuille du bas.
    pub fn place(&self, w: f64, h: f64, tous: &[Evenement]) -> Rect {
        match self.pop {
            Some(p) if self.volet => p,
            Some(p) => Rect::new(p.x, p.y, p.w, self.haut_contenu(tous).min(p.h)),
            None => Self::rect(w, h),
        }
    }

    /// Les filtres ; 🖥 fenêtre flottante : ils passent à la ligne quand la place manque (PropFirm n'était pas montré).
    fn puces(&self, text: &Text, r: Rect) -> Vec<Rect> {
        let (mut x, mut y) = (r.x + X, r.y + 9.0 + self.tete());
        FILTRES
            .iter()
            .map(|f| {
                let w = text.width(f, 13.0) as f64 + 22.0;
                if self.pop.is_some() && x > r.x + X && x + w > r.x + r.w - X {
                    (x, y) = (r.x + X, y + PUCES_RANG);
                }
                let q = Rect::new(x, y, w, 27.5);
                x += w + 6.0;
                q
            })
            .collect()
    }

    /// 🖥 Hauteur de l'en-tête du volet (« 🔔 Alertes », ⤢, ✕) ; 0 hors du volet.
    fn tete(&self) -> f64 {
        if self.volet {
            TETE
        } else {
            0.0
        }
    }

    /// Haut du fil depuis le haut de la feuille (sous les filtres : 40,5 pour une rangée).
    fn debut(&self) -> f64 {
        let w = self.pop.map_or(f64::INFINITY, |p| p.w);
        let rangs = self
            .puces(&Text::default(), Rect::new(0.0, 0.0, w, 100.0))
            .last()
            .map_or(0.0, |q| ((q.y - 9.0 - self.tete()) / PUCES_RANG).round());
        40.5 + rangs * PUCES_RANG + self.tete()
    }

    /// Les événements montrés (le filtre choisi).
    pub fn liste<'a>(&self, tous: &'a [Evenement]) -> Vec<(usize, &'a Evenement)> {
        tous.iter()
            .enumerate()
            .filter(|(_, e)| self.filtre == 0 || e.sorte == self.filtre)
            .take(40)
            .collect()
    }

    /// Zones (fenêtre) : puces, lignes du fil, leçon, nouvelle alerte, options.
    pub fn zones(&self, text: &Text, r: Rect, tous: &[Evenement]) -> Vec<(Rect, NotifClick)> {
        let mut out = vec![];
        if self.volet {
            out.push((Rect::new(r.x + r.w - 44.0, r.y + 10.0, 30.0, 30.0), NotifClick::Fermer));
            out.push((Rect::new(r.x + r.w - 78.0, r.y + 10.0, 30.0, 30.0), NotifClick::Agrandir));
        }
        for (k, q) in self.puces(text, r).into_iter().enumerate() {
            out.push((q, NotifClick::Bientot(FILTRES[k])));
        }
        let mut y = r.y + self.debut() - self.defile;
        let l = self.liste(tous);
        for (i, _) in &l {
            out.push((Rect::new(r.x + X, y, r.w - 2.0 * X, LIGNE), NotifClick::Evenement(*i)));
            y += LIGNE;
        }
        if l.is_empty() {
            y += LIGNE;
        }
        // 🎓 Alertes de leçon : la section, puis bientôt (📱 au doigt : pas de « bientôt »)
        if lecons_montrees() {
            out.push((
                Rect::new(r.x + X, y + 12.0, r.w - 2.0 * X, 18.8 + 30.0),
                NotifClick::Bientot("🎓 Alertes de leçon : bientôt"),
            ));
        }
        y += haut_lecons();
        // « + Nouvelle alerte » et « Toutes les options » : en bas de la feuille, à leur place de la maquette
        // (🖥 fenêtre flottante : juste après le fil, comme `#alertes`)
        if self.pop.is_none() {
            y = y.max(r.y + NOUVELLE - self.defile);
        }
        out.push((Rect::new(r.x + X, y, 125.2, 27.5), NotifClick::NouvelleAlerte));
        y += OPTIONS - NOUVELLE;
        // 📱 au doigt : la ligne fait 44 de haut (le texte reste à sa place)
        let marge = if crate::actions::tactile() { (44.0 - 17.5) / 2.0 } else { 0.0 };
        out.push((Rect::new(r.x + X, y - marge, 170.0, 17.5 + 2.0 * marge), NotifClick::Options));
        out
    }

    fn haut_contenu(&self, tous: &[Evenement]) -> f64 {
        let suite = self.debut() + self.liste(tous).len().max(1) as f64 * LIGNE + haut_lecons();
        if let Some(p) = self.pop {
            // (s'il défile : la marge de fin pour passer au-dessus du rond Argos AI)
            let h = suite + OPTIONS - NOUVELLE + 17.5 + 12.0;
            return if h > p.h { h + crate::aibtn::FIN_DE_LISTE } else { h };
        }
        suite.max(NOUVELLE) + OPTIONS - NOUVELLE + 17.5 + 70.0
    }

    /// Molette / doigt (`dy` > 0 : vers le bas).
    pub fn wheel(&mut self, dy: f64, tous: &[Evenement]) {
        let haut = self.pop.map_or(HAUT, |p| p.h);
        let max = (self.haut_contenu(tous) - haut).max(0.0);
        self.defile = (self.defile + dy).clamp(0.0, max);
    }

    /// Toucher (fenêtre `w` × `h`) ; à côté de la feuille : la fermer.
    pub fn click(&mut self, x: f64, y: f64, w: f64, h: f64, text: &Text, tous: &[Evenement]) -> NotifClick {
        let r = self.place(w, h, tous);
        if !r.contains(x, y) {
            return NotifClick::Fermer;
        }
        for (q, c) in self.zones(text, r, tous) {
            if !q.contains(x, y) {
                continue;
            }
            return match c {
                NotifClick::Bientot(f) if FILTRES.contains(&f) => {
                    self.filtre = FILTRES.iter().position(|x| *x == f).unwrap_or(0);
                    self.defile = 0.0;
                    NotifClick::Changed
                }
                c => c,
            };
        }
        NotifClick::None
    }

    /// Dessin de la feuille (image de sa taille) ; `now` en secondes ; `mouse` : fenêtre.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, text: &Text, w: f64, h: f64, tous: &[Evenement], now: i64, mouse: Option<(f64, f64)>, verre: bool) -> Option<(Pixmap, Rect)> {
        let r = self.place(w, h, tous);
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        // feuille de verre, coins du haut arrondis (le bas déborde, caché) ; 🖥 fenêtre flottante : les 4 coins (16)
        let (carte, rayon) = match self.pop {
            Some(_) if self.volet => (Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0), crate::jetons::R_VOLET),
            Some(_) => (Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0), POP_R),
            None => (Rect::new(0.0, 0.0, r.w, r.h + 30.0), 26.0),
        };
        // `verre` : la chart floutée est déjà dessous (teinte de la maquette, rgba(26, 30, 42, .6)) ; sinon opaque
        if verre {
            round_fill(&mut pm, carte, rayon, crate::jetons::VERRE.0, crate::jetons::VERRE.1);
        } else {
            round_fill(&mut pm, carte, rayon, crate::jetons::sur_fond(crate::jetons::VERRE), 1.0);
        }
        crate::shapes::round_stroke(&mut pm, carte, rayon, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let survol = |q: &Rect| mouse.is_some_and(|(x, y)| q.contains(x, y));
        if self.volet {
            crate::text::Text::graisse(crate::text::Graisse::Bold).draw(&mut pm, "🔔 Alertes", ((X + 6.0) as f32, 32.0), 16.0, TEXTE, Align::Left);
        }
        let l = self.liste(tous);
        let zones = self.zones(text, r, tous);
        let mut rang = 0;
        for (q, c) in &zones {
            let lq = loc(*q);
            if lq.y + lq.h < self.debut() - 2.5
                && !matches!(c, NotifClick::Bientot(f) if FILTRES.contains(f))
                && !matches!(c, NotifClick::Fermer | NotifClick::Agrandir)
            {
                continue;
            }
            match c {
                NotifClick::Fermer | NotifClick::Agrandir => {
                    if survol(q) {
                        round_fill(&mut pm, lq, lq.h / 2.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
                    }
                    if *c == NotifClick::Fermer {
                        text.draw(&mut pm, "✕", (lq.cx() as f32, (lq.cy() + 5.0) as f32), 14.0, DIM, Align::Center);
                    } else {
                        // (⤢ n'est pas dans la police : deux flèches en diagonale, comme les autres volets)
                        crate::shapes::lines(
                            &mut pm,
                            lq.cx(),
                            lq.cy(),
                            &[
                                &[(-5.0, 5.0), (5.0, -5.0)],
                                &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                                &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                            ],
                            DIM,
                            1.3,
                        );
                    }
                }
                NotifClick::Bientot(f) if FILTRES.contains(f) => {
                    let k = FILTRES.iter().position(|x| x == f).unwrap_or(0);
                    let on = k == self.filtre;
                    let (pc, pa) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
                    round_fill(&mut pm, lq, lq.h / 2.0, pc, pa + if survol(q) && !on { 0.05 } else { 0.0 });
                    text.draw(&mut pm, f, (lq.cx() as f32, (lq.cy() + 4.6) as f32), 13.0, TEXTE, Align::Center);
                }
                NotifClick::Evenement(_) => {
                    let (_, e) = l[rang];
                    if rang % 2 == 1 || survol(q) {
                        round_fill(&mut pm, lq, lq.h / 2.0, crate::jetons::BLANC, if survol(q) { 0.06 } else { 0.03 });
                    }
                    let quand = depuis(e.ts, now);
                    let qw = text.width(&quand, 13.0) as f64;
                    text.draw(&mut pm, &quand, ((lq.x + lq.w - 10.0) as f32, (lq.y + 21.0) as f32), 13.0, DIM, Align::Right);
                    let t = text.fit(&e.texte, 13.0, lq.w - 30.0 - qw);
                    text.draw(&mut pm, &t, ((lq.x + 10.0) as f32, (lq.y + 21.0) as f32), 13.0, TEXTE, Align::Left);
                    rang += 1;
                }
                NotifClick::Bientot(_) => {
                    text.draw(
                        &mut pm,
                        "🎓 ALERTES DE LEÇON",
                        ((lq.x + 2.0) as f32, (lq.y + 14.0) as f32),
                        11.0,
                        DIM,
                        Align::Left,
                    );
                    let tx = lq.x + 2.0 + text.width("🎓 ALERTES DE LEÇON", 11.0) as f64 + 8.0;
                    let tag = Rect::new(tx, lq.y, 70.5, 18.8);
                    round_fill(&mut pm, tag, tag.h / 2.0, crate::jetons::TIER2, 0.22);
                    text.draw(&mut pm, "bientôt", (tag.cx() as f32, (tag.y + 13.5) as f32), 11.0, DIM, Align::Center);
                    text.draw(
                        &mut pm,
                        "Une alerte liée à une leçon de l'Académie : bientôt",
                        ((lq.x + 2.0) as f32, (lq.y + 40.0) as f32),
                        12.5,
                        DIM,
                        Align::Left,
                    );
                }
                NotifClick::NouvelleAlerte => {
                    let (pc, pa) = crate::jetons::PUCE_ON;
                    round_fill(&mut pm, lq, lq.h / 2.0, pc, pa + if survol(q) { 0.1 } else { 0.0 });
                    text.draw(
                        &mut pm,
                        "+ Nouvelle alerte",
                        (lq.cx() as f32, (lq.cy() + 4.6) as f32),
                        13.0,
                        TEXTE,
                        Align::Center,
                    );
                }
                NotifClick::Options => {
                    text.draw(
                        &mut pm,
                        "▸ ⋯ Toutes les options",
                        (lq.x as f32, (lq.cy() + 4.75) as f32),
                        13.0,
                        DIM,
                        Align::Left,
                    );
                }
                _ => {}
            }
        }
        if l.is_empty() {
            let msg = if self.filtre == 4 {
                "Aucune liquidation reçue (module Liquidations)"
            } else {
                "Rien pour l'instant"
            };
            text.draw(
                &mut pm,
                msg,
                ((X + 10.0) as f32, (self.debut() - self.defile + 21.0) as f32),
                13.0,
                DIM,
                Align::Left,
            );
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🔔 Les filtres trient le fil ; un événement se touche ; « ＋ Nouvelle alerte » et « ⋯ Toutes les options »
    /// répondent ; à côté de la feuille, elle se ferme.
    #[test]
    fn feuille_des_notifications() {
        let t = Text::default();
        let ev = vec![
            Evenement {
                sorte: 1,
                texte: "🔔 BTC a franchi 76 000".into(),
                ts: Some(880),
            },
            Evenement {
                sorte: 3,
                texte: "🐋 BTC achat 2,1 M$".into(),
                ts: Some(460),
            },
            Evenement {
                sorte: 5,
                texte: "🛡 50 % de la perte du jour".into(),
                ts: None,
            },
        ];
        let (w, h) = (390.0, 844.0);
        let mut n = Notifs::default();
        assert_eq!(n.liste(&ev).len(), 3);
        let r = Notifs::rect(w, h);
        assert_eq!((r.y, r.h), (844.0 - HAUT, HAUT));
        let z = n.zones(&t, r, &ev);
        let baleines = z.iter().find(|(_, c)| *c == NotifClick::Bientot("Baleines")).unwrap().0;
        assert_eq!(n.click(baleines.cx(), baleines.cy(), w, h, &t, &ev), NotifClick::Changed);
        assert_eq!(n.liste(&ev).len(), 1, "seulement les baleines");
        let z = n.zones(&t, r, &ev);
        let e = z.iter().find(|(_, c)| matches!(c, NotifClick::Evenement(_))).unwrap().0;
        assert_eq!(n.click(e.cx(), e.cy(), w, h, &t, &ev), NotifClick::Evenement(1));
        let na = z.iter().find(|(_, c)| *c == NotifClick::NouvelleAlerte).unwrap().0;
        assert_eq!(n.click(na.cx(), na.cy(), w, h, &t, &ev), NotifClick::NouvelleAlerte);
        assert_eq!(n.click(200.0, 100.0, w, h, &t, &ev), NotifClick::Fermer);
        // « + Nouvelle alerte » et « Toutes les options » en bas, à leur place de la maquette
        let opt = z.iter().find(|(_, c)| *c == NotifClick::Options).unwrap().0;
        assert_eq!((na.y - r.y, opt.y - r.y), (NOUVELLE, OPTIONS));
        let ex: Vec<String> = exemples(10_000).iter().map(|e| depuis(e.ts, 10_000)).collect();
        assert_eq!(ex, ["2 min", "9 min", "21 min", "1 h", "hier"], "les temps de la maquette");
        assert_eq!(depuis(Some(880), 1000), "2 min");
        assert_eq!(depuis(None, 1000), "maintenant");
    }

    /// 🖥 PC : la fenêtre flottante sous 🔔 (380 de large, dans la fenêtre), « + Nouvelle alerte » juste après le fil,
    /// hauteur ajustée au contenu ; un clic à côté la ferme.
    #[test]
    fn fenetre_flottante_sur_pc() {
        let t = Text::default();
        let ev = exemples(10_000);
        let (w, h) = (1440.0, 900.0);
        let bouton = Rect::new(1300.0, 12.0, 40.0, 36.0);
        let n = Notifs {
            pop: Some(Notifs::sous(bouton, w, h)),
            ..Default::default()
        };
        let r = n.place(w, h, &ev);
        assert_eq!(r.w, POP_W);
        assert!(r.x >= 8.0 && r.x + r.w <= w - 8.0 && r.y > bouton.y + bouton.h, "{r:?}");
        assert!(r.h < HAUT, "ajustée au contenu : {r:?}");
        // les 6 filtres sont dans la fenêtre (PropFirm passe à la ligne), le fil commence sous eux
        let p = n.puces(&t, r);
        assert!(p.iter().all(|q| q.x + q.w <= r.x + r.w - X + 0.01), "{p:?}");
        assert!(n.debut() >= p.iter().map(|q| q.y + q.h - r.y).fold(0.0, f64::max), "le fil sous les filtres");
        let z = n.zones(&t, r, &ev);
        let na = z.iter().find(|(_, c)| *c == NotifClick::NouvelleAlerte).unwrap().0;
        let opt = z.iter().find(|(_, c)| *c == NotifClick::Options).unwrap().0;
        assert!(na.y - r.y < NOUVELLE && opt.y + opt.h <= r.y + r.h, "{na:?} {opt:?} {r:?}");
        let mut n2 = n.clone();
        assert_eq!(n2.click(na.cx(), na.cy(), w, h, &t, &ev), NotifClick::NouvelleAlerte);
        assert_eq!(n2.click(100.0, 500.0, w, h, &t, &ev), NotifClick::Fermer);
        let (pm, q) = n.draw(&t, w, h, &ev, 10_000, None, false).unwrap();
        assert_eq!((pm.width() as f64, q), (POP_W.ceil(), r));
        assert_eq!(pm.pixel(0, 0).unwrap().alpha(), 0, "coins arrondis");
    }

    /// 📱 Au doigt : pas de rubrique « Alertes de leçon · bientôt », « Toutes les options » fait 44 de haut ;
    /// le PC garde la rubrique et la ligne de la maquette.
    #[test]
    fn au_doigt_sans_bientot_et_options_de_44() {
        let t = Text::default();
        let ev = exemples(10_000);
        let (w, h) = (402.0, 874.0);
        let n = Notifs::default();
        let r = Notifs::rect(w, h);
        let lecon = |z: &[(Rect, NotifClick)]| z.iter().any(|(_, c)| *c == NotifClick::Bientot("🎓 Alertes de leçon : bientôt"));
        let options = |z: &[(Rect, NotifClick)]| z.iter().find(|(_, c)| *c == NotifClick::Options).unwrap().0;
        crate::actions::set_tactile(false);
        let pc = n.zones(&t, r, &ev);
        crate::actions::set_tactile(true);
        let doigt = n.zones(&t, r, &ev);
        let dessin = n.draw(&t, w, h, &ev, 10_000, None, false);
        crate::actions::set_tactile(false);
        assert!(lecon(&pc) && options(&pc).h == 17.5, "PC : comme la maquette");
        assert!(!lecon(&doigt), "au doigt : pas de « bientôt »");
        let (o_pc, o_doigt) = (options(&pc), options(&doigt));
        assert!(o_doigt.h >= 44.0 && (o_doigt.cy() - o_pc.cy()).abs() < 0.01, "{o_doigt:?} {o_pc:?}");
        assert!(dessin.is_some());
    }
}
