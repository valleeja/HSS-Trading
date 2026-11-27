//! 🧪 Volet « Simulateur » (en verre, à droite) et 🟠 bandeau DÉMO toujours visible.
//!
//! Le volet : compte démo (solde, équité, P&L du jour, ↺ remise à zéro avec confirmation), ticket (Achat / Vente,
//! Marché / Limite / Stop, risque en %, SL en % du prix, R:R → TP ; SL et TP toujours posés), positions et ordres,
//! ⏯ replay (×1 ×2 ×5, pause, bougie suivante), scénario d'examen (2 étapes classique, 1 étape serré, personnalisé)
//! avec le verdict et la leçon à revoir, courbe d'équité, statistiques et historique.
//! Tout se choisit au doigt (puces), rien à taper. Les calculs sont dans `argos_core::simu` et
//! `argos_propfirm::examen` ; ici : l'état de l'écran, le dessin et une table de zones pour les clics.

use crate::palette::{pnl as couleur, GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::couts::{estimer, pct_txt as taux_txt, Couts};
use argos_core::perp::{self as perp, Bareme, Perp};
use argos_core::portfolio::{ClosedTrade, Gestion, OrderKind, Palier, Portfolio, Stats};
use argos_core::simu::{Kind, Replay, Ticket};
use argos_core::Money;
use argos_core::Side;
use argos_propfirm::examen::{Etat, Regles, Scenario, Verdict, FUSEAUX};
use tiny_skia::Pixmap;

/// Risque par trade (% du compte).
pub const RISQUES: [f64; 4] = [0.25, 0.5, 1.0, 2.0];
/// Distance du SL (% du prix d'entrée).
pub const SL_PCT: [f64; 4] = [0.1, 0.25, 0.5, 1.0];
/// R:R → TP.
pub const RR: [f64; 3] = [1.5, 2.0, 3.0];
/// 🎯 Sorties proposées au ticket : un seul objectif, la moitié à 1 R, ou un tiers à 1 R et un tiers à 2 R (le reste
/// à l'objectif final).
pub const SORTIES: [&str; 3] = ["1 objectif", "½ à 1R", "⅓ 1R · ⅓ 2R"];
/// Écart du prix d'un ordre Limite / Stop (% du dernier prix).
pub const ECARTS: [f64; 3] = [0.1, 0.25, 0.5];
/// ⚡ Taux de funding du replay (pour 8 heures) : aucun, la moitié du taux de base, le taux de base (0,01 %), un
/// marché acheteur emballé, un marché vendeur (les vendeurs paient).
pub const FUNDING_REPLAY: [f64; 5] = [0.0, 0.00005, 0.0001, 0.0003, -0.0001];
/// Réglages du scénario personnalisé (cible, perte du jour, perte totale, jours min).
const P_CIBLE: [f64; 4] = [5.0, 8.0, 10.0, 12.0];
const P_JOUR: [f64; 4] = [2.0, 3.0, 4.0, 5.0];
const P_TOTAL: [f64; 4] = [6.0, 8.0, 10.0, 12.0];
const P_JOURS: [u32; 4] = [0, 2, 4, 5];

/// Hauteur de l'en-tête du volet (titre, ✕), au-dessus du contenu qui défile.
pub const TETE: f64 = 58.0;
const PAD: f64 = 16.0;
const PUCE: f64 = 30.0;
const LIGNE: f64 = 24.0;
const TITRE: f64 = 34.0;
const COURBE: f64 = 90.0;
/// 📱 Hauteur d'une ligne de rubrique repliable (au doigt : 44).
const RUBRIQUE: f64 = 44.0;
/// 🤖 Une ligne de l'Historique (maquette du lot 7) : le rond Argos AI, le trade, son détail dessous ; 44 au doigt.
const HIST: f64 = 44.0;
/// Ligne de détail de plus, quand le détail passe à la ligne (📱 étroit).
const HIST_SUITE: f64 = 15.0;
/// Le rond Argos AI d'une ligne de l'Historique, et où commence le texte.
const HIST_ROND: f64 = 24.0;
const HIST_X: f64 = 10.0 + 8.0 + HIST_ROND + 9.0;

/// 📱 Les rubriques repliées au doigt (une ligne › chacune, ouverte d'un toucher) ; le PC montre tout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Rub {
    Objectifs,
    Couts,
    Detail,
    Replay,
    Examen,
    Courbe,
}

/// Ce qu'un clic demande.
#[derive(Debug, Clone, PartialEq)]
pub enum SimClick {
    None,
    /// Un réglage du volet a changé (à redessiner, à garder).
    Changed,
    Close,
    Place,
    /// 🛡️ Allumer / éteindre le Gardien PropFirm (le réglage du volet Trading).
    Gardien,
    Cancel(u64),
    CloseTrade(u64),
    /// ✂ Sortir la moitié de cette position au prix actuel.
    Moitie(u64),
    /// ⚖ Monter le stop de cette position à son prix d'entrée, tout de suite.
    StopEntree(u64),
    /// 💸 « Sans frais » (`true`) ou coûts réels de la bourse (`false`) pour les nouveaux ordres.
    SansFrais(bool),
    /// ↺ confirmé : remettre le compte à zéro.
    Reset,
    ReplayStart,
    ReplayPlay,
    ReplayStep,
    ReplaySpeed,
    ReplayStop,
    /// 📅 Rejouer à partir d'une date choisie (ouvre la fenêtre « Aller à »).
    ReplayDate,
    /// 💾 Garder la séance de replay pour plus tard et revenir au direct.
    SeanceSauver,
    /// ▶ Reprendre la séance gardée.
    SeanceReprendre,
    /// 🗑 Oublier la séance gardée : ses positions ferment au dernier prix de la séance, ses ordres sont annulés.
    SeanceOublier,
    /// Ouvrir cette leçon de l'Académie.
    Lecon(&'static str),
    /// 🤖 Ouvrir (ou refermer) la carte Argos AI de ce trade : l'index de sa dernière sortie dans l'historique.
    Avis(usize),
    /// 📅 Open (or close) the « Rapport du mois » page.
    MonthReport,
}

/// Zones cliquables.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    ResetAsk,
    ResetOui,
    ResetNon,
    Sens(bool),
    Genre(Kind),
    Risque(usize),
    Sl(usize),
    Rr(usize),
    Ecart(usize),
    Sorties(usize),
    Be,
    Suiveur,
    Oco,
    Moitie(u64),
    StopEntree(u64),
    SansFrais(bool),
    /// ⚡ Levier (index dans `perp::leviers` du marché ; `usize::MAX` : l'étiquette).
    Levier(usize),
    /// ⚡ Taux de funding du replay (index dans `FUNDING_REPLAY` ; `usize::MAX` : l'étiquette).
    FundingRp(usize),
    Placer,
    Gardien,
    Annuler(u64),
    Sortir(u64),
    RpStart,
    RpPlay,
    RpStep,
    RpSpeed,
    RpStop,
    RpDate,
    SeSauver,
    SeReprendre,
    SeOublier,
    Scen(usize),
    Fuseau,
    PCible,
    PJour,
    PTotal,
    PJours,
    Lecon(&'static str),
    /// 📱 Ouvrir / replier une rubrique (au doigt).
    Rub(Rub),
    /// 🤖 Une ligne de l'Historique : la carte Argos AI de ce trade.
    Avis(usize),
    /// 📅 « Rapport du mois ».
    Month,
}

/// L'état du volet (ce qui est choisi à l'écran).
#[derive(Debug, Clone, PartialEq)]
pub struct Simulateur {
    pub buy: bool,
    pub kind: Kind,
    pub risque: usize,
    pub sl: usize,
    pub rr: usize,
    pub ecart: usize,
    /// Scénario d'examen choisi (index dans `Scenario::ALL`) et ses règles.
    pub scen: usize,
    pub regles: Regles,
    /// ↺ demandé : on attend « Oui » / « Non ».
    pub confirme: bool,
    pub scroll: f64,
    /// 🎯 Sorties (index dans `SORTIES`), ⚖ break-even à 1 R, 🪜 stop suiveur à 1 R, 🔀 OCO (ordres Stop : les deux
    /// côtés de la cassure).
    pub sorties: usize,
    pub be: bool,
    pub suiveur: bool,
    pub oco: bool,
    /// ⚡ Levier choisi pour un perpétuel (plafonné au maximum du marché) et taux de funding du replay (index dans
    /// `FUNDING_REPLAY`).
    pub levier: f64,
    pub funding_rp: usize,
    /// 📱 Rubriques ouvertes au doigt (repliées au départ).
    pub ouvertes: Vec<Rub>,
    /// 📅 The « Rapport du mois » page, when open (closing the panel closes it).
    pub month: Option<crate::month::MonthPage>,
}

impl Default for Simulateur {
    fn default() -> Self {
        Self {
            buy: true,
            kind: Kind::Market,
            risque: 2,
            sl: 2,
            rr: 1,
            ecart: 1,
            scen: 0,
            regles: Regles::of(Scenario::DeuxEtapes),
            confirme: false,
            scroll: 0.0,
            sorties: 0,
            be: false,
            suiveur: false,
            oco: false,
            levier: 10.0,
            funding_rp: 2,
            ouvertes: vec![],
            month: None,
        }
    }
}

/// ⚡ Le marché ouvert est un perpétuel : son barème (levier maximum, marge de maintien, intervalle du funding) et le
/// taux de funding utilisé, pour 8 heures.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerpVue {
    pub bareme: Bareme,
    pub taux_8h: f64,
    /// Le taux vient de la bourse (`true`) ; sinon le taux de base (0,01 % / 8 h), ou celui choisi pour le replay.
    pub reel: bool,
}

/// Ce que le volet montre (calculé par la fenêtre).
pub struct Vue<'a> {
    /// Le compte démo (le même que le volet Trading).
    pub acc: &'a Portfolio,
    /// Prix actuel d'un symbole du compte (la chart, sinon la watchlist).
    pub prix_de: &'a dyn Fn(&str) -> Option<f64>,
    /// 🛡️ Gardien PropFirm allumé (le réglage du volet Trading).
    pub gardien: bool,
    /// Dernier prix (clôture de la dernière bougie jouée) et son symbole.
    pub last: f64,
    pub symbole: &'a str,
    /// 🔢 Décimales des prix de ce marché, celles de la chart (la précision vue dans ses bougies).
    pub dec: usize,
    pub jour: Money,
    pub examen: Option<&'a Etat>,
    pub replay: Option<Replay>,
    /// Bougies jouées / toutes (replay).
    pub total: usize,
    /// 💾 Séance de replay gardée (« BTC · 15m · 12 mars 2026 14:00 · 2 ouverts »), à reprendre.
    pub seance: Option<String>,
    /// 📱 Bande gardée au bas du volet, sous le rond Argos AI posé par-dessus : le contenu défile au-dessus d'elle,
    /// jamais sous le rond (Suivi 5 : il couvrait les Statistiques pendant le défilement).
    pub marge_bas: f64,
    /// 💸 La bourse du marché ouvert et son barème (frais maker / taker, écart, glissement) ; `None` : pas de barème
    /// pour ce marché (symbole calculé, fichier…), les ordres sont sans frais.
    pub couts: Option<(&'a str, Couts)>,
    /// ⚡ Le marché ouvert est un perpétuel (levier, marge isolée, liquidation, funding) ; `None` : au comptant.
    pub perp: Option<PerpVue>,
    /// 🤖 Le trade dont la carte « Argos AI · ton trade » est ouverte (index de sa dernière sortie dans l'historique) :
    /// sa ligne de l'Historique est mise en avant.
    pub avis: Option<usize>,
    /// 🖥 In the Trading volet, under its header (PC: the 🧪 tab of the module 31 mock-up): neither a background nor a
    /// header of its own (title, ✕): the volet's; the content starts at the top.
    pub en_volet: bool,
}

/// 🎛 « 🎯 2 objectifs partiels · ⚖ BE à 1R · 🪜 suiveur » : la gestion d'une position, en court (vide sans gestion).
pub fn gestion_txt(g: &Gestion) -> String {
    let mut l: Vec<String> = vec![];
    match g.paliers.len() {
        0 => {}
        1 => l.push("🎯 1 objectif partiel".into()),
        n => l.push(format!("🎯 {n} objectifs partiels")),
    }
    if let Some(r) = g.be_r {
        l.push(format!("⚖ BE à {}R", r.to_string().replace('.', ",")));
    }
    if g.suiveur.is_some() {
        l.push("🪜 suiveur".into());
    }
    l.join(" · ")
}

/// « 100 000,00 $ » (centimes exacts), avec le signe si `signe`.
pub fn argent(m: Money, signe: bool) -> String {
    let c = m.cents();
    let (neg, a) = (c < 0, c.unsigned_abs());
    let ent = (a / 100).to_string();
    let mut g = String::new();
    for (i, ch) in ent.chars().enumerate() {
        if i > 0 && (ent.len() - i) % 3 == 0 {
            g.push(' ');
        }
        g.push(ch);
    }
    let s = if neg {
        "−"
    } else if signe && c > 0 {
        "+"
    } else {
        ""
    };
    format!("{s}{g},{:02} $", a % 100)
}

impl Vue<'_> {
    /// 🔢 Un prix du marché de la chart, écrit comme la chart l'écrit (« 53 836,5 » : sa précision, le format 🌐).
    fn prix(&self, p: f64) -> String {
        crate::viewport::format_price(p, self.dec)
    }

    /// 🔢 Un prix de `base` : comme la chart pour son marché ; un autre marché, 2 à 5 décimales selon sa taille.
    fn prix_de(&self, base: &str, p: f64) -> String {
        if base == self.symbole {
            return self.prix(p);
        }
        let d = if p >= 1000.0 {
            2
        } else if p >= 10.0 {
            3
        } else {
            5
        };
        crate::viewport::format_price(p, d)
    }
}

fn pct_txt(v: f64) -> String {
    format!("{v} %").replace('.', ",")
}

/// ⚡ Un taux de funding en % (« 0,01 % », « −0,0013 % », « 0 % ») : 4 décimales au plus, sans zéros inutiles.
pub fn funding_txt(taux: f64) -> String {
    let t = format!("{:.4}", (taux * 100.0).abs());
    let t = t.trim_end_matches('0').trim_end_matches('.');
    let signe = if taux < 0.0 && t != "0" { "−" } else { "" };
    format!("{signe}{t} %").replace('.', ",")
}

impl Simulateur {
    /// Le volet dans une fenêtre `w` × `h` (sous la barre du haut `top`) : 420 px à droite, plein écran sur téléphone.
    pub fn rect(w: f64, h: f64, top: f64) -> Rect {
        let vw = if w < 600.0 { w } else { 420.0f64.min(w) };
        Rect::new(w - vw, top, vw, (h - top).max(100.0))
    }

