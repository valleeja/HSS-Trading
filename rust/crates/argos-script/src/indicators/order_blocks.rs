//! 🧱 **Argos Order Blocks** (conception Argos, écrit de zéro ; affiché « Argos Price Action »).
//!
//! Définition d'Argos (idée générale du trading « smart money ») : un **order block** est la dernière bougie de sens
//! opposé avant une impulsion qui laisse un déséquilibre (FVG : le bas de la bougie d'après au-dessus du haut de la
//! bougie d'avant, ou l'inverse) **et** casse le dernier sommet (ou creux) de swing. La zone va du haut au bas de
//! cette bougie (ou de son corps).
//! - Quand une clôture traverse la zone de l'autre côté, elle devient un **breaker** (sa polarité s'inverse).
//! - Quand le prix revient toucher un breaker et clôture du nouveau côté, Argos marque le **changement de polarité**
//!   (▲ support confirmé, ▼ résistance confirmée).
//! - **Force Pwr %** (conception Argos) : la part du volume de l'impulsion de la zone parmi les zones affichées du même
//!   côté (100 % au total), ou le volume moyen de l'impulsion rapporté au volume moyen du marché.
//! - **BULL / BEAR n/4** (conception Argos) : quand le prix entre dans une zone, 4 critères comptés — prix dans la zone,
//!   sens de la dernière impulsion, force de la zone au-dessus du minimum, bougie vector dans le même sens.

use crate::inputs::{check, choice, color, num, Inputs, Param};
use argos_core::Bar;

/// Réglages (ordre de la fenêtre ; les nouveaux à la fin : les réglages enregistrés restent lus à leur place).
pub const PARAMS: [Param; 20] = [
    choice("portee", 0, "Portée des swings", 1, &["Courte (3)", "Moyenne (8)", "Longue (20)"]),
    num("nHaut", 0, "Zones haussières affichées", 3.0, 0.0, 20.0, 1.0),
    num("nBas", 0, "Zones baissières affichées", 3.0, 0.0, 20.0, 1.0),
    check("corps", 0, "Zone = corps de la bougie (sinon mèches comprises)", false),
    color("coulHaut", 1, "Zone haussière", 0x2962ff),
    color("coulHautCasse", 1, "Haussière cassée (breaker)", 0xf23645),
    color("coulBas", 1, "Zone baissière", 0xff9800),
    color("coulBasCasse", 1, "Baissière cassée (breaker)", 0x089981),
    check("polarite", 1, "Marquer les changements de polarité ▲▼", false),
    check("tableau", 1, "Tableau résumé", false),
    choice(
        "tblPos",
        1,
        "Tableau : position",
        0,
        &[
            "Haut droite",
            "Haut gauche",
            "Haut centre",
            "Milieu droite",
            "Milieu gauche",
            "Bas droite",
            "Bas gauche",
            "Bas centre",
        ],
    ),
    choice("tblSize", 1, "Tableau : taille", 1, &["Minuscule", "Petit", "Normal", "Grand", "Énorme"]),
    // ajoutés (force, étiquettes, n/4) : à la fin
    check("pwr", 2, "Afficher la force Pwr % sur chaque zone", true),
    choice(
        "pwrMode",
        2,
        "Calcul de la force",
        0,
        &["Part du volume des zones affichées (100 %)", "Volume de l'impulsion ÷ volume moyen"],
    ),
    check("buySell", 1, "Étiquettes BUY / SELL", true),
    choice("palette", 1, "Couleurs", 0, &["Argos (bleu / orange)", "Vert / mauve"]),
    check("n4", 3, "Étiquettes BULL / BEAR n/4", true),
    num("n4Min", 3, "Minimum pour l'étiquette (/4)", 2.0, 1.0, 4.0, 1.0),
    num("pwrMin", 3, "Force minimum pour compter (Pwr %)", 50.0, 0.0, 1000.0, 5.0),
    num("vecMult", 3, "Bougie vector : volume ≥ × la moyenne", 1.5, 1.0, 10.0, 0.1),
];
pub const GROUPS: [&str; 4] = ["⚙️ Détection", "🎨 Style", "💪 Force (Pwr %)", "🔢 BULL / BEAR n/4"];

