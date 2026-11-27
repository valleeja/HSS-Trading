//! 🧪 The full report of a strategy (lot 6b): its closed trades as TradingView's « List of trades » shows them (with
//! the run-up and drawdown of each), the drop of its equity from the highest point at each bar, its results month by
//! month, and the list as a CSV in TradingView's columns, so that the two lists can be compared line by line.

use super::strategie::{chemin, Trade};
use argos_core::{Bar, Money, Side};

/// A closed trade of the list.
#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    /// 1 for the first one.
    pub numero: usize,
    pub side: Side,
    /// The names of its entry and of its exit (TradingView's « Signal »).
    pub entree: String,
    pub sortie: String,
    pub bar_in: usize,
    pub bar_out: usize,
    pub t_in: i64,
    pub t_out: i64,
    pub prix_in: f64,
    pub prix_out: f64,
    pub qty: f64,
    /// Its result, fees taken off, in money and in % of what it bought or sold (entry price × quantity).
    pub net: Money,
    pub net_pct: f64,
    /// The results of the trades so far, this one included.
    pub cumul: Money,
    /// The best and the worst it has been while open (money, before fees, never below 0), from the prices of its bars
    /// after its entry and before its exit (along the path of the bar, like the fills).
    pub run_up: f64,
    pub drawdown: f64,
}

impl Ligne {
    /// What it bought or sold: entry price × quantity.
    pub fn valeur(&self) -> f64 {
        self.prix_in * self.qty
    }

    /// Number of bars it was open (0: in and out in the same bar).
    pub fn barres(&self) -> usize {
        self.bar_out - self.bar_in
    }
}

/// The lowest and the highest price of bar `b` along its path, from `de` (a price on the path; `None`: its open) to
/// `a` (a price after it on the path; `None`: its close).
fn extremes(b: &Bar, de: Option<f64>, a: Option<f64>) -> (f64, f64) {
    let p = chemin(b);
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    let mut voir = |x: f64| {
        lo = lo.min(x);
        hi = hi.max(x);
    };
    let mut parti = de.is_none();
    if parti {
        voir(p[0]);
    }
    for s in 0..3 {
        let (x, y) = (p[s], p[s + 1]);
        let dans = |v: f64| x.min(y) - 1e-9 <= v && v <= x.max(y) + 1e-9;
        let mut depuis = x;
        if !parti {
            match de.filter(|d| dans(*d)) {
                Some(d) => {
                    parti = true;
                    depuis = d;
                    voir(d);
                }
                None => continue,
            }
        }
        // (the exit after the start on this segment: in the way the price goes)
        if let Some(f) = a.filter(|f| dans(*f) && (f - depuis) * (y - x) >= -1e-9) {
            voir(f);
            return (lo, hi);
        }
        voir(y);
    }
    if !parti {
        // (a price off the path, like a fill with slippage: the whole bar)
        voir(b.low);
        voir(b.high);
    }
    (lo, hi)
}

/// The list of closed `trades` (played on `bars`, oldest first).
pub fn lignes(trades: &[Trade], bars: &[Bar]) -> Vec<Ligne> {
    let mut cumul = Money::ZERO;
    trades
        .iter()
        .enumerate()
        .map(|(k, t)| {
            let f = &t.ferme;
            let net = f.net();
            cumul += net;
            let (mut lo, mut hi) = (f.entry, f.entry);
            for (i, b) in bars.iter().enumerate().take(t.bar_out + 1).skip(t.bar_in) {
                let de = (i == t.bar_in).then_some(f.entry);
                let a = (i == t.bar_out).then_some(f.exit);
                let (l, h) = extremes(b, de, a);
                (lo, hi) = (lo.min(l), hi.max(h));
            }
            let (pour, contre) = match f.side {
                Side::Long => (hi - f.entry, f.entry - lo),
                Side::Short => (f.entry - lo, hi - f.entry),
            };
            let valeur = f.entry * f.qty;
            Ligne {
                numero: k + 1,
                side: f.side,
                entree: t.entree.clone(),
                sortie: t.sortie.clone(),
                bar_in: t.bar_in,
                bar_out: t.bar_out,
                t_in: f.opened_at,
                t_out: f.closed_at,
                prix_in: f.entry,
                prix_out: f.exit,
                qty: f.qty,
                net,
                net_pct: if valeur > 0.0 { net.to_f64() / valeur * 100.0 } else { 0.0 },
                cumul,
                run_up: pour.max(0.0) * f.qty,
                drawdown: contre.max(0.0) * f.qty,
            }
        })
        .collect()
}

/// The drop of the equity from its highest point so far, at each bar (money, 0 at a new high).
pub fn baisses(equite: &[f64]) -> Vec<f64> {
    let mut haut = f64::NEG_INFINITY;
    equite
        .iter()
        .map(|e| {
            haut = haut.max(*e);
            haut - e
        })
        .collect()
}

