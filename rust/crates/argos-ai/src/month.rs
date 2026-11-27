//! 📅 « Argos AI · ton mois » (lot 8 of the Simulator): over the finished trades of a month, what worked, the
//! deviation from the plan that comes back the most, a lesson, then the reminder. The verdicts are those of each
//! trade's card (lot 7): rules, no network, never an order nor an advice to buy or sell.

use crate::trade::{lecon_ecart, r_text, review, Ecart, Part, Tone, TradeReview, Verdict, TRADE_REMINDER};
use argos_core::portfolio::ClosedTrade;
use std::borrow::Borrow;

/// The summary of a month.
#[derive(Debug, Clone, PartialEq)]
pub struct MonthReview {
    /// The plan verdict of each trade, in the month's order.
    pub verdicts: Vec<Verdict>,
    /// The judged trades (without the ends of replay nor the PropFirm Guardian), and among them those whose plan was
    /// followed, partly followed, not followed.
    pub judged: usize,
    pub followed: usize,
    pub partly: usize,
    pub not_followed: usize,
    /// The sum of the R of the trades whose plan was followed (those that had a stop).
    pub r_followed: f64,
    /// « Ce qui a marché » and « À revoir »: their chip and their sentence.
    pub worked: Part,
    pub to_review: Part,
    /// « Une leçon », and the Academy lesson that goes with it (its id).
    pub lesson: String,
    pub academy: Option<&'static str>,
    pub reminder: &'static str,
}

fn part(tone: Tone, chip: &str, text: String) -> Part {
    Part { tone, chip: chip.into(), text }
}

/// « Ce qui a marché »: the followed plans and what they made.
fn worked(judged: usize, followed: usize, r: f64) -> Part {
    if judged == 0 {
        return part(
            Tone::Neutral,
            "—",
            "Pas de plan à juger : tes trades se sont arrêtés avant ta sortie (fin du replay, Gardien PropFirm).".into(),
        );
    }
    if followed == 0 {
        return part(Tone::Bad, "0 suivi", "Aucun plan suivi jusqu'au bout ce mois-ci.".into());
    }
    let (tes, ont) = if followed == 1 {
        ("Ton plan suivi".to_string(), "a")
    } else {
        (format!("Tes {followed} plans suivis"), "ont")
    };
    if r >= 0.005 {
        let seul = if followed == 1 { "à lui seul" } else { "à eux seuls" };
        part(Tone::Good, &r_text(r), format!("{tes} {ont} fait {} {seul}.", r_text(r)))
    } else {
        part(
            Tone::Good,
            &format!("{followed} sur {judged}"),
            format!("{tes} {ont} tenu tes pertes à ce qui était prévu."),
        )
    }
}

/// « À revoir »: the deviation `e`, back `n` times, its trades having made `r` R in all (`worse` of them beyond
/// −1 R).
fn to_review(e: Ecart, n: usize, r: f64, worse: usize) -> Part {
    let times = if n == 1 { String::new() } else { format!(" {n} fois") };
    let in_all = if n == 1 { "" } else { " en tout" };
    let rt = r_text(r);
    let (tone, text) = match e {
        Ecart::Liquide if n == 1 => (Tone::Bad, "Liquidé : toute la marge isolée perdue.".to_string()),
        Ecart::Liquide => (Tone::Bad, format!("Liquidé{times} : toute la marge isolée perdue à chaque fois.")),
        Ecart::SansStop => (Tone::Bad, format!("Entré sans stop{times} : la perte n'avait pas de limite.")),
        Ecart::StopEloigne if worse == n => (Tone::Bad, format!("Stop éloigné à la main : {rt} au lieu de −{n} R.")),
        Ecart::StopEloigne => (Tone::Bad, format!("Stop éloigné à la main{times} : le risque a dépassé ton plan.")),
        Ecart::CoupeAvantStop => (Tone::Mixed, format!("Coupé à la main avant le stop{times} : {rt}{in_all}.")),
        Ecart::SortieAvantObjectif => (Tone::Mixed, format!("Sorti à la main avant l'objectif{times} : {rt} pris{in_all}.")),
        Ecart::ObjectifEloigne if n == 1 => (Tone::Mixed, "Objectif éloigné à la main pendant le trade.".to_string()),
        Ecart::ObjectifEloigne => (Tone::Mixed, format!("Objectif éloigné à la main{times}.")),
        Ecart::Resserre if n == 1 => (Tone::Mixed, "Plan resserré à la main pendant le trade.".to_string()),
        Ecart::Resserre => (Tone::Mixed, format!("Plan resserré à la main{times}.")),
    };
    part(tone, &format!("{n} fois"), text)
}