/// Couleurs vert / mauve (option « Couleurs ») : zone haussière, haussière cassée, baissière, baissière cassée.
pub const VERT_MAUVE: [u32; 4] = [0x089981, 0x9c27b0, 0x9c27b0, 0x089981];

/// Une zone.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    pub high: f64,
    pub low: f64,
    /// Bougie de l'order block.
    pub origin: usize,
    /// Bougie de la clôture qui l'a traversée (breaker), sinon `None`.
    pub broken_at: Option<usize>,
    /// Bougie de l'impulsion qui a créé la zone.
    pub born: usize,
    /// Volume de l'impulsion (de l'order block à la bougie de l'impulsion).
    pub volume: f64,
    /// Volume moyen du marché (20 bougies) avant l'order block.
    pub avg: f64,
    /// Force Pwr % (voir `pwrMode`).
    pub pwr: f64,
}

/// Un changement de polarité confirmé.
#[derive(Debug, Clone, PartialEq)]
pub struct Flip {
    pub at: usize,
    pub price: f64,
    /// ▲ un ancien plafond devient support (sinon ▼ un ancien plancher devient résistance).
    pub support: bool,
}

/// Étiquette BULL / BEAR n/4 : bougie, haussière, nombre de critères.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Note {
    pub at: usize,
    pub bull: bool,
    pub n: u8,
}

/// Résultat : zones les plus récentes d'abord.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct OrderBlocks {
    pub rising: Vec<Zone>,
    pub falling: Vec<Zone>,
    pub flips: Vec<Flip>,
    pub notes: Vec<Note>,
}

/// Dernier sommet (ou creux) de swing confirmé avant la bougie `i` : `n` bougies de chaque côté.
fn last_swing(bars: &[Bar], i: usize, n: usize, high: bool) -> Option<f64> {
    (n..i.saturating_sub(n)).rev().find_map(|k| {
        let v = if high { bars[k].high } else { bars[k].low };
        let ok = (k - n..=k + n)
            .filter(|&j| j != k)
            .all(|j| if high { bars[j].high <= v } else { bars[j].low >= v });
        ok.then_some(v)
    })
}

/// Force Pwr % des zones : part du volume parmi les `shown` plus récentes (mode 0), ou volume moyen de l'impulsion ÷
/// volume moyen du marché × 100 (mode 1). `zones` : de la plus ancienne à la plus récente.
fn forces(zones: &mut [Zone], shown: usize, mode: usize) {
    if mode == 1 {
        for z in zones.iter_mut() {
            let len = (z.born - z.origin + 1) as f64;
            z.pwr = if z.avg > 0.0 { z.volume / len / z.avg * 100.0 } else { 0.0 };
        }
        return;
    }
    let from = zones.len().saturating_sub(shown);
    let total: f64 = zones[from..].iter().map(|z| z.volume).sum();
    for (k, z) in zones.iter_mut().enumerate() {
        z.pwr = if k >= from && total > 0.0 { z.volume / total * 100.0 } else { 0.0 };
    }
}

