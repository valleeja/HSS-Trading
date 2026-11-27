//! 📅 The month report of the demo account (lot 8 of the Simulator): its finished trades, put in the month of their
//! last exit at the time it really happened (a replay trade exits at the market time of back then: its real time
//! counts), in the chart's time zone. Then the result, the R, the win rate, the balance day by day, the largest drop,
//! the longest losing streak, the best and the worst trade, the costs and the detail by symbol. Computation only: the
//! time of day and the time zone are given, never read here.

use crate::money::Money;
use crate::portfolio::ClosedTrade;
use std::cmp::Ordering;

/// (year, month 1-12, day 1-31) of the time `t` (UTC seconds) in a time zone `offset` seconds from UTC.
pub fn date_of(t: i64, offset: i64) -> (i32, u32, u32) {
    // days → civil date (Howard Hinnant's algorithm)
    let z = (t + offset).div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    ((yoe + era * 400 + i64::from(m <= 2)) as i32, m as u32, d as u32)
}

/// The number of days of month `month` (1-12) of year `year`.
pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// The finished trades of `history`, in the order of their last exit: for each, its exits (their index in
/// `history`), the partial ones then the last one. The partial exits of a trade still open are left out.
pub fn finished_trades(history: &[ClosedTrade]) -> Vec<Vec<usize>> {
    // the partial exits waiting for their last exit, by trade (symbol, opening time)
    let mut pending: Vec<((&str, i64), Vec<usize>)> = vec![];
    let mut out = vec![];
    for (i, c) in history.iter().enumerate() {
        let key = (c.base.as_str(), c.opened_at);
        let k = pending.iter().position(|(x, _)| *x == key);
        if c.partiel {
            match k {
                Some(k) => pending[k].1.push(i),
                None => pending.push((key, vec![i])),
            }
        } else {
            let mut parts = k.map(|k| pending.swap_remove(k).1).unwrap_or_default();
            parts.push(i);
            out.push(parts);
        }
    }
    out
}

/// The net result of a finished trade (`parts`, its exits in `history`): costs and funding deducted.
pub fn trade_net(history: &[ClosedTrade], parts: &[usize]) -> Money {
    parts.iter().map(|&i| history[i].net()).sum()
}

/// The result of a finished trade in R (its P&L over its risk, partial exits included); `None` without a stop.
pub fn trade_r(history: &[ClosedTrade], parts: &[usize]) -> Option<f64> {
    let pnl: Money = parts.iter().map(|&i| history[i].pnl).sum();
    let risk: Money = parts.iter().map(|&i| history[i].risk).sum();
    (risk > Money::ZERO).then(|| pnl.to_f64() / risk.to_f64())
}

/// The date (year, month, day) of the last exit of the finished trade `parts`, at its real time, in the time zone `offset`.
fn exit_date(history: &[ClosedTrade], parts: &[usize], offset: &impl Fn(i64) -> i64) -> (i32, u32, u32) {
    let t = parts.last().map_or(0, |&i| history[i].real_close());
    date_of(t, offset(t))
}

/// 📅 The months that have finished trades, oldest first: the page's ‹ › goes from one to the next.
pub fn months(history: &[ClosedTrade], offset: impl Fn(i64) -> i64) -> Vec<(i32, u32)> {
    let mut v: Vec<(i32, u32)> = finished_trades(history)
        .iter()
        .map(|p| {
            let (y, m, _) = exit_date(history, p, &offset);
            (y, m)
        })
        .collect();
    v.sort_unstable();
    v.dedup();
    v
}

/// A line of the detail by symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolLine {
    pub base: String,
    pub trades: usize,
    /// The sum of the R of its trades that had a stop.
    pub r: f64,
    pub net: Money,
}

/// 📅 The report of one month.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MonthReport {
    pub year: i32,
    pub month: u32,
    /// Its finished trades, in order: their exits in the history (see [`finished_trades`]).
    pub trades: Vec<Vec<usize>>,
    /// 💰 The balance at the month's start (the starting capital and the trades finished before), the month's net result
    /// (costs and funding deducted) and its share of the starting balance, in %.
    pub start: Money,
    pub net: Money,
    pub pct: f64,
    /// ⚖ The sum of the R of the trades that had a stop, and how many they are.
    pub r: f64,
    pub with_r: usize,
    /// The winning trades (net result above 0).
    pub winners: usize,
    /// 📈 The balance at the end of each day, from the 1st to the last day shown: the month's last day, or today for the
    /// current month.
    pub days: Vec<Money>,
    /// The month's highest balance, and the largest drop from a high (in $ and in % of that high), trade after
    /// trade.
    pub high: Money,
    pub drawdown: Money,
    pub drawdown_pct: f64,
    /// The longest streak of losing trades.
    pub losing_streak: usize,
    /// 📅 The day of each trade's exit (aligned with `trades`), the days of the largest drop (the day of its high, 1
    /// when the high is the month's start, then the day of its low) and those of the longest losing streak (its first
    /// and its last losing trade).
    pub exit_days: Vec<u32>,
    pub drawdown_days: Option<(u32, u32)>,
    pub streak_days: Option<(u32, u32)>,
    /// 🏆 The best trade (if it won) and the worst one (if it lost): their index in `trades`.
    pub best: Option<usize>,
    pub worst: Option<usize>,
    /// 💸 The costs paid: the fees, the bid / ask spread and the slippage, the funding.
    pub fees: Money,
    pub slippage: Money,
    pub funding: Money,
    /// The detail by symbol, in alphabetical order.
    pub symbols: Vec<SymbolLine>,
}