/// 📅 The month's summary: `trades`, its finished trades (for each, its exits in order: the partial ones, then the
/// last one). `None` without a trade.
pub fn month_review<T: Borrow<ClosedTrade>>(trades: &[Vec<T>]) -> Option<MonthReview> {
    let reviews: Vec<TradeReview> = trades.iter().filter_map(|p| review(p.as_slice())).collect();
    if reviews.is_empty() {
        return None;
    }
    let verdicts: Vec<Verdict> = reviews.iter().map(|r| r.verdict).collect();
    let n = |v: Verdict| verdicts.iter().filter(|x| **x == v).count();
    let (followed, partly, not_followed) = (n(Verdict::Suivi), n(Verdict::EnPartie), n(Verdict::PasSuivi));
    let judged = followed + partly + not_followed;
    let r_followed: f64 = reviews.iter().filter(|r| r.verdict == Verdict::Suivi).filter_map(|r| r.r).sum();
    // each deviation: how many times, the R of its trades, how many beyond −1 R
    let mut deviations: Vec<(Ecart, usize, f64, usize)> = vec![];
    for r in &reviews {
        let Some(e) = r.ecart else { continue };
        let k = deviations.iter().position(|x| x.0 == e).unwrap_or_else(|| {
            deviations.push((e, 0, 0.0, 0));
            deviations.len() - 1
        });
        deviations[k].1 += 1;
        deviations[k].2 += r.r.unwrap_or(0.0);
        deviations[k].3 += usize::from(r.r.is_some_and(|x| x < -1.005));
    }
    // the one that comes back the most; on a tie, the one that cost the most R, then the most serious
    let top = deviations
        .iter()
        .max_by(|a, b| a.1.cmp(&b.1).then(b.2.total_cmp(&a.2)).then(b.0.cmp(&a.0)))
        .copied();
    let r_month: f64 = reviews.iter().filter_map(|r| r.r).sum();
    let (to_review, (lesson, academy)) = match top {
        Some((e, k, r, worse)) => (to_review(e, k, r, worse), lecon_ecart(e)),
        None if judged == 0 => (
            part(Tone::Neutral, "—", "Rien à revoir sans plan jugé.".into()),
            ("Mène tes trades jusqu'à leur sortie : c'est là que ton plan se juge.", None),
        ),
        None => (
            part(Tone::Good, "Aucun", "Aucun écart à ton plan ce mois-ci.".into()),
            if r_month >= 0.005 {
                ("Plan suivi tout le mois : garde la même routine le mois prochain.", None)
            } else {
                ("Pertes prévues et tenues : une perte dans le plan n'est pas une erreur.", None)
            },
        ),
    };
    Some(MonthReview {
        verdicts,
        judged,
        followed,
        partly,
        not_followed,
        r_followed,
        worked: worked(judged, followed, r_followed),
        to_review,
        lesson: lesson.to_string(),
        academy,
        reminder: TRADE_REMINDER,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trade::LECON_CAPITAL;
    use argos_core::portfolio::{Plan, Retouches};
    use argos_core::{Money, Side};

    /// A long trade at 100, stop at 99 ($1 of risk per unit, 100 units), target at 102, exited at `r` R.
    fn trade(reason: &str, r: f64, retouches: Retouches) -> Vec<ClosedTrade> {
        vec![ClosedTrade {
            base: "BTC".into(),
            side: Side::Long,
            qty: 100.0,
            entry: 100.0,
            exit: 100.0 + r,
            pnl: Money::from_f64(r * 100.0),
            fees: Money::ZERO,
            opened_at: 0,
            closed_at: 1,
            reason: reason.into(),
            risk: Money::from_f64(100.0),
            partiel: false,
            glissement: Money::ZERO,
            funding: Money::ZERO,
            levier: None,
            plan: Some(Plan {
                stop0: Some(99.0),
                target0: Some(102.0),
                stop: Some(99.0),
                target: Some(102.0),
                retouches,
                lecture: None,
            }),
            real_closed_at: None,
        }]
    }

    fn planned(reason: &str, r: f64) -> Vec<ClosedTrade> {
        trade(reason, r, Retouches::default())
    }

    fn stop_moved_away(r: f64) -> Vec<ClosedTrade> {
        trade(
            "stop",
            r,
            Retouches {
                stop_eloigne: Some(100.0 + r),
                ..Retouches::default()
            },
        )
    }

    #[test]
    fn the_mockup_month() {
        let month = vec![
            planned("objectif", 2.0),
            planned("stop", -1.0),
            planned("objectif", 2.0),
            stop_moved_away(-1.62),
            planned("fermé à la main", 0.8),
            planned("objectif", 2.0),
            planned("break-even", 0.0),
            planned("stop", -1.0),
            stop_moved_away(-1.4),
            planned("objectif", 2.5),
            planned("fermé à la main", -0.45),
            planned("fin du replay", 0.35),
        ];
        let m = month_review(&month).unwrap();
        assert_eq!((m.judged, m.followed, m.partly, m.not_followed), (11, 7, 2, 2));
        assert_eq!(m.verdicts[3], Verdict::PasSuivi);
        assert_eq!(m.verdicts[11], Verdict::Neutre);
        assert!((m.r_followed - 6.5).abs() < 1e-9);
        assert_eq!((m.worked.tone, m.worked.chip.as_str()), (Tone::Good, "+6,50 R"));
        assert_eq!(m.worked.text, "Tes 7 plans suivis ont fait +6,50 R à eux seuls.");
        assert_eq!((m.to_review.tone, m.to_review.chip.as_str()), (Tone::Bad, "2 fois"));
        assert_eq!(m.to_review.text, "Stop éloigné à la main : −3,02 R au lieu de −2 R.");
        assert_eq!(m.lesson, "Garde le stop où ton plan l'a mis : c'est lui qui limite ta perte.");
        assert_eq!(m.academy, Some(LECON_CAPITAL));
        assert_eq!(m.reminder, "Analyse affichée, pas un ordre.");
    }

    #[test]
    fn on_a_tie_the_deviation_that_cost_the_most() {
        let m = month_review(&[planned("fermé à la main", 0.8), planned("fermé à la main", -0.45), planned("objectif", 2.0)]).unwrap();
        assert_eq!(m.to_review.chip, "1 fois");
        assert_eq!(m.to_review.text, "Coupé à la main avant le stop : −0,45 R.");
        assert_eq!(m.lesson, "Note au journal ce qui t'a fait couper avant le stop.");
        assert_eq!(m.academy, None);
        assert_eq!(m.worked.text, "Ton plan suivi a fait +2,00 R à lui seul.");
        // two exits before the target come back more than one cut
        let m = month_review(&[
            planned("fermé à la main", 0.8),
            planned("fermé à la main", 0.3),
            planned("fermé à la main", -0.45),
        ])
        .unwrap();
        assert_eq!(m.to_review.text, "Sorti à la main avant l'objectif 2 fois : +1,10 R pris en tout.");
        assert_eq!(
            (m.worked.tone, m.worked.text.as_str()),
            (Tone::Bad, "Aucun plan suivi jusqu'au bout ce mois-ci.")
        );
        // a stop moved away that ends at the target: the risk went beyond the plan
        let m = month_review(&[trade(
            "objectif",
            2.0,
            Retouches {
                stop_eloigne: Some(98.0),
                ..Retouches::default()
            },
        )])
        .unwrap();
        assert_eq!(m.to_review.text, "Stop éloigné à la main : le risque a dépassé ton plan.");
    }

    #[test]
    fn month_without_deviation_or_judged_plan() {
        let m = month_review(&[planned("objectif", 2.0), planned("stop", -1.0)]).unwrap();
        assert_eq!((m.to_review.tone, m.to_review.chip.as_str()), (Tone::Good, "Aucun"));
        assert_eq!(m.to_review.text, "Aucun écart à ton plan ce mois-ci.");
        assert_eq!(m.lesson, "Plan suivi tout le mois : garde la même routine le mois prochain.");
        let m = month_review(&[planned("stop", -1.0), planned("stop", -1.0)]).unwrap();
        assert_eq!(
            (m.worked.chip.as_str(), m.worked.text.as_str()),
            ("2 sur 2", "Tes 2 plans suivis ont tenu tes pertes à ce qui était prévu.")
        );
        assert_eq!(m.lesson, "Pertes prévues et tenues : une perte dans le plan n'est pas une erreur.");
        let m = month_review(&[planned("fin du replay", 0.35)]).unwrap();
        assert_eq!((m.judged, m.worked.tone, m.to_review.chip.as_str()), (0, Tone::Neutral, "—"));
        assert!(month_review::<ClosedTrade>(&[]).is_none());
    }
}
