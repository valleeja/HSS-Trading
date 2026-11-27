//! 🤖 « Argos AI commente tes trades » (lot 7 du Simulateur) : après un trade démo ou replay fermé, trois choses, une
//! phrase chacune : ce que disaient les indicateurs à l'entrée (lus à l'ouverture de la position, rangés par rapport à
//! son sens), si le plan a été suivi (stop éloigné, sortie avant l'objectif…), une leçon à revoir ; puis le rappel.
//! Des règles sur ce que le trade a gardé ([`argos_core::portfolio::Plan`]) : pas de réseau, pas d'horloge, jamais un
//! ordre ni un conseil d'acheter ou de vendre.

use crate::format::num;
use crate::together::{list, Item};
use crate::{read_with, Coverage, Lean, ScreenData};
use argos_core::portfolio::{ClosedTrade, Lecture, Plan, LIQUIDATION};
use argos_core::{Bar, Money, Side};
use std::borrow::Borrow;

/// Le rappel de la carte d'un trade.
pub const TRADE_REMINDER: &str = "Analyse affichée, pas un ordre.";

/// 🎓 La leçon de l'Académie qui parle de protéger son capital (le stop, le levier) : « Savoir dans quoi tu
/// t'embarques ».
pub const LECON_CAPITAL: &str = "0-02-savoir-dans-quoi-tu-t-embarques";

/// Le ton d'une partie de la carte : la couleur de sa pastille.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Mixed,
    Bad,
    Neutral,
}

/// Le plan a-t-il été suivi ?
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Suivi,
    EnPartie,
    PasSuivi,
    /// Fermé par le Gardien PropFirm, la fin du replay… : sans jugement.
    Neutre,
}

/// 📅 Ce qui s'est écarté du plan, du plus grave au moins grave (le bilan du mois compte ceux qui reviennent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Ecart {
    Liquide,
    SansStop,
    StopEloigne,
    CoupeAvantStop,
    SortieAvantObjectif,
    ObjectifEloigne,
    Resserre,
}

/// Une partie de la carte : sa pastille (ton et texte) et sa phrase.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub tone: Tone,
    pub chip: String,
    pub text: String,
}

/// Le commentaire d'un trade fermé.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeReview {
    pub long: bool,
    pub base: String,
    /// Le résultat du trade entier (ses sorties partielles comprises) : en R (`None` sans stop : pas de R), et net
    /// (coûts et funding déduits).
    pub r: Option<f64>,
    pub net: Money,
    /// « stop touché », « objectif atteint »…
    pub exit: String,
    /// « À l'entrée ».
    pub entry: Part,
    /// « Ton plan ».
    pub plan: Part,
    pub verdict: Verdict,
    /// L'écart au plan, quand il y en a un.
    pub ecart: Option<Ecart>,
    /// « Une leçon à revoir », et la leçon de l'Académie qui va avec (son identifiant).
    pub lesson: String,
    pub academy: Option<&'static str>,
    pub reminder: &'static str,
}

/// 🤖 Ce que disent les indicateurs `items` sur `closed_bars` (les bougies clôturées quand la position s'ouvre), avec
/// ce que l'écran a pour chacun (`data[k]` pour `items[k]`), pour un trade à l'achat (`long`) ou à la vente : leurs
/// noms dans son sens, contre lui, sans sens net. Ceux sans direction (agitation, volume), pas lus ou sans assez de
/// bougies sont seulement comptés.
pub fn read_for_trade(items: &[Item], closed_bars: &[Bar], data: &[ScreenData], long: bool) -> Lecture {
    let mut l = Lecture::default();
    for (k, it) in items.iter().enumerate() {
        let r = read_with(it.id, closed_bars, it.settings, data.get(k).unwrap_or(&ScreenData::default()));
        let nom = |note: Option<&str>| note.map_or_else(|| it.name.to_string(), |n| format!("{} ({n})", it.name));
        match r.sense.filter(|_| r.coverage == Coverage::Rules) {
            Some(x) if x.lean == Lean::Neutral => l.neutres.push(nom(x.note)),
            Some(x) if x.lean == Lean::NoDirection => l.autres += 1,
            Some(x) if (x.lean == Lean::Up) == long => l.pour.push(nom(x.note)),
            Some(x) => l.contre.push(nom(x.note)),
            None => l.autres += 1,
        }
    }
    l
}

/// Comment le trade est sorti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fin {
    Stop,
    Objectif,
    BreakEven,
    Suiveur,
    Liquidation,
    Main,
    Gardien,
    Replay,
    Autre,
}

