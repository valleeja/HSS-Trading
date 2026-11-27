//! 🎓 Scénarios d'examen du Simulateur : « 2 étapes classique », « 1 étape serré » et « personnalisé », avec les
//! règles par défaut des profils (`propfirm-defaults.json`). On rejoue la courbe d'équité du compte démo et ses
//! trades fermés, et on rend un verdict : en cours, réussi, ou échoué avec la raison (et la leçon à revoir).
//!
//! La journée finit à minuit dans le fuseau du reset (Europe centrale par défaut, réglable) : c'est la fonction
//! `day_of` donnée par l'appelant qui dit à quel jour appartient une heure.

use crate::rules::{check_consistency, defaults, Consistency, MaxLossKind};
use argos_core::Money;
use std::collections::BTreeMap;

/// Leçon de l'Académie proposée après un échec.
pub const LECON_CHALLENGE: &str = "6-02-passer-un-challenge-propfirm";

/// Fuseaux proposés pour la remise à zéro du jour (Europe centrale par défaut).
pub const FUSEAUX: [&str; 3] = ["Europe/Prague", "America/New_York", "UTC"];

/// Les trois scénarios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scenario {
    DeuxEtapes,
    UneEtape,
    Perso,
}

impl Scenario {
    pub const ALL: [Scenario; 3] = [Scenario::DeuxEtapes, Scenario::UneEtape, Scenario::Perso];

    pub fn name(self) -> &'static str {
        match self {
            Scenario::DeuxEtapes => "2 étapes classique",
            Scenario::UneEtape => "1 étape serré",
            Scenario::Perso => "Personnalisé",
        }
    }
}

/// Les règles d'un examen (en % du capital de départ).
#[derive(Debug, Clone, PartialEq)]
pub struct Regles {
    pub scenario: Scenario,
    /// Objectif de chaque étape.
    pub phases: Vec<f64>,
    pub daily_loss: f64,
    pub max_loss: f64,
    pub max_loss_kind: MaxLossKind,
    pub min_days: u32,
    pub consistency: Consistency,
    /// Fuseau de la remise à zéro du jour.
    pub reset_tz: String,
}

impl Regles {
    /// Règles par défaut d'un scénario (le personnalisé part des 2 étapes, à régler).
    pub fn of(s: Scenario) -> Self {
        let id = if s == Scenario::UneEtape { "propfirm-1step" } else { "propfirm-2step" };
        let p = defaults().profiles.into_iter().find(|p| p.id == id).expect("profil par défaut");
        Self {
            scenario: s,
            phases: p.phases,
            daily_loss: p.daily_loss,
            max_loss: p.max_loss,
            max_loss_kind: p.max_loss_kind,
            min_days: p.min_days,
            consistency: p.consistency,
            reset_tz: FUSEAUX[0].into(),
        }
    }

    /// Les règles en une ligne : « Cible 10 % puis 5 % · jour 5 % · total 10 % · 4 jours min ».
    pub fn resume(&self) -> String {
        let pct = |v: f64| format!("{} %", v.to_string().replace('.', ","));
        let cibles = self.phases.iter().map(|v| pct(*v)).collect::<Vec<_>>().join(" puis ");
        let mut s = format!("Cible {cibles} · jour {} · total {}", pct(self.daily_loss), pct(self.max_loss));
        if self.max_loss_kind != MaxLossKind::Fixed {
            s += " suiveuse";
        }
        if self.min_days > 0 {
            s += &format!(" · {} jours min", self.min_days);
        }
        if let Consistency::TotalProfit { pct: p, .. } | Consistency::BestDay { pct: p } = self.consistency {
            s += &format!(" · règle des {} %", p);
        }
        s
    }
}

/// Le verdict.
#[derive(Debug, Clone, PartialEq)]
pub enum Verdict {
    EnCours,
    Reussi,
    /// La raison, et la leçon à revoir.
    Echoue {
        raison: String,
        lecon: &'static str,
    },
}

/// Où en est l'examen.
#[derive(Debug, Clone, PartialEq)]
pub struct Etat {
    /// Étape en cours (0 = la première).
    pub phase: usize,
    /// Profit de l'étape en cours, en % du capital.
    pub profit_pct: f64,
    /// Perte du jour en cours et perte depuis le départ (ou le plus haut), en % du capital.
    pub jour_pct: f64,
    pub total_pct: f64,
    pub jours: u32,
    /// Une note (constance pas encore bonne…).
    pub note: Option<String>,
    pub verdict: Verdict,
}

