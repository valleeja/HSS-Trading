//! 🎓 Module Argos HSS Trading « Sessions de l'Académie » (kit module Argos, interface v2) : trace sur la chart le
//! plus haut et le plus bas d'une fenêtre horaire des leçons, à l'heure de New York (heure d'été américaine
//! comprise) : range asiatique (20 h – minuit), killzone de Londres (2 h – 5 h), range d'ouverture ORB (9 h 30 –
//! 10 h) ou Silver Bullet (10 h – 11 h). Pendant la fenêtre, les lignes suivent le prix ; ensuite elles restent
//! figées jusqu'à la fenêtre suivante. Aucun droit : il reçoit les bougies et son réglage, rend 2 courbes, une
//! étiquette au début de chaque fenêtre et 3 cartes. Les prix restent des entiers à l'échelle `price_scale`.
//! Une bougie compte si elle commence dans la fenêtre : prendre une unité de temps plus courte que la fenêtre.
#![forbid(unsafe_code)]

use argos_module_sdk::{Card, Input, Label, Line, Output, EMPTY};

/// Les fenêtres, dans l'ordre du réglage `fenetre` : (nom, début, fin) en minutes depuis minuit à New York.
pub const FENETRES: [(&str, i64, i64); 4] = [
    ("Range asiatique", 20 * 60, 24 * 60),
    ("Killzone de Londres", 2 * 60, 5 * 60),
    ("Range d'ouverture (ORB)", 9 * 60 + 30, 10 * 60),
    ("Silver Bullet", 10 * 60, 11 * 60),
];

/// Au plus 1000 étiquettes (limite de l'interface v2) : on garde les plus récentes.
const MAX_LABELS: usize = 1000;

/// Jours depuis le 1er janvier 1970 d'une date (calendrier grégorien).
fn jours(an: i64, mois: i64, jour: i64) -> i64 {
    let a = if mois <= 2 { an - 1 } else { an };
    let ere = a.div_euclid(400);
    let ya = a - ere * 400;
    let jda = (153 * ((mois + 9) % 12) + 2) / 5 + jour - 1;
    ere * 146_097 + ya * 365 + ya / 4 - ya / 100 + jda - 719_468
}

/// L'année d'un instant (secondes Unix, UTC).
fn annee(t: i64) -> i64 {
    let z = t.div_euclid(86_400) + 719_468;
    let ere = z.div_euclid(146_097);
    let je = z - ere * 146_097;
    let ae = (je - je / 1460 + je / 36_524 - je / 146_096) / 365;
    let jda = je - (365 * ae + ae / 4 - ae / 100);
    let mp = (5 * jda + 2) / 153;
    ae + ere * 400 + i64::from(mp >= 10)
}

/// Le `n`-ième dimanche d'un mois (jours depuis 1970).
fn dimanche(an: i64, mois: i64, n: i64) -> i64 {
    let d = jours(an, mois, 1);
    d + (7 - (d + 4).rem_euclid(7)) % 7 + 7 * (n - 1)
}

/// Heure locale de New York (secondes Unix décalées) : UTC − 4 h l'été (2e dimanche de mars → 1er dimanche de
/// novembre, à 2 h locales), UTC − 5 h sinon.
pub fn new_york(t: i64) -> i64 {
    let a = annee(t);
    let ete = t >= dimanche(a, 3, 2) * 86_400 + 7 * 3600 && t < dimanche(a, 11, 1) * 86_400 + 6 * 3600;
    t - if ete { 4 } else { 5 } * 3600
}

/// L'entrée du module.
pub fn run(input: Input) -> Output {
    let choix = input.settings.first().copied().unwrap_or(0).clamp(0, FENETRES.len() as i64 - 1) as usize;
    let (nom, debut, fin) = FENETRES[choix];
    let n = input.bars.len();
    let (mut haut, mut bas) = (vec![EMPTY; n], vec![EMPTY; n]);
    let mut labels = Vec::new();
    // fenêtre en cours : (jour de New York, plus haut, plus bas)
    let mut cours: Option<(i64, i64, i64)> = None;
    for (i, b) in input.bars.iter().enumerate() {
        let local = new_york(b.t);
        let (jour, minute) = (local.div_euclid(86_400), local.rem_euclid(86_400) / 60);
        if minute >= debut && minute < fin && b.h != EMPTY && b.l != EMPTY {
            match &mut cours {
                Some((j, h, l)) if *j == jour => {
                    *h = (*h).max(b.h);
                    *l = (*l).min(b.l);
                }
                _ => {
                    cours = Some((jour, b.h, b.l));
                    labels.push(Label {
                        bar: i as u32,
                        text: nom.to_string(),
                    });
                }
            }
        }
        if let Some((_, h, l)) = cours {
            (haut[i], bas[i]) = (h, l);
        }
    }
    if labels.len() > MAX_LABELS {
        labels.drain(..labels.len() - MAX_LABELS);
    }
    let scale = input.price_scale;
    let mut cards = Vec::new();
    if let Some((_, h, l)) = cours {
        cards.push(Card {
            name: format!("Haut · {nom}"),
            value: h,
            scale,
        });
        cards.push(Card {
            name: format!("Bas · {nom}"),
            value: l,
            scale,
        });
        cards.push(Card {
            name: "Écart haut − bas".into(),
            value: h.saturating_sub(l),
            scale,
        });
    }
    Output {
        lines: vec![
            Line {
                name: format!("Haut · {nom}"),
                scale,
                values: haut,
            },
            Line {
                name: format!("Bas · {nom}"),
                scale,
                values: bas,
            },
        ],
        labels,
        cards,
    }
}

