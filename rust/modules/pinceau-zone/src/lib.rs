//! 🖌 Module Argos HSS Trading « Pinceau de zone » (kit module Argos, interface v2) : pour une zone du graphique
//! (début et fin dans le temps, haut et bas en prix, donnés par ses réglages), le volume par prix, le POC, la zone de
//! valeur (VAH / VAL), le VWAP et le delta estimé des bougies de la zone. Aucun droit : il reçoit les bougies et ses
//! réglages, rend des courbes plates sur les bougies de la zone et des cartes.
//!
//! Tout se calcule en **entiers** (i128, aucune virgule flottante) : même règle de répartition que le profil de volume
//! intégré d'Argos (`classic::volume_profile`) et même formule de delta estimé (`flow::delta_estime`). Les prix
//! sortent à l'échelle `price_scale + 2`, les volumes à `volume_scale + 6` (au plus 12), arrondis une fois (moitié
//! loin de zéro). `EMPTY` = pas de valeur.
#![forbid(unsafe_code)]

use argos_module_sdk::{Bar, Card, Input, Line, Output, EMPTY};

/// Rangées par défaut et bornes, part de la zone de valeur par défaut (%), comme dans `argos-module.json`.
pub const DEFAULT_ROWS: i64 = 24;
pub const MIN_ROWS: i64 = 2;
pub const MAX_ROWS: i64 = 200;
pub const DEFAULT_VA: i64 = 70;
/// Décimales des réglages de prix (haut, bas).
pub const SETTING_SCALE: u8 = 6;
/// Décimales ajoutées aux prix rendus (POC, VAH, VAL, VWAP) et aux volumes rendus.
pub const EXTRA_PRICE: u8 = 2;
pub const EXTRA_VOLUME: u8 = 6;
/// Plus grande échelle du kit.
const MAX_SCALE: u8 = 12;

fn pow10(n: u8) -> i128 {
    10i128.pow(u32::from(n))
}

/// `a / b` arrondi à l'entier le plus proche, moitié loin de zéro (`b` > 0).
fn div_round(a: i128, b: i128) -> i128 {
    if a >= 0 {
        (2 * a + b) / (2 * b)
    } else {
        -((-2 * a + b) / (2 * b))
    }
}

/// Un entier du calcul vers la sortie du kit ; trop grand : `EMPTY`.
fn sortie(x: i128) -> i64 {
    i64::try_from(x).ok().filter(|v| *v != EMPTY).unwrap_or(EMPTY)
}

/// Un réglage de prix (`SETTING_SCALE` décimales) à l'échelle des bougies (`ps` décimales).
fn prix_du_reglage(v: i64, ps: u8) -> i128 {
    let v = i128::from(v);
    if ps >= SETTING_SCALE {
        v * pow10(ps - SETTING_SCALE)
    } else {
        div_round(v, pow10(SETTING_SCALE - ps))
    }
}

/// Le profil d'une zone : volume de chaque rangée (du bas vers le haut, à l'échelle des volumes × `f`), POC et zone de
/// valeur (rangées `va_lo..=va_hi`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Profil {
    pub vol: Vec<i128>,
    pub poc: usize,
    pub va_lo: usize,
    pub va_hi: usize,
}

/// Rangée d'un prix dans `[bas, haut]` découpé en `n` rangées (le haut compte dans la dernière) ; hors de la zone :
/// une rangée négative ou ≥ `n`.
fn rangee(x: i128, haut: i128, bas: i128, n: i128) -> i128 {
    if x == haut {
        n - 1
    } else {
        ((x - bas) * n).div_euclid(haut - bas)
    }
}

/// Les rangées touchées par la mèche d'une bougie (`None` : bougie vide, incohérente ou hors de la zone).
fn rangees(b: &Bar, haut: i128, bas: i128, n: i128) -> Option<(i128, i128)> {
    if [b.h, b.l, b.v].contains(&EMPTY) || b.h < b.l {
        return None;
    }
    let (a, z) = (rangee(i128::from(b.l), haut, bas, n), rangee(i128::from(b.h), haut, bas, n));
    (z >= 0 && a < n).then_some((a, z))
}