    /// Le ticket au dernier prix `last` pour un compte de `solde` : entrée, SL et TP calculés depuis les puces.
    pub fn ticket(&self, last: f64, solde: Money) -> Ticket {
        let e = ECARTS[self.ecart] / 100.0 * last;
        // Limite : meilleur prix que le marché ; Stop : sur la cassure
        let entry = match (self.kind, self.buy) {
            (Kind::Market, _) => last,
            (Kind::Limit, true) | (Kind::Stop, false) => last - e,
            (Kind::Limit, false) | (Kind::Stop, true) => last + e,
        };
        let d = SL_PCT[self.sl] / 100.0 * entry;
        let s = if self.buy { 1.0 } else { -1.0 };
        Ticket {
            buy: self.buy,
            kind: self.kind,
            price: entry,
            sl: entry - s * d,
            tp: entry + s * d * RR[self.rr],
            risk: solde.max(Money::ZERO).pct(RISQUES[self.risque]),
        }
    }

    /// 🎛 La gestion choisie au ticket pour `t` : objectifs partiels à 1 R / 2 R (seulement ceux avant l'objectif
    /// final), break-even à 1 R, stop suiveur à 1 R de distance.
    pub fn gestion(&self, t: &Ticket) -> Gestion {
        let r = (t.price - t.sl).abs();
        let s = if t.buy { 1.0 } else { -1.0 };
        let parts: &[(f64, f64)] = match self.sorties {
            1 => &[(1.0, 0.5)],
            2 => &[(1.0, 1.0 / 3.0), (2.0, 1.0 / 3.0)],
            _ => &[],
        };
        Gestion {
            be_r: self.be.then_some(1.0),
            suiveur: (self.suiveur && r > 0.0).then_some(r),
            paliers: parts
                .iter()
                .map(|&(n, part)| Palier {
                    prix: t.price + s * n * r,
                    part,
                })
                .filter(|p| (t.tp - p.prix) * s > 0.0)
                .collect(),
        }
    }

    /// 🔀 Les deux tickets d'un OCO (ordre Stop, OCO choisi) : un Buy Stop au-dessus et un Sell Stop dessous, au même
    /// écart, même SL, même R:R et même risque. `None` hors OCO.
    pub fn tickets_oco(&self, last: f64, solde: Money) -> Option<(Ticket, Ticket)> {
        if !(self.oco && self.kind == Kind::Stop) {
            return None;
        }
        let (mut a, mut b) = (self.clone(), self.clone());
        (a.buy, b.buy) = (true, false);
        Some((a.ticket(last, solde), b.ticket(last, solde)))
    }

    /// ⚡ Les conditions perpétuelles des ordres du ticket (levier choisi, plafonné au maximum du marché) ; `None` :
    /// marché au comptant.
    pub fn perp(&self, v: &Vue) -> Option<Perp> {
        v.perp.map(|p| p.bareme.perp(self.levier))
    }

    /// ⚡ Le taux de funding choisi pour le replay (pour 8 heures).
    pub fn taux_replay(&self) -> f64 {
        FUNDING_REPLAY[self.funding_rp.min(FUNDING_REPLAY.len() - 1)]
    }

    /// Change de scénario (règles par défaut du scénario ; le fuseau choisi est gardé).
    pub fn choisir(&mut self, i: usize) {
        let tz = self.regles.reset_tz.clone();
        self.scen = i.min(Scenario::ALL.len() - 1);
        self.regles = Regles::of(Scenario::ALL[self.scen]);
        self.regles.reset_tz = tz;
    }

    /// Hauteur du contenu défilant (largeur `w`) : la fin du plan.
    fn hauteur(&self, v: &Vue, w: f64) -> f64 {
        let text = Text::default();
        let fin = self.plan(v, w, &text).iter().fold(0.0f64, |m, b| {
            m.max(match b {
                Bloc::Titre(y, _) | Bloc::Note(y, ..) => *y + 10.0,
                Bloc::Ligne(y, ..) | Bloc::Action(y, ..) => *y + LIGNE,
                Bloc::Rubrique(y, ..) => *y + RUBRIQUE,
                Bloc::Puce(p, ..) | Bloc::Courbe(p) => p.y + p.h,
                Bloc::Hist(y, l) => *y + l.h(),
                Bloc::MonthButton(y) => *y + MONTH_BUTTON,
            })
        });
        fin + 24.0
    }

    /// La hauteur où le contenu se voit dans le volet `r` : sous l'en-tête, au-dessus de la bande du rond Argos AI.
    pub fn visible(r: Rect, v: &Vue) -> f64 {
        (r.h - Self::tete(v) - v.marge_bas.max(0.0)).max(1.0)
    }

    /// 🖥 Hauteur de l'en-tête du volet (🧪 Simulateur, ✕) ; 0 dans le volet Trading, qui a le sien.
    pub fn tete(v: &Vue) -> f64 {
        if v.en_volet {
            0.0
        } else {
            TETE
        }
    }

    /// Le bas de la dernière ligne du contenu (historique compris), défilement non compté : pour vérifier qu'elle
    /// s'atteint.
    pub fn fin_des_lignes(&self, v: &Vue, w: f64) -> f64 {
        self.plan(v, w, &Text::default()).iter().fold(0.0f64, |m, b| match b {
            Bloc::Ligne(y, ..) | Bloc::Action(y, ..) => m.max(*y + LIGNE),
            Bloc::Note(y, ..) => m.max(*y + 6.0),
            Bloc::Rubrique(y, ..) => m.max(*y + RUBRIQUE),
            Bloc::Hist(y, l) => m.max(*y + l.h()),
            Bloc::MonthButton(y) => m.max(*y + MONTH_BUTTON),
            _ => m,
        })
    }

    /// Molette sur le volet `r`.
    pub fn molette(&mut self, dy: f64, r: Rect, v: &Vue) {
        self.scroll = (self.scroll + dy).clamp(0.0, (self.hauteur(v, r.w) - Self::visible(r, v)).max(0.0));
    }

    /// Le contenu, ligne par ligne : on le parcourt pour dessiner et pour trouver la zone d'un clic.
    /// 🛡 Défilement qui montre la section dont le titre commence par `debut` (« 🎓 Scénario » : la partie PropFirm).
    pub fn defile_vers(&mut self, v: &Vue, w: f64, text: &Text, debut: &str) {
        let trouve = self.plan(v, w, text).into_iter().find_map(|b| match b {
            Bloc::Titre(y, t) if t.starts_with(&debut.to_uppercase()) => Some((y, None)),
            // 📱 au doigt : la rubrique repliée de ce nom, ouverte
            Bloc::Rubrique(y, t, _, r) if t.to_uppercase().starts_with(&debut.to_uppercase()) => Some((y, Some(r))),
            _ => None,
        });
        if let Some((y, r)) = trouve {
            if let Some(r) = r.filter(|r| !self.ouvertes.contains(r)) {
                self.ouvertes.push(r);
            }
            self.scroll = (y - 12.0).max(0.0);
        }
    }