argos_module_sdk::entry_data!(run);

#[cfg(test)]
mod tests {
    use super::*;
    use argos_module_sdk::Bar;

    /// Bougies de 30 min depuis `t0` (UTC) ; le prix vaut l'index × 100 (haut + 50, bas − 50).
    fn input(t0: i64, n: usize, fenetre: i64) -> Input {
        Input {
            timeframe: 1800,
            price_scale: 2,
            volume_scale: 0,
            symbol: "XAUUSD".into(),
            bars: (0..n)
                .map(|i| {
                    let c = 10_000 + i as i64 * 100;
                    Bar {
                        t: t0 + i as i64 * 1800,
                        o: c,
                        h: c + 50,
                        l: c - 50,
                        c,
                        v: 1,
                    }
                })
                .collect(),
            settings: vec![fenetre],
        }
    }

    #[test]
    fn heure_de_new_york_ete_et_hiver() {
        // 15 janvier 2026 à 13 h UTC = 8 h à New York ; 15 juillet 2026 à 12 h UTC = 8 h aussi
        let hiver = jours(2026, 1, 15) * 86_400 + 13 * 3600;
        let ete = jours(2026, 7, 15) * 86_400 + 12 * 3600;
        assert_eq!(new_york(hiver).rem_euclid(86_400), 8 * 3600);
        assert_eq!(new_york(ete).rem_euclid(86_400), 8 * 3600);
        assert_eq!(annee(jours(2026, 12, 31) * 86_400), 2026);
        // 2026 : passage à l'heure d'été le dimanche 8 mars à 7 h UTC
        let bascule = jours(2026, 3, 8) * 86_400 + 7 * 3600;
        assert_eq!(new_york(bascule - 1).rem_euclid(86_400), 2 * 3600 - 1, "1 h 59 59 heure normale");
        assert_eq!(new_york(bascule).rem_euclid(86_400), 3 * 3600, "3 h heure d'été");
    }

    #[test]
    fn range_asiatique_suivi_puis_fige() {
        // 15 janvier 2026, 18 h à New York (23 h UTC) : 4 bougies avant 20 h, 8 dans la fenêtre, puis la suite
        let t0 = jours(2026, 1, 15) * 86_400 + 23 * 3600;
        let out = run(input(t0, 20, 0));
        let (h, l) = (&out.lines[0].values, &out.lines[1].values);
        assert!(h[..4].iter().all(|v| *v == EMPTY), "rien avant 20 h");
        assert_eq!((h[4], l[4]), (10_450, 10_350), "première bougie de la fenêtre");
        assert_eq!((h[11], l[11]), (11_150, 10_350), "le range suit le prix jusqu'à minuit");
        assert_eq!((h[19], l[19]), (11_150, 10_350), "puis il reste figé");
        assert_eq!(out.labels.len(), 1);
        assert_eq!((out.labels[0].bar, out.labels[0].text.as_str()), (4, "Range asiatique"));
        assert_eq!(out.cards[2].value, 800, "écart 8,00");
    }

    #[test]
    fn une_fenetre_par_jour_et_reglage_borne() {
        // 3 jours de bougies de 30 min (heure d'été) : 3 Silver Bullet, chacune remise à zéro
        let t0 = jours(2026, 7, 1) * 86_400;
        let out = run(input(t0, 144, 3));
        assert_eq!(out.labels.len(), 3);
        assert!(out.labels.iter().all(|l| l.text == "Silver Bullet"));
        // 10 h à New York l'été = 14 h UTC = bougie 28 ; deux bougies (10 h et 10 h 30)
        assert_eq!(out.labels[0].bar, 28);
        assert_eq!(out.lines[0].values[29], 10_000 + 29 * 100 + 50);
        assert_eq!(out.lines[1].values[29], 10_000 + 28 * 100 - 50);
        assert_eq!(out.lines[0].values[30], out.lines[0].values[29], "figée après 11 h");
        // un réglage hors bornes prend la dernière fenêtre, sans planter ; pas de bougies : pas de carte
        assert_eq!(run(input(t0, 144, 99)).labels.len(), 3);
        let vide = run(input(t0, 0, 0));
        assert!(vide.cards.is_empty() && vide.lines[0].values.is_empty());
    }
}
