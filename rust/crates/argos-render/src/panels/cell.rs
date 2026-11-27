//! 📱🖥 La feuille Trading (cell, tablette, volet du PC). En haut : 📈 Trading, ⬓ ⤢ ✕ (📱 ✕ seul) et 🎫 Ticket /
//! 🧪 Simulateur ; cet en-tête reste en haut, le reste défile dessous. Puis le ticket du module 31 (`ticket.rs`,
//! maquette v1 du 22 nov.), ou la vue « Positions et ordres » (les positions démo, les ordres en attente et leurs ✕,
//! Tout fermer). La carte 🛡 Gardien PropFirm complète (perte du jour et sa jauge, perte max, positions, risque max,
//! le bloc 🔒) s'ouvre en touchant sa ligne du ticket : ses places sont celles de l'ancienne feuille (maquette-verre,
//! mesures dans `panels/maquette-trading-cell.txt`), la carte entière décalée sous la ligne du compte.
//! Ce qui n'existe pas encore dans le moteur (routine du jour, plan de trading) dit « bientôt ».

use super::ticket::Champ;
use super::{Hit, PanelClick, Panels};
use crate::journal::GuardLevel;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use tiny_skia::Pixmap;

const TEXTE: Rgb = crate::jetons::TEXTE;
const DIM: Rgb = crate::jetons::DIM;
const LIEN: Rgb = Rgb(143, 180, 255);
const VERT: Rgb = Rgb(22, 199, 132);
const ROUGE: Rgb = Rgb(234, 57, 67);
const OR: Rgb = crate::jetons::TIER2;

/// Bord gauche du contenu (maquette : 13 sur 390 ; la largeur utile : `large`).
pub(super) const X: f64 = 13.0;
/// 💼 Room for « Ordres en attente » above the pending orders.
const ORDRES_TITRE: f64 = 22.0;
/// 💼 The detail under « Tout fermer »: its size and line height.
const DETAIL_PX: f32 = 11.5;
const DETAIL_LIGNE: f64 = 15.0;
/// 💼 « Positions et ordres »: its ‹ row (under the sheet's header) and the top of the first position card.
const RETOUR_Y: f64 = 125.5;
const CARTES_Y: f64 = 181.8;
/// 🛡 Unticking TP/SL while the Gardien is on.
pub(super) const GARDIEN_SL: &str = "🛡 Gardien PropFirm allumé : le SL reste obligatoire (il vérifie ton risque max)";

/// Ce qu'on touche dans la feuille (en plus des `Hit` des panneaux).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Cell {
    Ancrer,
    Agrandir,
    Fermer,
    /// 🎫 Ticket (from « Positions et ordres »: back to the ticket) / 🧪 Simulateur.
    Ticket,
    Simulateur,
    Routine,
    Plan,
    /// VENDRE (`false`) / ACHETER (`true`) : l'ordre démo du ticket part.
    Passer(bool),
    /// 🛡 The Gardien's line (its card opens), or its card's title (it folds).
    Gardien,
    /// Mid, ⌖ (the next touch on the chart gives the price), a field typed in a box, the % slider.
    Mid,
    Cible,
    Champ(Champ),
    Curseur,
    /// TP/SL, Reduce-only, the validity (its menu), one of the validities (`Validity::ALL`).
    TpSl,
    Reduire,
    Validite,
    Duree(usize),
    /// 📒 Compte (folded ⇄ unfolded), « Positions et ordres › », « Carnet › », « ⚙ Tous les réglages › », ‹ back.
    Compte,
    Positions,
    Carnet,
    Reglages,
    Retour,
}

/// 📱 The sheet on a phone (touch, phone-wide): no ⚙ at the bottom of its screen, VENDRE / ACHETER kept at the bottom.
pub(super) fn telephone(pw: f64) -> bool {
    crate::actions::tactile() && pw < crate::formfactor::PHONE_MAX
}

/// Largeur utile (`pw` − les deux marges de la maquette).
pub(super) fn large(pw: f64) -> f64 {
    pw - 2.0 * X
}

/// ☀ Routine, à droite du titre de la carte du gardien (largeur de la feuille `pw`).
fn routine_rect(pw: f64) -> Rect {
    Rect::new(297.7 + pw - 390.0, 142.4, 67.3, 20.8)
}

/// 📐 Les deux en-têtes « titre + boutons à droite » de la feuille (largeur `pw`) : « Trading » avant ⬓ ⤢ ✕ (✕ seul
/// au cell), « 🛡 Gardien PropFirm · `etat` » avant ☀ Routine. Le titre se coupe (…) avant ses boutons.
pub(crate) fn cell_entetes(pw: f64, etat: &str) -> [(crate::entete::Entete, Rect); 2] {
    let gras = Text::graisse(Graisse::Bold);
    let d = pw - 390.0;
    let haut = Rect::new(if crate::actions::tactile() { 345.0 } else { 269.0 } + d, 38.0, 30.0, 30.0);
    let r = routine_rect(pw);
    [
        (crate::entete::placer(&gras, "Trading", 15.0, (39.0, 59.0), &[haut]), haut),
        (
            crate::entete::placer(&gras, &format!("🛡 Gardien PropFirm · {etat}"), 13.0, (25.0, 157.0), &[r]),
            r,
        ),
    ]
}

/// Décimales d'un prix (75 852,0 ; 2 130,55 ; 0,4321).
pub(super) fn deci(p: f64) -> usize {
    if p >= 1000.0 {
        1
    } else if p >= 1.0 {
        2
    } else {
        4
    }
}

pub(super) fn prix(p: f64) -> String {
    if p > 0.0 && p.is_finite() {
        crate::viewport::format_price(p, deci(p))
    } else {
        "—".into()
    }
}

/// A pill of the sheet (🎫 Ticket, ☀ Routine, 🔓 Déverrouiller…): glass, or blue when `on`.
fn puce(pm: &mut Pixmap, text: &Text, r: Rect, s: &str, on: bool, taille: f32, survol: bool) {
    let (c, a) = if on { crate::jetons::PUCE_ON } else { crate::jetons::VERRE2 };
    round_fill(pm, r, 12.0, c, a + if survol && !on { 0.05 } else { 0.0 });
    text.draw(pm, s, (r.cx() as f32, (r.cy() + taille as f64 * 0.36) as f32), taille, TEXTE, Align::Center);
}

/// 🛡 La carte du Gardien de la feuille : allumé ou non, et ses chiffres.
pub(super) struct Garde {
    on: bool,
    c: crate::journal::GuardCard,
    /// 🔓 Raison notée au Journal si la journée a été déverrouillée aujourd'hui.
    raison: Option<String>,
    /// 🧊 Minutes left of the anti-tilt pause (`JCfg::pause_until`), rounded up; `None` outside a pause.
    pause: Option<i64>,
}