fn fin(reason: &str) -> Fin {
    match reason {
        "stop" => Fin::Stop,
        "objectif" | "objectif partiel" => Fin::Objectif,
        "break-even" => Fin::BreakEven,
        "stop suiveur" => Fin::Suiveur,
        LIQUIDATION => Fin::Liquidation,
        "fermé à la main" | "manuel" | "sortie de la moitié" | "tout fermer" => Fin::Main,
        "gardien PropFirm" => Fin::Gardien,
        "fin du replay" | "séance oubliée" => Fin::Replay,
        _ => Fin::Autre,
    }
}

/// « +2,00 R », « −1,62 R », « 0 R » (dans le format de nombres choisi).
pub fn r_text(r: f64) -> String {
    if r.abs() < 0.005 {
        return "0 R".into();
    }
    format!("{}{} R", if r < 0.0 { "−" } else { "+" }, num(r.abs(), 2))
}

/// « A, B et C » ; au-delà de 4 noms : « A, B, C, D et 2 autres ».
fn court(noms: &[String]) -> String {
    if noms.len() <= 4 {
        list(noms)
    } else {
        format!("{} et {} autres", noms[..4].join(", "), noms.len() - 4)
    }
}

/// Le plan d'un trade d'avant le lot 7 : un stop quand il avait un risque, l'objectif inconnu, aucune retouche connue.
fn ancien_plan(t: &ClosedTrade) -> Plan {
    let s = if t.side == Side::Long { 1.0 } else { -1.0 };
    let par_unite = (t.qty > 0.0).then(|| t.risk.to_f64() / t.qty).filter(|d| *d > 0.0);
    Plan {
        stop0: par_unite.map(|d| t.entry - d * s),
        ..Plan::default()
    }
}

/// ⚖ L'objectif prévu, en R (`None` sans stop ou sans objectif).
fn objectif_en_r(p: &Plan, entry: f64, long: bool) -> Option<f64> {
    let s = if long { 1.0 } else { -1.0 };
    let risque = (entry - p.stop0?) * s;
    let gain = (p.target0? - entry) * s;
    (risque > 0.0 && gain > 0.0).then(|| gain / risque)
}

/// « À l'entrée » : la pastille (combien dans ton sens) et ceux qui étaient contre toi (sinon ceux dans ton sens).
fn entree(l: Option<&Lecture>, ancien: bool) -> Part {
    let neutre = |chip: &str, text: &str| Part {
        tone: Tone::Neutral,
        chip: chip.into(),
        text: text.into(),
    };
    let Some(l) = l else {
        return neutre(
            "Pas lus",
            if ancien {
                "Trade d'avant les commentaires d'Argos AI : rien n'a été lu à l'entrée."
            } else {
                "Les indicateurs n'ont pas été lus à l'entrée."
            },
        );
    };
    if !l.pas_lue.is_empty() {
        return neutre("Pas lus", &format!("Pas lus à l'entrée : {}.", l.pas_lue));
    }
    let n = l.pour.len() + l.contre.len() + l.neutres.len();
    if n == 0 {
        return if l.autres == 0 {
            neutre("Aucun", "Aucun indicateur sur la chart à l'entrée.")
        } else {
            neutre("Sans direction", "Tes indicateurs ne donnaient pas de sens à l'entrée.")
        };
    }
    let tone = match l.pour.len().cmp(&l.contre.len()) {
        std::cmp::Ordering::Greater => Tone::Good,
        std::cmp::Ordering::Equal => Tone::Mixed,
        std::cmp::Ordering::Less => Tone::Bad,
    };
    let text = if !l.contre.is_empty() {
        format!("Contre toi : {}.", court(&l.contre))
    } else if l.pour.is_empty() {
        format!("Sans sens net : {}.", court(&l.neutres))
    } else {
        format!("Dans ton sens : {}.", court(&l.pour))
    };
    Part {
        tone,
        chip: format!("{} sur {n} dans ton sens", l.pour.len()),
        text,
    }
}

/// Ce que le trade a gardé et fait, pour juger le plan et choisir la leçon.
struct Faits<'a> {
    plan: &'a Plan,
    fin: Fin,
    reason: &'a str,
    long: bool,
    entry: f64,
    r: f64,
    /// Des sorties partielles avant la dernière.
    partiels: bool,
}

/// « puis sorti au stop à −0,40 R » : l'issue du trade, après une retouche.
fn issue(f: Fin, rt: &str) -> String {
    match f {
        Fin::Stop => format!("sorti au stop à {rt}"),
        Fin::Objectif => format!("objectif atteint à {rt}"),
        Fin::BreakEven => "sorti au break-even".into(),
        Fin::Suiveur => format!("stop suiveur touché à {rt}"),
        _ => format!("sorti à {rt}"),
    }
}