/// Profil de volume des bougies dans `[bas, haut]` sur `n` rangées, comme le profil de volume intégré d'Argos : le
/// volume d'une bougie est réparti à parts égales sur les rangées que couvre sa mèche ; la part qui sort de la zone est
/// laissée. POC = rangée la plus chargée (la plus basse en cas d'égalité) ; zone de valeur = rangées ajoutées autour
/// du POC (la plus chargée des deux voisines, celle du dessus en cas d'égalité) jusqu'à `va` % du volume. `None` :
/// zone plate ou aucun volume.
pub fn profil(bars: &[Bar], haut: i128, bas: i128, n: usize, va: i128, f: i128) -> Option<Profil> {
    if haut <= bas || n == 0 {
        return None;
    }
    let ni = n as i128;
    let mut vol = vec![0i128; n];
    for b in bars.iter().filter(|b| b.v > 0) {
        let Some((a, z)) = rangees(b, haut, bas, ni) else { continue };
        let part = div_round(i128::from(b.v) * f, z - a + 1);
        for k in a.max(0)..=z.min(ni - 1) {
            vol[k as usize] += part;
        }
    }
    let total: i128 = vol.iter().sum();
    if total <= 0 {
        return None;
    }
    let poc = (0..n).fold(0, |best, k| if vol[k] > vol[best] { k } else { best });
    let (mut lo, mut hi, mut pris) = (poc, poc, vol[poc]);
    while pris * 100 < total * va && (lo > 0 || hi + 1 < n) {
        let dessous = (lo > 0).then(|| vol[lo - 1]);
        let dessus = (hi + 1 < n).then(|| vol[hi + 1]);
        if dessus >= dessous {
            hi += 1;
            pris += vol[hi];
        } else {
            lo -= 1;
            pris += vol[lo];
        }
    }
    Some(Profil {
        vol,
        poc,
        va_lo: lo,
        va_hi: hi,
    })
}

/// Delta estimé d'une bougie (achats − ventes), comme `delta_estime` d'Argos : `volume × (2 × clôture − haut − bas) /
/// (haut − bas)`, 0 sans plage ; à l'échelle des volumes × `f`.
fn delta(b: &Bar, f: i128) -> i128 {
    let (h, l, c, v) = (i128::from(b.h), i128::from(b.l), i128::from(b.c), i128::from(b.v));
    if b.c == EMPTY || h == l {
        0
    } else {
        div_round(v * f * (2 * c - h - l), h - l)
    }
}