/// Haut du message ⚠ dans la carte (sous Perte max / Positions / Risque max) et sa hauteur avec l'espace.
const MSG_Y: f64 = 302.0;
pub(super) const MSG_H: f64 = 34.0;
/// Hauteur de la ligne 🔓 (journée déverrouillée aujourd'hui, et sa raison) avec l'espace.
pub(super) const RAISON_H: f64 = 50.0;
/// Haut du bloc 🔒 (sous la carte) et sa hauteur avec l'espace.
const BLOC_Y: f64 = 343.7;
const BLOC_H: f64 = 96.0;
/// 🛡 Top of the card at its own places (the ticket moves it under the account's line).
pub(super) const GARDE_Y0: f64 = 132.4;

impl Garde {
    /// % de la limite la plus entamée, et son nom.
    pub(super) fn pire(&self) -> (f64, &'static str) {
        let p = |v: f64, l: f64| if l > 0.0 { v / l * 100.0 } else { 0.0 };
        let (j, m) = (p(self.c.day_loss, self.c.day_lim), p(self.c.tot_loss, self.c.max_lim));
        if j >= m {
            (j, "ta perte max du jour")
        } else {
            (m, "ta perte max totale")
        }
    }

    /// Its state in a few words (« éteint », « dans les règles »…) and the card's colour.
    pub(super) fn etat(&self) -> (&'static str, (Rgb, f32)) {
        if !self.on {
            return ("éteint", (crate::jetons::BLANC, 0.04));
        }
        match self.c.level() {
            GuardLevel::Bad if self.c.locked => ("journée verrouillée", (ROUGE, 0.08)),
            GuardLevel::Bad => ("limite atteinte", (ROUGE, 0.08)),
            GuardLevel::Warn => ("prudence", (OR, 0.08)),
            GuardLevel::Half => ("moitié d'une limite", (OR, 0.05)),
            GuardLevel::Ok => ("dans les règles", (VERT, 0.07)),
        }
    }

    /// 🎫 Its state on the ticket's one line: the anti-tilt pause first, else `etat`.
    pub(super) fn ligne(&self) -> String {
        match self.pause.filter(|_| self.on) {
            Some(min) => format!("pause anti-tilt · {min} min"),
            None => self.etat().0.to_string(),
        }
    }

    /// Le message de la carte : 🧊 la pause anti-tilt en cours ou ⚠ la prudence (`true`), sinon 🔓 la journée
    /// déverrouillée aujourd'hui et sa raison notée au Journal (`false`).
    pub(super) fn message(&self) -> Option<(String, bool)> {
        self.pause_txt().or_else(|| self.alerte()).map(|s| (s, true)).or_else(|| {
            self.raison
                .as_ref()
                .filter(|_| self.on && !self.c.locked)
                .map(|r| (format!("🔓 Déverrouillée aujourd'hui · raison notée au Journal :\n« {r} »"), false))
        })
    }

    /// 🧊 The anti-tilt pause in progress (guardian on): the minutes left, every new order is refused until then.
    pub(super) fn pause_txt(&self) -> Option<String> {
        self.pause
            .filter(|_| self.on)
            .map(|min| format!("🧊 Pause anti-tilt : encore {min} min, aucun nouvel ordre"))
    }

    /// ⚠ Le message de prudence (à partir du seuil d'alerte, 80 % par défaut), sauf journée verrouillée.
    pub(super) fn alerte(&self) -> Option<String> {
        (self.on && self.c.level() == GuardLevel::Warn).then(|| {
            let (p, nom) = self.pire();
            format!("⚠ {} % de {nom} : réduis la taille ou arrête pour aujourd'hui.", p.round())
        })
    }

    /// 🔒 Journée verrouillée (gardien allumé) : le bloc sous la carte et le ticket grisé.
    pub(super) fn verrou(&self) -> bool {
        self.on && self.c.locked
    }

    /// Place prise par le message ⚠ dans la carte.
    pub(super) fn msg_h(&self) -> f64 {
        match self.message() {
            Some((_, true)) => MSG_H,
            // 🔓 deux lignes : le titre, puis la raison
            Some((_, false)) => RAISON_H,
            None => 0.0,
        }
    }

    /// De combien le ticket descend sous la carte ouverte (message ⚠ + bloc 🔒).
    pub(super) fn decale(&self) -> f64 {
        self.msg_h() + if self.verrou() { BLOC_H } else { 0.0 }
    }

    /// Le bouton « 🔓 Déverrouiller aujourd'hui » du bloc 🔒 (largeur utile `l`).
    fn deverrouiller(&self, l: f64) -> Option<Rect> {
        self.verrou()
            .then(|| Rect::new(25.0, BLOC_Y + self.msg_h() + 56.0, (l - 24.0).min(230.0), 28.0))
    }
}

impl Panels {
    /// 📱 Zones de la feuille Trading (coordonnées de la feuille, largeur `pw`) : l'en-tête, puis le ticket ou
    /// « Positions et ordres ».
    pub(super) fn cell_zones(&self, pw: f64) -> Vec<(Rect, Hit)> {
        let mut z = self.entete_zones(pw);
        if self.trading.sim_onglet {
            // (🧪 the Simulator answers under the header: nothing of the ticket under it)
        } else if self.trading.positions_vue {
            z.extend(self.positions_zones(pw));
        } else {
            z.extend(self.ticket_zones(pw));
        }
        z
    }

    /// 📌 The header's touch areas: ⬓ ⤢ ✕ (📱 ✕ only: ⬓ ⤢ are for the big screen), 🎫 Ticket, 🧪 Simulateur.
    pub(super) fn entete_zones(&self, pw: f64) -> Vec<(Rect, Hit)> {
        let d = pw - 390.0; // la maquette fait 390 de large : ce qui est à droite suit le bord droit
        let mut z = vec![
            (Rect::new(269.0 + d, 38.0, 30.0, 30.0), Hit::Cell(Cell::Ancrer)),
            (Rect::new(307.0 + d, 38.0, 30.0, 30.0), Hit::Cell(Cell::Agrandir)),
            (Rect::new(345.0 + d, 38.0, 30.0, 30.0), Hit::Cell(Cell::Fermer)),
            (Rect::new(X, 80.0, 74.5, 27.5), Hit::Cell(Cell::Ticket)),
            (Rect::new(93.5, 80.0, 102.5, 27.5), Hit::Cell(Cell::Simulateur)),
        ];
        if crate::actions::tactile() {
            z.retain(|(_, h)| !matches!(h, Hit::Cell(Cell::Ancrer | Cell::Agrandir)));
        }
        z
    }