/// The largest drop of the equity from its highest point, in % of that point (0 without one).
pub fn baisse_max_pct(equite: &[f64]) -> f64 {
    let mut haut = f64::NEG_INFINITY;
    equite.iter().fold(0.0, |pire: f64, e| {
        haut = haut.max(*e);
        if haut > 0.0 {
            pire.max((haut - e) / haut * 100.0)
        } else {
            pire
        }
    })
}

/// The results of one month (by the trades' exit time).
#[derive(Debug, Clone, PartialEq)]
pub struct Mois {
    pub annee: i32,
    /// 1 to 12.
    pub mois: u32,
    pub net: Money,
    pub trades: usize,
    pub gagnants: usize,
    /// `net` in % of the account at the month's start (capital and the results before it).
    pub pct: f64,
}

/// (year, month) of `t` (seconds) in the time zone `decalage` seconds from UTC.
fn mois_de(t: i64, decalage: i64) -> (i32, u32) {
    use chrono::Datelike;
    let d = chrono::DateTime::from_timestamp(t + decalage, 0).unwrap_or_default();
    (d.year(), d.month())
}

/// The months that have trades (by their exit, in the chart's time zone), oldest first.
pub fn par_mois(lignes: &[Ligne], capital: f64, decalage: i64) -> Vec<Mois> {
    let mut out: Vec<Mois> = vec![];
    let mut avant = Money::ZERO;
    for l in lignes {
        let (annee, mois) = mois_de(l.t_out, decalage);
        if out.last().is_none_or(|m| (m.annee, m.mois) != (annee, mois)) {
            avant = l.cumul - l.net;
            out.push(Mois {
                annee,
                mois,
                net: Money::ZERO,
                trades: 0,
                gagnants: 0,
                pct: 0.0,
            });
        }
        let m = out.last_mut().expect("just pushed");
        m.net += l.net;
        m.trades += 1;
        m.gagnants += usize::from(l.net > Money::ZERO);
        let debut = capital + avant.to_f64();
        m.pct = if debut > 0.0 { m.net.to_f64() / debut * 100.0 } else { 0.0 };
    }
    out
}

/// « 2026-11-21 14:30 » in the time zone `decalage` seconds from UTC (TradingView's export).
fn date_heure(t: i64, decalage: i64) -> String {
    use chrono::{Datelike, Timelike};
    let d = chrono::DateTime::from_timestamp(t + decalage, 0).unwrap_or_default();
    format!("{}-{:02}-{:02} {:02}:{:02}", d.year(), d.month(), d.day(), d.hour(), d.minute())
}

/// A number for the CSV: up to 8 decimals, no trailing zeros.
fn n(x: f64) -> String {
    let s = format!("{x:.8}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".into()
    } else {
        s.into()
    }
}