    fn plan(&self, v: &Vue, w: f64, text: &Text) -> Vec<Bloc> {
        let mut b: Vec<Bloc> = vec![];
        let mut y = 0.0;
        let titre = |b: &mut Vec<Bloc>, y: &mut f64, t: &str| {
            *y += TITRE;
            b.push(Bloc::Titre(*y - 10.0, t.to_uppercase()));
        };
        let ligne = |b: &mut Vec<Bloc>, y: &mut f64, g: String, d: String, c: Rgb| {
            b.push(Bloc::Ligne(*y, g, d, c));
            *y += LIGNE;
        };
        // une rangée de puces, coupée à la largeur
        let puces = |b: &mut Vec<Bloc>, y: &mut f64, l: Vec<(String, bool, Zone)>| {
            let mut x = PAD;
            for (t, on, z) in l {
                let pw = text.width(&t, 12.5) as f64 + 22.0;
                if x + pw > w - PAD && x > PAD {
                    x = PAD;
                    *y += PUCE + 6.0;
                }
                b.push(Bloc::Puce(Rect::new(x, *y, pw, PUCE), t, on, z));
                x += pw + 6.0;
            }
            *y += PUCE + 8.0;
        };
        // 📱 au doigt : une ligne › par rubrique, son contenu seulement quand elle est ouverte ; le PC montre tout, avec
        // le titre de la rubrique si `titre_pc` (`true` : montrer le contenu)
        let doigt = crate::actions::tactile();
        let rubrique = |b: &mut Vec<Bloc>, y: &mut f64, t: &str, r: Rub, titre_pc: bool| -> bool {
            if !doigt {
                if titre_pc {
                    titre(b, y, t);
                }
                return true;
            }
            let o = self.ouvertes.contains(&r);
            b.push(Bloc::Rubrique(*y, t.to_string(), o, r));
            *y += RUBRIQUE;
            o
        };
        let st: Stats = v.acc.stats();
        // — compte —
        titre(&mut b, &mut y, "🧪 Compte démo");
        let eq = v.acc.equity(v.prix_de);
        ligne(&mut b, &mut y, "Solde".into(), argent(v.acc.balance, false), WHITE);
        ligne(&mut b, &mut y, "Équité".into(), argent(eq, false), WHITE);
        let c = couleur(v.jour.to_f64());
        ligne(&mut b, &mut y, "P&L du jour".into(), argent(v.jour, true), c);
        let tenue = v.acc.marge_utilisee();
        if tenue > Money::ZERO {
            ligne(
                &mut b,
                &mut y,
                format!("⚡ Marge tenue {}", argent(tenue, false)),
                format!("dispo {}", argent(v.acc.disponible(), false)),
                GREY,
            );
        }
        if self.confirme {
            b.push(Bloc::Note(
                y + 16.0,
                "Remettre le compte à zéro ? Tout l'historique part.".into(),
                crate::palette::down(),
            ));
            y += LIGNE;
            puces(
                &mut b,
                &mut y,
                vec![("Oui, à zéro".into(), true, Zone::ResetOui), ("Non".into(), false, Zone::ResetNon)],
            );
        } else {
            puces(&mut b, &mut y, vec![("↺ Remise à zéro".into(), false, Zone::ResetAsk)]);
        }
        // — ticket —
        titre(&mut b, &mut y, &format!("Ticket démo · {}", v.symbole));
        puces(
            &mut b,
            &mut y,
            vec![("▼ VENTE".into(), !self.buy, Zone::Sens(false)), ("▲ ACHAT".into(), self.buy, Zone::Sens(true))],
        );
        puces(
            &mut b,
            &mut y,
            Kind::ALL.iter().map(|k| (k.name().to_string(), *k == self.kind, Zone::Genre(*k))).collect(),
        );
        let mut l: Vec<(String, bool, Zone)> = vec![("Risque".into(), false, Zone::Risque(usize::MAX))];
        l.extend(RISQUES.iter().enumerate().map(|(i, r)| (pct_txt(*r), i == self.risque, Zone::Risque(i))));
        puces(&mut b, &mut y, l);
        let objectifs = rubrique(&mut b, &mut y, "🎯 Objectifs et stops", Rub::Objectifs, false);
        let mut l: Vec<(String, bool, Zone)> = vec![("SL".into(), false, Zone::Sl(usize::MAX))];
        l.extend(SL_PCT.iter().enumerate().map(|(i, r)| (pct_txt(*r), i == self.sl, Zone::Sl(i))));
        l.extend(
            RR.iter()
                .enumerate()
                .map(|(i, r)| (format!("R:R {}", r.to_string().replace('.', ",")), i == self.rr, Zone::Rr(i))),
        );
        if objectifs {
            puces(&mut b, &mut y, l);
        }
        if self.kind != Kind::Market {
            let mut l: Vec<(String, bool, Zone)> = vec![("Écart".into(), false, Zone::Ecart(usize::MAX))];
            l.extend(ECARTS.iter().enumerate().map(|(i, r)| (pct_txt(*r), i == self.ecart, Zone::Ecart(i))));
            if objectifs {
                puces(&mut b, &mut y, l);
            }
        }
        // 🎛 gestion de la position
        if objectifs {
            puces(
                &mut b,
                &mut y,
                SORTIES
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (format!("🎯 {x}"), i == self.sorties, Zone::Sorties(i)))
                    .collect(),
            );
        }
        let mut l: Vec<(String, bool, Zone)> = vec![
            ("⚖ Break-even à 1R".into(), self.be, Zone::Be),
            ("🪜 Stop suiveur 1R".into(), self.suiveur, Zone::Suiveur),
        ];
        if self.kind == Kind::Stop {
            l.push(("🔀 OCO : les deux côtés".into(), self.oco, Zone::Oco));
        }
        if objectifs {
            puces(&mut b, &mut y, l);
        }
        // ⚡ perpétuel : le levier, plafonné au maximum du marché (marge isolée)
        if let Some(pv) = v.perp {
            let choisi = self.levier.min(pv.bareme.levier_max);
            let mut l: Vec<(String, bool, Zone)> = vec![("Levier".into(), false, Zone::Levier(usize::MAX))];
            l.extend(
                perp::leviers(pv.bareme.levier_max)
                    .iter()
                    .enumerate()
                    .map(|(i, x)| (perp::levier_txt(*x), *x == choisi, Zone::Levier(i))),
            );
            puces(&mut b, &mut y, l);
        }
        // 💸 coûts de la bourse, ou « sans frais »
        let couts = v.couts.filter(|_| !v.acc.sans_frais);
        if rubrique(&mut b, &mut y, "💸 Coûts", Rub::Couts, false) {
            match v.couts {
                Some((nom, _)) => puces(
                    &mut b,
                    &mut y,
                    vec![
                        (format!("💸 Coûts réels · {nom}"), couts.is_some(), Zone::SansFrais(false)),
                        ("Sans frais".into(), couts.is_none(), Zone::SansFrais(true)),
                    ],
                ),
                None => ligne(&mut b, &mut y, "💸 Pas de barème pour ce marché".into(), "sans frais".into(), GREY),
            }
        }
        let t = self.ticket(v.last, v.acc.balance);
        // les chiffres du ticket (📱 au doigt : Entrée, SL / TP et Net au-dessus de Placer, le reste dans « Détail »)
        let mut chiffres: Vec<Chiffre> = vec![];
        if let Some((a, z)) = self.tickets_oco(v.last, v.acc.balance) {
            chiffres.push(Chiffre::Ligne(
                false,
                format!("Buy Stop {}", v.prix(a.price)),
                format!("Sell Stop {}", v.prix(z.price)),
                GREY,
            ));
        }
        let risque = t.risk;
        let qty = argos_core::simu::size_for_risk(risque, t.price, t.sl);
        chiffres.push(Chiffre::Ligne(
            true,
            format!("Entrée {}", v.prix(t.price)),
            format!("SL {} · TP {}", v.prix(t.sl), v.prix(t.tp)),
            GREY,
        ));
        chiffres.push(Chiffre::Ligne(
            false,
            format!("Taille {qty:.4}").replace('.', ","),
            format!("risque {}", argent(risque, false)),
            GREY,
        ));
        // ⚡ perpétuel : la marge isolée tenue, le prix de liquidation et le funding, avant d'envoyer
        if let (Some(pv), Some(x)) = (v.perp, self.perp(v)) {
            let marge = match self.tickets_oco(v.last, v.acc.balance) {
                // un OCO : un seul des deux part, la plus grande marge est tenue
                Some((a, z)) => [a, z]
                    .iter()
                    .map(|t| x.marge(argos_core::simu::size_for_risk(t.risk, t.price, t.sl), t.price))
                    .max()
                    .unwrap_or(Money::ZERO),
                None => x.marge(qty, t.price),
            };
            let dispo = v.acc.disponible();
            let rouge = crate::palette::down();
            chiffres.push(Chiffre::Ligne(
                false,
                format!("⚡ Marge isolée {} · {}", argent(marge, false), perp::levier_txt(x.levier)),
                format!("dispo {}", argent(dispo, false)),
                if marge > dispo { rouge } else { GREY },
            ));
            if marge > dispo {
                chiffres.push(Chiffre::Note("Marge insuffisante : baisse le risque ou monte le levier".into(), rouge));
            }
            let liq = x.liquidation(t.buy, t.price);
            let dist = (liq - t.price).abs() / t.price * 100.0;
            chiffres.push(Chiffre::Ligne(
                false,
                format!("💀 Liquidation {}", v.prix(liq)),
                format!("à {dist:.1} % de l'entrée").replace('.', ","),
                GREY,
            ));
            // le SL après la liquidation : la position serait liquidée avant de toucher son stop
            if (t.sl - liq) * if t.buy { 1.0 } else { -1.0 } <= 0.0 {
                chiffres.push(Chiffre::Note("⚠ La liquidation arrive avant ton SL : baisse le levier".into(), rouge));
            }
            // le taux pour 8 h (comme les bourses l'affichent) ; à droite : un paiement, à chaque intervalle
            let r = x.taux_par_paiement(pv.taux_8h);
            let source = if v.replay.is_some() {
                "replay"
            } else if pv.reel {
                "réel"
            } else {
                "de base"
            };
            let h = format!("{}", x.funding_h).replace('.', ",");
            chiffres.push(Chiffre::Ligne(
                false,
                format!("⚡ Funding {} / 8 h · {source}", funding_txt(pv.taux_8h)),
                format!("{} / {h} h", argent(Money::ZERO - perp::funding(t.buy, qty, t.price, r), true)),
                GREY,
            ));
        }
        // 💸 avant d'envoyer : ce que coûte le trade et ce qu'il rapporte vraiment au SL et au TP
        if let Some((_, c)) = couts {
            chiffres.push(Chiffre::Ligne(
                false,
                format!("💸 maker {} · taker {}", taux_txt(c.maker), taux_txt(c.taker)),
                format!("écart {} · gliss. {}", taux_txt(c.ecart), taux_txt(c.glissement)),
                GREY,
            ));
            let e = estimer(&c, t.kind != Kind::Limit, qty, t.price, t.sl, t.tp);
            chiffres.push(Chiffre::Ligne(
                false,
                format!("Coûts au SL {}", argent(e.au_sl, false)),
                format!("au TP {}", argent(e.au_tp, false)),
                GREY,
            ));
            let s = if t.buy { 1.0 } else { -1.0 };
            let gain = Money::resultat(t.tp, t.price, qty, s) - e.au_tp;
            chiffres.push(Chiffre::Ligne(
                true,
                format!("Net au SL {}", argent(Money::ZERO - risque - e.au_sl, true)),
                format!("au TP {}", argent(gain, true)),
                GREY,
            ));
        }
        let pose = |b: &mut Vec<Bloc>, y: &mut f64, c: Chiffre| match c {
            Chiffre::Ligne(_, g, d, c) => ligne(b, y, g, d, c),
            Chiffre::Note(t, c) => {
                b.push(Bloc::Note(*y + 10.0, t, c));
                *y += 18.0;
            }
        };
        let (dessus, detail): (Vec<Chiffre>, Vec<Chiffre>) = chiffres.into_iter().partition(|c| !doigt || !matches!(c, Chiffre::Ligne(false, ..)));
        for c in dessus {
            pose(&mut b, &mut y, c);
        }
        puces(
            &mut b,
            &mut y,
            vec![
                (
                    if self.tickets_oco(v.last, v.acc.balance).is_some() {
                        "Placer l'OCO (démo)".to_string()
                    } else {
                        format!("Placer l'ordre {} (démo)", t.kind.name().to_lowercase())
                    },
                    true,
                    Zone::Placer,
                ),
                (format!("🛡 Gardien {}", if v.gardien { "allumé" } else { "éteint" }), v.gardien, Zone::Gardien),
            ],
        );
        if !detail.is_empty() && rubrique(&mut b, &mut y, "Détail", Rub::Detail, false) {
            for c in detail {
                pose(&mut b, &mut y, c);
            }
        }
        // — positions et ordres —
        titre(&mut b, &mut y, "Positions et ordres");
        let opt = |base: &str, x: Option<f64>| x.map_or("—".to_string(), |px| v.prix_de(base, px));
        for p in &v.acc.positions {
            let pnl = (v.prix_de)(&p.base).map_or(Money::ZERO, |px| p.resultat(px));
            b.push(Bloc::Action(
                y,
                format!(
                    "{} {} {} · SL {} · TP {}",
                    if p.side == Side::Long { "▲" } else { "▼" },
                    p.base,
                    v.prix_de(&p.base, p.entry),
                    opt(&p.base, p.stop),
                    opt(&p.base, p.target)
                ),
                argent(pnl, true),
                couleur(pnl.to_f64()),
                Zone::Sortir(p.id),
            ));
            y += LIGNE;
            // ⚡ perpétuel : où il serait liquidé, la marge tenue, le funding déjà payé (ou reçu)
            if let (Some(liq), Some(x)) = (p.liquidation(), p.perp) {
                ligne(
                    &mut b,
                    &mut y,
                    format!(
                        "{} · 💀 liq. {} · marge {}",
                        perp::levier_txt(x.levier),
                        v.prix_de(&p.base, liq),
                        argent(p.marge(), false)
                    ),
                    if p.funding == Money::ZERO {
                        String::new()
                    } else {
                        format!("funding {}", argent(Money::ZERO - p.funding, true))
                    },
                    GREY,
                );
            }
            let g = gestion_txt(&p.gestion);
            if !g.is_empty() {
                ligne(&mut b, &mut y, g, String::new(), GREY);
            }
            let mut l = vec![("✂ Sortir ½".to_string(), false, Zone::Moitie(p.id))];
            if p.stop.is_none_or(|st| (p.entry - st) * p.side.sign() > 0.0) {
                l.push(("⚖ Stop à l'entrée".into(), false, Zone::StopEntree(p.id)));
            }
            puces(&mut b, &mut y, l);
        }
        for o in &v.acc.pending {
            b.push(Bloc::Action(
                y,
                format!(
                    "⏳ {} {} {}{} {}",
                    if o.kind == OrderKind::Limit { "Limite" } else { "Stop" },
                    if o.order.side == Side::Long { "▲" } else { "▼" },
                    o.order.base,
                    o.order.perp.map_or(String::new(), |x| format!(" {}", perp::levier_txt(x.levier))),
                    v.prix_de(&o.order.base, o.order.price)
                ),
                "en attente".into(),
                GREY,
                Zone::Annuler(o.id),
            ));
            y += LIGNE;
        }
        if v.acc.positions.is_empty() && v.acc.pending.is_empty() {
            ligne(&mut b, &mut y, "aucune".into(), String::new(), GREY);
        }
        // — replay —
        if rubrique(&mut b, &mut y, "▶ Replay", Rub::Replay, true) {
            match v.replay {
                None => {
                    puces(
                        &mut b,
                        &mut y,
                        vec![
                            ("↺ Rejouer les bougies passées".into(), false, Zone::RpStart),
                            ("📅 Choisir la date".into(), false, Zone::RpDate),
                        ],
                    );
                    if let Some(se) = &v.seance {
                        ligne(&mut b, &mut y, "💾 Séance gardée".into(), se.clone(), GREY);
                        puces(
                            &mut b,
                            &mut y,
                            vec![
                                ("▶ Reprendre la séance".into(), true, Zone::SeReprendre),
                                ("🗑 Oublier".into(), false, Zone::SeOublier),
                            ],
                        );
                    }
                }
                Some(r) => {
                    puces(
                        &mut b,
                        &mut y,
                        vec![
                            ((if r.play { "Pause" } else { "▶ Lecture" }).into(), r.play, Zone::RpPlay),
                            ("+1 bougie".into(), false, Zone::RpStep),
                            (format!("×{}", r.speed), false, Zone::RpSpeed),
                        ],
                    );
                    puces(
                        &mut b,
                        &mut y,
                        vec![("💾 Sauver et quitter".into(), false, Zone::SeSauver), ("✕ Direct".into(), false, Zone::RpStop)],
                    );
                    ligne(
                        &mut b,
                        &mut y,
                        format!("bougie {} / {}", r.shown, v.total),
                        "pas de ticks : bougie par bougie".into(),
                        GREY,
                    );
                    // ⚡ perpétuel : le funding du replay, un taux fixe choisi (en %, pour 8 heures : dit au ticket)
                    if v.perp.is_some() {
                        let mut l: Vec<(String, bool, Zone)> = vec![("Funding %".into(), false, Zone::FundingRp(usize::MAX))];
                        l.extend(
                            FUNDING_REPLAY
                                .iter()
                                .enumerate()
                                .map(|(i, t)| (funding_txt(*t).trim_end_matches(" %").to_string(), i == self.funding_rp, Zone::FundingRp(i))),
                        );
                        puces(&mut b, &mut y, l);
                    }
                }
            }
        }
        // — examen —
        if rubrique(&mut b, &mut y, "🎓 Scénario d'examen", Rub::Examen, true) {
            puces(
                &mut b,
                &mut y,
                Scenario::ALL
                    .iter()
                    .enumerate()
                    .map(|(i, s)| (s.name().to_string(), i == self.scen, Zone::Scen(i)))
                    .collect(),
            );
            if Scenario::ALL[self.scen] == Scenario::Perso {
                let r = &self.regles;
                puces(
                    &mut b,
                    &mut y,
                    vec![
                        (format!("Cible {} ↻", pct_txt(r.phases.first().copied().unwrap_or(10.0))), false, Zone::PCible),
                        (format!("Jour {} ↻", pct_txt(r.daily_loss)), false, Zone::PJour),
                        (format!("Total {} ↻", pct_txt(r.max_loss)), false, Zone::PTotal),
                        (format!("{} j min ↻", r.min_days), false, Zone::PJours),
                    ],
                );
            }
            b.push(Bloc::Note(y + 16.0, self.regles.resume(), GREY));
            y += LIGNE;
            puces(
                &mut b,
                &mut y,
                vec![(format!("Remise du jour : minuit {} ↻", self.regles.reset_tz), false, Zone::Fuseau)],
            );
            match v.examen {
                None => ligne(&mut b, &mut y, "Place un trade pour démarrer l'examen".into(), String::new(), GREY),
                Some(e) => {
                    let n = self.regles.phases.len();
                    ligne(
                        &mut b,
                        &mut y,
                        format!("Étape {} / {n}", (e.phase + 1).min(n)),
                        format!("profit {:+.2} %", e.profit_pct).replace('.', ","),
                        WHITE,
                    );
                    ligne(
                        &mut b,
                        &mut y,
                        "Perte du jour".into(),
                        format!("{:.2} % / {}", e.jour_pct, pct_txt(self.regles.daily_loss)).replace('.', ","),
                        GREY,
                    );
                    ligne(
                        &mut b,
                        &mut y,
                        "Perte totale".into(),
                        format!("{:.2} % / {}", e.total_pct, pct_txt(self.regles.max_loss)).replace('.', ","),
                        GREY,
                    );
                    ligne(
                        &mut b,
                        &mut y,
                        "Jours de trading".into(),
                        format!("{} / {}", e.jours, self.regles.min_days),
                        GREY,
                    );
                    match &e.verdict {
                        Verdict::EnCours => ligne(&mut b, &mut y, e.note.clone().unwrap_or_else(|| "En cours".into()), String::new(), GREY),
                        Verdict::Reussi => ligne(
                            &mut b,
                            &mut y,
                            "✅ Réussi : toutes les étapes sont passées".into(),
                            String::new(),
                            crate::palette::up(),
                        ),
                        Verdict::Echoue { raison, lecon } => {
                            // la raison en entier, sur plusieurs lignes
                            let mut l = String::new();
                            for m in format!("❌ Échoué : {raison}").split(' ') {
                                let essai = if l.is_empty() { m.to_string() } else { format!("{l} {m}") };
                                if !l.is_empty() && text.width(&essai, 12.0) as f64 > w - 2.0 * PAD {
                                    b.push(Bloc::Note(y + 16.0, std::mem::take(&mut l), crate::palette::down()));
                                    y += 18.0;
                                    l = m.to_string();
                                } else {
                                    l = essai;
                                }
                            }
                            b.push(Bloc::Note(y + 16.0, l, crate::palette::down()));
                            y += LIGNE;
                            puces(
                                &mut b,
                                &mut y,
                                vec![("🎓 Leçon : Passer un challenge PropFirm".into(), true, Zone::Lecon(lecon))],
                            );
                        }
                    }
                }
            }
        }
        // — courbe —
        if rubrique(&mut b, &mut y, "Courbe d'équité", Rub::Courbe, true) {
            b.push(Bloc::Courbe(Rect::new(PAD, y, w - 2.0 * PAD, COURBE - 10.0)));
            y += COURBE;
        }
        // — stats —
        titre(&mut b, &mut y, "Statistiques");
        ligne(
            &mut b,
            &mut y,
            "Trades · taux de réussite".into(),
            format!("{} · {:.0} %", st.trades, st.win_rate),
            WHITE,
        );
        ligne(
            &mut b,
            &mut y,
            "Ratio moyen (gain / perte)".into(),
            format!("{:.2}", st.avg_ratio).replace('.', ","),
            WHITE,
        );
        ligne(
            &mut b,
            &mut y,
            "Drawdown max".into(),
            format!("{} ({:.2} %)", argent(st.max_dd, false), st.max_dd_pct).replace(".", ","),
            WHITE,
        );
        // ⚡ le funding des trades fermés et des positions ouvertes ; 💀 les liquidations
        let funding: Money = v.acc.history.iter().map(|h| h.funding).chain(v.acc.positions.iter().map(|p| p.funding)).sum();
        let payes: Money = v.acc.history.iter().map(|h| h.fees).sum();
        if payes > Money::ZERO {
            // (le Rapport du mois compte le funding dans ses coûts payés : ici, il a sa ligne, juste dessous)
            let t = if funding != Money::ZERO {
                "💸 Coûts payés (hors funding)"
            } else {
                "💸 Coûts payés"
            };
            ligne(&mut b, &mut y, t.into(), argent(payes, false), WHITE);
        }
        if funding != Money::ZERO {
            let t = if funding > Money::ZERO { "⚡ Funding payé" } else { "⚡ Funding reçu" };
            ligne(&mut b, &mut y, t.into(), argent(funding.abs(), false), WHITE);
        }
        let liquides = v.acc.history.iter().filter(|h| h.liquide()).count();
        if liquides > 0 {
            ligne(&mut b, &mut y, "💀 Liquidations".into(), liquides.to_string(), crate::palette::down());
        }
        let c = couleur(st.total.to_f64());
        ligne(&mut b, &mut y, "Résultat".into(), argent(st.total, true), c);
        // 📅 the month report (once a trade is finished)
        if !v.acc.history.is_empty() {
            b.push(Bloc::MonthButton(y));
            y += MONTH_BUTTON;
        }
        // — historique —
        titre(&mut b, &mut y, "Historique");
        // 💸 le résultat net : ce que le solde a vraiment reçu. 🤖 Chaque trade fini a son rond Argos AI (un toucher
        // ouvre sa carte) ; le trade et son montant, le détail (raison, R, coûts) en gris dessous, en entier : il passe
        // à la ligne s'il ne tient pas (📱 étroit). Une sortie partielle n'a pas de rond (sa carte vient à la dernière).
        for (k, h) in v.acc.history.iter().enumerate().rev().take(8) {
            let (principal, detail) = hist_txt(h, v);
            let montant = argent(h.net(), true);
            let a_cote = w - PAD - HIST_X - Text::graisse(Graisse::Medium).width(&montant, 12.5) as f64 - 12.0;
            let centre = text.width(&detail, 11.5) as f64 <= a_cote;
            let details = if centre {
                vec![detail]
            } else {
                crate::avis::couper(text, &detail, 11.5, w - PAD - HIST_X)
            };
            let l = LigneHist {
                principal,
                details,
                montant,
                couleur: couleur(h.net().to_f64()),
                avis: (!h.partiel).then_some(k),
                centre,
            };
            let hh = l.h();
            b.push(Bloc::Hist(y, l));
            y += hh + 2.0;
        }
        if v.acc.history.is_empty() {
            ligne(&mut b, &mut y, "les trades fermés arrivent ici".into(), String::new(), GREY);
        }
        b
    }

    /// Clic aux coordonnées du volet `r` (fenêtre) ; `x`, `y` : souris dans la fenêtre.
    pub fn click(&mut self, text: &Text, r: Rect, v: &Vue, x: f64, y: f64) -> SimClick {
        let (lx, ly, tete) = (x - r.x, y - r.y, Self::tete(v));
        if !v.en_volet && Self::fermer(r).contains(lx, ly) {
            return SimClick::Close;
        }
        // (📱 the round's band: nothing of the content there)
        if ly < tete || (v.marge_bas > 0.0 && ly >= tete + Self::visible(r, v)) {
            return SimClick::None;
        }
        let cy = ly - tete + self.scroll;
        let z = self.plan(v, r.w, text).into_iter().find_map(|b| match b {
            Bloc::Puce(p, _, _, z) if p.contains(lx, cy) => Some(z),
            Bloc::Action(by, _, _, _, z) if cy >= by && cy < by + LIGNE && lx > r.w - 44.0 => Some(z),
            Bloc::Rubrique(by, _, _, r) if cy >= by && cy < by + RUBRIQUE => Some(Zone::Rub(r)),
            Bloc::Hist(by, l) if cy >= by && cy < by + l.h() => l.avis.map(Zone::Avis),
            Bloc::MonthButton(by) if month_button(by, r.w).contains(lx, cy) => Some(Zone::Month),
            _ => None,
        });
        let Some(z) = z else { return SimClick::None };
        let cycle = |l: &[f64], v: f64| l[(l.iter().position(|x| *x == v).unwrap_or(0) + 1) % l.len()];
        match z {
            Zone::ResetAsk => self.confirme = true,
            Zone::ResetNon => self.confirme = false,
            Zone::ResetOui => {
                self.confirme = false;
                return SimClick::Reset;
            }
            Zone::Sens(b) => self.buy = b,
            Zone::Genre(k) => self.kind = k,
            Zone::Risque(i) if i < RISQUES.len() => self.risque = i,
            Zone::Sl(i) if i < SL_PCT.len() => self.sl = i,
            Zone::Rr(i) => self.rr = i,
            Zone::Ecart(i) if i < ECARTS.len() => self.ecart = i,
            Zone::Risque(_) | Zone::Sl(_) | Zone::Ecart(_) => return SimClick::None,
            Zone::Sorties(i) if i < SORTIES.len() => self.sorties = i,
            Zone::Sorties(_) => return SimClick::None,
            Zone::Be => self.be = !self.be,
            Zone::Suiveur => self.suiveur = !self.suiveur,
            Zone::Oco => self.oco = !self.oco,
            Zone::Moitie(id) => return SimClick::Moitie(id),
            Zone::StopEntree(id) => return SimClick::StopEntree(id),
            Zone::SansFrais(b) => return SimClick::SansFrais(b),
            Zone::Levier(i) => match v.perp.and_then(|p| perp::leviers(p.bareme.levier_max).get(i).copied()) {
                Some(l) => self.levier = l,
                None => return SimClick::None,
            },
            Zone::FundingRp(i) if i < FUNDING_REPLAY.len() => self.funding_rp = i,
            Zone::FundingRp(_) => return SimClick::None,
            Zone::Placer => return SimClick::Place,
            Zone::Gardien => return SimClick::Gardien,
            Zone::Annuler(id) => return SimClick::Cancel(id),
            Zone::Sortir(id) => return SimClick::CloseTrade(id),
            Zone::RpStart => return SimClick::ReplayStart,
            Zone::RpPlay => return SimClick::ReplayPlay,
            Zone::RpStep => return SimClick::ReplayStep,
            Zone::RpSpeed => return SimClick::ReplaySpeed,
            Zone::RpStop => return SimClick::ReplayStop,
            Zone::RpDate => return SimClick::ReplayDate,
            Zone::SeSauver => return SimClick::SeanceSauver,
            Zone::SeReprendre => return SimClick::SeanceReprendre,
            Zone::SeOublier => return SimClick::SeanceOublier,
            Zone::Scen(i) => self.choisir(i),
            Zone::Fuseau => {
                let i = FUSEAUX.iter().position(|f| *f == self.regles.reset_tz).unwrap_or(0);
                self.regles.reset_tz = FUSEAUX[(i + 1) % FUSEAUX.len()].into();
            }
            Zone::PCible => {
                let c = cycle(&P_CIBLE, self.regles.phases.first().copied().unwrap_or(10.0));
                self.regles.phases = vec![c];
            }
            Zone::PJour => self.regles.daily_loss = cycle(&P_JOUR, self.regles.daily_loss),
            Zone::PTotal => self.regles.max_loss = cycle(&P_TOTAL, self.regles.max_loss),
            Zone::PJours => {
                let i = P_JOURS.iter().position(|d| *d == self.regles.min_days).unwrap_or(0);
                self.regles.min_days = P_JOURS[(i + 1) % P_JOURS.len()];
            }
            Zone::Lecon(id) => return SimClick::Lecon(id),
            Zone::Avis(k) => return SimClick::Avis(k),
            Zone::Month => return SimClick::MonthReport,
            Zone::Rub(r) => match self.ouvertes.iter().position(|x| *x == r) {
                Some(k) => {
                    self.ouvertes.remove(k);
                }
                None => self.ouvertes.push(r),
            },
        }
        SimClick::Changed
    }

    /// Le ✕ (coordonnées du volet).
    pub fn fermer(r: Rect) -> Rect {
        Rect::new(r.w - 48.0, 11.0, 36.0, 36.0)
    }

    pub fn draw(&self, text: &Text, r: Rect, v: &Vue, mouse: (f64, f64)) -> Option<Pixmap> {
        let mut pm = crate::px::new(r.w.max(1.0) as u32, r.h.max(1.0) as u32)?;
        let tete = Self::tete(v);
        // (🖥 in the Trading volet: its glass shows through, no background of its own)
        if !v.en_volet {
            crate::fill(&mut pm, 0.0, 0.0, r.w, r.h, crate::jetons::FOND, 0.94);
            crate::fill(&mut pm, 0.0, 0.0, r.w, r.h, crate::jetons::VERRE.0, crate::jetons::VERRE.1);
            crate::fill(&mut pm, 0.0, 0.0, 1.0, r.h, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        }
        let mut body = crate::px::new(r.w.max(1.0) as u32, Self::visible(r, v) as u32)?;
        let (mx, my) = (mouse.0 - r.x, mouse.1 - r.y - tete + self.scroll);
        let dy = -self.scroll;
        for b in self.plan(v, r.w, text) {
            let action = matches!(b, Bloc::Action(..));
            match b {
                Bloc::Titre(y, t) => text.draw(&mut body, &t, (PAD as f32, (y + dy) as f32), 11.5, GREY, Align::Left),
                Bloc::Ligne(y, g, d, c) | Bloc::Action(y, g, d, c, _) => {
                    // une ligne à ✕ : le montant s'arrête avant le ✕
                    let droite = if action { r.w - 40.0 } else { r.w - PAD };
                    let dw = text.width(&d, 12.5) as f64;
                    text.draw(
                        &mut body,
                        &text.fit(&g, 12.5, droite - dw - 2.0 * PAD),
                        (PAD as f32, (y + dy + 17.0) as f32),
                        12.5,
                        WHITE,
                        Align::Left,
                    );
                    text.draw(&mut body, &d, (droite as f32, (y + dy + 17.0) as f32), 12.5, c, Align::Right);
                }
                Bloc::Note(y, t, c) => text.draw(
                    &mut body,
                    &text.fit(&t, 12.0, r.w - 2.0 * PAD),
                    (PAD as f32, (y + dy) as f32),
                    12.0,
                    c,
                    Align::Left,
                ),
                Bloc::Puce(p, t, on, z) => {
                    let p = Rect::new(p.x, p.y + dy, p.w, p.h);
                    let etiquette = matches!(
                        z,
                        Zone::Risque(usize::MAX) | Zone::Sl(usize::MAX) | Zone::Ecart(usize::MAX) | Zone::Levier(usize::MAX) | Zone::FundingRp(usize::MAX)
                    );
                    if etiquette {
                        text.draw(&mut body, &t, (p.x as f32, (p.cy() + 4.0) as f32), 12.0, GREY, Align::Left);
                        continue;
                    }
                    let hover = p.contains(mx, my + dy);
                    let fond = if on {
                        crate::palette::accent()
                    } else if hover {
                        crate::palette::hover()
                    } else {
                        Rgb(0x2a, 0x2e, 0x39)
                    };
                    round_fill(&mut body, p, 8.0, fond, if on { 0.9 } else { 1.0 });
                    round_stroke(&mut body, p, 8.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    text.draw(&mut body, &t, (p.cx() as f32, (p.cy() + 4.5) as f32), 12.5, WHITE, Align::Center);
                }
                Bloc::Courbe(c) => courbe(&mut body, Rect::new(c.x, c.y + dy, c.w, c.h), v.acc),
                Bloc::Hist(y, l) => {
                    let row = Rect::new(10.0, y + dy, r.w - 20.0, l.h());
                    // la ligne du trade dont la carte est ouverte (maquette : bleu à 14 %, liseré à 35 %) ; au survol
                    let bleu = Rgb(77, 163, 255);
                    if l.avis.is_some() && l.avis == v.avis {
                        round_fill(&mut body, row, 12.0, bleu, 0.14);
                        round_stroke(&mut body, row, 12.0, crate::jetons::sur_fond((bleu, 0.35)), 1.0);
                    } else if l.avis.is_some() && row.contains(mx, my + dy) {
                        round_fill(&mut body, row, 12.0, WHITE, 0.05);
                    }
                    let milieu = row.y + HIST / 2.0;
                    if l.avis.is_some() {
                        let rond = Rect::new(18.0, milieu - HIST_ROND / 2.0, HIST_ROND, HIST_ROND);
                        crate::aibtn::bouton_verre(&mut body, rond, false);
                    }
                    // le montant : au milieu des deux lignes quand le détail tient à côté, sinon sur la ligne du trade
                    let moyen = Text::graisse(Graisse::Medium);
                    let y_montant = if l.centre { milieu + 4.5 } else { row.y + 19.5 };
                    let droite = (r.w - PAD) as f32;
                    moyen.draw(&mut body, &l.montant, (droite, y_montant as f32), 12.5, l.couleur, Align::Right);
                    let place = r.w - PAD - HIST_X - moyen.width(&l.montant, 12.5) as f64 - 12.0;
                    let principal = text.fit(&l.principal, 12.5, place);
                    text.draw(&mut body, &principal, (HIST_X as f32, (row.y + 19.5) as f32), 12.5, WHITE, Align::Left);
                    for (i, d) in l.details.iter().enumerate() {
                        let yd = row.y + 35.0 + i as f64 * HIST_SUITE;
                        text.draw(&mut body, d, (HIST_X as f32, yd as f32), 11.5, GREY, Align::Left);
                    }
                }
                Bloc::MonthButton(y) => {
                    // 📅 the mock-up's button: blue while the page is open (as the Historique's row of an open card)
                    let p = month_button(y + dy, r.w);
                    let bleu = Rgb(77, 163, 255);
                    if self.month.is_some() {
                        round_fill(&mut body, p, 14.0, bleu, 0.14);
                        round_stroke(&mut body, p, 14.0, crate::jetons::sur_fond((bleu, 0.45)), 1.0);
                    } else {
                        let a = if p.contains(mx, my + dy) { 0.1 } else { 0.05 };
                        round_fill(&mut body, p, 14.0, WHITE, a);
                        round_stroke(&mut body, p, 14.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
                    }
                    if let Some(img) = crate::emoji::image("📅", crate::px::dev(18.0).round() as u32) {
                        crate::px::put(&mut body, p.x + 14.0, p.cy() - 9.0, &img);
                    }
                    let moyen = Text::graisse(Graisse::Medium);
                    let base = (p.cy() + 5.0) as f32;
                    moyen.draw(&mut body, "Rapport du mois", ((p.x + 41.0) as f32, base), 13.5, WHITE, Align::Left);
                    moyen.draw(&mut body, "›", ((p.x + p.w - 14.0) as f32, base), 13.5, Rgb(0x93, 0xa0, 0xc2), Align::Right);
                }
                Bloc::Rubrique(y, t, o, _) => {
                    crate::fill(
                        &mut body,
                        PAD,
                        y + dy,
                        r.w - 2.0 * PAD,
                        1.0,
                        crate::jetons::sur_fond(crate::jetons::LISERE),
                        1.0,
                    );
                    let cy = (y + dy + RUBRIQUE / 2.0 + 5.0) as f32;
                    text.draw(&mut body, &t, (PAD as f32, cy), 14.0, WHITE, Align::Left);
                    text.draw(&mut body, if o { "⌄" } else { "›" }, ((r.w - PAD) as f32, cy), 16.0, GREY, Align::Right);
                }
            }
        }
        // les ✕ des positions et ordres
        for b in self.plan(v, r.w, text) {
            if let Bloc::Action(y, ..) = b {
                let cx = r.w - 22.0;
                lines(
                    &mut body,
                    cx,
                    y + dy + 12.0,
                    &[&[(-4.0, -4.0), (4.0, 4.0)], &[(4.0, -4.0), (-4.0, 4.0)]],
                    GREY,
                    1.4,
                );
            }
        }
        crate::px::put(&mut pm, 0.0, tete, &body);
        // 🫧 la barre de défilement : on voit qu'il y a une suite, et où on en est
        let vue_h = Self::visible(r, v);
        let total = self.hauteur(v, r.w);
        if total > vue_h + 1.0 {
            let k = vue_h / total;
            let pouce = (vue_h * k - 8.0).max(24.0);
            let y = tete + 4.0 + (vue_h - 8.0 - pouce) * (self.scroll / (total - vue_h)).clamp(0.0, 1.0);
            crate::jetons::barre_defilement(&mut pm, r.w - 2.0, y, pouce, false);
        }
        if v.en_volet {
            return Some(pm);
        }
        text.draw(&mut pm, "🧪 Simulateur", (18.0, 36.0), 17.0, WHITE, Align::Left);
        text.draw(&mut pm, "démo · aucun ordre réel", (150.0, 36.0), 11.5, GREY, Align::Left);
        let x = Self::fermer(r);
        if x.contains(mouse.0 - r.x, mouse.1 - r.y) {
            round_fill(&mut pm, x, 8.0, crate::palette::hover(), 1.0);
        }
        lines(&mut pm, x.cx(), x.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.6);
        crate::fill(&mut pm, 0.0, TETE - 1.0, r.w, 1.0, crate::jetons::sur_fond(crate::jetons::LISERE), 1.0);
        Some(pm)
    }
}

/// Ce que le plan contient (y : haut, dans le contenu défilant).
enum Bloc {
    Titre(f64, String),
    Ligne(f64, String, String, Rgb),
    Note(f64, String, Rgb),
    /// Une ligne avec un ✕ à droite (fermer la position, annuler l'ordre).
    Action(f64, String, String, Rgb, Zone),
    Puce(Rect, String, bool, Zone),
    Courbe(Rect),
    /// 📱 Une rubrique repliable au doigt : une ligne de 44, › fermée, ⌄ ouverte.
    Rubrique(f64, String, bool, Rub),
    /// 🤖 Une ligne de l'Historique.
    Hist(f64, LigneHist),
    /// 📅 The « Rapport du mois » button (its top, the margin above it included).
    MonthButton(f64),
}

/// 📅 The « Rapport du mois » button: 12 under the statistics, 38 high, 14 from the panel's sides (the mock-up's).
const MONTH_BUTTON: f64 = 12.0 + 38.0;

/// The button whose block starts at `y`, in a panel `w` wide.
fn month_button(y: f64, w: f64) -> Rect {
    Rect::new(14.0, y + 12.0, w - 28.0, 38.0)
}

/// 🤖 Une ligne de l'Historique : le trade, son détail (une ligne, ou plusieurs s'il ne tient pas), son montant et sa
/// couleur, l'index de la sortie dans l'historique quand elle finit le trade (son rond Argos AI).
struct LigneHist {
    principal: String,
    details: Vec<String>,
    montant: String,
    couleur: Rgb,
    avis: Option<usize>,
    /// Le détail tient à côté du montant : le montant est au milieu des deux lignes (sinon sur celle du trade).
    centre: bool,
}

impl LigneHist {
    fn h(&self) -> f64 {
        HIST + HIST_SUITE * (self.details.len().max(1) - 1) as f64
    }
}

/// Un chiffre du ticket (`true` : gardé au-dessus de Placer au doigt), ou un avertissement (toujours montré).
enum Chiffre {
    Ligne(bool, String, String, Rgb),
    Note(String, Rgb),
}

/// Une ligne d'historique en deux morceaux : le trade (« ▲ BTC 92 512,3 → 92 400,1 ») et son détail (« fermé à la
/// main · +0,11 R · coûts 260,07 $ » ; les coûts seulement s'il y en a).
fn hist_txt(h: &ClosedTrade, v: &Vue) -> (String, String) {
    let principal = format!(
        "{} {}{} {} → {}",
        if h.side == Side::Long { "▲" } else { "▼" },
        h.base,
        h.levier.map_or(String::new(), |l| format!(" {}", perp::levier_txt(l))),
        v.prix_de(&h.base, h.entry),
        v.prix_de(&h.base, h.exit)
    );
    let raison = if h.liquide() { "💀 liquidation".to_string() } else { h.reason.clone() };
    let mut detail = format!("{raison} · {:+.2} R", h.r()).replace('.', ",");
    if h.fees > Money::ZERO {
        detail = format!("{detail} · coûts {}", argent(h.fees, false));
    }
    // ⚡ le funding payé (−) ou reçu (+) pendant le trade
    if h.funding != Money::ZERO {
        detail = format!("{detail} · funding {}", argent(Money::ZERO - h.funding, true));
    }
    (principal, detail)
}

/// La courbe d'équité dans `r` (ligne de départ en pointillé gris).
fn courbe(pm: &mut Pixmap, r: Rect, acc: &Portfolio) {
    round_fill(pm, r, 8.0, Rgb(0x13, 0x15, 0x1b), 1.0);
    let pts: Vec<f64> = std::iter::once(acc.start).chain(acc.curve.iter().map(|e| e.1)).map(|m| m.to_f64()).collect();
    if pts.len() < 2 {
        return;
    }
    let (lo, hi) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), v| (a.min(*v), b.max(*v)));
    let span = (hi - lo).max(1.0);
    let x = |i: usize| (r.x + 6.0 + i as f64 * (r.w - 12.0) / (pts.len() - 1) as f64) as f32;
    let y = |v: f64| (r.y + 6.0 + (hi - v) / span * (r.h - 12.0)) as f32;
    let base = y(acc.start.to_f64());
    lines(pm, 0.0, 0.0, &[&[(r.x as f32 + 6.0, base), ((r.x + r.w) as f32 - 6.0, base)]], GREY, 0.6);
    let line: Vec<(f32, f32)> = pts.iter().enumerate().map(|(i, v)| (x(i), y(*v))).collect();
    let c = couleur(*pts.last().unwrap_or(&0.0) - acc.start.to_f64());
    lines(pm, 0.0, 0.0, &[&line], c, 1.6);
}