    /// 🛡 The open Gardien card's touch areas, the card moved down by `dy`: ☀ Routine, its title (folds it), the line
    /// under the gauge (🛡 Activer when off, else the day's routine), 📋 Mon plan, 🔓 Déverrouiller.
    pub(super) fn garde_zones(&self, pw: f64, dy: f64) -> Vec<(Rect, Hit)> {
        let l = large(pw);
        let g = self.cell_garde();
        let ligne = if self.trading.guardian { Hit::Cell(Cell::Routine) } else { Hit::Guardian };
        let mut z = vec![
            (routine_rect(pw), Hit::Cell(Cell::Routine)),
            (Rect::new(X, GARDE_Y0, l, 36.0), Hit::Cell(Cell::Gardien)),
            (Rect::new(25.0, 207.1, l - 24.0, 28.2), ligne),
            (Rect::new(60.0, 306.0 + g.msg_h(), 80.0, 20.0), Hit::Cell(Cell::Plan)),
        ];
        if let Some(r) = g.deverrouiller(l) {
            z.push((r, Hit::GuardUnlock));
        }
        for (r, _) in &mut z {
            r.y += dy;
        }
        z
    }

    /// 💼 « Positions et ordres »: ‹ back, then the same targets as the old ticket: each pending order's ✕
    /// (`Hit::CancelOrder`) and « Tout fermer » (`Hit::CloseAll`), so the same actions.
    fn positions_zones(&self, pw: f64) -> Vec<(Rect, Hit)> {
        let mut z = vec![(Rect::new(X, RETOUR_Y, large(pw), 30.0), Hit::Cell(Cell::Retour))];
        let (lignes, fermer) = self.cell_ordres(pw);
        z.extend(lignes.into_iter().enumerate().map(|(i, (_, croix))| (croix, Hit::CancelOrder(i))));
        if let Some(r) = fermer {
            z.push((r, Hit::CloseAll));
        }
        z
    }

    /// 💼 Top of the pending orders' section: under the positions' cards.
    fn cell_ordres_haut(&self) -> f64 {
        let n = self.trading.open.len().max(1) as f64;
        CARTES_Y + n * 45.6 + 4.0
    }

    /// 💼 The pending orders' rows (the row, its ✕: 40 points square) and the « Tout fermer » button (`None`: nothing
    /// to close), 40 points high, 8 apart, under the positions; the whole width of the sheet.
    fn cell_ordres(&self, pw: f64) -> (Vec<(Rect, Rect)>, Option<Rect>) {
        let t = &self.trading;
        let l = large(pw);
        let mut y = self.cell_ordres_haut();
        let mut lignes = vec![];
        if !t.pending.is_empty() {
            y += ORDRES_TITRE;
            for _ in &t.pending {
                let r = Rect::new(X, y, l, super::TOUCHER);
                lignes.push((r, Rect::new(X + l - super::TOUCHER, y, super::TOUCHER, super::TOUCHER)));
                y += super::TOUCHER + super::ECART;
            }
        }
        let fermer = (!t.open.is_empty() || !t.pending.is_empty()).then(|| Rect::new(X, y, l, super::TOUCHER));
        (lignes, fermer)
    }

    /// 💼 The detail under « Tout fermer » (`close_all_txt`), on one line, or two when it does not fit (cut at « + »).
    fn cell_fermer_detail(&self, text: &Text, l: f64) -> Vec<String> {
        let t = &self.trading;
        let pnl: f64 = t.open.iter().map(|o| o.1).sum();
        let s = super::close_all_txt(t.open.len(), pnl, t.pending.len());
        if (text.width(&s, DETAIL_PX) as f64) <= l {
            return vec![s];
        }
        match s.split_once(" + ") {
            Some((a, b)) => vec![text.fit(a, DETAIL_PX, l), text.fit(&format!("+ {b}"), DETAIL_PX, l)],
            None => vec![text.fit(&s, DETAIL_PX, l)],
        }
    }

    /// 💼 Height of the pending orders and « Tout fermer » (with its detail), 0 when there is nothing.
    fn cell_ordres_h(&self) -> f64 {
        let t = &self.trading;
        let mut h = 0.0;
        if !t.pending.is_empty() {
            h += ORDRES_TITRE + t.pending.len() as f64 * (super::TOUCHER + super::ECART);
        }
        if !t.open.is_empty() || !t.pending.is_empty() {
            h += super::TOUCHER + 2.0 * DETAIL_LIGNE + super::ECART;
        }
        h
    }

    /// 💼 Bottom of « Positions et ordres » (under « Tout fermer » and its detail).
    fn cell_bas(&self) -> f64 {
        self.cell_ordres_haut() + self.cell_ordres_h() + 16.0
    }

    /// 🛡 État du Gardien pour la carte : les chiffres de `journal.guard_card` (positions démo ouvertes comprises).
    pub(super) fn cell_garde(&self) -> Garde {
        let t = &self.trading;
        let flottant: f64 = t.open.iter().map(|o| o.1).sum();
        let now = crate::journal::now_ms();
        Garde {
            on: t.guardian,
            c: self.journal.guard_card(flottant, t.open.len(), now),
            raison: self.journal.deverrouillee(now).map(str::to_string),
            pause: self.journal.cfg.pause_left(now),
        }
    }

    /// Hauteur du contenu de la feuille (largeur `pw`).
    pub(super) fn cell_haut(&self, pw: f64) -> f64 {
        if self.trading.positions_vue {
            self.cell_bas() + 24.0
        } else {
            self.ticket_places(pw).fin
        }
    }

    /// 📱 Toucher dans la feuille (le curseur %, qui a besoin de l'abscisse : `Panels::click`).
    pub(super) fn cell_click(&mut self, c: Cell) -> PanelClick {
        let t = &mut self.trading;
        match c {
            Cell::Fermer => return PanelClick::Fermer,
            // 🖥 the 🧪 tab already shown: nothing; 🎫 Ticket brings the ticket back in its place
            Cell::Simulateur if t.sim_onglet => return PanelClick::None,
            Cell::Simulateur => return PanelClick::Simulateur,
            Cell::Ticket if t.sim_onglet => return PanelClick::FermerSimulateur,
            Cell::Ticket | Cell::Retour => t.positions_vue = false,
            // ⬓ : les positions démo dans la colonne de modules ; ⤢ : volet large ⇄ normal (comme les autres volets)
            Cell::Ancrer => return PanelClick::Ancrer,
            Cell::Agrandir => return PanelClick::Agrandir,
            Cell::Routine => return PanelClick::Soon("☀ Routine du jour : bientôt".into()),
            Cell::Plan => return PanelClick::Soon("📋 Mon plan de trading : bientôt".into()),
            Cell::Passer(achat) => {
                t.buy = achat;
                return PanelClick::Passer(achat);
            }
            Cell::Gardien => t.garde_ouverte = !t.garde_ouverte,
            // ⚖ Mid: the book's middle (without a book, the last price); the quantity stays, the slider follows
            Cell::Mid => {
                t.px = if t.mid > 0.0 { t.mid } else { t.prix };
                t.set_qty(t.qty);
            }
            Cell::Cible => {
                t.cible = !t.cible;
                if t.cible {
                    return PanelClick::Cible;
                }
            }
            Cell::Champ(c) => return PanelClick::Saisir(c),
            Cell::Curseur => {}
            // TP/SL: in reduce-only it brings TP / SL back; 🛡 the Gardien on keeps the SL (it checks the max risk)
            Cell::TpSl if t.reduce_only => (t.reduce_only, t.tpsl) = (false, true),
            Cell::TpSl if t.tpsl && t.guardian => return PanelClick::Soon(GARDIEN_SL.into()),
            Cell::TpSl => t.tpsl = !t.tpsl,
            Cell::Reduire => {
                t.reduce_only = !t.reduce_only;
                t.set_qty(t.qty);
            }
            Cell::Validite => t.menu_validite = !t.menu_validite,
            Cell::Duree(k) => {
                t.validity = argos_core::simu::Validity::ALL[k.min(2)];
                t.menu_validite = false;
            }
            Cell::Compte => t.compte_ouvert = !t.compte_ouvert,
            Cell::Positions => t.positions_vue = true,
            Cell::Carnet => return PanelClick::Carnet,
            // ⚙ Tous les réglages: the old full ticket (leverage, margin, size by the risk…)
            Cell::Reglages => t.options = true,
        }
        PanelClick::Changed
    }