/// The list as TradingView exports it (« List of trades »): two rows a trade, its exit then its entry, newest trade
/// first; money in `devise`; dates in the time zone `decalage` seconds from UTC; `capital` for the cumulative %.
pub fn csv(lignes: &[Ligne], capital: f64, devise: &str, decalage: i64) -> String {
    let q = |s: &str| {
        if s.contains([',', '"', '\n']) {
            format!("\"{}\"", s.replace('"', "\"\""))
        } else {
            s.to_string()
        }
    };
    let d = devise;
    let mut out = format!(
        "Trade #,Type,Signal,Date/Time,Price {d},Contracts,Profit {d},Profit %,Cumulative profit {d},Cumulative profit %,Run-up {d},Run-up %,Drawdown {d},Drawdown %\n"
    );
    let pct = |x: f64, de: f64| if de > 0.0 { n((x / de * 100.0 * 100.0).round() / 100.0) } else { "0".into() };
    for l in lignes.iter().rev() {
        let sens = if l.side == Side::Long { "long" } else { "short" };
        let (net, cumul) = (l.net.to_f64(), l.cumul.to_f64());
        let reste = format!(
            "{},{},{},{},{},{},{},{},{}",
            n(l.qty),
            n(net),
            n((l.net_pct * 100.0).round() / 100.0),
            n(cumul),
            pct(cumul, capital),
            n((l.run_up * 100.0).round() / 100.0),
            pct(l.run_up, l.valeur()),
            n((l.drawdown * 100.0).round() / 100.0),
            pct(l.drawdown, l.valeur()),
        );
        for (type_, signal, t, prix) in [
            (format!("Exit {sens}"), &l.sortie, l.t_out, l.prix_out),
            (format!("Entry {sens}"), &l.entree, l.t_in, l.prix_in),
        ] {
            out.push_str(&format!("{},{type_},{},{},{},{reste}\n", l.numero, q(signal), date_heure(t, decalage), n(prix)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn barres(v: &[(f64, f64, f64, f64)]) -> Vec<Bar> {
        v.iter()
            .enumerate()
            .map(|(i, &(open, high, low, close))| Bar {
                // from 2026-01-31 22:00 UTC, one hour each
                time: 1_769_896_800 + i as i64 * 3600,
                open,
                high,
                low,
                close,
                volume: 1.0,
            })
            .collect()
    }

    /// Two trades worked out by hand: a long with a stop, a short closed at the open; their run-up and drawdown along
    /// the path of each bar; the months by their exit; the CSV in TradingView's columns.
    #[test]
    fn liste_des_trades_mois_et_csv() {
        let b = barres(&[
            (100.0, 101.0, 99.0, 100.0),
            // the long fills at 100 (open), its path: 100 → 99 (nearer) → 104 → 103
            (100.0, 104.0, 99.0, 103.0),
            // 103 → 103.5 (nearer) → 95: the stop at 97 is hit on the way down, the low 95 is after it
            (103.0, 103.5, 95.0, 96.0),
            (96.0, 97.0, 95.0, 96.0),
            // the short fills at 96, closed at the next open (94)
            (96.0, 98.0, 93.0, 94.0),
            (94.0, 95.0, 93.0, 94.0),
        ]);
        let src = r#"//@version=6
strategy("Test", default_qty_value = 2, commission_type = strategy.commission.percent, commission_value = 0.1)
if bar_index == 0
    strategy.entry("Achat", strategy.long)
    strategy.exit("Stop", "Achat", stop = 97)
if bar_index == 3
    strategy.entry("Vente", strategy.short)
if bar_index == 4
    strategy.close("Vente", comment = "Fin")
"#;
        let o = super::super::run(src, &b, 3600, &HashMap::new()).unwrap();
        let bilan = o.strategie.unwrap();
        let l = lignes(&bilan.trades, &b);
        assert_eq!(l.len(), 2);
        let (a, v) = (&l[0], &l[1]);
        assert_eq!((a.numero, a.side, a.entree.as_str(), a.sortie.as_str()), (1, Side::Long, "Achat", "Stop"));
        assert_eq!((a.prix_in, a.prix_out, a.qty, a.barres()), (100.0, 97.0, 2.0, 1));
        // −3 × 2, fees: 200 × 0.1 % + 194 × 0.1 % = 0.20 + 0.19 (in cents)
        assert_eq!(a.net, Money::from_f64(-6.39));
        // best 104 (bar 1), worst 97: the 95 of bar 2 comes after the stop
        assert_eq!((a.run_up, a.drawdown), (8.0, 6.0));
        assert_eq!(
            (v.side, v.prix_in, v.prix_out, v.sortie.as_str()),
            (Side::Short, 96.0, 94.0, "Close entry(s) order Vente")
        );
        // the short: from 96 (bar 4's open: 96 → 98 → 93 → 94) to 94 at bar 5's open
        assert_eq!((v.run_up, v.drawdown), (6.0, 4.0));
        assert_eq!(v.cumul, a.net + v.net);
        // months by the exit, in UTC: both leave on 1 Feb
        let m = par_mois(&l, 1_000_000.0, 0);
        assert_eq!(m.len(), 1);
        assert_eq!((m[0].annee, m[0].mois, m[0].trades, m[0].gagnants), (2026, 2, 2, 1));
        // in New York (UTC − 5): both leave on 31 Jan
        let ny = par_mois(&l, 1_000_000.0, -5 * 3600);
        assert_eq!((ny[0].annee, ny[0].mois), (2026, 1));
        let c = csv(&l, 1_000_000.0, "USD", 0);
        let rangs: Vec<&str> = c.lines().collect();
        assert_eq!(rangs.len(), 5);
        assert!(rangs[0].starts_with("Trade #,Type,Signal,Date/Time,Price USD,Contracts,Profit USD"));
        assert!(
            rangs[1].starts_with("2,Exit short,Close entry(s) order Vente,2026-02-01 03:00,94,2,"),
            "{}",
            rangs[1]
        );
        assert!(rangs[2].starts_with("2,Entry short,Vente,2026-02-01 02:00,96,2,"), "{}", rangs[2]);
        assert!(rangs[4].starts_with("1,Entry long,Achat,2026-01-31 23:00,100,2,"), "{}", rangs[4]);
        // run-up 8 of 200: 4 %
        assert!(rangs[4].ends_with(",8,4,6,3"), "{}", rangs[4]);
    }

    #[test]
    fn baisse_depuis_le_plus_haut() {
        let e = [100.0, 110.0, 99.0, 120.0, 90.0];
        assert_eq!(baisses(&e), vec![0.0, 0.0, 11.0, 0.0, 30.0]);
        assert_eq!(baisse_max_pct(&e), 25.0);
        assert_eq!(baisse_max_pct(&[]), 0.0);
    }
}
