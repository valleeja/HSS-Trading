//! 📊 Argos Tableau multi-UT : pour chaque unité de temps (15m, 1H, 4H, D, W) au moins égale à celle de la chart,
//! regroupée depuis ses bougies : tendance (EMA rapide contre EMA lente, et le prix par rapport à l'EMA rapide),
//! RSI, variation de la bougie en cours et temps avant sa clôture. Une UT plus petite que la chart : « — ».

use crate::inputs::{check, int, Inputs, Param};
use crate::ta::{ema, rsi};
use argos_core::{aggregate, bucket_end, bucket_start, Bar};

pub const GROUPS: [&str; 2] = ["⚙️ Calcul", "📊 Unités de temps"];
pub const PARAMS: [Param; 8] = [
    int("fast", 0, "EMA rapide", 20.0, 2.0, 200.0),
    int("slow", 0, "EMA lente", 50.0, 3.0, 400.0),
    int("rsi", 0, "Longueur du RSI", 14.0, 2.0, 100.0),
    check("m15", 1, "15m", true),
    check("h1", 1, "1H", true),
    check("h4", 1, "4H", true),
    check("d1", 1, "D", true),
    check("w1", 1, "W", true),
];

/// Les unités de temps du tableau : (secondes, nom, réglage).
pub const TFS: [(i64, &str, &str); 5] = [
    (900, "15m", "m15"),
    (3600, "1H", "h1"),
    (14_400, "4H", "h4"),
    (86_400, "D", "d1"),
    (604_800, "W", "w1"),
];

/// Une case remplie : tendance (+1 haussière, −1 baissière, 0 neutre), RSI, variation % de la bougie en cours,
/// secondes avant sa clôture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cell {
    pub trend: i8,
    pub rsi: f64,
    pub chg: f64,
    pub left: i64,
}

/// Une ligne : nom de l'UT et sa case (`None` : UT plus petite que la chart, ou trop peu de bougies).
#[derive(Debug, Clone, PartialEq)]
pub struct Row {
    pub name: &'static str,
    pub cell: Option<Cell>,
}

/// Le tableau (lignes des UT cochées).
pub fn compute(bars: &[Bar], tf: i64, now: i64, vals: &[f64]) -> Vec<Row> {
    let p = Inputs { params: &PARAMS, vals };
    let len = |k: &str| p.get(k).round().max(1.0) as usize;
    let (fast, slow, rl) = (len("fast"), len("slow"), len("rsi"));
    TFS.iter()
        .filter(|t| p.on(t.2))
        .map(|&(sec, name, _)| Row {
            name,
            cell: (tf > 0 && sec >= tf)
                .then(|| if sec == tf { bars.to_vec() } else { aggregate(bars, sec) })
                .filter(|b| b.len() > slow.max(rl) + 1)
                .map(|b| {
                    let close: Vec<f64> = b.iter().map(|x| x.close).collect();
                    let k = b.len() - 1;
                    let (f, s) = (ema(&close, fast)[k], ema(&close, slow)[k]);
                    let c = close[k];
                    let trend = if f > s && c >= f {
                        1
                    } else if f < s && c <= f {
                        -1
                    } else {
                        0
                    };
                    let chg = if b[k].open > 0.0 { (c / b[k].open - 1.0) * 100.0 } else { 0.0 };
                    Cell {
                        trend,
                        rsi: rsi(&close, rl)[k],
                        chg,
                        left: (bucket_end(bucket_start(b[k].time, sec), sec) - now).max(0),
                    }
                }),
        })
        .collect()
}

/// Nombre d'UT alignées dans chaque sens (haussières, baissières).
pub fn score(rows: &[Row]) -> (usize, usize) {
    let t = |d: i8| rows.iter().filter(|r| r.cell.is_some_and(|c| c.trend == d)).count();
    (t(1), t(-1))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(n: usize, step: i64, f: impl Fn(usize) -> f64) -> Vec<Bar> {
        (0..n)
            .map(|i| {
                let c = f(i);
                Bar {
                    time: i as i64 * step,
                    open: c - 0.1,
                    high: c + 0.5,
                    low: c - 0.5,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect()
    }

    #[test]
    fn hausse_reguliere_toutes_les_ut_haussieres() {
        let b = bars(6000, 900, |i| 100.0 + i as f64 * 0.05);
        let now = b.last().unwrap().time + 60;
        let r = compute(&b, 900, now, &crate::inputs::defaults(&PARAMS));
        assert_eq!(r.len(), 5);
        let names: Vec<&str> = r.iter().map(|x| x.name).collect();
        assert_eq!(names, ["15m", "1H", "4H", "D", "W"]);
        for row in &r[..4] {
            let c = row.cell.unwrap_or_else(|| panic!("{} rempli", row.name));
            assert_eq!(c.trend, 1, "{}", row.name);
            assert!(c.rsi > 50.0);
        }
        // 6000 × 15 min ≈ 62 jours : trop peu de semaines pour l'EMA 50
        assert!(r[4].cell.is_none());
        assert_eq!(score(&r).0, 4);
        // la 1re ligne : 15 min, bougie en cours qui se ferme dans 14 min
        assert_eq!(r[0].cell.unwrap().left, 840);
    }

    #[test]
    fn ut_plus_petite_que_la_chart_vide_et_baisse() {
        let b = bars(400, 3600, |i| 500.0 - i as f64 * 0.5);
        let r = compute(&b, 3600, 400 * 3600, &crate::inputs::defaults(&PARAMS));
        assert!(r[0].cell.is_none(), "15m sur une chart 1H : vide");
        assert_eq!(r[1].cell.unwrap().trend, -1);
        assert_eq!(score(&r), (0, 2));
    }
}