/// 🟠 Bandeau DÉMO (en haut au centre, toujours visible) : « ● DÉMO · Solde · P&L du jour · ↺ ».
pub struct Bandeau;

impl Bandeau {
    /// Le texte et la place du bandeau (fenêtre `w`, sous la barre du haut `top`).
    pub fn texte(solde: Money, jour: Money) -> String {
        format!("DÉMO · {} · jour {}", argent(solde, false), argent(jour, true))
    }

    pub fn rect(text: &Text, w: f64, top: f64, solde: Money, jour: Money) -> Rect {
        let tw = text.width(&Self::texte(solde, jour), 11.5) as f64 + 22.0 + 30.0 + 12.0;
        Rect::new(((w - tw) / 2.0).max(4.0), top + 6.0, tw.min(w - 8.0), 24.0)
    }

    /// 🖥 Le texte raccourci au niveau `n` (0 : tout ; 1 : sans « jour » ; 2 : « DÉMO » seul).
    pub fn texte_n(solde: Money, jour: Money, n: usize) -> String {
        match n {
            0 => Self::texte(solde, jour),
            1 => format!("DÉMO · {}", argent(solde, false)),
            _ => "DÉMO".to_string(),
        }
    }

    /// 🖥 La pilule dans l'espace libre de la 1re rangée (`x0` … `x1`, rangée `y` × `h`) : centrée, avec le texte le
    /// plus complet qui tient (son niveau) ; `None` : même « DÉMO » ne tient pas.
    pub fn rect_dans(text: &Text, x0: f64, x1: f64, y: f64, h: f64, solde: Money, jour: Money) -> Option<(Rect, usize)> {
        (0..3).find_map(|n| {
            let tw = text.width(&Self::texte_n(solde, jour, n), 11.5) as f64 + 22.0 + 30.0 + 12.0;
            (tw <= x1 - x0).then(|| (Rect::new((x0 + x1 - tw) / 2.0, y + (h - 24.0) / 2.0, tw, 24.0), n))
        })
    }