/// Rejoue l'examen. `equity` : (heure ms, équité) dans l'ordre ; `closed` : (heure de fermeture ms, P&L) des trades.
pub fn evaluer(r: &Regles, start: Money, equity: &[(i64, Money)], closed: &[(i64, Money)], day_of: &dyn Fn(i64) -> String) -> Etat {
    let size = start.to_f64().max(0.01);
    let pct = |m: Money| m.to_f64() / size * 100.0;
    let lim_jour = start.pct(r.daily_loss);
    let lim_total = start.pct(r.max_loss);
    let mut etat = Etat {
        phase: 0,
        profit_pct: 0.0,
        jour_pct: 0.0,
        total_pct: 0.0,
        jours: 0,
        note: None,
        verdict: Verdict::EnCours,
    };
    let (mut jour, mut ref_jour, mut prec) = (String::new(), start, start);
    let (mut plus_haut_soir, mut base) = (start, start);
    let echec = |raison: String| Verdict::Echoue {
        raison,
        lecon: LECON_CHALLENGE,
    };
    for &(t, e) in equity {
        let d = day_of(t);
        if d != jour {
            // nouvelle journée : la perte du jour repart de l'équité de la veille au soir
            if !jour.is_empty() {
                plus_haut_soir = plus_haut_soir.max(prec);
            }
            (jour, ref_jour) = (d.clone(), prec);
        }
        prec = e;
        let perte_jour = ref_jour - e;
        let plancher = match r.max_loss_kind {
            MaxLossKind::Fixed => start - lim_total,
            MaxLossKind::TrailingEndOfDay => (plus_haut_soir - lim_total).min(start),
            MaxLossKind::TrailingIntraday => (plus_haut_soir.max(e) - lim_total).min(start),
        };
        etat.jour_pct = pct(perte_jour.max(Money::ZERO));
        etat.total_pct = pct((plancher + lim_total - e).max(Money::ZERO));
        if perte_jour >= lim_jour {
            etat.verdict = echec(format!(
                "Perte du jour : −{:.2} % (limite {} %), le Gardien aurait verrouillé la journée",
                pct(perte_jour),
                r.daily_loss
            ));
            return etat;
        }
        if e <= plancher {
            etat.verdict = echec(format!("Perte maximale touchée : −{:.2} % (limite {} %)", pct(start - e), r.max_loss));
            return etat;
        }
        // jours de trading et constance : les trades fermés jusqu'ici
        let mut par_jour: BTreeMap<String, Money> = BTreeMap::new();
        for &(tc, p) in closed.iter().filter(|(tc, _)| *tc <= t) {
            *par_jour.entry(day_of(tc)).or_default() += p;
        }
        etat.jours = par_jour.len() as u32;
        let cible = r.phases.get(etat.phase).copied().unwrap_or(0.0);
        etat.profit_pct = pct(e - base);
        if etat.profit_pct >= cible && etat.jours >= r.min_days {
            let jours: Vec<Money> = par_jour.values().copied().collect();
            if let Some(c) = check_consistency(&r.consistency, &jours, None).filter(|c| !c.ok) {
                etat.note = Some(format!(
                    "Constance : meilleur jour {} $ au-dessus de la limite {} $ : continue à trader",
                    c.best_day.to_f64().round(),
                    c.limit.to_f64().round()
                ));
                continue;
            }
            etat.note = None;
            etat.phase += 1;
            base = e;
            if etat.phase >= r.phases.len() {
                etat.verdict = Verdict::Reussi;
                return etat;
            }
        }
    }
    etat
}

#[cfg(test)]
mod tests {
    use super::*;

    const JOUR: i64 = 86_400_000;
    fn jour(ms: i64) -> String {
        (ms / JOUR).to_string()
    }
    fn m(d: f64) -> Money {
        Money::from_f64(d)
    }
    const K100: f64 = 100_000.0;

    #[test]
    fn regles_par_defaut_des_scenarios() {
        let d = Regles::of(Scenario::DeuxEtapes);
        assert_eq!((d.phases.clone(), d.daily_loss, d.max_loss, d.min_days), (vec![10.0, 5.0], 5.0, 10.0, 4));
        assert_eq!(d.reset_tz, "Europe/Prague", "minuit heure d'Europe centrale par défaut");
        let u = Regles::of(Scenario::UneEtape);
        assert_eq!((u.phases.clone(), u.daily_loss), (vec![10.0], 3.0));
        assert!(matches!(u.consistency, Consistency::TotalProfit { pct, .. } if pct == 50.0), "règle des 50 %");
        assert!(u.resume().contains("règle des 50 %"));
        assert_eq!(d.resume(), "Cible 10 % puis 5 % · jour 5 % · total 10 % · 4 jours min");
    }