pub fn compute(bars: &[Bar], vals: &[f64]) -> OrderBlocks {
    let p = Inputs { params: &PARAMS, vals };
    let n = [3, 8, 20][(p.get("portee") as usize).min(2)];
    let body = p.on("corps");
    let mode = p.pick("pwrMode");
    let (nu, nd) = (p.get("nHaut").max(0.0) as usize, p.get("nBas").max(0.0) as usize);
    let (n4_min, pwr_min, vmult) = (p.get("n4Min").round().max(1.0) as u8, p.get("pwrMin"), p.get("vecMult"));
    let vol: Vec<f64> = bars.iter().map(|b| b.volume).collect();
    let avg20 = crate::ta::sma(&vol, 20);
    let avg10 = crate::ta::sma(&vol, 10);
    let mut out = OrderBlocks::default();
    let (mut rising, mut falling): (Vec<Zone>, Vec<Zone>) = (vec![], vec![]);
    let zone_of = |b: &Bar, k: usize, i: usize| {
        let (hi, lo) = if body { (b.open.max(b.close), b.open.min(b.close)) } else { (b.high, b.low) };
        Zone {
            high: hi,
            low: lo,
            origin: k,
            broken_at: None,
            born: i,
            volume: vol[k..=i].iter().sum(),
            avg: if k > 0 { avg20[k - 1] } else { f64::NAN },
            pwr: 0.0,
        }
    };
    // sens de la dernière impulsion (1 haussière, −1 baissière) et prix déjà dans une zone à la bougie d'avant
    let (mut trend, mut was_up, mut was_dn) = (0i8, false, false);
    for i in 2..bars.len() {
        let (a, c) = (&bars[i - 2], &bars[i]);
        // impulsion haussière : FVG (bas de i au-dessus du haut de i-2) et clôture au-dessus du dernier sommet
        if c.low > a.high && last_swing(bars, i, n, true).is_some_and(|s| c.close > s) {
            trend = 1;
            if let Some(k) = (0..i).rev().take(n + 2).find(|&k| bars[k].close < bars[k].open) {
                if !rising.iter().any(|z| z.origin == k) {
                    rising.push(zone_of(&bars[k], k, i));
                }
            }
        }
        if c.high < a.low && last_swing(bars, i, n, false).is_some_and(|s| c.close < s) {
            trend = -1;
            if let Some(k) = (0..i).rev().take(n + 2).find(|&k| bars[k].close > bars[k].open) {
                if !falling.iter().any(|z| z.origin == k) {
                    falling.push(zone_of(&bars[k], k, i));
                }
            }
        }
        let close = bars[i].close;
        // cassures, puis retests des breakers
        for z in rising.iter_mut().filter(|z| z.origin < i) {
            match z.broken_at {
                None if close < z.low => z.broken_at = Some(i),
                Some(b) if b < i && bars[i].high >= z.low && close < z.low && !out.flips.iter().any(|f| f.at == i) => out.flips.push(Flip {
                    at: i,
                    price: bars[i].high,
                    support: false,
                }),
                _ => {}
            }
        }
        for z in falling.iter_mut().filter(|z| z.origin < i) {
            match z.broken_at {
                None if close > z.high => z.broken_at = Some(i),
                Some(b) if b < i && bars[i].low <= z.high && close > z.high && !out.flips.iter().any(|f| f.at == i) => out.flips.push(Flip {
                    at: i,
                    price: bars[i].low,
                    support: true,
                }),
                _ => {}
            }
        }
        // entrée du prix dans une zone encore valide (née avant cette bougie) : alerte et étiquette BULL / BEAR n/4
        let vector = |up: bool| {
            let a = avg10[i - 1];
            a > 0.0 && vol[i] >= a * vmult && (bars[i].close >= bars[i].open) == up
        };
        for up in [true, false] {
            let (list, shown) = if up { (&mut rising, nu) } else { (&mut falling, nd) };
            forces(list, shown, mode);
            let touch = list
                .iter()
                .rev()
                .take(shown)
                .filter(|z| z.broken_at.is_none() && z.born < i)
                .find(|z| bars[i].low <= z.high && bars[i].high >= z.low)
                .map(|z| z.pwr);
            let was = if up { &mut was_up } else { &mut was_dn };
            if let (Some(pwr), false) = (touch, *was) {
                let k = 1 + u8::from(trend == if up { 1 } else { -1 }) + u8::from(pwr >= pwr_min) + u8::from(vector(up));
                if k >= n4_min {
                    out.notes.push(Note { at: i, bull: up, n: k });
                }
            }
            *was = touch.is_some();
        }
    }
    forces(&mut rising, nu, mode);
    forces(&mut falling, nd, mode);
    rising.reverse();
    falling.reverse();
    (out.rising, out.falling) = (rising, falling);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn b(o: f64, h: f64, l: f64, c: f64) -> Bar {
        Bar {
            time: 0,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: 1.0,
        }
    }

    #[test]
    fn order_block_puis_breaker() {
        let d = crate::inputs::defaults(&PARAMS);
        let mut v = d.clone();
        v[0] = 0.0; // portée courte (3)
        v[3] = 1.0; // zone = corps de la bougie
                    // un petit sommet à 103, une bougie baissière (OB), puis une impulsion qui laisse une FVG et casse 103
        let mut bars = vec![
            b(99.0, 99.5, 98.5, 99.2),
            b(99.2, 99.8, 98.8, 99.5),
            b(99.5, 100.2, 99.0, 100.0),
            b(100.0, 101.0, 99.0, 100.5),
            b(100.5, 102.0, 100.0, 101.5),
            b(101.5, 103.0, 101.0, 102.0),
            b(102.0, 102.5, 100.5, 101.0),
            b(101.0, 101.5, 99.5, 100.0),
            b(100.0, 100.8, 99.2, 99.6),
            b(99.6, 100.0, 98.8, 99.0), // dernière bougie baissière avant l'impulsion
            b(99.0, 102.0, 98.9, 101.8),
            b(101.8, 105.0, 101.2, 104.5), // bas 101,2 > haut 100,0 de deux bougies avant : FVG, clôture > 103
        ];
        let r = compute(&bars, &v);
        assert_eq!(r.rising.len(), 1);
        let z = &r.rising[0];
        assert_eq!((z.origin, z.high, z.low, z.broken_at), (9, 99.6, 99.0, None), "corps de la bougie baissière");
        // une clôture sous la zone : breaker ; puis un retour sous la zone qui la touche : ▼
        bars.push(b(104.0, 104.2, 98.0, 98.5));
        bars.push(b(98.5, 99.2, 97.5, 98.0));
        let r = compute(&bars, &v);
        assert_eq!(r.rising[0].broken_at, Some(12));
        assert_eq!(r.flips.len(), 1);
        assert!(!r.flips[0].support && r.flips[0].at == 13);
        assert!(compute(&[], &d).rising.is_empty());
    }

    #[test]
    fn force_pwr_et_notes_sur_bougies_reelles() {
        let raw: serde_json::Value = serde_json::from_str(include_str!("../../tests/fixtures/ai_trend_js.json")).unwrap();
        let f = |b: &serde_json::Value, k: &str| b[k].as_f64().unwrap();
        let bars: Vec<Bar> = raw["bars"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| Bar {
                time: b["time"].as_i64().unwrap(),
                open: f(b, "open"),
                high: f(b, "high"),
                low: f(b, "low"),
                close: f(b, "close"),
                volume: f(b, "volume"),
            })
            .collect();
        let d = crate::inputs::defaults(&PARAMS);
        let r = compute(&bars, &d);
        // part du volume : les 3 zones affichées de chaque côté font 100 %, les autres 0
        for list in [&r.rising, &r.falling] {
            assert!(list.len() >= 3, "assez de zones : {}", list.len());
            let s: f64 = list.iter().take(3).map(|z| z.pwr).sum();
            assert!((s - 100.0).abs() < 1e-6, "total {s}");
            assert!(list.iter().skip(3).all(|z| z.pwr == 0.0));
            assert!(list.iter().all(|z| z.born >= z.origin && z.volume > 0.0));
        }
        // étiquettes n/4 : au moins le minimum (2), au plus 4, une par entrée dans une zone
        assert!(!r.notes.is_empty());
        assert!(r.notes.iter().all(|t| (2..=4).contains(&t.n)));
        // mode 2 : volume moyen de l'impulsion ÷ volume moyen du marché
        let mut v = d.clone();
        v[PARAMS.iter().position(|p| p.key == "pwrMode").unwrap()] = 1.0;
        let r2 = compute(&bars, &v);
        assert!(r2.rising.iter().filter(|z| z.avg > 0.0).all(|z| z.pwr > 0.0));
    }
}