/// L'entrée du module.
pub fn run(input: Input) -> Output {
    let reglage = |k: usize, defaut: i64| input.settings.get(k).copied().unwrap_or(defaut);
    let (debut, fin, haut_r, bas_r) = (reglage(0, 0), reglage(1, 0), reglage(2, 0), reglage(3, 0));
    let n = reglage(4, DEFAULT_ROWS).clamp(MIN_ROWS, MAX_ROWS) as usize;
    let va = i128::from(reglage(5, DEFAULT_VA).clamp(1, 100));
    let (ps, vs) = (input.price_scale, input.volume_scale);
    let (ps2, vs2) = ((ps + EXTRA_PRICE).min(MAX_SCALE), (vs + EXTRA_VOLUME).min(MAX_SCALE));
    let (fp, fv) = (pow10(ps2 - ps), pow10(vs2 - vs));
    // les bougies de la zone dans le temps (0 = pas de limite)
    let dedans: Vec<bool> = input.bars.iter().map(|b| (debut == 0 || b.t >= debut) && (fin == 0 || b.t <= fin)).collect();
    let zone: Vec<Bar> = input.bars.iter().zip(&dedans).filter(|(_, d)| **d).map(|(b, _)| *b).collect();
    // la zone en prix : les réglages, sinon le haut et le bas des bougies de la zone
    let (haut, bas) = if haut_r > bas_r && bas_r > 0 {
        (prix_du_reglage(haut_r, ps), prix_du_reglage(bas_r, ps))
    } else {
        let ok = zone.iter().filter(|b| b.h != EMPTY && b.l != EMPTY);
        let haut = ok.clone().map(|b| i128::from(b.h)).max().unwrap_or(0);
        (haut, ok.map(|b| i128::from(b.l)).min().unwrap_or(0))
    };
    let nn = n as i128;
    let calcul = profil(&zone, haut, bas, n, va, fv).map(|p| {
        let touche: Vec<&Bar> = zone.iter().filter(|b| b.v > 0 && rangees(b, haut, bas, nn).is_some()).collect();
        let delta: i128 = touche.iter().map(|b| delta(b, fv)).sum();
        let (num, den) = touche.iter().fold((0i128, 0i128), |(num, den), b| {
            let v = i128::from(b.v);
            (num + (i128::from(b.h) + i128::from(b.l) + i128::from(b.c)) * v, den + v)
        });
        let prix_bord = |k: i128| bas * fp + div_round(k * (haut - bas) * fp, nn);
        let poc = bas * fp + div_round((2 * p.poc as i128 + 1) * (haut - bas) * fp, 2 * nn);
        let vwap = div_round(num * fp, 3 * den);
        let volume: i128 = p.vol.iter().sum();
        [poc, prix_bord(p.va_hi as i128 + 1), prix_bord(p.va_lo as i128), vwap, volume, delta]
    });
    let noms = ["POC", "VAH", "VAL", "VWAP"];
    let lines = noms
        .iter()
        .enumerate()
        .map(|(k, nom)| Line {
            name: format!("🖌 {nom}"),
            scale: ps2,
            values: dedans.iter().map(|d| calcul.filter(|_| *d).map_or(EMPTY, |c| sortie(c[k]))).collect(),
        })
        .collect();
    let cards = calcul.map_or_else(Vec::new, |c| {
        let carte = |name: &str, value: i128, scale: u8| Card {
            name: name.into(),
            value: sortie(value),
            scale,
        };
        vec![
            carte("Volume de la zone", c[4], vs2),
            carte("Delta (estimé)", c[5], vs2),
            carte("POC", c[0], ps2),
            carte("VAH", c[1], ps2),
            carte("VAL", c[2], ps2),
            carte("VWAP", c[3], ps2),
        ]
    });
    Output {
        lines,
        labels: Vec::new(),
        cards,
    }
}

argos_module_sdk::entry_data!(run);

#[cfg(test)]
mod tests {
    use super::*;

    /// Une bougie à l'échelle 0 (prix et volumes entiers) : temps, ouverture, haut, bas, clôture, volume.
    fn bar(t: i64, o: i64, h: i64, l: i64, c: i64, v: i64) -> Bar {
        Bar { t, o, h, l, c, v }
    }

    fn input(bars: Vec<Bar>, settings: Vec<i64>) -> Input {
        Input {
            timeframe: 60,
            price_scale: 0,
            volume_scale: 0,
            symbol: "X".into(),
            bars,
            settings,
        }
    }

    fn carte(o: &Output, nom: &str) -> i64 {
        o.cards.iter().find(|c| c.name == nom).map(|c| c.value).unwrap()
    }

    /// Zone 2 → 12 en 5 rangées de 2 : la 1re bougie (2-5, 4 de volume) couvre les rangées 0 et 1 (2 chacune), la
    /// 2e (6-7, 6 de volume) la rangée 2 → POC rangée 2 (prix 7), zone de valeur rangées 1-2 (8 / 10 ≥ 70 %).
    #[test]
    fn calcul_a_la_main() {
        let b = vec![bar(0, 3, 5, 2, 5, 4), bar(60, 7, 7, 6, 6, 6)];
        let o = run(input(b, vec![0, 0, 12_000_000, 2_000_000, 5, 70]));
        assert_eq!(o.cards.len(), 6);
        assert_eq!(carte(&o, "Volume de la zone"), 10_000_000, "10 à l'échelle 0 + 6");
        // delta : 4 × (10 − 5 − 2) / 3 = +4 ; 6 × (12 − 7 − 6) / 1 = −6
        assert_eq!(carte(&o, "Delta (estimé)"), -2_000_000);
        assert_eq!(
            (carte(&o, "POC"), carte(&o, "VAH"), carte(&o, "VAL")),
            (700, 800, 400),
            "prix à l'échelle 0 + 2"
        );
        // VWAP : ((5 + 2 + 5) × 4 + (7 + 6 + 6) × 6) / (3 × 10) = 5,4
        assert_eq!(carte(&o, "VWAP"), 540);
        assert_eq!(
            o.lines.iter().map(|l| l.name.as_str()).collect::<Vec<_>>(),
            ["🖌 POC", "🖌 VAH", "🖌 VAL", "🖌 VWAP"]
        );
        assert_eq!(o.lines[0].values, vec![700, 700], "courbe plate sur les bougies de la zone");
        let p = profil(&[bar(0, 3, 5, 2, 5, 4), bar(60, 7, 7, 6, 6, 6)], 12, 2, 5, 70, 1).unwrap();
        assert_eq!((p.vol, p.poc, p.va_lo, p.va_hi), (vec![2, 2, 6, 0, 0], 2, 1, 2));
    }