    #[test]
    fn deux_etapes_reussi_apres_4_jours() {
        let r = Regles::of(Scenario::DeuxEtapes);
        // 4 jours à +3 % puis 2 jours à +3 % : étape 1 (10 %) au 4e jour, étape 2 (5 %) au 6e
        let mut eq = vec![];
        let mut cl = vec![];
        let mut e = K100;
        for d in 0..6 {
            e += 3_000.0;
            eq.push((d * JOUR + 1, m(e)));
            cl.push((d * JOUR + 1, m(3_000.0)));
        }
        let s = evaluer(&r, m(K100), &eq[..3], &cl, &jour);
        assert_eq!((s.phase, s.verdict.clone()), (0, Verdict::EnCours));
        let s = evaluer(&r, m(K100), &eq, &cl, &jour);
        assert_eq!(s.verdict, Verdict::Reussi);
        assert_eq!(s.phase, 2);
    }

    #[test]
    fn perte_du_jour_et_perte_max() {
        let r = Regles::of(Scenario::DeuxEtapes);
        // −5 % dans la journée : échec (toucher la limite suffit)
        let s = evaluer(&r, m(K100), &[(1, m(98_000.0)), (2, m(95_000.0))], &[], &jour);
        let Verdict::Echoue { raison, lecon } = s.verdict else { panic!() };
        assert!(raison.starts_with("Perte du jour"), "{raison}");
        assert_eq!(lecon, LECON_CHALLENGE);
        // −4,99 % : encore dans les règles
        assert_eq!(evaluer(&r, m(K100), &[(1, m(95_001.0))], &[], &jour).verdict, Verdict::EnCours);
        // la journée repart de minuit : −4 % trois jours de suite = −12 % au total → perte max
        let eq = [(1, m(96_000.0)), (JOUR + 1, m(92_000.0)), (2 * JOUR + 1, m(88_000.0))];
        let Verdict::Echoue { raison, .. } = evaluer(&r, m(K100), &eq, &[], &jour).verdict else {
            panic!()
        };
        assert!(raison.starts_with("Perte maximale"), "{raison}");
    }

    #[test]
    fn une_etape_perte_jour_3_pct_et_regle_des_50() {
        let r = Regles::of(Scenario::UneEtape);
        let Verdict::Echoue { raison, .. } = evaluer(&r, m(K100), &[(1, m(97_000.0))], &[], &jour).verdict else {
            panic!()
        };
        assert!(raison.contains("limite 3 %"));
        // +10 % en un seul jour : la constance (meilleur jour ≤ 50 % du total) bloque, sans échec
        let s = evaluer(&r, m(K100), &[(1, m(110_000.0))], &[(1, m(10_000.0))], &jour);
        assert_eq!(s.verdict, Verdict::EnCours);
        assert!(s.note.unwrap().starts_with("Constance"));
        // deux jours à +5,5 % : réussi
        let eq = [(1, m(105_500.0)), (JOUR + 1, m(111_000.0))];
        let cl = [(1, m(5_500.0)), (JOUR + 1, m(5_500.0))];
        assert_eq!(evaluer(&r, m(K100), &eq, &cl, &jour).verdict, Verdict::Reussi);
        // perte max suiveuse (fin de journée) : après un soir à 108 000 $, le plancher monte à 98 000 $
        let eq = [
            (1, m(108_000.0)),
            (JOUR + 1, m(106_000.0)),
            (2 * JOUR + 1, m(104_500.0)),
            (3 * JOUR + 1, m(102_000.0)),
            (4 * JOUR + 1, m(99_500.0)),
            (5 * JOUR + 1, m(97_900.0)),
        ];
        let Verdict::Echoue { raison, .. } = evaluer(&r, m(K100), &eq, &[], &jour).verdict else {
            panic!()
        };
        assert!(raison.starts_with("Perte maximale"), "{raison}");
    }

    #[test]
    fn remise_a_zero_suit_le_fuseau_donne() {
        let r = Regles::of(Scenario::DeuxEtapes);
        // −3 % puis −3 % : la même journée (échec) ou deux journées (dans les règles) selon le fuseau
        let eq = [(10, m(97_000.0)), (20, m(94_000.0))];
        let meme = |_: i64| "j".to_string();
        let coupe = |t: i64| if t < 15 { "a".into() } else { "b".to_string() };
        assert!(matches!(evaluer(&r, m(K100), &eq, &[], &meme).verdict, Verdict::Echoue { .. }));
        assert_eq!(evaluer(&r, m(K100), &eq, &[], &coupe).verdict, Verdict::EnCours);
    }
}