impl MonthReport {
    /// The average R per trade (those that had a stop); `None` without any.
    pub fn r_avg(&self) -> Option<f64> {
        (self.with_r > 0).then(|| self.r / self.with_r as f64)
    }

    /// The % of winning trades (0 without a trade).
    pub fn win_rate(&self) -> f64 {
        if self.trades.is_empty() {
            0.0
        } else {
            self.winners as f64 * 100.0 / self.trades.len() as f64
        }
    }

    /// 💸 All the costs paid.
    pub fn costs(&self) -> Money {
        self.fees + self.slippage + self.funding
    }

    /// 💰 The balance at the month's end (today for the current month).
    pub fn end(&self) -> Money {
        self.start + self.net
    }
}

/// 📅 The report of the month `(year, month)`: `start`, the account's starting capital; `today`, the time now (UTC
/// seconds: the current month's curve stops there); `offset(t)`, the offset of the chart's time zone at the time `t`
/// (seconds, daylight saving time included).
pub fn month_report(history: &[ClosedTrade], start: Money, (year, month): (i32, u32), today: i64, offset: impl Fn(i64) -> i64) -> MonthReport {
    let mut r = MonthReport {
        year,
        month,
        start,
        ..MonthReport::default()
    };
    // the exit day of each trade of the month
    let mut day = vec![];
    for p in finished_trades(history) {
        let (y, m, d) = exit_date(history, &p, &offset);
        match (y, m).cmp(&(year, month)) {
            Ordering::Less => r.start += trade_net(history, &p),
            Ordering::Equal => {
                day.push(d);
                r.trades.push(p);
            }
            Ordering::Greater => {}
        }
    }
    let mut balance = r.start;
    r.high = r.start;
    let (mut streak, mut best, mut worst) = (0, Money::ZERO, Money::ZERO);
    let (mut high_day, mut streak_start) = (1, 1);
    let mut nets = vec![];
    for (k, p) in r.trades.iter().enumerate() {
        let net = trade_net(history, p);
        nets.push(net);
        r.net += net;
        r.winners += usize::from(net > Money::ZERO);
        let tr = trade_r(history, p);
        if let Some(x) = tr {
            r.r += x;
            r.with_r += 1;
        }
        // 📉 the largest drop from a high, trade after trade
        balance += net;
        if balance > r.high {
            (r.high, high_day) = (balance, day[k]);
        }
        if r.high - balance > r.drawdown {
            r.drawdown = r.high - balance;
            r.drawdown_pct = if r.high > Money::ZERO {
                r.drawdown.to_f64() / r.high.to_f64() * 100.0
            } else {
                0.0
            };
            r.drawdown_days = Some((high_day, day[k]));
        }
        streak = if net < Money::ZERO { streak + 1 } else { 0 };
        if streak == 1 {
            streak_start = day[k];
        }
        if streak > r.losing_streak {
            r.losing_streak = streak;
            r.streak_days = Some((streak_start, day[k]));
        }
        if net > best {
            (best, r.best) = (net, Some(k));
        }
        if net < worst {
            (worst, r.worst) = (net, Some(k));
        }
        for &i in p {
            let c = &history[i];
            r.fees += c.fees - c.glissement;
            r.slippage += c.glissement;
            r.funding += c.funding;
        }
        let base = p.last().map_or("", |&i| history[i].base.as_str());
        match r.symbols.iter_mut().find(|s| s.base == base) {
            Some(s) => {
                s.trades += 1;
                s.r += tr.unwrap_or(0.0);
                s.net += net;
            }
            None => r.symbols.push(SymbolLine {
                base: base.to_string(),
                trades: 1,
                r: tr.unwrap_or(0.0),
                net,
            }),
        }
    }
    r.symbols.sort_by(|a, b| a.base.cmp(&b.base));
    r.pct = if r.start > Money::ZERO {
        r.net.to_f64() / r.start.to_f64() * 100.0
    } else {
        0.0
    };
    // 📈 the balance at the end of each day, up to today for the current month
    let (ty, tm, td) = date_of(today, offset(today));
    let shown = match (ty, tm).cmp(&(year, month)) {
        Ordering::Equal => td,
        Ordering::Greater => days_in_month(year, month),
        Ordering::Less => 0,
    };
    r.days = (1..=shown)
        .map(|d| r.start + day.iter().zip(&nets).filter(|(x, _)| **x <= d).map(|(_, n)| *n).sum::<Money>())
        .collect();
    r.exit_days = day;
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Side;

    /// November 1st 2026, 00:00 UTC.
    const NOV1: i64 = 1_793_491_200;
    const DAY: i64 = 86_400;
    /// New York in November (winter time).
    const NY: i64 = -5 * 3600;

    /// An exit of `base` opened at `opened`, exited at `closed`: P&L, risk and fees in dollars.
    fn exit(base: &str, opened: i64, closed: i64, pnl: f64, risk: f64, fees: f64) -> ClosedTrade {
        ClosedTrade {
            base: base.into(),
            side: Side::Long,
            qty: 1.0,
            entry: 100.0,
            exit: 100.0,
            pnl: Money::from_f64(pnl),
            fees: Money::from_f64(fees),
            opened_at: opened,
            closed_at: closed,
            reason: "stop".into(),
            risk: Money::from_f64(risk),
            partiel: false,
            glissement: Money::ZERO,
            funding: Money::ZERO,
            levier: None,
            plan: None,
            real_closed_at: None,
        }
    }

    #[test]
    fn dates_and_days_of_the_month() {
        assert_eq!(date_of(NOV1, 0), (2026, 11, 1));
        // in New York, midnight on November 1st comes at 5:00 UTC (winter time)
        assert_eq!(date_of(NOV1 + 5 * 3600 - 1, NY), (2026, 10, 31));
        assert_eq!(date_of(NOV1 + 5 * 3600, NY), (2026, 11, 1));
        assert_eq!(date_of(0, 0), (1970, 1, 1));
        assert_eq!(date_of(951_782_400, 0), (2000, 2, 29));
        assert_eq!(date_of(-1, 0), (1969, 12, 31));
        assert_eq!(days_in_month(2024, 2), 29);
        assert_eq!(days_in_month(2026, 2), 28);
        assert_eq!(days_in_month(2000, 2), 29);
        assert_eq!(days_in_month(2100, 2), 28);
        assert_eq!(days_in_month(2026, 11), 30);
        assert_eq!(days_in_month(2026, 12), 31);
    }

    #[test]
    fn finished_trades_with_their_partial_exits() {
        let mut h = vec![
            exit("BTC", 10, 20, 50.0, 100.0, 1.0),
            exit("ETH", 30, 40, -100.0, 100.0, 1.0),
            exit("BTC", 10, 50, 120.0, 100.0, 1.0),
            exit("SOL", 60, 70, 10.0, 100.0, 1.0),
            // a second BTC trade opened at the same time, after the first one
            exit("BTC", 10, 80, -30.0, 100.0, 1.0),
        ];
        h[0].partiel = true;
        h[3].partiel = true; // SOL: still open
        assert_eq!(finished_trades(&h), vec![vec![1], vec![0, 2], vec![4]]);
        assert_eq!(trade_net(&h, &[0, 2]), Money::from_f64(168.0));
        assert_eq!(trade_r(&h, &[0, 2]), Some(170.0 / 200.0));
        let mut no_stop = h[1].clone();
        no_stop.risk = Money::ZERO;
        assert_eq!(trade_r(&[no_stop], &[0]), None);
    }

    #[test]
    fn a_replay_trade_counts_in_the_month_it_was_made() {
        let mut replay = exit("SOL", NOV1 - 600 * DAY, NOV1 - 599 * DAY, -100.0, 100.0, 0.0);
        // made on November 13th, on bars of March 2025
        replay.real_closed_at = Some(NOV1 + 12 * DAY + 20 * 3600);
        let h = vec![exit("BTC", NOV1 - 40 * DAY, NOV1 - 30 * DAY, 50.0, 100.0, 0.0), replay];
        assert_eq!(months(&h, |_| NY), vec![(2026, 10), (2026, 11)]);
        let r = month_report(&h, Money::from_f64(1000.0), (2026, 11), NOV1 + 20 * DAY, |_| NY);
        assert_eq!(r.trades, vec![vec![1]]);
        assert_eq!(r.start, Money::from_f64(1050.0));
        // without a real time (older saves), the market time counts
        let mut old = h.clone();
        old[1].real_closed_at = None;
        assert_eq!(months(&old, |_| NY), vec![(2025, 3), (2026, 10)]);
    }

    #[test]
    fn month_report_figures() {
        let t = |day: i64, h: i64| NOV1 + (day - 1) * DAY + h * 3600;
        let mut h = vec![
            // October: counts in the starting balance
            exit("BTC", t(-5, 14), t(-4, 14), 200.0, 100.0, 4.0),
            exit("BTC", t(2, 14), t(2, 16), 200.0, 100.0, 5.0),
            exit("ETH", t(3, 14), t(3, 15), -100.0, 100.0, 5.0),
            exit("SOL", t(5, 14), t(5, 15), -100.0, 100.0, 2.0),
            // the 5th at 22:00 in New York: still the 5th
            exit("BTC", t(5, 16), t(6, 3), -50.0, 0.0, 1.0),
            exit("ETH", t(9, 14), t(9, 18), 300.0, 100.0, 6.0),
            // December: not in this month
            exit("SOL", t(31, 14), t(31, 15), 10.0, 100.0, 0.0),
        ];
        h[1].glissement = Money::from_f64(2.0);
        h[3].funding = Money::from_f64(1.5);
        let r = month_report(&h, Money::from_f64(10_000.0), (2026, 11), t(21, 18), |_| NY);
        assert_eq!(r.trades.len(), 5);
        assert_eq!(r.start, Money::from_f64(10_196.0));
        // 195 − 105 − 103,50 − 51 + 294
        assert_eq!(r.net, Money::from_f64(229.5));
        assert!((r.pct - 229.5 / 10_196.0 * 100.0).abs() < 1e-9);
        assert_eq!((r.r, r.with_r), (2.0 - 1.0 - 1.0 + 3.0, 4));
        assert_eq!(r.r_avg(), Some(0.75));
        assert_eq!(r.winners, 2);
        assert!((r.win_rate() - 40.0).abs() < 1e-9);
        assert_eq!(r.losing_streak, 3);
        assert_eq!((r.best, r.worst), (Some(4), Some(1)));
        // from the high (10,391) to the low (10,131.50)
        assert_eq!(r.high, Money::from_f64(10_425.5));
        assert_eq!(r.drawdown, Money::from_f64(259.5));
        // the high on the 2nd, the low on the 5th; the three losses from the 3rd to the 5th
        assert_eq!(r.exit_days, vec![2, 3, 5, 5, 9]);
        assert_eq!((r.drawdown_days, r.streak_days), (Some((2, 5)), Some((3, 5))));
        assert!((r.drawdown_pct - 259.5 / 10_391.0 * 100.0).abs() < 1e-9);
        assert_eq!(
            (r.fees, r.slippage, r.funding),
            (Money::from_f64(17.0), Money::from_f64(2.0), Money::from_f64(1.5))
        );
        assert_eq!(r.costs(), Money::from_f64(20.5));
        assert_eq!(r.end(), Money::from_f64(10_425.5));
        let names: Vec<(&str, usize)> = r.symbols.iter().map(|s| (s.base.as_str(), s.trades)).collect();
        assert_eq!(names, vec![("BTC", 2), ("ETH", 2), ("SOL", 1)]);
        assert_eq!(r.symbols[1].net, Money::from_f64(189.0));
        // 21 days shown (today is the 21st): the balance changes on trade days only
        assert_eq!(r.days.len(), 21);
        let on = |d: usize| r.days[d - 1].to_f64();
        assert_eq!((on(1), on(2), on(3), on(4)), (10_196.0, 10_391.0, 10_286.0, 10_286.0));
        assert_eq!((on(5), on(6), on(9), on(21)), (10_131.5, 10_131.5, 10_425.5, 10_425.5));
        // a past month shows all its days; a month to come, none
        assert_eq!(month_report(&h, Money::from_f64(10_000.0), (2026, 10), t(21, 18), |_| NY).days.len(), 31);
        assert!(month_report(&h, Money::from_f64(10_000.0), (2026, 12), t(21, 18), |_| NY).days.is_empty());
    }

    #[test]
    fn month_without_loss_or_trade() {
        let h = vec![exit("BTC", NOV1, NOV1 + 3600, 100.0, 100.0, 0.0)];
        let r = month_report(&h, Money::from_f64(1000.0), (2026, 11), NOV1 + 2 * DAY, |_| 0);
        assert_eq!((r.best, r.worst, r.losing_streak), (Some(0), None, 0));
        assert_eq!((r.drawdown, r.drawdown_days, r.streak_days), (Money::ZERO, None, None));
        let empty = month_report(&[], Money::from_f64(1000.0), (2026, 11), NOV1 + 2 * DAY, |_| 0);
        assert_eq!((empty.trades.len(), empty.win_rate(), empty.r_avg(), empty.pct), (0, 0.0, None, 0.0));
        assert_eq!(empty.days, vec![Money::from_f64(1000.0); 3]);
    }
}