    #[test]
    fn zone_vide() {
        let o = run(input(vec![], vec![]));
        assert!(o.cards.is_empty() && o.lines.iter().all(|l| l.values.is_empty()));
        // des bougies, mais aucune dans le temps de la zone : rien de calculé, courbes vides
        let o = run(input(vec![bar(0, 1, 2, 1, 2, 5), bar(60, 2, 3, 2, 3, 5)], vec![1_000, 2_000, 0, 0, 24, 70]));
        assert!(o.cards.is_empty());
        assert!(o.lines.iter().all(|l| l.values == vec![EMPTY, EMPTY]));
        // aucun volume : rien
        assert!(run(input(vec![bar(0, 1, 2, 1, 2, 0)], vec![])).cards.is_empty());
    }

    /// Une seule bougie, zone = sa mèche : tout son volume, réparti sur toutes les rangées ; POC la plus basse (égalité).
    #[test]
    fn une_seule_bougie() {
        let o = run(input(vec![bar(0, 10, 20, 10, 15, 8)], vec![0, 0, 0, 0, 4, 70]));
        assert_eq!(carte(&o, "Volume de la zone"), 8_000_000);
        assert_eq!(carte(&o, "POC"), 1_125, "milieu de la rangée 10-12,5");
        assert_eq!(carte(&o, "Delta (estimé)"), 0, "clôture au milieu");
        assert_eq!(carte(&o, "VWAP"), 1_500, "(20 + 10 + 15) / 3");
    }

    /// Une bougie plus grande que la zone : seule la part de sa mèche dans la zone compte.
    #[test]
    fn bougie_qui_depasse_la_zone() {
        // zone 2 → 12 en 5 rangées ; mèche 0 → 20 = rangées −1 à 9 (11 rangées) : 1 par rangée, 5 dans la zone
        let p = profil(&[bar(0, 0, 20, 0, 20, 11)], 12, 2, 5, 70, 1).unwrap();
        assert_eq!(p.vol, vec![1, 1, 1, 1, 1]);
        // une bougie entièrement au-dessus de la zone ne compte pas
        assert_eq!(profil(&[bar(0, 30, 40, 30, 40, 5)], 12, 2, 5, 70, 1), None);
    }

    #[test]
    fn deterministe_et_fenetre_dans_le_temps() {
        let b: Vec<Bar> = (0..50)
            .map(|i| bar(i * 60, 100 + i % 7, 105 + i % 11, 95 - i % 5, 101 + i % 3, 10 + i))
            .collect();
        let s = vec![600, 1_800, 0, 0, 24, 70];
        let (a, z) = (run(input(b.clone(), s.clone())), run(input(b, s)));
        assert_eq!(a, z, "mêmes bougies, mêmes réglages : même résultat");
        let v = &a.lines[0].values;
        assert!(
            v[..10].iter().all(|x| *x == EMPTY) && v[31..].iter().all(|x| *x == EMPTY),
            "hors de la zone : vide"
        );
        assert!(v[10..=30].iter().all(|x| *x == v[10] && *x != EMPTY), "dans la zone : plate");
    }
}