/// « Ton plan » : suivi, en partie, pas suivi (ou sans jugement), pourquoi, et l'écart au plan quand il y en a un.
fn plan_suivi(x: &Faits) -> (Verdict, Part, Option<Ecart>) {
    let rt = r_text(x.r);
    let s = if x.long { 1.0 } else { -1.0 };
    let p = x.plan;
    let dire = |v: Verdict, e: Option<Ecart>, text: String| {
        let (tone, chip) = match v {
            Verdict::Suivi => (Tone::Good, "Suivi"),
            Verdict::EnPartie => (Tone::Mixed, "En partie"),
            Verdict::PasSuivi => (Tone::Bad, "Pas suivi"),
            Verdict::Neutre => (Tone::Neutral, "Sortie"),
        };
        (v, Part { tone, chip: chip.into(), text }, e)
    };
    let neutre = |chip: &str, text: String| {
        (
            Verdict::Neutre,
            Part {
                tone: Tone::Neutral,
                chip: chip.into(),
                text,
            },
            None,
        )
    };
    if x.fin == Fin::Liquidation {
        let t = if p.stop0.is_none() { "Liquidé sans stop" } else { "Liquidé avant le stop" };
        return dire(Verdict::PasSuivi, Some(Ecart::Liquide), format!("{t} : toute la marge isolée est perdue."));
    }
    let Some(stop0) = p.stop0 else {
        return dire(
            Verdict::PasSuivi,
            Some(Ecart::SansStop),
            "Pas de stop à l'entrée : la perte n'avait pas de limite.".into(),
        );
    };
    if let Some(loin) = p.retouches.stop_eloigne {
        let (d0, d) = ((x.entry - stop0) * s, (x.entry - loin) * s);
        let t = if x.fin == Fin::Stop && x.r < -1.005 {
            format!("Stop éloigné à la main : la perte a fait {rt} au lieu de −1 R.")
        } else if d0 > 0.0 {
            format!("Stop éloigné à la main : le risque est passé à {} R.", num(d / d0, 1))
        } else {
            "Stop éloigné à la main pendant le trade.".into()
        };
        return dire(Verdict::PasSuivi, Some(Ecart::StopEloigne), t);
    }
    match x.fin {
        Fin::Gardien => return neutre("Gardien PropFirm", format!("Fermé par le Gardien PropFirm (limite du jour) : {rt}.")),
        Fin::Replay => {
            let chip = if x.reason == "séance oubliée" {
                "Séance oubliée"
            } else {
                "Fin du replay"
            };
            return neutre(chip, format!("Le replay s'est arrêté avant ta sortie : {rt} à ce moment-là."));
        }
        Fin::Main => {
            let t = match objectif_en_r(p, x.entry, x.long) {
                _ if x.r.abs() < 0.005 => "Sorti à la main au prix d'entrée : 0 R.".to_string(),
                _ if x.r < 0.0 => format!("Sorti à la main avant le stop : {rt} au lieu de −1 R."),
                Some(prevu) => format!("Sorti à la main avant l'objectif : {rt} pris sur {} prévus.", r_text(prevu)),
                None => format!("Sorti à la main : {rt}."),
            };
            let e = if x.r <= -0.005 { Ecart::CoupeAvantStop } else { Ecart::SortieAvantObjectif };
            return dire(Verdict::EnPartie, Some(e), t);
        }
        _ => {}
    }
    let rt_issue = issue(x.fin, &rt);
    if p.retouches.objectif_eloigne {
        return dire(
            Verdict::EnPartie,
            Some(Ecart::ObjectifEloigne),
            format!("Objectif éloigné à la main, puis {rt_issue}."),
        );
    }
    if p.retouches.stop_rapproche || p.retouches.objectif_rapproche {
        let quoi = if p.retouches.stop_rapproche { "Stop" } else { "Objectif" };
        return dire(
            Verdict::EnPartie,
            Some(Ecart::Resserre),
            format!("{quoi} rapproché à la main, puis {rt_issue}."),
        );
    }
    match x.fin {
        Fin::Stop if x.partiels => dire(Verdict::Suivi, None, format!("Objectif partiel pris, puis sorti au stop : {rt}.")),
        Fin::Stop if x.r < -1.05 => dire(
            Verdict::Suivi,
            None,
            format!("Sorti au stop prévu, plus loin à cause d'un trou de cotation : {rt}."),
        ),
        Fin::Stop => dire(Verdict::Suivi, None, "Sorti au stop prévu : perte tenue à 1 R.".into()),
        Fin::Objectif if x.partiels => dire(Verdict::Suivi, None, format!("Objectifs partiels, puis objectif atteint : {rt}.")),
        Fin::Objectif if objectif_en_r(p, x.entry, x.long).is_some() => dire(Verdict::Suivi, None, format!("Objectif atteint : {rt} comme prévu.")),
        Fin::Objectif => dire(Verdict::Suivi, None, format!("Objectif atteint : {rt}.")),
        Fin::BreakEven if x.partiels => dire(Verdict::Suivi, None, format!("Objectif partiel pris, puis stop à l'entrée touché : {rt}.")),
        Fin::BreakEven => dire(Verdict::Suivi, None, format!("Stop monté à l'entrée, sorti à {rt}.")),
        Fin::Suiveur => dire(Verdict::Suivi, None, format!("Stop suiveur touché : {rt}.")),
        _ => dire(Verdict::Neutre, None, format!("Sorti ({}) : {rt}.", x.reason)),
    }
}