    /// 📱 Dessine la feuille Trading : l'en-tête, puis le ticket ou « Positions et ordres ».
    pub(super) fn cell_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>) {
        self.cell_entete_draw(pm, text, pw, mouse);
        if self.trading.sim_onglet {
            // (🧪 the Simulator is drawn in the sheet's place, under the header)
        } else if self.trading.positions_vue {
            self.positions_draw(pm, text, pw, mouse);
        } else {
            self.ticket_draw(pm, text, pw, mouse);
        }
    }

    /// 📌 The sheet's header: 📈 Trading, ⬓ ⤢ ✕ (📱 ✕ only), 🎫 Ticket / 🧪 Simulateur.
    pub(super) fn cell_entete_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>) {
        let gras = Text::graisse(Graisse::Bold);
        let d = pw - 390.0;
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        crate::phosphor::draw(pm, "chart-line-up", 25.0, 53.6, 20, DIM);
        let [(tr, _), _] = cell_entetes(pw, "");
        gras.draw(pm, &tr.titre, (39.0, 59.0), 15.0, TEXTE, Align::Left);
        for (k, s) in ["⬓", "⤢", "✕"].into_iter().enumerate() {
            // 📱 phone view: ⬓ ⤢ are for the big screen (they only said so): only ✕ (Jee, 20 Nov)
            if s != "✕" && crate::actions::tactile() {
                continue;
            }
            let r = Rect::new(269.0 + 38.0 * k as f64 + d, 38.0, 30.0, 30.0);
            round_fill(pm, r, 15.0, crate::jetons::BLANC, if survol(r) { 0.12 } else { 0.07 });
            if s == "⤢" {
                // (⤢ n'est pas dans la police : deux flèches en diagonale, dessinées)
                crate::shapes::lines(
                    pm,
                    r.cx(),
                    r.cy(),
                    &[
                        &[(-5.0, 5.0), (5.0, -5.0)],
                        &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                        &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                    ],
                    TEXTE,
                    1.3,
                );
            } else {
                text.draw(pm, s, (r.cx() as f32, (r.cy() + 4.7) as f32), 13.0, TEXTE, Align::Center);
            }
        }
        let sim = self.trading.sim_onglet;
        for (r, s, on) in [
            (Rect::new(X, 80.0, 74.5, 27.5), "🎫 Ticket", !sim),
            (Rect::new(93.5, 80.0, 102.5, 27.5), "🧪 Simulateur", sim),
        ] {
            puce(pm, text, r, s, on, 13.0, survol(r));
        }
    }

    /// 🛡 The Gardien's whole card (and its 🔒 block), moved down by `dy`: drawn at its own places, then laid.
    pub(super) fn garde_carte(&self, pm: &mut Pixmap, text: &Text, pw: f64, dy: f64, mouse: Option<(f64, f64)>) {
        let gd = self.cell_garde();
        let bas = BLOC_Y + gd.msg_h() + BLOC_H;
        let Some(mut sub) = crate::px::new(pw.ceil() as u32, bas.ceil() as u32) else {
            return;
        };
        self.garde_dessin(&mut sub, text, pw, &gd, mouse.map(|(x, y)| (x, y - dy)));
        crate::px::put(pm, 0.0, dy, &sub);
    }

    /// 🛡 La carte du gardien à ses places (chiffres de journal.guard_card, comme la carte du PC), puis le bloc 🔒.
    fn garde_dessin(&self, pm: &mut Pixmap, text: &Text, pw: f64, gd: &Garde, mouse: Option<(f64, f64)>) {
        let gras = Text::graisse(Graisse::Bold);
        let l = large(pw);
        let d = pw - 390.0;
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let c = &gd.c;
        let a = gd.msg_h();
        let g = Rect::new(X, GARDE_Y0, l, 203.3 + a);
        let (etat, fond) = gd.etat();
        round_fill(pm, g, 14.0, fond.0, fond.1);
        round_stroke(pm, g, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        let [_, (gt, routine)] = cell_entetes(pw, etat);
        gras.draw(pm, &gt.titre, (25.0, 157.0), 13.0, TEXTE, Align::Left);
        puce(pm, text, routine, "☀ Routine", false, 11.0, survol(routine));
        let m = |v: f64| crate::journal::money(Some(v));
        text.draw(pm, "Perte du jour", (25.0, 182.0), 12.5, TEXTE, Align::Left);
        gras.draw(
            pm,
            &format!("{} / {}", m(c.day_loss), m(c.day_lim)),
            ((25.0 + l - 24.0) as f32, 182.0),
            12.5,
            TEXTE,
            Align::Right,
        );
        let j = Rect::new(25.0, 191.1, l - 24.0, 8.0);
        round_fill(pm, j, 4.0, crate::jetons::BLANC, 0.08);
        let part = if c.day_lim > 0.0 { (c.day_loss / c.day_lim).clamp(0.0, 1.0) } else { 0.0 };
        // le seuil d'alerte du gardien (80 % par défaut) : le trait orange
        let seuil = if c.warn > 0.0 { c.warn / 100.0 } else { 0.8 };
        if part > 0.0 {
            let coul = if part >= seuil {
                ROUGE
            } else if part >= 0.5 {
                OR
            } else {
                VERT
            };
            round_fill(pm, Rect::new(j.x, j.y, (j.w * part).max(8.0), j.h), 4.0, coul, 1.0);
        }
        round_fill(pm, Rect::new(j.x + j.w * seuil.clamp(0.0, 1.0) - 1.0, 188.1, 2.0, 14.0), 1.0, OR, 1.0);
        let msg = Rect::new(25.0, 207.1, l - 24.0, 28.2);
        round_fill(pm, msg, 10.0, crate::jetons::BLANC, 0.06);
        if gd.on {
            text.draw(pm, "☑ Avant ton premier trade démo :", (37.0, 225.0), 12.0, TEXTE, Align::Left);
            let x1 = 37.0 + text.width("☑ Avant ton premier trade démo : ", 12.0) as f64;
            text.draw(
                pm,
                &text.fit("Routine du jour · bientôt", 12.0, 25.0 + l - 24.0 - 8.0 - x1),
                (x1 as f32, 225.0),
                12.0,
                LIEN,
                Align::Left,
            );
        } else {
            // gardien éteint : le toucher le rallume (même réglage que la ligne du ticket complet)
            text.draw(
                pm,
                &text.fit("🛡 Activer le Gardien PropFirm sur ce compte ›", 12.0, l - 48.0),
                (37.0, 225.0),
                12.0,
                LIEN,
                Align::Left,
            );
        }
        let cases = [
            (25.0, 241.3, "Perte max", format!("{} / {}", m(c.tot_loss), m(c.max_lim))),
            (
                197.0 + d / 2.0,
                241.3,
                "Positions",
                if c.max_pos > 0 {
                    format!("{} / {}", c.open, c.max_pos)
                } else {
                    format!("{}", c.open)
                },
            ),
            (
                25.0,
                273.7,
                "Risque max",
                if c.risk_pct > 0.0 {
                    format!("{} % ({})", crate::viewport::format_number(c.risk_pct, 1), m(c.risk_max))
                } else {
                    "—".into()
                },
            ),
        ];
        // (📱 phone view: bigger, read at arm's length; Jee, 21 Nov, list 2)
        let (pt, pv) = if crate::actions::tactile() { (11.0, 13.0) } else { (9.6, 11.5) };
        for (x, y, titre, v) in cases {
            text.draw(pm, titre, (x as f32, (y + 10.0) as f32), pt, DIM, Align::Left);
            text.draw(pm, &v, (x as f32, (y + 24.5) as f32), pv, TEXTE, Align::Left);
        }
        // ⚠ prudence : à partir du seuil d'alerte
        // (ou 🔓 la journée déverrouillée aujourd'hui, avec sa raison notée au Journal)
        if let Some((s, alerte)) = gd.message() {
            let r = Rect::new(25.0, MSG_Y, l - 24.0, gd.msg_h() - 6.0);
            round_fill(pm, r, 10.0, if alerte { OR } else { VERT }, 0.12);
            for (k, ligne) in s.lines().enumerate() {
                text.draw(
                    pm,
                    &text.fit(ligne, 11.5, r.w - 20.0),
                    (37.0, (r.y + 18.5 + 16.0 * k as f64) as f32),
                    11.5,
                    TEXTE,
                    Align::Left,
                );
            }
        }
        // (📱 phone view: bigger, and no « · bientôt » glued to the link: touching it says so)
        let (pr, plan) = if crate::actions::tactile() {
            (12.0, "📋 Mon plan ›")
        } else {
            (10.8, "📋 Mon plan › · bientôt")
        };
        text.draw(pm, "Règles :", (25.0, (320.0 + a) as f32), pr, DIM, Align::Left);
        text.draw(pm, plan, (29.0 + text.width("Règles : ", pr), (320.0 + a) as f32), pr, LIEN, Align::Left);
        // 🔒 journée verrouillée : le bloc sous la carte, puis le ticket grisé
        if let Some(bouton) = gd.deverrouiller(l) {
            let r = Rect::new(X, BLOC_Y + a, l, BLOC_H - 8.0);
            round_fill(pm, r, 14.0, ROUGE, 0.12);
            round_stroke(pm, r, 14.0, ROUGE, 1.0);
            gras.draw(
                pm,
                &text.fit("🔒 Journée verrouillée jusqu'à minuit (heure du reset)", 12.5, r.w - 24.0),
                (25.0, (r.y + 20.0) as f32),
                12.5,
                TEXTE,
                Align::Left,
            );
            let (_, nom) = gd.pire();
            let lim = if nom.ends_with("jour") { c.day_lim } else { c.max_lim };
            let l1 = format!("{} : la limite de {} est atteinte.", &nom[3..], m(lim));
            text.draw(pm, &text.fit(&l1, 11.0, r.w - 24.0), (25.0, (r.y + 37.0) as f32), 11.0, DIM, Align::Left);
            text.draw(
                pm,
                &text.fit("Le ticket est en pause ; les positions restent visibles.", 11.0, r.w - 24.0),
                (25.0, (r.y + 51.0) as f32),
                11.0,
                DIM,
                Align::Left,
            );
            puce(pm, text, bouton, "🔓 Déverrouiller aujourd'hui", false, 12.0, survol(bouton));
        }
    }

    /// 💼 « Positions et ordres » : ‹ (back to the ticket), the open demo positions and their P&L, the pending orders
    /// (each its ✕), « Tout fermer » and its detail.
    fn positions_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>) {
        let moyen = Text::graisse(Graisse::Medium);
        let t = &self.trading;
        let l = large(pw);
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let retour = Rect::new(X, RETOUR_Y, l, 30.0);
        if survol(retour) {
            round_fill(pm, retour, 12.0, crate::jetons::BLANC, 0.05);
        }
        crate::shapes::lines(pm, X + 8.0, retour.cy(), &[&[(2.5, -5.0), (-2.5, 0.0), (2.5, 5.0)]], TEXTE, 1.6);
        moyen.draw(
            pm,
            "Positions et ordres",
            ((X + 20.0) as f32, (retour.y + 20.0) as f32),
            13.5,
            TEXTE,
            Align::Left,
        );
        // les positions démo ouvertes
        text.draw(pm, "Positions", (15.0, (CARTES_Y - 11.8) as f32), 11.0, DIM, Align::Left);
        if t.open.is_empty() {
            text.draw(pm, "Aucune position ouverte", (25.0, (CARTES_Y + 23.2) as f32), 13.0, DIM, Align::Left);
        }
        for (k, (s, pnl)) in t.open.iter().enumerate() {
            let r = Rect::new(X, CARTES_Y + 45.6 * k as f64, l, 37.5);
            round_fill(pm, r, 14.0, crate::jetons::BLANC, 0.07);
            text.draw(pm, s, (25.0, (r.y + 23.5) as f32), 13.0, TEXTE, Align::Left);
            let c = if *pnl >= 0.0 { VERT } else { ROUGE };
            let v = format!("{}{}", if *pnl > 0.0 { "+" } else { "" }, crate::journal::money(Some(*pnl)));
            text.draw(pm, &v, ((r.x + r.w - 12.0) as f32, (r.y + 23.5) as f32), 13.0, c, Align::Right);
        }
        // 💼 the pending orders, each with its ✕, then « Tout fermer » and its detail (the same as the old ticket)
        let (lignes, fermer) = self.cell_ordres(pw);
        if !lignes.is_empty() {
            let y0 = lignes[0].0.y;
            text.draw(pm, "Ordres en attente", (15.0, (y0 - 8.0) as f32), 11.0, DIM, Align::Left);
        }
        for ((r, croix), (_, s)) in lignes.iter().zip(&t.pending) {
            round_fill(pm, *r, r.h / 2.0, crate::jetons::BLANC, 0.07);
            text.draw(
                pm,
                &text.fit(s, 13.0, r.w - 24.0 - croix.w),
                (25.0, (r.y + 25.0) as f32),
                13.0,
                TEXTE,
                Align::Left,
            );
            if survol(*croix) {
                round_fill(pm, *croix, croix.h / 2.0, crate::jetons::BLANC, 0.1);
            }
            text.draw(pm, "✕", (croix.cx() as f32, (croix.y + 25.0) as f32), 14.0, DIM, Align::Center);
        }
        if let Some(r) = fermer {
            round_fill(pm, r, r.h / 2.0, ROUGE, if survol(r) { 0.28 } else { 0.18 });
            moyen.draw(pm, "Tout fermer", (r.cx() as f32, (r.y + 25.0) as f32), 13.5, ROUGE, Align::Center);
            for (k, d) in self.cell_fermer_detail(text, r.w).iter().enumerate() {
                let y = r.y + r.h + 14.0 + k as f64 * DETAIL_LIGNE;
                text.draw(pm, d, (X as f32, y as f32), DETAIL_PX, DIM, Align::Left);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Kind, PanelClick, Panels};
    use super::*;

    /// The sheet's zone for `hit` (sheet `pw` wide).
    fn zone(p: &Panels, pw: f64, hit: Hit) -> Option<Rect> {
        p.cell_zones(pw).into_iter().find(|(_, h)| *h == hit).map(|(r, _)| r)
    }

    /// 💼 « Positions et ordres » (phone, and the PC panel) lists the pending demo orders, each with its ✕ (40 points),
    /// and « Tout fermer », with the same targets as the old ticket: ✕ cancels that order, « Tout fermer » closes all;
    /// none overlaps, all inside the sheet, the detail never cut (on two lines when the panel is 400 wide).
    #[test]
    fn ordres_en_attente_et_tout_fermer_dans_la_feuille() {
        for pw in [390.0, 400.0, 402.0] {
            let mut p = Panels::default();
            (p.trading.cell, p.trading.positions_vue) = (true, true);
            assert!(p.cell_ordres(pw).1.is_none(), "nothing to close: no button");
            p.trading.open = vec![("Long BTC 0,010".into(), 12.5)];
            p.trading.pending = vec![(7, "Buy Limit BTC @ 62 300,00".into()), (9, "Sell Stop BTC @ 61 000,00".into())];
            let z = p.cell_zones(pw);
            let cibles: Vec<(Rect, Hit)> = z.iter().filter(|(_, h)| matches!(h, Hit::CancelOrder(_) | Hit::CloseAll)).copied().collect();
            assert_eq!(cibles.len(), 3, "two ✕ and « Tout fermer »");
            for (i, (r, _)) in cibles.iter().enumerate() {
                assert!(r.w >= 40.0 && r.h >= 40.0, "{r:?}: 40 points");
                assert!(r.x >= 0.0 && r.x + r.w <= pw + 0.5, "{r:?} inside the sheet ({pw})");
                for (q, _) in &cibles[i + 1..] {
                    assert!(r.y + r.h + 8.0 <= q.y + 0.01 || q.y + q.h + 8.0 <= r.y + 0.01, "{r:?} and {q:?} apart");
                }
            }
            let bas = p.cell_bas();
            assert!(cibles.iter().all(|(r, _)| r.y + r.h < bas), "inside the view's content");
            let t = Text::default();
            let (_, fermer) = p.cell_ordres(pw);
            let fermer = fermer.unwrap();
            let detail = p.cell_fermer_detail(&t, fermer.w);
            assert!(!detail.iter().any(|d| d.contains('…')), "never cut: {detail:?}");
            assert!(detail.iter().all(|d| t.width(d, DETAIL_PX) as f64 <= fermer.w), "{detail:?} fits ({pw})");
            let at = |p: &mut Panels, r: Rect| p.click(Kind::Trading, r.cx(), r.cy(), pw, 1200.0);
            let croix = cibles.iter().find(|(_, h)| *h == Hit::CancelOrder(1)).unwrap().0;
            assert_eq!(at(&mut p, croix), PanelClick::CancelOrder(9), "✕ of the 2nd order: its id");
            assert_eq!(at(&mut p, fermer), PanelClick::CloseAll, "Tout fermer");
            let mut pm = crate::px::new(pw as u32, p.cell_haut(pw) as u32).unwrap();
            p.cell_draw(&mut pm, &t, pw, None);
        }
    }

    /// 💼 Only pending orders (no position): « Tout fermer » is there and answers CloseAll; ten orders on a
    /// 390 × 760 sheet: the view grows past the screen (it scrolls), each ✕ keeps its own order's id, the 40-point
    /// targets never overlap; ‹ (above the list) and 🎫 Ticket bring the ticket back.
    #[test]
    fn ordres_seuls_et_dix_ordres_dans_la_feuille() {
        let pw = 390.0;
        let mut p = Panels::default();
        p.trading.cell = true;
        let positions = zone(&p, pw, Hit::Cell(Cell::Positions)).expect("« Positions et ordres › »");
        assert_eq!(p.click(Kind::Trading, positions.cx(), positions.cy(), pw, 1200.0), PanelClick::Changed);
        assert!(p.trading.positions_vue);
        p.trading.pending = vec![(4, "Buy Limit BTC @ 60 000,00".into())];
        let fermer = p.cell_ordres(pw).1.expect("orders only: « Tout fermer » is there");
        assert_eq!(p.click(Kind::Trading, fermer.cx(), fermer.cy(), pw, 1200.0), PanelClick::CloseAll);
        p.trading.pending = (0..10).map(|k| (100 + k as u64, format!("Sell Stop ETH @ {} 000,00", 3 + k))).collect();
        assert!(p.cell_haut(pw) > 760.0, "taller than the view: the sheet scrolls ({})", p.cell_haut(pw));
        let z = p.cell_zones(pw);
        let cibles: Vec<(Rect, Hit)> = z.iter().filter(|(_, h)| matches!(h, Hit::CancelOrder(_) | Hit::CloseAll)).copied().collect();
        assert_eq!(cibles.len(), 11);
        for (i, (r, _)) in cibles.iter().enumerate() {
            assert!(r.w >= 40.0 && r.h >= 40.0 && r.x + r.w <= pw + 0.5, "{r:?}");
            for (q, _) in &cibles[i + 1..] {
                assert!(r.y + r.h + 8.0 <= q.y + 0.01 || q.y + q.h + 8.0 <= r.y + 0.01, "{r:?} / {q:?}");
            }
        }
        let h = p.cell_haut(pw);
        for (r, hit) in &cibles {
            if let Hit::CancelOrder(i) = hit {
                assert_eq!(
                    p.click(Kind::Trading, r.cx(), r.cy(), pw, h),
                    PanelClick::CancelOrder(100 + *i as u64),
                    "✕ {i}: its id"
                );
            }
        }
        let retour = zone(&p, pw, Hit::Cell(Cell::Retour)).expect("‹");
        assert!(retour.y + retour.h < cibles[0].0.y, "‹ above the orders");
        p.click(Kind::Trading, retour.cx(), retour.cy(), pw, h);
        assert!(!p.trading.positions_vue, "‹: the ticket");
        p.trading.positions_vue = true;
        p.click(Kind::Trading, 50.0, 93.0, pw, h);
        assert!(!p.trading.positions_vue, "🎫 Ticket: the ticket");
    }

    /// 📱 L'en-tête et le ticket de la feuille Trading répondent : ✕ ferme, 🧪 ouvre le Simulateur, VENDRE / ACHETER
    /// passent l'ordre démo du bon côté, Marché / Limite / Stop changent l'onglet, « ⚙ Tous les réglages » ouvre
    /// l'ancien ticket complet ; ce qui n'existe pas encore dit « bientôt ».
    #[test]
    fn feuille_trading_du_cell() {
        crate::actions::set_tactile(false);
        let mut p = Panels::default();
        p.trading.cell = true;
        let pw = 390.0;
        let z = p.cell_zones(pw);
        assert!(z.len() >= 20, "{}", z.len());
        for (r, _) in &z {
            assert!(r.x >= 0.0 && r.x + r.w <= pw + 0.5, "{r:?} dans la feuille");
        }
        let touche = |p: &mut Panels, hit: Hit| {
            let r = zone(p, pw, hit).unwrap_or_else(|| panic!("{hit:?}"));
            p.click(Kind::Trading, r.cx(), r.cy(), pw, 900.0)
        };
        assert_eq!(p.click(Kind::Trading, 360.0, 53.0, pw, 900.0), PanelClick::Fermer, "✕");
        assert_eq!(p.click(Kind::Trading, 140.0, 93.0, pw, 900.0), PanelClick::Simulateur, "🧪");
        assert_eq!(touche(&mut p, Hit::Cell(Cell::Passer(false))), PanelClick::Passer(false), "VENDRE");
        assert!(!p.trading.buy);
        assert_eq!(touche(&mut p, Hit::Cell(Cell::Passer(true))), PanelClick::Passer(true), "ACHETER");
        assert!(p.trading.buy);
        touche(&mut p, Hit::Tab(1));
        assert_eq!(p.trading.tab, 1, "Limite");
        touche(&mut p, Hit::Cell(Cell::Gardien));
        assert!(matches!(touche(&mut p, Hit::Cell(Cell::Routine)), PanelClick::Soon(_)), "Routine : bientôt");
        assert_eq!(touche(&mut p, Hit::Cell(Cell::Reglages)), PanelClick::Changed);
        assert!(p.trading.options, "⚙ Tous les réglages : l'ancien ticket");
        assert!(!p.cell_trading());
    }

    /// 📱 Jee (20 Nov): on the phone and the touch tablet the Trading sheet has no ⬓ ⤢ (big screen only: they just
    /// said so), neither drawn nor touchable; ✕ stays. The PC sheet keeps ⬓ ⤢ ✕.
    #[test]
    fn feuille_trading_sans_ancrer_agrandir_sur_cell() {
        let mut p = Panels::default();
        p.trading.cell = true;
        let pw = 390.0;
        let a_les_deux = |p: &Panels| {
            let z = p.cell_zones(pw);
            (
                z.iter().any(|(_, h)| *h == Hit::Cell(Cell::Ancrer)),
                z.iter().any(|(_, h)| *h == Hit::Cell(Cell::Agrandir)),
            )
        };
        let dessin = |p: &Panels| {
            let mut pm = Pixmap::new(pw as u32, 120).unwrap();
            p.cell_draw(&mut pm, &Text::default(), pw, None);
            // the ⤢ button's round (269 + 38, 38, 30 × 30): painted or not
            let px = pm.pixel(322, 53).unwrap();
            px.alpha() > 0
        };
        crate::actions::set_tactile(false);
        assert_eq!(a_les_deux(&p), (true, true), "PC");
        assert!(dessin(&p), "⤢ drawn on the PC");
        assert_eq!(p.click(Kind::Trading, 322.0, 53.0, pw, 600.0), PanelClick::Agrandir);
        crate::actions::set_tactile(true);
        assert_eq!(a_les_deux(&p), (false, false), "cell");
        assert!(!dessin(&p), "⤢ not drawn on the cell");
        assert_ne!(p.click(Kind::Trading, 322.0, 53.0, pw, 600.0), PanelClick::Agrandir);
        assert_ne!(p.click(Kind::Trading, 284.0, 53.0, pw, 600.0), PanelClick::Ancrer);
        assert_eq!(p.click(Kind::Trading, 360.0, 53.0, pw, 600.0), PanelClick::Fermer, "✕ stays");
        crate::actions::set_tactile(false);
    }

    /// 🛡 Le Gardien du ticket : une ligne, touchée sa carte complète (son titre la replie). Éteint, la ligne sous la
    /// jauge le rallume ; à 82 % de la perte du jour, le message ⚠ fait descendre le ticket ; journée verrouillée,
    /// la carte reste ouverte avec le bloc 🔒 et son bouton 🔓 Déverrouiller, et le ticket ne répond plus.
    #[test]
    fn carte_gardien_du_cell() {
        use crate::journal::now_ms;
        crate::actions::set_tactile(false);
        let mut p = Panels::default();
        p.trading.cell = true;
        let pw = 390.0;
        let l = large(pw);
        let at = |p: &mut Panels, r: Rect| p.click(Kind::Trading, r.cx(), r.cy(), pw, 900.0);
        let touche = |p: &mut Panels, hit: Hit| {
            let r = zone(p, pw, hit).unwrap_or_else(|| panic!("{hit:?}"));
            p.click(Kind::Trading, r.cx(), r.cy(), pw, 900.0)
        };
        assert!(!p.trading.guardian && zone(&p, pw, Hit::Guardian).is_none(), "folded: one line");
        let ligne = zone(&p, pw, Hit::Cell(Cell::Gardien)).unwrap();
        assert_eq!((ligne.y, ligne.h), (155.5, 29.0));
        at(&mut p, ligne);
        assert!(p.trading.garde_ouverte);
        let dy = p.ticket_places(pw).gard - GARDE_Y0;
        assert_eq!(p.cell_garde().decale(), 0.0);
        let activer = zone(&p, pw, Hit::Guardian).unwrap();
        assert_eq!(activer.y, 207.1 + dy);
        at(&mut p, activer);
        assert!(p.trading.guardian, "🛡 Activer");
        assert!(matches!(at(&mut p, activer), PanelClick::Soon(_)), "allumé : la ligne est la Routine du jour");
        assert!(p.trading.guardian);
        // 4 100 $ de perte flottante sur 5 000 $ : 82 %, prudence
        let types = p.ticket_places(pw).types;
        p.trading.open = vec![("BTC".into(), -4_100.0)];
        let g = p.cell_garde();
        assert!(g.alerte().unwrap().contains("82 %"), "{:?}", g.alerte());
        assert_eq!(g.decale(), MSG_H);
        assert_eq!(p.ticket_places(pw).types, types + MSG_H, "the ticket goes down under the message");
        assert_eq!(touche(&mut p, Hit::Cell(Cell::Passer(false))), PanelClick::Passer(false));
        assert!(matches!(touche(&mut p, Hit::Cell(Cell::Plan)), PanelClick::Soon(_)), "📋 Mon plan");
        // journée verrouillée : la carte s'ouvre d'elle-même
        p.trading.open.clear();
        p.trading.garde_ouverte = false;
        p.journal.cfg.lock = p.journal.propfirm_day(now_ms());
        let g = p.cell_garde();
        assert!(g.verrou() && g.alerte().is_none());
        assert_eq!(touche(&mut p, Hit::GuardUnlock), PanelClick::Unlock, "🔓 Déverrouiller");
        assert!(
            zone(&p, pw, Hit::Cell(Cell::Passer(true))).is_none(),
            "ACHETER bloqué tant que c'est verrouillé"
        );
        let go = p.ticket_places(pw).go.unwrap();
        let acheter = Rect::new(X + (l + 8.0) / 2.0, go, (l - 8.0) / 2.0, 54.0);
        assert_ne!(at(&mut p, acheter), PanelClick::Passer(true));
        assert!(p.ticket_places(pw).types > 155.5 + 203.3 + BLOC_H);
        // 🔓 déverrouillée avec une raison : la ligne dans la carte, le ticket répond à nouveau
        p.journal.unlock("Erreur de saisie corrigée", now_ms()).unwrap();
        p.trading.garde_ouverte = true;
        let g = p.cell_garde();
        let (m, alerte) = g.message().expect("ligne 🔓");
        assert!(
            !alerte && m.contains("Déverrouillée aujourd'hui") && m.contains("« Erreur de saisie corrigée »"),
            "{m}"
        );
        assert!(!g.verrou() && g.decale() == RAISON_H);
        assert_eq!(touche(&mut p, Hit::Cell(Cell::Passer(true))), PanelClick::Passer(true));
        // its title folds it
        touche(&mut p, Hit::Cell(Cell::Gardien));
        assert!(!p.trading.garde_ouverte);
        // 📸 ARGOS_GARDIEN_PNG=dossier : la carte déverrouillée sur cell (390) et sur PC (380), pour la regarder
        p.trading.garde_ouverte = true;
        if let Ok(dir) = std::env::var("ARGOS_GARDIEN_PNG") {
            let (t, th) = (crate::text::Text::default(), crate::Theme::default());
            for (w, nom) in [(390.0, "cell"), (380.0, "pc")] {
                let mut pm = crate::px::new(w as u32, 900).unwrap();
                p.draw(Kind::Trading, &mut pm, &t, (w, 900.0), None, &th);
                pm.save_png(format!("{dir}/gardien-raison-{nom}.png")).unwrap();
            }
        }
        // 🧊 anti-tilt pause: the minutes left in the card (before the ⚠ warning) and on the line; over: gone
        p.trading.open = vec![("BTC".into(), -4_100.0)];
        let now = now_ms();
        p.journal.cfg.pause_until = now + 12 * 60_000 + 30_000;
        let g = p.cell_garde();
        let (m, alerte) = g.message().expect("🧊 line");
        assert!(alerte && m.contains("encore 13 min") && m.contains("aucun nouvel ordre"), "{m}");
        assert_eq!(g.ligne(), "pause anti-tilt · 13 min");
        assert_eq!(g.decale(), MSG_H);
        p.journal.cfg.pause_until = now - 1;
        assert!(p.cell_garde().message().unwrap().0.contains("82 %"), "pause over: the ⚠ warning again");
        assert_eq!(p.cell_garde().ligne(), "prudence");
        // gardien éteint (réglage du ticket complet) : ni bloc ni décalage
        p.trading.guardian = false;
        assert_eq!((p.cell_garde().decale(), p.cell_garde().ligne()), (0.0, "éteint".to_string()));
    }

    /// 📱 Jee (21 Nov, list 2): on the phone the Gardien's card has no « · bientôt » glued to « Mon plan » (touching it
    /// says so); the PC keeps it.
    #[test]
    fn feuille_trading_textes_au_doigt() {
        let mut p = Panels::default();
        (p.trading.cell, p.trading.garde_ouverte) = (true, true);
        let bientot = |p: &Panels, pw: f64| {
            let mut pm = Pixmap::new(pw as u32, p.cell_haut(pw) as u32 + 40).unwrap();
            let _ = crate::text::ecrits();
            p.cell_draw(&mut pm, &Text::default(), pw, None);
            crate::text::ecrits().iter().any(|s| s.contains("· bientôt"))
        };
        crate::actions::set_tactile(false);
        assert!(bientot(&p, 402.0), "PC");
        crate::actions::set_tactile(true);
        assert!(!bientot(&p, 402.0), "phone");
        crate::actions::set_tactile(false);
    }

    /// 🖥 The Simulator in the sheet's place (PC volet, its 🧪 tab): 🧪 is lit and answers nothing more, 🎫 Ticket closes
    /// it; under the header nothing of the ticket is drawn nor touched (the Simulator is there).
    #[test]
    fn feuille_trading_avec_le_simulateur_dans_son_onglet() {
        crate::actions::set_tactile(false);
        let mut p = Panels::default();
        p.trading.cell = true;
        let pw = 418.0;
        let text = Text::default();
        let dessin = |p: &Panels| {
            let mut pm = Pixmap::new(pw as u32, 900).unwrap();
            p.cell_draw(&mut pm, &text, pw, None);
            pm
        };
        let avant = dessin(&p);
        p.trading.sim_onglet = true;
        let apres = dessin(&p);
        assert_eq!(p.click(Kind::Trading, 140.0, 93.0, pw, 900.0), PanelClick::None, "🧪 already shown");
        assert_eq!(p.click(Kind::Trading, 40.0, 93.0, pw, 900.0), PanelClick::FermerSimulateur, "🎫 Ticket");
        assert_eq!(p.cell_zones(pw), p.entete_zones(pw), "only the header answers");
        assert_eq!(p.content_h(Kind::Trading, pw), super::super::ticket::ENTETE, "nothing to scroll under it");
        let w = pw as usize;
        let rangee = |pm: &Pixmap, y: usize, x0: usize, x1: usize| pm.pixels()[y * w + x0..y * w + x1].to_vec();
        assert!(
            apres.pixels()[120 * w..].iter().all(|c| c.alpha() == 0),
            "nothing of the ticket under the header"
        );
        assert!(avant.pixels()[120 * w..].iter().any(|c| c.alpha() > 0), "(the ticket was there)");
        // the two tabs swap their light (🎫 13 → 87.5, 🧪 93.5 → 196, from 80 to 107.5)
        assert_ne!(rangee(&avant, 84, 20, 80), rangee(&apres, 84, 20, 80), "🎫 unlit");
        assert_ne!(rangee(&avant, 84, 100, 190), rangee(&apres, 84, 100, 190), "🧪 lit");
    }
}
