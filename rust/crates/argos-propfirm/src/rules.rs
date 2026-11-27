//! 📏 Règles des challenges. Les nombres sont en % du capital de départ (sauf le partage, en % des profits).

use argos_core::Money;
use serde::{Deserialize, Serialize};

/// Comment la perte maximale est calculée.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MaxLossKind {
    /// Fixe : sous le capital de départ.
    Fixed,
    /// Suit le plus haut du compte, recalculée en fin de journée (par exemple à 00:00 CE(S)T).
    TrailingEndOfDay,
    /// Suit le plus haut du compte à chaque trade fermé.
    TrailingIntraday,
}

/// Règle de constance (le meilleur jour ne doit pas peser trop lourd).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum Consistency {
    None,
    /// Ancienne règle « Best Day » : meilleur jour ≤ `pct` % du profit des jours positifs.
    BestDay {
        pct: f64,
    },
    /// Règle « Consistency » : meilleur jour ≤ `pct` % du profit total (gains et pertes fermés).
    /// `since` : commandes à partir de ce moment (ISO 8601, UTC) ; avant : `legacy_best_day_pct` (Best Day).
    /// `violation` : dépasser la limite brise-t-il les règles ? (non dans les règles standard)
    TotalProfit {
        pct: f64,
        since: Option<String>,
        legacy_best_day_pct: Option<f64>,
        violation: bool,
    },
}

/// Un profil de challenge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    /// 💰 Capital du compte (centimes).
    pub size: Money,
    /// Objectif de chaque phase (vide : pas d'évaluation).
    pub phases: Vec<f64>,
    pub daily_loss: f64,
    pub max_loss: f64,
    pub max_loss_kind: MaxLossKind,
    pub consistency: Consistency,
    pub profit_split: f64,
    pub profit_split_max: f64,
    pub fee_refunded: bool,
    /// 💰 Prix (inconnu : `None`).
    pub fee: Option<Money>,
    pub min_days: u32,
    /// Paies aux N jours (inconnu : `None`).
    pub payout_days: Option<u32>,
    /// Précisions et points à vérifier.
    pub notes: Vec<String>,
}

/// Les défauts versionnés.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Defaults {
    pub version: String,
    pub note: String,
    pub profiles: Vec<Profile>,
}

/// Les profils par défaut (`propfirm-defaults.json`).
pub fn defaults() -> Defaults {
    serde_json::from_str(include_str!("../propfirm-defaults.json")).expect("propfirm-defaults.json valide")
}

/// Résultat du contrôle de constance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConsistencyCheck {
    /// 💰 Meilleur jour (profit fermé) et limite permise, en centimes.
    pub best_day: Money,
    pub limit: Money,
    pub ok: bool,
    /// Dépasser compte comme une violation des règles ?
    pub violation: bool,
}

/// Contrôle de constance sur les profits fermés de chaque jour. `ordered_at` : moment de la commande du challenge
/// (ISO 8601) ; une commande d'avant `since` garde l'ancienne règle Best Day.
pub fn check_consistency(rule: &Consistency, days: &[Money], ordered_at: Option<&str>) -> Option<ConsistencyCheck> {
    let best = days.iter().copied().fold(Money::ZERO, Money::max);
    let positive: Money = days.iter().filter(|d| **d > Money::ZERO).sum();
    let total: Money = days.iter().sum();
    // limite arrondie une fois au centime, comparaison exacte
    let mk = |limit: Money, violation: bool| {
        Some(ConsistencyCheck {
            best_day: best,
            limit,
            ok: best <= limit,
            violation,
        })
    };
    match rule {
        Consistency::None => None,
        Consistency::BestDay { pct } => mk(positive.pct(*pct), true),
        Consistency::TotalProfit {
            pct,
            since,
            legacy_best_day_pct,
            violation,
        } => {
            // les dates ISO 8601 en UTC se comparent comme du texte
            let legacy = matches!((since, ordered_at), (Some(s), Some(o)) if o < s.as_str());
            match (legacy, legacy_best_day_pct) {
                (true, Some(old)) => mk(positive.pct(*old), true),
                _ => mk(total.max(Money::ZERO).pct(*pct), *violation),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defauts_propfirm_de_la_semaine() {
        let d = defaults();
        assert_eq!(d.version, "2026-10-25");
        let p = |id: &str| d.profiles.iter().find(|p| p.id == id).unwrap().clone();
        let two = p("propfirm-2step");
        assert_eq!(
            (two.phases.clone(), two.daily_loss, two.max_loss, two.max_loss_kind),
            (vec![10.0, 5.0], 5.0, 10.0, MaxLossKind::Fixed)
        );
        assert_eq!((two.profit_split, two.profit_split_max, two.fee_refunded), (80.0, 90.0, true));
        let one = p("propfirm-1step");
        assert_eq!(
            (one.daily_loss, one.max_loss_kind, one.min_days, one.profit_split),
            (3.0, MaxLossKind::TrailingEndOfDay, 0, 90.0)
        );
        assert!(two.size > Money::ZERO, "capital lu du JSON en centimes");
        let inst = p("propfirm-instant");
        assert!(inst.phases.is_empty());
        assert_eq!(inst.max_loss_kind, MaxLossKind::TrailingIntraday);
        assert!(matches!(inst.consistency, Consistency::TotalProfit { pct, .. } if pct == 30.0));
    }

    #[test]
    fn constance_1_step_nouvelle_et_ancienne() {
        let rule = defaults().profiles.into_iter().find(|p| p.id == "propfirm-1step").unwrap().consistency;
        // jours : +3000, +1000, -1500 → total 2500, jours positifs 4000, meilleur 3000
        let m = Money::from_f64;
        let days = [m(3000.0), m(1000.0), m(-1500.0)];
        // nouvelle règle (commande du 20 oct.) : limite = 50 % de 2500 = 1250 → dépassé, mais pas une violation
        let new = check_consistency(&rule, &days, Some("2026-10-20T10:00:00Z")).unwrap();
        assert_eq!((new.limit, new.ok, new.violation), (m(1250.0), false, false));
        // ancienne (commande du 1er oct.) : Best Day = 50 % de 4000 = 2000
        let old = check_consistency(&rule, &days, Some("2026-10-01T10:00:00Z")).unwrap();
        assert_eq!((old.limit, old.ok), (m(2000.0), false));
        // jours réguliers : ça passe
        assert!(
            check_consistency(&rule, &[m(1000.0), m(900.0), m(1100.0)], Some("2026-10-20T10:00:00Z"))
                .unwrap()
                .ok
        );
        // meilleur jour exactement à la limite (centime près) : accepté ; un centime de plus : refusé
        let r = Consistency::BestDay { pct: 50.0 };
        assert!(check_consistency(&r, &[m(1000.01), m(1000.01)], None).unwrap().ok, "1000,01 ≤ 50 % de 2000,02");
        assert!(!check_consistency(&r, &[m(1000.02), m(1000.0)], None).unwrap().ok, "1000,02 > 1000,01");
        assert!(check_consistency(&Consistency::None, &days, None).is_none());
    }
}