/// 🎓 La leçon qui va avec un écart au plan, et la leçon de l'Académie quand une va avec.
pub(crate) fn lecon_ecart(e: Ecart) -> (&'static str, Option<&'static str>) {
    match e {
        Ecart::Liquide => ("Pose le stop avant le prix de liquidation, ou prends moins de levier.", Some(LECON_CAPITAL)),
        Ecart::SansStop => ("Pose un stop dès l'entrée : c'est lui qui fixe ta perte.", Some(LECON_CAPITAL)),
        Ecart::StopEloigne => ("Garde le stop où ton plan l'a mis : c'est lui qui limite ta perte.", Some(LECON_CAPITAL)),
        Ecart::CoupeAvantStop => ("Note au journal ce qui t'a fait couper avant le stop.", None),
        Ecart::SortieAvantObjectif => ("Note au journal pourquoi tu es sorti avant l'objectif : peur, nouvelle ou signal ?", None),
        Ecart::ObjectifEloigne => ("Fixe l'objectif avant d'entrer : le déplacer en route change ton plan.", None),
        Ecart::Resserre => ("Note au journal pourquoi tu as resserré ton plan pendant le trade.", None),
    }
}

/// « Une leçon à revoir » : la première qui s'applique, et la leçon de l'Académie quand une va avec.
fn lecon(x: &Faits, lecture: Option<&Lecture>, gain: Money, couts: Money) -> (String, Option<&'static str>) {
    let p = x.plan;
    let dit = |t: &str| (t.to_string(), None);
    let de = |e: Ecart| {
        let (t, a) = lecon_ecart(e);
        (t.to_string(), a)
    };
    if x.fin == Fin::Liquidation {
        return de(Ecart::Liquide);
    }
    if p.stop0.is_none() {
        return de(Ecart::SansStop);
    }
    if p.retouches.stop_eloigne.is_some() {
        return de(Ecart::StopEloigne);
    }
    if lecture.is_some_and(|l| l.pas_lue.is_empty() && l.contre.len() > l.pour.len()) {
        return dit("Tes indicateurs penchaient contre ce trade : note au journal ce qui t'a fait entrer.");
    }
    if x.fin == Fin::Main && x.r >= 0.005 && p.target0.is_some() {
        return de(Ecart::SortieAvantObjectif);
    }
    if x.fin == Fin::Main && x.r <= -0.005 {
        return de(Ecart::CoupeAvantStop);
    }
    if p.retouches.objectif_eloigne {
        return de(Ecart::ObjectifEloigne);
    }
    if p.retouches.stop_rapproche || p.retouches.objectif_rapproche {
        return de(Ecart::Resserre);
    }
    if gain > Money::ZERO && couts.to_f64() >= 0.3 * gain.to_f64() {
        let pct = (couts.to_f64() / gain.to_f64() * 100.0).round();
        return (format!("Les coûts ont pris {pct:.0} % du gain : un ordre Limite paie moins de frais."), None);
    }
    if let Some(prevu) = objectif_en_r(p, x.entry, x.long).filter(|r| *r < 0.95) {
        return (
            format!("L'objectif valait {} R pour 1 R de risque : un gain ne couvrait pas une perte.", num(prevu, 1)),
            None,
        );
    }
    match x.fin {
        Fin::Gardien => dit("Le Gardien a arrêté ta journée : relis au journal les trades qui y ont mené."),
        Fin::Replay => dit("Le replay s'est arrêté avant ta sortie : relance-le plus loin pour voir la fin du trade."),
        Fin::Stop if x.r < 0.0 => dit("Perte prévue et tenue : une perte dans le plan n'est pas une erreur."),
        Fin::BreakEven if x.r.abs() < 0.005 => dit("Le break-even a protégé ton capital sur ce trade."),
        _ if x.r >= 0.005 => dit("Plan suivi jusqu'au bout : note au journal ce qui rendait l'entrée claire."),
        _ => dit("Note au journal ce que tu retiens de ce trade."),
    }
}