    /// Le ↺ (coordonnées du bandeau).
    pub fn reset(r: Rect) -> Rect {
        Rect::new(r.w - 30.0, 0.0, 30.0, r.h)
    }

    pub fn draw(text: &Text, r: Rect, solde: Money, jour: Money, mouse: (f64, f64)) -> Option<Pixmap> {
        Self::draw_n(text, r, solde, jour, mouse, 0)
    }

    /// Dessine la pilule au niveau de texte `n` (voir `texte_n`).
    pub fn draw_n(text: &Text, r: Rect, solde: Money, jour: Money, mouse: (f64, f64), n: usize) -> Option<Pixmap> {
        let mut pm = crate::px::new(r.w.max(1.0) as u32, r.h.max(1.0) as u32)?;
        let b = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0);
        let orange = crate::palette::ORANGE;
        round_fill(&mut pm, b, r.h / 2.0, Rgb(0x2a, 0x1f, 0x10), 0.92);
        round_stroke(&mut pm, b, r.h / 2.0, orange, 1.0);
        round_fill(&mut pm, Rect::new(10.0, r.h / 2.0 - 3.5, 7.0, 7.0), 3.5, orange, 1.0);
        // « DÉMO · solde · jour » en or, le montant du jour en vert ou rouge
        let montant = if n == 0 { argent(jour, true) } else { String::new() };
        let debut = Self::texte_n(solde, jour, n);
        let debut = debut.strip_suffix(&montant).unwrap_or(&debut);
        let y = (r.h / 2.0 + 4.0) as f32;
        text.draw(&mut pm, debut, (24.0, y), 11.5, crate::palette::GOLD, Align::Left);
        let x = 24.0 + text.width(debut, 11.5);
        text.draw(&mut pm, &montant, (x, y), 11.5, couleur(jour.to_f64()), Align::Left);
        let z = Self::reset(r);
        if z.contains(mouse.0 - r.x, mouse.1 - r.y) {
            round_fill(&mut pm, Rect::new(z.x + 2.0, 3.0, z.w - 5.0, r.h - 6.0), 8.0, Rgb(0x4a, 0x3a, 0x20), 1.0);
        }
        text.draw(
            &mut pm,
            "↺",
            (z.cx() as f32, (r.h / 2.0 + 5.0) as f32),
            14.0,
            crate::palette::GOLD,
            Align::Center,
        );
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sans_prix(_: &str) -> Option<f64> {
        None
    }