/// 🤖 Le commentaire d'un trade fermé : `parts`, ses sorties dans l'ordre (les partielles, puis la dernière). Le
/// résultat est celui du trade entier ; le plan et la lecture, ceux que la dernière sortie a gardés. `None` sans
/// sortie.
pub fn review<T: Borrow<ClosedTrade>>(parts: &[T]) -> Option<TradeReview> {
    let parts: Vec<&ClosedTrade> = parts.iter().map(Borrow::borrow).collect();
    let last = *parts.last()?;
    let long = last.side == Side::Long;
    let pnl: Money = parts.iter().map(|c| c.pnl).sum();
    let risk: Money = parts.iter().map(|c| c.risk).sum();
    let r = (risk > Money::ZERO).then(|| pnl.to_f64() / risk.to_f64());
    let plan = last.plan.clone().unwrap_or_else(|| ancien_plan(last));
    let lecture = last.plan.as_ref().and_then(|p| p.lecture.as_ref());
    let f = fin(&last.reason);
    let faits = Faits {
        plan: &plan,
        fin: f,
        reason: &last.reason,
        long,
        entry: last.entry,
        r: r.unwrap_or(0.0),
        partiels: parts.len() > 1,
    };
    let (verdict, plan_part, ecart) = plan_suivi(&faits);
    let couts: Money = parts.iter().map(|c| c.fees + c.funding).sum();
    let (lesson, academy) = lecon(&faits, lecture, pnl, couts);
    let exit = match f {
        Fin::Stop => "stop touché",
        Fin::Objectif => "objectif atteint",
        Fin::BreakEven => "sorti au break-even",
        Fin::Suiveur => "stop suiveur touché",
        Fin::Liquidation => "liquidé",
        Fin::Main => "fermé à la main",
        Fin::Gardien => "fermé par le Gardien PropFirm",
        Fin::Replay | Fin::Autre => last.reason.as_str(),
    };
    Some(TradeReview {
        long,
        base: last.base.clone(),
        r,
        net: parts.iter().map(|c| c.net()).sum(),
        exit: exit.to_string(),
        entry: entree(lecture, last.plan.is_none()),
        plan: plan_part,
        verdict,
        ecart,
        lesson,
        academy,
        reminder: TRADE_REMINDER,
    })
}

/// Les sorties du trade `t` dans `history` (même symbole, même heure d'ouverture), dans l'ordre : ses sorties
/// partielles, puis `t`.
pub fn trade_parts<'a>(history: &'a [ClosedTrade], t: &ClosedTrade) -> Vec<&'a ClosedTrade> {
    history.iter().filter(|h| h.base == t.base && h.opened_at == t.opened_at).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::portfolio::{Gestion, Order, Palier, Portfolio, Retouches};

    fn ordre(side: Side, stop: Option<f64>, target: Option<f64>) -> Order {
        Order {
            base: "BTC".into(),
            side,
            qty: 1.0,
            price: 100.0,
            stop,
            target,
            couts: None,
            perp: None,
        }
    }

    /// Un compte sans frais avec une position ouverte à 100 (stop 90, objectif 120 à l'achat).
    fn compte(side: Side) -> (Portfolio, u64) {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let (st, tp) = if side == Side::Long { (90.0, 120.0) } else { (110.0, 80.0) };
        let id = pf.open(ordre(side, Some(st), Some(tp)), 0).unwrap();
        (pf, id)
    }

    fn lecture(pour: &[&str], contre: &[&str]) -> Lecture {
        Lecture {
            pour: pour.iter().map(|s| s.to_string()).collect(),
            contre: contre.iter().map(|s| s.to_string()).collect(),
            ..Lecture::default()
        }
    }

    fn un(pf: &Portfolio) -> TradeReview {
        review(std::slice::from_ref(pf.history.last().unwrap())).unwrap()
    }

    #[test]
    fn objectif_atteint_plan_suivi_indicateurs_dans_le_sens() {
        let (mut pf, id) = compte(Side::Long);
        pf.positions[0].lecture = Some(lecture(&["EMA 9/21", "MACD", "RSI 14"], &[]));
        assert_eq!(pf.positions[0].id, id);
        pf.check_exits("BTC", 121.0, 101.0, 10);
        let v = un(&pf);
        assert_eq!((v.long, v.base.as_str(), v.exit.as_str()), (true, "BTC", "objectif atteint"));
        assert!((v.r.unwrap() - 2.0).abs() < 1e-9 && v.net == Money::from_f64(20.0));
        assert_eq!(
            v.entry,
            Part {
                tone: Tone::Good,
                chip: "3 sur 3 dans ton sens".into(),
                text: "Dans ton sens : EMA 9/21, MACD et RSI 14.".into()
            }
        );
        assert_eq!((v.verdict, v.plan.chip.as_str()), (Verdict::Suivi, "Suivi"));
        assert_eq!(v.plan.text, "Objectif atteint : +2,00 R comme prévu.");
        assert_eq!(v.lesson, "Plan suivi jusqu'au bout : note au journal ce qui rendait l'entrée claire.");
        assert_eq!((v.academy, v.reminder), (None, TRADE_REMINDER));
    }

    #[test]
    fn stop_eloigne_plan_pas_suivi_et_lecon_de_l_academie() {
        let (mut pf, id) = compte(Side::Long);
        pf.positions[0].lecture = Some(lecture(&["RSI 14"], &["Supertrend", "MACD"]));
        pf.set_level(id, true, 85.0).unwrap();
        pf.check_exits("BTC", 99.0, 84.0, 10);
        let v = un(&pf);
        assert_eq!(v.exit, "stop touché");
        assert_eq!((v.entry.tone, v.entry.chip.as_str()), (Tone::Bad, "1 sur 3 dans ton sens"));
        assert_eq!(v.entry.text, "Contre toi : Supertrend et MACD.");
        assert_eq!((v.verdict, v.plan.tone), (Verdict::PasSuivi, Tone::Bad));
        assert_eq!(v.plan.text, "Stop éloigné à la main : la perte a fait −1,50 R au lieu de −1 R.");
        assert_eq!(v.lesson, "Garde le stop où ton plan l'a mis : c'est lui qui limite ta perte.");
        assert_eq!(v.academy, Some(LECON_CAPITAL));
        // éloigné puis le trade sort autrement : le risque devenu plus grand est dit
        let (mut pf, id) = compte(Side::Short);
        pf.set_level(id, true, 115.0).unwrap();
        pf.close(id, 104.0, 5, "fermé à la main").unwrap();
        assert_eq!(un(&pf).plan.text, "Stop éloigné à la main : le risque est passé à 1,5 R.");
    }

    #[test]
    fn sortie_a_la_main_avant_l_objectif_ou_avant_le_stop() {
        let (mut pf, id) = compte(Side::Long);
        pf.close(id, 105.0, 5, "fermé à la main").unwrap();
        let v = un(&pf);
        assert_eq!((v.verdict, v.plan.chip.as_str()), (Verdict::EnPartie, "En partie"));
        assert_eq!(v.plan.text, "Sorti à la main avant l'objectif : +0,50 R pris sur +2,00 R prévus.");
        assert_eq!(v.lesson, "Note au journal pourquoi tu es sorti avant l'objectif : peur, nouvelle ou signal ?");
        let (mut pf, id) = compte(Side::Short);
        pf.close(id, 104.0, 5, "fermé à la main").unwrap();
        let v = un(&pf);
        assert_eq!(v.plan.text, "Sorti à la main avant le stop : −0,40 R au lieu de −1 R.");
        assert_eq!(v.lesson, "Note au journal ce qui t'a fait couper avant le stop.");
    }

    #[test]
    fn perte_au_stop_dans_le_plan_n_est_pas_une_erreur() {
        let (mut pf, _) = compte(Side::Short);
        pf.check_exits("BTC", 111.0, 101.0, 10);
        let v = un(&pf);
        assert_eq!((v.long, v.verdict), (false, Verdict::Suivi));
        assert_eq!(v.plan.text, "Sorti au stop prévu : perte tenue à 1 R.");
        assert_eq!(v.lesson, "Perte prévue et tenue : une perte dans le plan n'est pas une erreur.");
        // rien de lu à l'entrée (une position d'un chemin sans lecture) : dit clairement, sans pastille de couleur
        assert_eq!((v.entry.tone, v.entry.chip.as_str()), (Tone::Neutral, "Pas lus"));
        assert_eq!(v.entry.text, "Les indicateurs n'ont pas été lus à l'entrée.");
    }

    /// Un trade sorti en plusieurs fois est commenté entier, à sa dernière sortie.
    #[test]
    fn objectif_partiel_puis_break_even_le_trade_entier() {
        let (mut pf, id) = compte(Side::Long);
        pf.gerer(
            id,
            Gestion {
                be_r: Some(1.0),
                paliers: vec![Palier { prix: 110.0, part: 0.5 }],
                ..Gestion::default()
            },
        )
        .unwrap();
        pf.check_exits("BTC", 111.0, 105.0, 10);
        let c = pf.check_exits("BTC", 101.0, 99.0, 20);
        assert_eq!(c[0].reason, "break-even");
        let parts = trade_parts(&pf.history, &c[0]);
        assert_eq!(parts.len(), 2);
        let v = review(&parts).unwrap();
        assert!((v.r.unwrap() - 0.5).abs() < 1e-9, "{:?}", v.r);
        assert_eq!(v.plan.text, "Objectif partiel pris, puis stop à l'entrée touché : +0,50 R.");
        assert_eq!(v.lesson, "Plan suivi jusqu'au bout : note au journal ce qui rendait l'entrée claire.");
        // le break-even seul : protégé, à 0 R
        let (mut pf, id) = compte(Side::Long);
        pf.set_stop_entree(id).unwrap();
        pf.check_exits("BTC", 101.0, 99.0, 20);
        let v = un(&pf);
        assert_eq!(
            (v.exit.as_str(), v.plan.text.as_str()),
            ("sorti au break-even", "Stop monté à l'entrée, sorti à 0 R.")
        );
        assert_eq!(v.lesson, "Le break-even a protégé ton capital sur ce trade.");
    }

    #[test]
    fn retouches_de_l_objectif_et_du_stop_en_partie() {
        let (mut pf, id) = compte(Side::Long);
        pf.set_level(id, false, 130.0).unwrap();
        pf.check_exits("BTC", 131.0, 101.0, 10);
        let v = un(&pf);
        assert_eq!(
            (v.verdict, v.plan.text.as_str()),
            (Verdict::EnPartie, "Objectif éloigné à la main, puis objectif atteint à +3,00 R.")
        );
        assert_eq!(v.lesson, "Fixe l'objectif avant d'entrer : le déplacer en route change ton plan.");
        let (mut pf, id) = compte(Side::Long);
        pf.set_level(id, true, 95.0).unwrap();
        pf.check_exits("BTC", 99.0, 94.0, 10);
        let v = un(&pf);
        assert_eq!(v.plan.text, "Stop rapproché à la main, puis sorti au stop à −0,50 R.");
        assert_eq!(v.lesson, "Note au journal pourquoi tu as resserré ton plan pendant le trade.");
        assert!(pf.history[0].plan.as_ref().unwrap().retouches.stop_rapproche);
    }

    #[test]
    fn sans_stop_liquidation_gardien_et_fin_du_replay() {
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        let id = pf.open(ordre(Side::Long, None, Some(120.0)), 0).unwrap();
        pf.close(id, 95.0, 5, "fermé à la main").unwrap();
        let v = un(&pf);
        assert_eq!(
            (v.verdict, v.plan.text.as_str()),
            (Verdict::PasSuivi, "Pas de stop à l'entrée : la perte n'avait pas de limite.")
        );
        assert_eq!(
            (v.lesson.as_str(), v.academy),
            ("Pose un stop dès l'entrée : c'est lui qui fixe ta perte.", Some(LECON_CAPITAL))
        );
        assert_eq!(v.r, None, "sans stop : pas de R");
        // liquidé : le trade est fermé par la bourse, la marge perdue
        let mut t = pf.history[0].clone();
        t.reason = LIQUIDATION.into();
        let v = review(&[t]).unwrap();
        assert_eq!(
            (v.exit.as_str(), v.plan.text.as_str()),
            ("liquidé", "Liquidé sans stop : toute la marge isolée est perdue.")
        );
        assert_eq!(v.lesson, "Pose le stop avant le prix de liquidation, ou prends moins de levier.");
        // le Gardien et la fin du replay : sans jugement
        for (raison, chip) in [
            ("gardien PropFirm", "Gardien PropFirm"),
            ("fin du replay", "Fin du replay"),
            ("séance oubliée", "Séance oubliée"),
        ] {
            let (mut pf, id) = compte(Side::Long);
            pf.close(id, 104.0, 5, raison).unwrap();
            let v = un(&pf);
            assert_eq!(
                (v.verdict, v.plan.tone, v.plan.chip.as_str()),
                (Verdict::Neutre, Tone::Neutral, chip),
                "{raison}"
            );
        }
    }

    #[test]
    fn couts_et_objectif_plus_petit_que_le_risque() {
        // un gain de 10 $ et 4,20 $ de frais (2 % à l'entrée sur 100, 2 % à la sortie sur 110) : 42 % du gain
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.02);
        let id = pf.open(ordre(Side::Long, Some(90.0), Some(110.0)), 0).unwrap();
        pf.check_exits("BTC", 111.0, 101.0, 10);
        let v = un(&pf);
        assert_eq!(v.lesson, "Les coûts ont pris 42 % du gain : un ordre Limite paie moins de frais.");
        assert_eq!(pf.positions.len(), 0, "{id}");
        // objectif à 0,5 R
        let mut pf = Portfolio::new("Démo", 10_000.0, 0.0);
        pf.open(ordre(Side::Long, Some(90.0), Some(105.0)), 0).unwrap();
        pf.check_exits("BTC", 106.0, 101.0, 10);
        assert_eq!(
            un(&pf).lesson,
            "L'objectif valait 0,5 R pour 1 R de risque : un gain ne couvrait pas une perte."
        );
    }

    /// Les trades d'avant le lot 7 (sans plan) : commentés avec ce qu'ils ont gardé, sans lecture ni retouche.
    #[test]
    fn trade_d_avant_le_lot_7() {
        let (mut pf, _) = compte(Side::Long);
        pf.check_exits("BTC", 101.0, 89.0, 10);
        let mut t = pf.history[0].clone();
        t.plan = None;
        let v = review(&[t]).unwrap();
        assert_eq!(v.entry.text, "Trade d'avant les commentaires d'Argos AI : rien n'a été lu à l'entrée.");
        assert_eq!((v.verdict, v.plan.text.as_str()), (Verdict::Suivi, "Sorti au stop prévu : perte tenue à 1 R."));
        assert_eq!(review::<ClosedTrade>(&[]), None);
    }

    #[test]
    fn lecture_rangee_par_rapport_au_sens_du_trade() {
        // 300 bougies d'une heure qui descendent : RSI, Supertrend et VWAP penchent à la baisse
        let bars: Vec<Bar> = (0..300)
            .map(|i| {
                let c = |i: usize| 400.0 - i as f64 * 0.5 + (i as f64 / 3.0).sin();
                let o = if i == 0 { c(0) } else { c(i - 1) };
                Bar {
                    time: 1_699_920_000 + i as i64 * 3_600,
                    open: o,
                    high: o.max(c(i)) + 0.4,
                    low: o.min(c(i)) - 0.4,
                    close: c(i),
                    volume: 100.0 + (i % 7) as f64 * 15.0,
                }
            })
            .collect();
        let item = |id: &'static str, name: &'static str| Item { id, name, settings: &[] };
        let items = [
            item("Rsi", "RSI 14"),
            item("Supertrend", "Supertrend"),
            item("Vwap", "VWAP"),
            item("Atr", "ATR 14"),
            item("Lens", "Argos Lens"),
        ];
        let short = read_for_trade(&items, &bars, &[], false);
        assert_eq!(short.pour, vec!["RSI 14 (en survente)", "Supertrend", "VWAP"]);
        assert!(short.contre.is_empty() && short.neutres.is_empty());
        // l'ATR (sans direction) et Argos Lens (pas lu) sont seulement comptés
        assert_eq!((short.autres, short.pas_lue.as_str()), (2, ""));
        let long = read_for_trade(&items, &bars, &[], true);
        assert_eq!((long.pour.len(), long.contre.len()), (0, 3));
        let p = entree(Some(&long), false);
        assert_eq!((p.tone, p.chip.as_str()), (Tone::Bad, "0 sur 3 dans ton sens"));
        assert_eq!(p.text, "Contre toi : RSI 14 (en survente), Supertrend et VWAP.");
        assert_eq!(read_for_trade(&[], &bars, &[], true), Lecture::default());
        assert_eq!(entree(Some(&Lecture::default()), false).text, "Aucun indicateur sur la chart à l'entrée.");
        let seuls = read_for_trade(&items[3..], &bars, &[], true);
        assert_eq!(entree(Some(&seuls), false).text, "Tes indicateurs ne donnaient pas de sens à l'entrée.");
        let ailleurs = Lecture {
            pas_lue: "la chart montrait ETH".into(),
            ..Lecture::default()
        };
        assert_eq!(entree(Some(&ailleurs), false).text, "Pas lus à l'entrée : la chart montrait ETH.");
        // plus de 4 noms : les 4 premiers et le nombre des autres
        let beaucoup = lecture(&["A", "B", "C", "D", "E", "F"], &[]);
        assert_eq!(entree(Some(&beaucoup), false).text, "Dans ton sens : A, B, C, D et 2 autres.");
        assert_eq!(Retouches::default().stop_eloigne, None);
    }

    /// Jamais un ordre ni un conseil d'acheter ou de vendre, dans aucun commentaire.
    #[test]
    fn jamais_un_ordre() {
        let mut textes = vec![];
        for raison in [
            "stop",
            "objectif",
            "break-even",
            "stop suiveur",
            "fermé à la main",
            "gardien PropFirm",
            "fin du replay",
            LIQUIDATION,
            "autre",
        ] {
            for side in [Side::Long, Side::Short] {
                let (mut pf, id) = compte(side);
                pf.close(id, 100.0 + if side == Side::Long { 5.0 } else { -5.0 }, 5, raison).unwrap();
                let v = un(&pf);
                textes.push(format!("{} {} {} {}", v.entry.text, v.plan.text, v.lesson, v.reminder).to_lowercase());
            }
        }
        for t in textes {
            for mot in ["achète", "vends", "achete", "prends position", "entre maintenant", "acheter", "vendre"] {
                assert!(!t.contains(mot), "« {mot} » dans : {t}");
            }
        }
    }
}