    fn vue(acc: &Portfolio) -> Vue<'_> {
        Vue {
            acc,
            prix_de: &sans_prix,
            gardien: true,
            last: 2650.0,
            symbole: "XAU",
            dec: 2,
            jour: Money::ZERO,
            examen: None,
            replay: None,
            total: 0,
            seance: None,
            marge_bas: 0.0,
            couts: None,
            perp: None,
            avis: None,
            en_volet: false,
        }
    }

    /// Clique au milieu de la première puce dont le texte commence par `t`.
    fn clic(s: &mut Simulateur, acc: &Portfolio, t: &str) -> SimClick {
        clic_vue(s, &vue(acc), t)
    }

    /// Comme `clic`, avec une vue choisie (replay en cours, séance gardée…).
    fn clic_vue(s: &mut Simulateur, v: &Vue, t: &str) -> SimClick {
        let text = Text::default();
        let r = Simulateur::rect(1200.0, 800.0, 40.0);
        let p = s
            .plan(v, r.w, &text)
            .into_iter()
            .find_map(|b| match b {
                Bloc::Puce(p, x, _, _) if x.starts_with(t) => Some(p),
                _ => None,
            })
            .unwrap_or_else(|| panic!("puce {t}"));
        s.scroll = 0.0;
        s.click(&text, r, v, r.x + p.cx(), r.y + TETE + p.cy())
    }

    /// Ce que le volet montre : les textes des puces.
    fn puces_de(v: &Vue) -> Vec<String> {
        Simulateur::default()
            .plan(v, 1200.0, &Text::default())
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Puce(_, x, _, _) => Some(x),
                _ => None,
            })
            .collect()
    }

    /// Jee (20 nov.) : le volet défile jusqu'au bout (Statistiques, Résultat, Historique) sur PC, tablette et cell,
    /// même avec le rond Argos AI posé sur le bas.
    #[test]
    fn le_volet_defile_jusqu_au_bout() {
        let mut acc = Portfolio::default();
        for i in 0..12 {
            let id = acc
                .open(
                    argos_core::portfolio::Order {
                        base: "BTC".into(),
                        side: Side::Long,
                        qty: 1.0,
                        price: 100.0,
                        stop: Some(90.0),
                        target: None,
                        couts: None,
                        perp: None,
                    },
                    i,
                )
                .unwrap();
            acc.close(id, 101.0, i + 1, "objectif").unwrap();
        }
        let text = Text::default();
        for (w, h, marge) in [(1280.0, 800.0, 0.0), (1024.0, 700.0, 0.0), (390.0, 844.0, 70.0)] {
            let r = Simulateur::rect(w, h, 56.0);
            let mut v = vue(&acc);
            v.marge_bas = marge;
            let mut s = Simulateur::default();
            for _ in 0..400 {
                s.molette(40.0, r, &v);
            }
            // la dernière ligne de l'historique est visible, au-dessus de la marge
            let fin = s
                .plan(&v, r.w, &text)
                .iter()
                .filter_map(|b| match b {
                    Bloc::Ligne(y, ..) => Some(*y + LIGNE),
                    Bloc::Hist(y, l) => Some(*y + l.h()),
                    _ => None,
                })
                .fold(0.0f64, f64::max);
            let bas_visible = s.scroll + (r.h - TETE) - marge;
            assert!(fin <= bas_visible, "{w}×{h} : fin {fin} sous le bas visible {bas_visible}");
            assert!(puces_de(&v).len() > 3);
        }
    }

    /// 📱 Suivi 5 (Jee, 21 Nov): the Argos AI round lay over the panel's rows (Statistiques) while scrolling. The panel
    /// keeps a band at its bottom for the round: the content scrolls above it; nothing of it is drawn in the band, and
    /// a touch there takes nothing. (Without the band, the same row is drawn there and opens.)
    #[test]
    fn rien_du_contenu_sous_le_rond() {
        crate::actions::set_tactile(true);
        let acc = Portfolio::default();
        let text = Text::default();
        let r = Simulateur::rect(402.0, 600.0, 0.0);
        let mut v = vue(&acc);
        // the last folded row (« Courbe d'équité »), its middle at the band's middle
        let (y, rub) = Simulateur::default()
            .plan(&v, r.w, &text)
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Rubrique(y, _, _, rub) => Some((y, rub)),
                _ => None,
            })
            .next_back()
            .unwrap();
        let marge = 74.0;
        let scroll = TETE + y + RUBRIQUE / 2.0 - (r.h - marge / 2.0);
        assert!(scroll > 0.0, "{scroll}");
        let band_y = (r.h - marge) as u32..r.h as u32;
        for (m, dans) in [(marge, false), (0.0, true)] {
            v.marge_bas = m;
            let mut s = Simulateur {
                scroll,
                ..Simulateur::default()
            };
            let pm = s.draw(&text, r, &v, (0.0, 0.0)).unwrap();
            let fond = pm.pixel(4, band_y.start).unwrap();
            let vide = band_y.clone().all(|y| (1..r.w as u32 - 4).all(|x| pm.pixel(x, y) == Some(fond)));
            assert_eq!(vide, !dans, "band {m}: the row drawn in it: {dans}");
            s.click(&text, r, &v, r.x + r.w / 2.0, r.y + r.h - marge / 2.0);
            assert_eq!(s.ouvertes.contains(&rub), dans, "band {m}: the touch opens the row: {dans}");
        }
    }

    #[test]
    fn gestion_et_oco_au_ticket() {
        let pf = Portfolio::default();
        let mut s = Simulateur::default();
        // achat au marché à 2 000, SL 0,5 % (10 points = 1 R), R:R 2 → TP 2 020
        let t = s.ticket(2000.0, pf.balance);
        assert!(s.gestion(&t).vide(), "rien de choisi : rien");
        s.sorties = 2;
        (s.be, s.suiveur) = (true, true);
        let g = s.gestion(&t);
        assert_eq!(g.paliers.len(), 1, "2 R = l'objectif final : seul le palier à 1 R reste");
        assert_eq!((g.paliers[0].prix, g.be_r, g.suiveur), (2010.0, Some(1.0), Some(10.0)));
        s.rr = 2; // R:R 3
        assert_eq!(s.gestion(&s.ticket(2000.0, pf.balance)).paliers.len(), 2);
        assert_eq!(gestion_txt(&g), "🎯 1 objectif partiel · ⚖ BE à 1R · 🪜 suiveur");
        // OCO : seulement en ordre Stop, un Buy Stop au-dessus et un Sell Stop dessous
        s.oco = true;
        assert!(s.tickets_oco(2000.0, pf.balance).is_none());
        s.kind = Kind::Stop;
        let (a, b) = s.tickets_oco(2000.0, pf.balance).unwrap();
        assert!(a.buy && !b.buy && a.price > 2000.0 && b.price < 2000.0);
        assert_eq!(a.risk, b.risk);
        // les puces : choix au ticket, ✂ et ⚖ sur une position ouverte
        let mut acc = Portfolio::default();
        let id = acc
            .open(
                argos_core::portfolio::Order {
                    base: "XAU".into(),
                    side: Side::Long,
                    qty: 1.0,
                    price: 2600.0,
                    stop: Some(2590.0),
                    target: Some(2700.0),
                    couts: None,
                    perp: None,
                },
                0,
            )
            .unwrap();
        let mut s = Simulateur::default();
        assert_eq!(clic(&mut s, &acc, "🎯 ½ à 1R"), SimClick::Changed);
        assert_eq!(s.sorties, 1);
        clic(&mut s, &acc, "⚖ Break-even");
        assert!(s.be);
        assert!(!puces_de(&vue(&acc)).iter().any(|p| p.starts_with("🔀")), "OCO : ordre Stop seulement");
        assert_eq!(clic(&mut s, &acc, "✂ Sortir ½"), SimClick::Moitie(id));
        assert_eq!(clic(&mut s, &acc, "⚖ Stop à l'entrée"), SimClick::StopEntree(id));
    }

    /// Les lignes (gauche, droite) que le volet montre.
    fn lignes_de(v: &Vue) -> Vec<(String, String)> {
        Simulateur::default()
            .plan(v, 420.0, &Text::default())
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Ligne(_, g, d, _) => Some((g, d)),
                Bloc::Hist(_, l) => Some((l.principal, l.montant)),
                _ => None,
            })
            .collect()
    }

    /// Les notes grises que le volet montre (largeur 420), et le détail de chaque ligne de l'Historique.
    fn notes_de(v: &Vue) -> Vec<String> {
        Simulateur::default()
            .plan(v, 420.0, &Text::default())
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Note(_, t, _) => Some(t),
                Bloc::Hist(_, l) => Some(l.details.join(" ")),
                _ => None,
            })
            .collect()
    }

    /// ⚡ Lot 4 : sur un perpétuel, le levier au ticket, la marge isolée, le prix de liquidation et le funding avant
    /// d'envoyer ; au comptant, rien de tout ça.
    #[test]
    fn perpetuel_au_ticket() {
        let acc = Portfolio::default();
        let hl = PerpVue {
            bareme: Bareme {
                levier_max: 40.0,
                maintenance: 0.0125,
                funding_h: 1.0,
            },
            taux_8h: 0.0001,
            reel: true,
        };
        let v = Vue {
            perp: Some(hl),
            last: 100_000.0,
            symbole: "BTC",
            ..vue(&acc)
        };
        let p = puces_de(&v);
        assert!(p.contains(&"Levier".to_string()) && p.contains(&"×1".to_string()) && p.contains(&"×40".to_string()));
        assert!(!p.contains(&"×50".to_string()), "plafonné au maximum du marché");
        assert!(
            !puces_de(&vue(&acc)).iter().any(|x| x == "Levier" || x.starts_with('×')),
            "au comptant : pas de levier"
        );
        // 1 % de risque, SL 0,5 % : 2 BTC à 100 000 $, ×10 → 20 000 $ de marge ; liquidation à 91 250
        let l = lignes_de(&v);
        let trouve = |debut: &str| {
            l.iter()
                .find(|(g, _)| g.starts_with(debut))
                .cloned()
                .unwrap_or_else(|| panic!("{debut} : {l:?}"))
        };
        assert_eq!(
            trouve("⚡ Marge isolée"),
            ("⚡ Marge isolée 20 000,00 $ · ×10".into(), "dispo 100 000,00 $".into())
        );
        assert_eq!(trouve("💀 Liquidation").0, "💀 Liquidation 91\u{202f}250,00");
        let (g, d) = trouve("⚡ Funding");
        assert_eq!(g, "⚡ Funding 0,01 % / 8 h · réel", "le taux pour 8 h, comme les bourses l'écrivent");
        assert_eq!(d, "−2,50 $ / 1 h", "un paiement par heure : 2 BTC × 100 000 $ × 0,01 % / 8");
        // un clic sur ×40 : le levier suit (et les ordres partent avec)
        let mut s = Simulateur::default();
        assert_eq!(clic_vue(&mut s, &v, "×40"), SimClick::Changed);
        assert_eq!(s.perp(&v).map(|x| x.levier), Some(40.0));
        // ×100 avec un SL à 0,5 % : la liquidation (99 500) arrive avant le SL → dit en rouge
        let b = Bareme {
            levier_max: 100.0,
            maintenance: 0.005,
            funding_h: 8.0,
        };
        let v100 = Vue {
            perp: Some(PerpVue { bareme: b, ..hl }),
            last: 100_000.0,
            ..vue(&acc)
        };
        s.levier = 100.0;
        let notes: Vec<String> = s
            .plan(&v100, 420.0, &Text::default())
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Note(_, t, _) => Some(t),
                _ => None,
            })
            .collect();
        assert!(notes.iter().any(|n| n.starts_with("⚠ La liquidation arrive avant ton SL")), "{notes:?}");
        // un petit compte : la marge manque, dit avant d'envoyer
        let petit = Portfolio::new("Démo", 1_000.0, 0.0);
        let s1 = Simulateur {
            levier: 1.0,
            ..Default::default()
        };
        let v1 = Vue {
            perp: Some(hl),
            last: 100_000.0,
            ..vue(&petit)
        };
        let notes: Vec<String> = s1
            .plan(&v1, 420.0, &Text::default())
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Note(_, t, _) => Some(t),
                _ => None,
            })
            .collect();
        assert!(notes.iter().any(|n| n.starts_with("Marge insuffisante")), "{notes:?}");
        // ⏯ en replay : le taux de funding se choisit
        let vr = Vue {
            replay: Some(Replay {
                shown: 5,
                play: false,
                speed: 1,
            }),
            ..v
        };
        let p = puces_de(&vr);
        assert!(p.contains(&"Funding %".to_string()) && p.contains(&"0,01".to_string()) && p.contains(&"−0,01".to_string()));
        assert_eq!(clic_vue(&mut s, &vr, "−0,01"), SimClick::Changed);
        assert_eq!(s.taux_replay(), -0.0001);
        assert_eq!(
            (funding_txt(0.0), funding_txt(0.00005), funding_txt(-0.0003)),
            ("0 %".into(), "0,005 %".into(), "−0,03 %".into())
        );
    }

    /// 📱 Lot 4 (captures du 20 nov.) : la rangée du levier (jusqu'à ×100) et celle du funding du replay tiennent
    /// chacune sur une ligne, sur PC (volet de 420) comme sur un téléphone (390).
    #[test]
    fn rangees_perpetuel_sur_une_ligne() {
        let acc = Portfolio::default();
        let perp = PerpVue {
            bareme: Bareme {
                levier_max: 100.0,
                maintenance: 0.005,
                funding_h: 8.0,
            },
            taux_8h: 0.0001,
            reel: false,
        };
        let v = Vue {
            perp: Some(perp),
            last: 100_000.0,
            replay: Some(Replay {
                shown: 5,
                play: false,
                speed: 1,
            }),
            ..vue(&acc)
        };
        for w in [420.0, 390.0] {
            let puces: Vec<(String, f64)> = Simulateur::default()
                .plan(&v, w, &Text::default())
                .into_iter()
                .filter_map(|b| match b {
                    Bloc::Puce(p, t, _, _) => Some((t, p.y)),
                    _ => None,
                })
                .collect();
            let y = |t: &str| puces.iter().find(|p| p.0 == t).map(|p| p.1).unwrap_or_else(|| panic!("{t} : {puces:?}"));
            assert_eq!(y("Levier"), y("×100"), "largeur {w} : ×100 sur la ligne du levier");
            assert_eq!(y("Funding %"), y("−0,01"), "largeur {w} : −0,01 sur la ligne du funding");
        }
    }

    /// ⚡ 💀 Lot 4 : la position montre son levier, sa liquidation et son funding ; l'historique dit la liquidation et le
    /// funding, les statistiques les comptent.
    #[test]
    fn perpetuel_position_et_historique() {
        let mut acc = Portfolio::default();
        let ordre = |side: Side, prix: f64| argos_core::portfolio::Order {
            base: "BTC".into(),
            side,
            qty: 1.0,
            price: prix,
            stop: None,
            target: None,
            couts: None,
            perp: Some(Perp::new(10.0, 0.005, 8.0)),
        };
        let jour = 1_795_132_800;
        acc.open(ordre(Side::Long, 100_000.0), jour).unwrap();
        acc.payer_funding(jour + 8 * 3600, |_| Some(0.0001), |_| Some(100_000.0));
        let l = lignes_de(&vue(&acc));
        assert!(
            l.iter()
                .any(|(g, d)| g == "×10 · 💀 liq. 90\u{202f}500,00 · marge 10 000,00 $" && d == "funding −10,00 $"),
            "{l:?}"
        );
        assert!(l.iter().any(|(g, _)| g.starts_with("⚡ Marge tenue 10 000,00 $")), "{l:?}");
        let text = Text::default();
        let actions: Vec<String> = Simulateur::default()
            .plan(&vue(&acc), 420.0, &text)
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Action(_, g, ..) => Some(g),
                _ => None,
            })
            .collect();
        assert!(
            actions[0].starts_with("▲ BTC 100\u{202f}000,00 · SL"),
            "la ligne de la position garde sa place : {actions:?}"
        );
        // la bougie descend sous 90 500 : liquidée
        let b = argos_core::types::Bar {
            time: jour + 9 * 3600,
            open: 95_000.0,
            high: 95_000.0,
            low: 90_000.0,
            close: 90_200.0,
            volume: 1.0,
        };
        let (_, c) = acc.on_bar("BTC", &b, |_| None);
        assert!(c[0].liquide());
        let (principal, detail) = hist_txt(&c[0], &vue(&acc));
        assert!(principal.starts_with("▲ BTC ×10 "), "{principal}");
        assert!(detail.starts_with("💀 liquidation") && detail.ends_with("· funding −10,00 $"), "{detail}");
        let l = lignes_de(&vue(&acc));
        assert!(l.iter().any(|(g, d)| g == "💀 Liquidations" && d == "1"), "{l:?}");
        assert!(l.iter().any(|(g, d)| g == "⚡ Funding payé" && d == "10,00 $"), "{l:?}");
    }

    /// 💸 ⚡ Suivi (22 Nov : « Coûts payés » 563,90 $ ici, 593,90 $ au Rapport du mois, qui y compte le funding) : avec
    /// du funding, la ligne dit qu'elle le laisse à part (il a sa ligne, juste dessous).
    #[test]
    fn couts_payes_hors_funding() {
        let binance = Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0001,
            glissement: 0.0001,
        };
        let ordre = |perp: Option<Perp>| argos_core::portfolio::Order {
            base: "BTC".into(),
            side: Side::Long,
            qty: 1.0,
            price: 100_000.0,
            stop: None,
            target: None,
            couts: Some(binance),
            perp,
        };
        let jour = 1_795_132_800;
        let mut acc = Portfolio::default();
        let id = acc.open(ordre(None), jour).unwrap();
        acc.close(id, 100_000.0, jour + 60, "fermé à la main").unwrap();
        let l = lignes_de(&vue(&acc));
        assert!(l.iter().any(|(g, _)| g == "💸 Coûts payés"), "sans funding : telle quelle {l:?}");
        let id = acc.open(ordre(Some(Perp::new(10.0, 0.005, 8.0))), jour + 3600).unwrap();
        acc.payer_funding(jour + 8 * 3600, |_| Some(0.0001), |_| Some(100_000.0));
        acc.close(id, 100_000.0, jour + 9 * 3600, "fermé à la main").unwrap();
        let l = lignes_de(&vue(&acc));
        assert!(l.iter().any(|(g, _)| g == "💸 Coûts payés (hors funding)"), "{l:?}");
        assert!(l.iter().any(|(g, d)| g == "⚡ Funding payé" && d == "10,00 $"), "{l:?}");
    }

    /// 📱 Lot 3 : sur un téléphone (390 de large), aucune ligne d'historique n'est coupée : le trade et son montant
    /// tiennent sur la ligne, et le détail (raison, R, coûts) se lit en entier dessous (🤖 lot 7 : à côté du rond
    /// Argos AI ; le détail passe à la ligne s'il ne tient pas).
    #[test]
    fn historique_lisible_sur_telephone() {
        let text = Text::default();
        let c = Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0001,
            glissement: 0.0001,
        };
        let mut acc = Portfolio::default();
        for (i, side) in [Side::Long, Side::Short, Side::Long].into_iter().enumerate() {
            let id = acc
                .open(
                    argos_core::portfolio::Order {
                        base: "BTC".into(),
                        side,
                        qty: 3.2375,
                        price: 92_512.3,
                        stop: Some(if side == Side::Long { 92_050.0 } else { 92_980.0 }),
                        target: None,
                        couts: Some(c),
                        perp: None,
                    },
                    i as i64 * 100,
                )
                .unwrap();
            acc.close(id, 92_400.1, i as i64 * 100 + 50, "fermé à la main").unwrap();
        }
        let w = 390.0;
        let plan = Simulateur::default().plan(&vue(&acc), w, &text);
        let debut = plan
            .iter()
            .position(|b| matches!(b, Bloc::Titre(_, t) if t == "HISTORIQUE"))
            .expect("le titre Historique");
        let mut vus = 0;
        for b in &plan[debut + 1..] {
            if let Bloc::Hist(_, l) = b {
                let m = Text::graisse(Graisse::Medium).width(&l.montant, 12.5) as f64;
                let place = w - PAD - HIST_X - m - 12.0;
                assert!(text.width(&l.principal, 12.5) as f64 <= place, "ligne coupée : {} | {}", l.principal, l.montant);
                for d in &l.details {
                    assert!(text.width(d, 11.5) as f64 <= w - PAD - HIST_X + 1.0, "détail coupé : {d}");
                }
                let tout = l.details.join(" ");
                assert!(tout.contains("coûts ") && tout.ends_with(" $"), "les coûts se lisent en entier : {tout}");
                assert!(l.avis.is_some(), "chaque trade fini a son rond Argos AI");
                vus += 1;
            }
        }
        assert_eq!(vus, 3, "les trois trades, chacun avec son détail dessous");
    }

    /// 🤖 Lot 7 : chaque trade fini de l'Historique a son rond Argos AI ; un toucher sur sa ligne demande sa carte.
    /// Une sortie partielle n'a pas de rond (sa carte vient à la dernière sortie) ; la ligne du trade dont la carte
    /// est ouverte est mise en avant (bleu de la maquette).
    #[test]
    fn historique_le_rond_argos_ai_ouvre_la_carte() {
        let mut acc = Portfolio::default();
        let ordre = |qty: f64| argos_core::portfolio::Order {
            base: "BTC".into(),
            side: Side::Long,
            qty,
            price: 100.0,
            stop: Some(90.0),
            target: Some(120.0),
            couts: None,
            perp: None,
        };
        let id = acc.open(ordre(2.0), 0).unwrap();
        acc.close_part(id, 1.0, 105.0, 5, "sortie de la moitié").unwrap();
        acc.close(id, 110.0, 10, "fermé à la main").unwrap();
        let id = acc.open(ordre(1.0), 20).unwrap();
        acc.close(id, 90.0, 30, "stop").unwrap();
        let text = Text::default();
        let r = Simulateur::rect(1200.0, 800.0, 40.0);
        let v = vue(&acc);
        let mut s = Simulateur::default();
        let lignes: Vec<(f64, Option<usize>, String)> = s
            .plan(&v, r.w, &text)
            .into_iter()
            .filter_map(|b| match b {
                Bloc::Hist(y, l) => Some((y, l.avis, l.details.join(" "))),
                _ => None,
            })
            .collect();
        // la plus récente en haut ; la sortie partielle (index 0) sans rond
        let avis: Vec<Option<usize>> = lignes.iter().map(|l| l.1).collect();
        assert_eq!(avis, vec![Some(2), Some(1), None], "{lignes:?}");
        assert!(lignes[2].2.starts_with("sortie de la moitié"), "{lignes:?}");
        // un toucher sur la ligne (pas seulement sur le rond) : la carte de ce trade
        let clic = |s: &mut Simulateur, y: f64, x: f64| s.click(&text, r, &v, r.x + x, r.y + TETE + y + HIST / 2.0);
        assert_eq!(clic(&mut s, lignes[0].0, 30.0), SimClick::Avis(2));
        assert_eq!(clic(&mut s, lignes[1].0, r.w / 2.0), SimClick::Avis(1));
        assert_eq!(clic(&mut s, lignes[2].0, 30.0), SimClick::None, "une sortie partielle : pas de carte");
        // la ligne du trade dont la carte est ouverte : fond bleu derrière le texte
        s.scroll = (lignes[1].0 - 100.0).max(0.0);
        let bleu = |avis: Option<usize>| {
            let v = Vue { avis, ..vue(&acc) };
            let pm = s.draw(&text, r, &v, (0.0, 0.0)).unwrap();
            let k = crate::px::scale();
            let (x, y) = (r.w - 60.0, TETE + lignes[1].0 - s.scroll + 2.0);
            let p = pm.pixel((x as f32 * k) as u32, (y as f32 * k) as u32).unwrap();
            p.blue() as i32 - p.red() as i32
        };
        assert!(bleu(Some(1)) > bleu(None) + 10, "{} / {}", bleu(Some(1)), bleu(None));
    }

    /// 📅 Lot 8: « Rapport du mois » under the statistics once a trade is finished; a touch opens the page, and the
    /// button stays blue while the page is open.
    #[test]
    fn the_month_report_button() {
        let text = Text::default();
        // a tall window: the whole panel is seen without scrolling
        let r = Simulateur::rect(1200.0, 2400.0, 40.0);
        let buttons = |s: &Simulateur, v: &Vue| -> Vec<f64> {
            s.plan(v, r.w, &text)
                .into_iter()
                .filter_map(|b| match b {
                    Bloc::MonthButton(y) => Some(y),
                    _ => None,
                })
                .collect()
        };
        let empty = Portfolio::default();
        assert!(buttons(&Simulateur::default(), &vue(&empty)).is_empty(), "no trade yet: no report");
        let mut acc = Portfolio::default();
        let order = argos_core::portfolio::Order {
            base: "BTC".into(),
            side: Side::Long,
            qty: 1.0,
            price: 100.0,
            stop: Some(90.0),
            target: None,
            couts: None,
            perp: None,
        };
        let id = acc.open(order, 0).unwrap();
        acc.close(id, 105.0, 10, "objectif").unwrap();
        let v = vue(&acc);
        let mut s = Simulateur::default();
        let ys = buttons(&s, &v);
        assert_eq!(ys.len(), 1);
        // right under the « Résultat » line, before the Historique
        let plan = s.plan(&v, r.w, &text);
        let k = plan.iter().position(|b| matches!(b, Bloc::MonthButton(_))).unwrap();
        assert!(matches!(&plan[k - 1], Bloc::Ligne(_, t, ..) if t == "Résultat"));
        let p = month_button(ys[0], r.w);
        assert_eq!((p.x, p.w, p.h), (14.0, r.w - 28.0, 38.0));
        let at = |s: &mut Simulateur, x: f64, y: f64| s.click(&text, r, &v, r.x + x, r.y + TETE + y);
        assert_eq!(at(&mut s, p.cx(), p.cy()), SimClick::MonthReport);
        assert_eq!(at(&mut s, p.x + 3.0, p.y + 2.0), SimClick::MonthReport);
        assert_eq!(at(&mut s, 6.0, p.cy()), SimClick::None, "beside the button");
        assert_eq!(at(&mut s, p.cx(), p.y - 4.0), SimClick::None, "above the button");
        // its words, and blue while the page is open
        let _ = crate::text::ecrits();
        let pixel = |s: &Simulateur| {
            let pm = s.draw(&text, r, &v, (0.0, 0.0)).unwrap();
            let k = crate::px::scale();
            let (x, y) = (p.x + p.w / 2.0, TETE + p.y + 4.0);
            let c = pm.pixel((x as f32 * k) as u32, (y as f32 * k) as u32).unwrap();
            c.blue() as i32 - c.red() as i32
        };
        let closed = pixel(&s);
        let said = crate::text::ecrits();
        assert!(said.iter().any(|t| t == "Rapport du mois"), "{said:?}");
        s.month = Some(crate::month::MonthPage::default());
        assert!(pixel(&s) > closed + 10, "{} / {closed}", pixel(&s));
    }

    /// 💸 Lot 3 : les coûts de la bourse dans le ticket (avant l'envoi) et dans l'historique (résultat net).
    #[test]
    fn couts_au_ticket_et_a_l_historique() {
        let binance = Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0001,
            glissement: 0.0001,
        };
        let mut acc = Portfolio::default();
        let v = Vue {
            couts: Some(("Binance", binance)),
            ..vue(&acc)
        };
        let puces = puces_de(&v);
        assert!(puces.contains(&"💸 Coûts réels · Binance".to_string()) && puces.contains(&"Sans frais".to_string()));
        let l = lignes_de(&v);
        let trouve = |debut: &str| {
            l.iter()
                .find(|(g, _)| g.starts_with(debut))
                .cloned()
                .unwrap_or_else(|| panic!("{debut} : {l:?}"))
        };
        assert_eq!(
            trouve("💸 maker"),
            ("💸 maker 0,02 % · taker 0,05 %".into(), "écart 0,01 % · gliss. 0,01 %".into())
        );
        // achat au marché à 2 650, SL 0,5 % (13,25), 1 % de risque (1 000 $) → 75,47… onces
        let t = Simulateur::default().ticket(2650.0, acc.balance);
        let qty = argos_core::simu::size_for_risk(t.risk, t.price, t.sl);
        let e = estimer(&binance, true, qty, t.price, t.sl, t.tp);
        assert_eq!(
            trouve("Coûts au SL"),
            (format!("Coûts au SL {}", argent(e.au_sl, false)), format!("au TP {}", argent(e.au_tp, false)))
        );
        let (g, _) = trouve("Net au SL");
        assert_eq!(g, format!("Net au SL {}", argent(Money::ZERO - t.risk - e.au_sl, true)));
        assert!(e.au_sl > e.au_tp, "le TP attend dans le carnet (maker) : il coûte moins que le SL");
        // « Sans frais » : un clic, puis plus de coûts au ticket
        let mut s = Simulateur::default();
        assert_eq!(clic_vue(&mut s, &v, "Sans frais"), SimClick::SansFrais(true));
        acc.sans_frais = true;
        let v = Vue {
            couts: Some(("Binance", binance)),
            ..vue(&acc)
        };
        assert!(!lignes_de(&v).iter().any(|(g, _)| g.starts_with("Net au SL")));
        assert_eq!(clic_vue(&mut s, &v, "💸 Coûts réels"), SimClick::SansFrais(false));
        // pas de barème : dit tel quel
        assert!(lignes_de(&vue(&acc))
            .iter()
            .any(|(g, d)| g.starts_with("💸 Pas de barème") && d == "sans frais"));
        // l'historique montre le résultat net et les coûts payés
        acc.sans_frais = false;
        let id = acc
            .open(
                argos_core::portfolio::Order {
                    base: "BTC".into(),
                    side: Side::Long,
                    qty: 1.0,
                    price: 10_000.0,
                    stop: Some(9_900.0),
                    target: None,
                    couts: Some(binance),
                    perp: None,
                },
                0,
            )
            .unwrap();
        let c = acc.close(id, 10_100.0, 10, "fermé à la main").unwrap();
        // taker 0,05 % + demi-écart 0,005 % + glissement 0,01 % : 6,50 $ à l'entrée, 6,57 $ à la sortie
        assert_eq!(c.fees, Money::from_f64(6.5 + 6.57));
        let l = lignes_de(&vue(&acc));
        let h = l.iter().find(|(g, _)| g.starts_with("▲ BTC")).unwrap();
        // les coûts au bout de la ligne, ou dans le détail gris juste dessous si la ligne est trop longue
        let notes = notes_de(&vue(&acc));
        assert!(
            h.0.ends_with("· coûts 13,07 $") || notes.iter().any(|n| n.ends_with("· coûts 13,07 $")),
            "{} / {notes:?}",
            h.0
        );
        assert_eq!(h.1, "+86,93 $", "le résultat net : 100 $ moins les coûts");
        assert!(l.iter().any(|(g, d)| g == "💸 Coûts payés" && d == "13,07 $"));
    }

    #[test]
    fn seance_de_replay_choisir_sauver_reprendre() {
        let acc = Portfolio::default();
        let mut s = Simulateur::default();
        let mut v = vue(&acc);
        // hors replay, sans séance : rejouer ou choisir la date, rien à reprendre
        assert_eq!(clic_vue(&mut s, &v, "📅 Choisir la date"), SimClick::ReplayDate);
        assert!(!puces_de(&v).iter().any(|p| p.starts_with("▶ Reprendre")));
        // une séance gardée : reprendre ou oublier
        v.seance = Some("BTC · 15m · 12 mars 2026 14:00".into());
        assert_eq!(clic_vue(&mut s, &v, "▶ Reprendre la séance"), SimClick::SeanceReprendre);
        assert_eq!(clic_vue(&mut s, &v, "🗑 Oublier"), SimClick::SeanceOublier);
        // pendant le replay : sauver et quitter, et plus de « reprendre »
        v.replay = Some(Replay {
            shown: 10,
            play: false,
            speed: 10,
        });
        let p = puces_de(&v);
        assert!(p.contains(&"×10".to_string()), "{p:?}");
        assert!(!p.iter().any(|x| x.starts_with("▶ Reprendre") || x.starts_with("📅")), "{p:?}");
        assert_eq!(clic_vue(&mut s, &v, "💾 Sauver et quitter"), SimClick::SeanceSauver);
        assert_eq!(clic_vue(&mut s, &v, "✕ Direct"), SimClick::ReplayStop);
    }

    #[test]
    fn argent_en_cents_exacts() {
        assert_eq!(argent(Money::from_cents(10_000_000), false), "100 000,00 $");
        assert_eq!(argent(Money::from_cents(14_205), true), "+142,05 $");
        assert_eq!(argent(Money::from_cents(-5), true), "−0,05 $");
    }

    /// 🔢 Suivi 6 (Jee, 21 Nov): the panel wrote « 53836,55 » where the chart wrote « 53 836,5 ». The panel's prices
    /// are written as the chart writes them: the chart's precision for its market, the separators of the 🌐 format.
    #[test]
    fn prix_ecrits_comme_la_chart() {
        let mut acc = Portfolio::default();
        let chart = crate::viewport::format_price(53_836.55, 1);
        assert_eq!(chart, "53\u{202f}836,6", "the chart: thousands apart, its one decimal");
        {
            let v = Vue {
                symbole: "BTC",
                dec: 1,
                last: 53_836.55,
                ..vue(&acc)
            };
            assert_eq!((v.prix(53_836.55), v.prix_de("BTC", 53_836.55)), (chart.clone(), chart.clone()));
            assert_eq!(v.prix_de("XAU", 2650.5), "2\u{202f}650,50", "another market: by its size");
            assert_eq!(v.prix_de("EURUSD", 1.08123), "1,08123");
            // the ticket's entry (a market order: the last price)
            let l = lignes_de(&v);
            assert!(l.iter().any(|(g, _)| *g == format!("Entrée {chart}")), "{l:?}");
        }
        // the history: a BTC trade from 53 836,55 to 54 000
        let o = argos_core::portfolio::Order {
            base: "BTC".into(),
            side: Side::Long,
            qty: 0.1,
            price: 53_836.55,
            stop: Some(53_500.0),
            target: None,
            couts: None,
            perp: None,
        };
        let id = acc.open(o, 0).unwrap();
        let c = acc.close(id, 54_000.0, 60, "fermé à la main").unwrap();
        let v = Vue {
            symbole: "BTC",
            dec: 1,
            ..vue(&acc)
        };
        let fin = crate::viewport::format_price(54_000.0, 1);
        assert_eq!(hist_txt(&c, &v).0, format!("▲ BTC {chart} → {fin}"));
    }

    #[test]
    fn ticket_sl_tp_toujours_poses() {
        let mut s = Simulateur::default();
        let pf = Portfolio::default();
        let t = s.ticket(2000.0, pf.balance);
        // achat au marché, SL 0,5 % sous l'entrée, TP à 2 R, 1 % du compte
        assert_eq!((t.price, t.sl, t.tp, t.risk), (2000.0, 1990.0, 2020.0, Money::from_cents(100_000)));
        (s.buy, s.kind) = (false, Kind::Limit);
        let t = s.ticket(2000.0, pf.balance);
        assert_eq!(t.price, 2005.0, "vente limite au-dessus du prix");
        assert!(t.sl > t.price && t.tp < t.price);
        assert!(t.check(&pf, 2000.0).is_ok());
        s.kind = Kind::Stop;
        let t = s.ticket(2000.0, pf.balance);
        assert_eq!(t.price, 1995.0, "vente stop sous le prix");
        assert!(t.check(&pf, 2000.0).is_ok());
    }

    #[test]
    fn clics_du_volet() {
        let acc = Portfolio::default();
        let mut s = Simulateur::default();
        assert_eq!(clic(&mut s, &acc, "▼ VENTE"), SimClick::Changed);
        assert!(!s.buy);
        assert_eq!(clic(&mut s, &acc, "Stop"), SimClick::Changed);
        assert_eq!(s.kind, Kind::Stop);
        assert_eq!(clic(&mut s, &acc, "Placer"), SimClick::Place);
        // ↺ demande une confirmation
        assert_eq!(clic(&mut s, &acc, "↺ Remise"), SimClick::Changed);
        assert!(s.confirme);
        assert_eq!(clic(&mut s, &acc, "Non"), SimClick::Changed);
        assert!(!s.confirme);
        clic(&mut s, &acc, "↺ Remise");
        assert_eq!(clic(&mut s, &acc, "Oui"), SimClick::Reset);
        // scénarios et fuseau du reset
        assert_eq!(s.regles.reset_tz, "Europe/Prague");
        clic(&mut s, &acc, "1 étape");
        assert_eq!(s.regles.daily_loss, 3.0);
        clic(&mut s, &acc, "Remise du jour");
        assert_eq!(s.regles.reset_tz, "America/New_York", "réglable");
        clic(&mut s, &acc, "Personnalisé");
        assert_eq!(s.regles.reset_tz, "America/New_York", "fuseau gardé en changeant de scénario");
        clic(&mut s, &acc, "Jour 5 %");
        assert_eq!(s.regles.daily_loss, 2.0, "5 → 2 (on tourne)");
        assert_eq!(clic(&mut s, &acc, "↺ Rejouer"), SimClick::ReplayStart);
        assert!(s
            .draw(&Text::default(), Simulateur::rect(1200.0, 800.0, 40.0), &vue(&acc), (0.0, 0.0))
            .is_some());
    }

    #[test]
    fn echec_propose_la_lecon() {
        let acc = Portfolio::default();
        let e = Etat {
            phase: 0,
            profit_pct: -5.0,
            jour_pct: 5.0,
            total_pct: 5.0,
            jours: 1,
            note: None,
            verdict: Verdict::Echoue {
                raison: "Perte du jour".into(),
                lecon: argos_propfirm::examen::LECON_CHALLENGE,
            },
        };
        let mut s = Simulateur::default();
        let text = Text::default();
        let r = Simulateur::rect(1200.0, 800.0, 40.0);
        let mut v = vue(&acc);
        v.examen = Some(&e);
        let p = s
            .plan(&v, r.w, &text)
            .into_iter()
            .find_map(|b| match b {
                Bloc::Puce(p, x, _, _) if x.starts_with("🎓 Leçon") => Some(p),
                _ => None,
            })
            .unwrap();
        assert_eq!(
            s.click(&text, r, &v, r.x + p.cx(), r.y + TETE + p.cy()),
            SimClick::Lecon("6-02-passer-un-challenge-propfirm")
        );
        assert!(Bandeau::texte(acc.balance, Money::ZERO).starts_with("DÉMO · 100"));
    }

    /// 📸 Captures du volet et du bandeau pour Jee : seulement si `ARGOS_CAPTURES` donne un dossier.
    #[test]
    fn captures_pour_jee() {
        let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
        use argos_core::types::Bar;
        let t = Text::default();
        let mut acc = Portfolio::default();
        let mut px = 2650.0;
        let rien = |_: &argos_core::portfolio::Order| None;
        let ticket = |buy: bool, kind: Kind, price: f64, sl: f64, tp: f64, pct: f64, solde: Money| Ticket {
            buy,
            kind,
            price,
            sl,
            tp,
            risk: solde.pct(pct),
        };
        // une petite séance : 3 gagnants, 2 perdants, une position encore ouverte et un ordre limite
        for (i, gagne) in [true, false, true, true, false].into_iter().enumerate() {
            let tk = ticket(true, Kind::Market, 0.0, px - 10.0, px + 20.0, 1.0, acc.balance);
            tk.place(&mut acc, "XAUUSD", px, i as i64 * 3_600, None, None).unwrap();
            let (h, l) = if gagne { (px + 21.0, px - 2.0) } else { (px + 2.0, px - 11.0) };
            let b = Bar {
                time: i as i64 * 3_600 + 1_800,
                open: px,
                high: h,
                low: l,
                close: px,
                volume: 0.0,
            };
            acc.on_bar("XAUUSD", &b, rien);
            px += if gagne { 15.0 } else { -8.0 };
        }
        let tk = ticket(false, Kind::Market, 0.0, px + 8.0, px - 16.0, 0.5, acc.balance);
        tk.place(&mut acc, "XAUUSD", px, 20_000, None, None).unwrap();
        let tk = ticket(true, Kind::Limit, px - 6.0, px - 14.0, px + 10.0, 0.5, acc.balance);
        tk.place(&mut acc, "XAUUSD", px, 20_000, None, None).unwrap();
        let e = Etat {
            phase: 0,
            profit_pct: 3.97,
            jour_pct: 0.4,
            total_pct: 0.0,
            jours: 2,
            note: None,
            verdict: Verdict::EnCours,
        };
        let r = Simulateur::rect(1280.0, 900.0, 40.0);
        let mut s = Simulateur::default();
        let mut v = vue(&acc);
        let prix_ouvert = move |_: &str| Some(px - 3.0);
        v.prix_de = &prix_ouvert;
        (v.last, v.symbole, v.jour, v.examen) = (px, "XAUUSD", Money::from_cents(14_205), Some(&e));
        v.replay = Some(Replay {
            shown: 120,
            play: true,
            speed: 2,
        });
        v.total = 200;
        let save = |s: &Simulateur, v: &Vue, nom: &str| s.draw(&t, r, v, (0.0, 0.0)).unwrap().save_png(format!("{dir}/{nom}.png")).unwrap();
        save(&s, &v, "01-volet-compte-et-ticket");
        s.scroll = 560.0;
        save(&s, &v, "02-volet-replay-et-examen");
        s.scroll = 5000.0;
        s.molette(0.0, r, &v);
        save(&s, &v, "03-volet-courbe-stats-historique");
        let rate = Etat {
            verdict: Verdict::Echoue {
                raison: "Perte du jour : −5,02 % (limite 5 %), le Gardien aurait verrouillé la journée".into(),
                lecon: argos_propfirm::examen::LECON_CHALLENGE,
            },
            ..e.clone()
        };
        v.examen = Some(&rate);
        s.scroll = 560.0;
        save(&s, &v, "04-examen-echoue-lecon");
        s.scroll = 0.0;
        s.confirme = true;
        save(&s, &v, "05-remise-a-zero-confirmation");
        let b = Bandeau::rect(&t, 1280.0, 40.0, acc.balance, v.jour);
        Bandeau::draw(&t, b, acc.balance, v.jour, (0.0, 0.0))
            .unwrap()
            .save_png(format!("{dir}/06-bandeau-demo.png"))
            .unwrap();
    }
    /// 📱 Liste 3 (points 1, 2 ; « ok liste 3 » de Jee, 21 nov.) : au doigt, le ticket ouvert (sens, type, risque,
    /// Placer, avec Entrée · SL / TP et Net), puis une ligne › par rubrique (Objectifs et stops, Coûts, Détail sous
    /// Placer, Replay, Scénario d'examen, Courbe) qui s'ouvre et se replie d'un toucher ; le PC montre tout, comme avant.
    #[test]
    fn au_doigt_rubriques_repliees() {
        let text = Text::default();
        let couts = Couts {
            maker: 0.0002,
            taker: 0.0005,
            ecart: 0.0001,
            glissement: 0.0001,
        };
        let acc = Portfolio::default();
        let v = Vue {
            couts: Some(("Binance", couts)),
            ..vue(&acc)
        };
        let r = Simulateur::rect(402.0, 874.0, 0.0);
        let rubriques = |s: &Simulateur| -> Vec<String> {
            s.plan(&v, r.w, &text)
                .into_iter()
                .filter_map(|b| match b {
                    Bloc::Rubrique(_, t, ..) => Some(t),
                    _ => None,
                })
                .collect()
        };
        let lignes = |s: &Simulateur| -> Vec<String> {
            s.plan(&v, r.w, &text)
                .into_iter()
                .filter_map(|b| match b {
                    Bloc::Ligne(_, g, ..) => Some(g),
                    _ => None,
                })
                .collect()
        };
        let puces = |s: &Simulateur| -> Vec<String> {
            s.plan(&v, r.w, &text)
                .into_iter()
                .filter_map(|b| match b {
                    Bloc::Puce(_, t, ..) => Some(t),
                    _ => None,
                })
                .collect()
        };
        let toucher = |s: &mut Simulateur, nom: &str| {
            let y = s
                .plan(&v, r.w, &text)
                .into_iter()
                .find_map(|b| match b {
                    Bloc::Rubrique(y, t, ..) if t == nom => Some(y),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("rubrique {nom}"));
            s.scroll = 0.0;
            s.click(&text, r, &v, r.x + r.w / 2.0, r.y + TETE + y + RUBRIQUE / 2.0)
        };
        // 🖥 PC : aucune rubrique, tout est là
        crate::actions::set_tactile(false);
        let pc = Simulateur::default();
        assert!(rubriques(&pc).is_empty());
        assert!(lignes(&pc).iter().any(|g| g.starts_with("Taille")) && puces(&pc).iter().any(|t| t == "Sans frais"));
        // 📱 au doigt : le ticket ouvert, les rubriques repliées
        crate::actions::set_tactile(true);
        let mut s = Simulateur::default();
        assert_eq!(
            rubriques(&s),
            [
                "🎯 Objectifs et stops",
                "💸 Coûts",
                "Détail",
                "▶ Replay",
                "🎓 Scénario d'examen",
                "Courbe d'équité"
            ]
        );
        let p = puces(&s);
        assert!(p.iter().any(|t| t == "▲ ACHAT") && p.iter().any(|t| t == Kind::Limit.name()), "{p:?}");
        assert!(p.iter().any(|t| t.starts_with("Placer l'ordre")), "{p:?}");
        assert!(
            !p.iter().any(|t| t.starts_with("🎯 ") || t == "Sans frais" || t.starts_with("↺ Rejouer")),
            "{p:?}"
        );
        let l = lignes(&s);
        assert!(
            l.iter().any(|g| g.starts_with("Entrée")) && l.iter().any(|g| g.starts_with("Net au SL")),
            "{l:?}"
        );
        assert!(!l.iter().any(|g| g.starts_with("Taille") || g.starts_with("Coûts au SL")), "{l:?}");
        // un toucher ouvre « Détail », un autre le replie
        assert_eq!(toucher(&mut s, "Détail"), SimClick::Changed);
        assert!(lignes(&s).iter().any(|g| g.starts_with("Taille")));
        for n in ["🎯 Objectifs et stops", "💸 Coûts", "▶ Replay", "🎓 Scénario d'examen", "Courbe d'équité"] {
            assert_eq!(toucher(&mut s, n), SimClick::Changed);
        }
        let p = puces(&s);
        assert!(p.iter().any(|t| t == "Sans frais") && p.iter().any(|t| t.starts_with("↺ Rejouer")), "{p:?}");
        toucher(&mut s, "Détail");
        assert!(!lignes(&s).iter().any(|g| g.starts_with("Taille")));
        // 🛡 « 🎓 Scénario » (la partie PropFirm) : la rubrique s'ouvre
        let mut e = Simulateur::default();
        e.defile_vers(&v, r.w, &text, "🎓 Scénario");
        assert!(e.ouvertes.contains(&Rub::Examen));
        crate::actions::set_tactile(false);
    }

    /// 🖥 In the Trading volet (its 🧪 tab, PC): no background nor header of its own (the volet's), the content from the
    /// top, and where its ✕ was is content (the volet has its own ✕).
    #[test]
    fn dans_le_volet_trading_sans_en_tete() {
        let acc = Portfolio::default();
        let text = Text::default();
        let r = Rect::new(966.0, 182.0, 418.0, 600.0);
        let v = Vue { en_volet: true, ..vue(&acc) };
        let mut s = Simulateur::default();
        assert_eq!(Simulateur::tete(&v), 0.0);
        assert_eq!(Simulateur::visible(r, &v), r.h, "the whole height for the content");
        let x = Simulateur::fermer(r);
        assert_ne!(s.click(&text, r, &v, r.x + x.cx(), r.y + x.cy()), SimClick::Close, "no ✕ of its own");
        // a chip answers at its place, counted from the top
        let p = s
            .plan(&v, r.w, &text)
            .into_iter()
            .find_map(|b| match b {
                Bloc::Puce(p, t, _, _) if t.starts_with("▼ VENTE") => Some(p),
                _ => None,
            })
            .expect("VENTE");
        assert_eq!(s.click(&text, r, &v, r.x + p.cx(), r.y + p.cy()), SimClick::Changed);
        assert!(!s.buy, "VENTE chosen");
        // drawn without a background: the corners stay clear (the volet's glass shows), the content is there
        let pm = s.draw(&text, r, &v, (0.0, 0.0)).unwrap();
        assert_eq!(pm.pixel(1, 1).unwrap().alpha(), 0, "no background of its own");
        assert_eq!(pm.pixel(1, (r.h - 2.0) as u32).unwrap().alpha(), 0);
        assert!(pm.pixels().iter().any(|c| c.alpha() > 0), "its content");
        // the floating panel keeps its header and its ✕
        let flottant = vue(&acc);
        assert_eq!(Simulateur::tete(&flottant), TETE);
        assert_eq!(s.click(&text, r, &flottant, r.x + x.cx(), r.y + x.cy()), SimClick::Close);
    }
}
