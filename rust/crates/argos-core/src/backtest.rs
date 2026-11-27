//! Backtest : rejoue une stratégie bougie par bougie sur un portfolio, puis donne les stats.
//! La stratégie ne voit que les bougies déjà fermées (aucun regard vers le futur).

use crate::alea::Alea;
use crate::money::Money;
use crate::portfolio::{ClosedTrade, Order, Portfolio};
use crate::types::{Bar, Side};

/// Ce que la stratégie demande à la fermeture d'une bougie.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    pub side: Side,
    pub stop: f64,
    pub target: Option<f64>,
}

/// Réglages du backtest.
#[derive(Debug, Clone, PartialEq)]
pub struct BacktestConfig {
    pub start: f64,
    /// Risque par trade en % du solde (ex. 0.5 = 0,5 %).
    pub risk_pct: f64,
    pub fee_rate: f64,
    /// Glissement en fraction du prix (0.0005 = 0,05 %), compté comme un coût à l'entrée et à la sortie.
    pub slippage: f64,
    /// Une seule position à la fois.
    pub one_at_a_time: bool,
}

impl Default for BacktestConfig {
    fn default() -> Self {
        Self {
            start: 100_000.0,
            risk_pct: 0.5,
            fee_rate: 0.0005,
            slippage: 0.0,
            one_at_a_time: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BacktestReport {
    pub trades: Vec<ClosedTrade>,
    /// 💰 Solde final et P&L net, en centimes.
    pub end_balance: Money,
    pub net_pnl: Money,
    pub win_rate: f64,
    /// Gains totaux ÷ pertes totales.
    pub profit_factor: f64,
    /// Plus grosse baisse du solde depuis un sommet, en %.
    pub max_drawdown_pct: f64,
    /// R moyen : P&L net de chaque trade ÷ le risque pris à l'entrée (0 sans trade).
    pub avg_r: f64,
    /// Courbe du solde après chaque trade.
    pub equity_curve: Vec<Money>,
}

/// Lance le backtest. `strategy(bars_fermees)` reçoit l'historique jusqu'à la bougie qui vient de fermer.
/// L'entrée se fait à l'ouverture de la bougie suivante ; stop et objectif sont vérifiés sur chaque bougie.
pub fn run(base: &str, bars: &[Bar], cfg: &BacktestConfig, mut strategy: impl FnMut(&[Bar]) -> Option<Signal>) -> BacktestReport {
    // frais et glissement payés à l'entrée et à la sortie
    let mut pf = Portfolio::new("Backtest", cfg.start, cfg.fee_rate + cfg.slippage.max(0.0));
    let mut pending: Option<Signal> = None;
    // risque pris à l'ouverture de chaque trade (heure d'ouverture → montant), pour le R
    let mut risks: Vec<(i64, Money)> = vec![];
    for (i, b) in bars.iter().enumerate() {
        if let Some(sig) = pending.take() {
            let risk_per_unit = (b.open - sig.stop) * sig.side.sign();
            if risk_per_unit > 0.0 {
                let risk = pf.balance.pct(cfg.risk_pct);
                let qty = risk.to_f64() / risk_per_unit;
                risks.push((b.time, risk));
                let _ = pf.open(
                    Order {
                        base: base.to_string(),
                        side: sig.side,
                        qty,
                        price: b.open,
                        stop: Some(sig.stop),
                        target: sig.target,
                        couts: None,
                        perp: None,
                    },
                    b.time,
                );
            }
        }
        pf.check_exits(base, b.high, b.low, b.time);
        if !(cfg.one_at_a_time && !pf.positions.is_empty()) {
            pending = strategy(&bars[..=i]);
        }
    }
    if let Some(last) = bars.last() {
        let ids: Vec<u64> = pf.positions.iter().map(|p| p.id).collect();
        for id in ids {
            let _ = pf.close(id, last.close, last.time, "fin du test");
        }
    }
    rapport(cfg.start, pf.balance, pf.history, &risks)
}

/// The stats of closed `trades` (oldest first) of an account that started at `start` and ends at `end_balance`;
/// `risks` = the money at risk when each trade opened (by its opening time), for the R (none: R of 0). Used by `run`,
/// and by the strategies of Argos Script, which trade on their own account.
pub fn rapport(start: f64, end_balance: Money, trades: Vec<ClosedTrade>, risks: &[(i64, Money)]) -> BacktestReport {
    let start = Money::from_f64(start);
    let mut bal = start;
    let mut peak = start;
    let mut max_dd = 0.0f64;
    let mut curve = Vec::with_capacity(trades.len());
    for t in &trades {
        bal += t.pnl - t.fees;
        peak = peak.max(bal);
        if peak > Money::ZERO {
            max_dd = max_dd.max((peak - bal).to_f64() / peak.to_f64() * 100.0);
        }
        curve.push(bal);
    }
    let net: Vec<Money> = trades.iter().map(|t| t.pnl - t.fees).collect();
    let wins = net.iter().filter(|x| **x > Money::ZERO).count();
    let gains: f64 = net.iter().filter(|x| **x > Money::ZERO).sum::<Money>().to_f64();
    let losses: f64 = -net.iter().filter(|x| **x < Money::ZERO).sum::<Money>().to_f64();
    let rs: Vec<f64> = trades
        .iter()
        .zip(&net)
        .filter_map(|(t, n)| {
            let r = risks.iter().find(|x| x.0 == t.opened_at)?.1;
            (r > Money::ZERO).then(|| n.to_f64() / r.to_f64())
        })
        .collect();
    let avg_r = if rs.is_empty() { 0.0 } else { rs.iter().sum::<f64>() / rs.len() as f64 };
    BacktestReport {
        win_rate: if trades.is_empty() { 0.0 } else { wins as f64 / trades.len() as f64 * 100.0 },
        profit_factor: if losses > 0.0 {
            gains / losses
        } else if gains > 0.0 {
            f64::INFINITY
        } else {
            0.0
        },
        max_drawdown_pct: max_dd,
        avg_r,
        end_balance,
        net_pnl: end_balance - start,
        equity_curve: curve,
        trades,
    }
}

/// 🎲 Nombre de tirages par défaut du test contre le hasard.
pub const TIRAGES: usize = 1000;

/// 🎲 La stratégie face à des entrées prises au hasard (voir `contre_hasard`).
#[derive(Debug, Clone, PartialEq)]
pub struct Hasard {
    /// Nombre de tirages faits.
    pub tirages: usize,
    /// Rang de la stratégie : 1 + nombre de tirages strictement meilleurs (1 = meilleure que tous).
    pub rang: usize,
    /// % de tirages strictement moins bons que la stratégie.
    pub battus_pct: f64,
    /// 💰 P&L net des tirages, en centimes : médiane (valeur du milieu), meilleur et pire.
    pub mediane: Money,
    pub meilleur: Money,
    pub pire: Money,
}

/// 🎲 La stratégie fait-elle mieux que le hasard ? Chaque tirage refait les trades de `rapport` (même nombre, même
/// sens, même durée en bougies) mais chacun entre sur une bougie tirée au hasard : entrée à son ouverture, sortie à
/// la clôture de la bougie entrée + durée (le tirage garde la sortie dans les données), sans stop ni objectif.
/// Même argent risqué : même risque en argent et même distance de stop en % de l'entrée que le trade d'origine, ce
/// qui revient à engager la même valeur (quantité × prix d'entrée) ; mêmes frais et glissement que `cfg`. Le P&L net
/// de chaque tirage (centimes) se compare à `rapport.net_pnl`.
/// `None` sans tirage, sans trade, ou si un trade ne se retrouve pas dans `bars` (mêmes bougies que le backtest).
/// Déterministe : même `graine` (par défaut `alea::GRAINE`) → même résultat.
pub fn contre_hasard(bars: &[Bar], cfg: &BacktestConfig, rapport: &BacktestReport, tirages: usize, graine: u64) -> Option<Hasard> {
    if tirages == 0 {
        return None;
    }
    let modeles = modeles(bars, rapport)?;
    let mut alea = Alea::new(graine);
    let depart = Money::from_f64(cfg.start);
    let mut nets: Vec<Money> = (0..tirages).map(|_| un_tirage(bars, cfg, &modeles, &mut alea).balance - depart).collect();
    let strat = rapport.net_pnl;
    let mieux = nets.iter().filter(|&&x| x > strat).count();
    let moins = nets.iter().filter(|&&x| x < strat).count();
    nets.sort();
    Some(Hasard {
        tirages,
        rang: 1 + mieux,
        battus_pct: moins as f64 / tirages as f64 * 100.0,
        mediane: nets[tirages / 2],
        meilleur: nets[tirages - 1],
        pire: nets[0],
    })
}

/// 🎲 Ce que chaque tirage refait de chaque trade : (sens, durée en bougies, valeur engagée = quantité × entrée).
/// `None` sans trade ou si un trade ne tombe pas sur une bougie de `bars`.
fn modeles(bars: &[Bar], rapport: &BacktestReport) -> Option<Vec<(Side, usize, f64)>> {
    let index = |t: i64| bars.binary_search_by_key(&t, |b| b.time).ok();
    let m: Vec<_> = rapport
        .trades
        .iter()
        .map(|t| {
            let (a, b) = (index(t.opened_at)?, index(t.closed_at)?);
            (b >= a).then_some((t.side, b - a, t.qty * t.entry))
        })
        .collect::<Option<_>>()?;
    (!m.is_empty()).then_some(m)
}

/// 🎲 Un tirage : chaque trade entre sur une bougie au hasard (la sortie reste dans les données) ; rend le compte.
fn un_tirage(bars: &[Bar], cfg: &BacktestConfig, modeles: &[(Side, usize, f64)], alea: &mut Alea) -> Portfolio {
    let mut pf = Portfolio::new("Hasard", cfg.start, cfg.fee_rate + cfg.slippage.max(0.0));
    for &(side, duree, valeur) in modeles {
        let i = alea.entier((bars.len() - duree) as u64) as usize;
        let (entree, sortie) = (&bars[i], &bars[i + duree]);
        let ordre = Order {
            base: "hasard".into(),
            side,
            qty: valeur / entree.open,
            price: entree.open,
            stop: None,
            target: None,
            couts: None,
            perp: None,
        };
        if let Ok(id) = pf.open(ordre, entree.time) {
            let _ = pf.close(id, sortie.close, sortie.time, "hasard");
        }
    }
    pf
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bars(closes: &[f64]) -> Vec<Bar> {
        closes
            .iter()
            .enumerate()
            .map(|(i, &c)| Bar {
                time: i as i64 * 60,
                open: c,
                high: c + 1.0,
                low: c - 1.0,
                close: c,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn achete_et_touche_objectif() {
        let b = bars(&[100.0, 100.0, 101.0, 103.0, 106.0, 110.0]);
        let cfg = BacktestConfig {
            start: 10_000.0,
            risk_pct: 1.0,
            fee_rate: 0.0,
            slippage: 0.0,
            one_at_a_time: true,
        };
        let mut done = false;
        let r = run("BTC", &b, &cfg, |h| {
            if done || h.len() < 2 {
                return None;
            }
            done = true;
            Some(Signal {
                side: Side::Long,
                stop: 95.0,
                target: Some(105.0),
            })
        });
        assert_eq!(r.trades.len(), 1);
        assert_eq!(r.trades[0].reason, "objectif");
        // entrée à l'ouverture suivante (101), stop 95 → 6 $ de risque par unité ; risque 1 % = 100 $ → 16,67 unités ; sortie à 105 → +4 $ × 16,67
        assert!((r.net_pnl.to_f64() - 100.0 * 4.0 / 6.0).abs() < 0.01, "au centime près");
        assert_eq!(r.win_rate, 100.0);
        // R : +66,67 $ pour 100 $ risqués → +0,67 R
        assert!((r.avg_r - 4.0 / 6.0).abs() < 1e-3, "R moyen {}", r.avg_r);
        // le même trade avec 0,1 % de glissement : moins bien
        let mut once = false;
        let slip = run(
            "BTC",
            &b,
            &BacktestConfig {
                slippage: 0.001,
                ..cfg.clone()
            },
            |h| {
                if once || h.len() < 2 {
                    return None;
                }
                once = true;
                Some(Signal {
                    side: Side::Long,
                    stop: 95.0,
                    target: Some(105.0),
                })
            },
        );
        assert!(slip.net_pnl < r.net_pnl && slip.avg_r < r.avg_r);
    }

    #[test]
    fn aucune_strategie_aucun_trade() {
        let r = run("BTC", &bars(&[1.0, 2.0, 3.0]), &BacktestConfig::default(), |_| None);
        assert!(r.trades.is_empty());
        assert_eq!(r.net_pnl, Money::ZERO);
    }

    /// 🎲 Zigzag fabriqué : 20 bougies de hausse (100 → 120) puis 20 de baisse, répété.
    fn zigzag(n: usize) -> Vec<Bar> {
        let c: Vec<f64> = (0..n)
            .map(|k| {
                let p = (k % 40) as f64;
                100.0 + if p < 20.0 { p } else { 40.0 - p }
            })
            .collect();
        bars(&c)
    }

    /// Une stratégie qui voit l'avenir (`bonne`) achète les creux et vend les sommets ; l'inverse (`!bonne`) fait
    /// le contraire et se fait sortir au stop.
    fn zigzag_strategie(b: &[Bar], bonne: bool) -> BacktestReport {
        run("BTC", b, &BacktestConfig::default(), |h| {
            let next = h.len() % 40; // phase de la bougie d'entrée
            let creux = match next {
                0 => true,
                20 => false,
                _ => return None,
            };
            Some(match (bonne, creux) {
                (true, true) => Signal {
                    side: Side::Long,
                    stop: 90.0,
                    target: Some(119.0),
                },
                (true, false) => Signal {
                    side: Side::Short,
                    stop: 130.0,
                    target: Some(101.0),
                },
                (false, true) => Signal {
                    side: Side::Short,
                    stop: 110.0,
                    target: None,
                },
                (false, false) => Signal {
                    side: Side::Long,
                    stop: 110.0,
                    target: None,
                },
            })
        })
    }

    #[test]
    fn bonne_strategie_au_premier_rang_mauvaise_au_dernier() {
        let b = zigzag(400);
        let cfg = BacktestConfig::default();
        let bonne = zigzag_strategie(&b, true);
        assert!(bonne.trades.len() >= 15 && bonne.net_pnl > Money::ZERO);
        let h = contre_hasard(&b, &cfg, &bonne, TIRAGES, crate::alea::GRAINE).unwrap();
        assert_eq!(h.tirages, TIRAGES);
        assert!(h.rang <= 10, "voit l'avenir : en tête, rang {}", h.rang);
        assert!(h.battus_pct >= 99.0);
        assert!(h.pire <= h.mediane && h.mediane <= h.meilleur);
        let mauvaise = zigzag_strategie(&b, false);
        assert!(mauvaise.net_pnl < Money::ZERO);
        let h = contre_hasard(&b, &cfg, &mauvaise, TIRAGES, crate::alea::GRAINE).unwrap();
        assert!(h.rang > TIRAGES * 9 / 10, "toujours à contre-sens : rang {}", h.rang);
        assert!(h.battus_pct < 10.0);
    }

    #[test]
    fn meme_graine_meme_rang() {
        let b = zigzag(200);
        let cfg = BacktestConfig::default();
        let r = zigzag_strategie(&b, true);
        let a = contre_hasard(&b, &cfg, &r, 200, 7).unwrap();
        assert_eq!(a, contre_hasard(&b, &cfg, &r, 200, 7).unwrap(), "même graine : même résultat");
        let c = contre_hasard(&b, &cfg, &r, 200, 8).unwrap();
        assert_ne!(
            (a.mediane, a.meilleur, a.pire),
            (c.mediane, c.meilleur, c.pire),
            "autre graine : autres tirages"
        );
    }

    #[test]
    fn chaque_tirage_garde_nombre_sens_et_durees() {
        let b = zigzag(300);
        let cfg = BacktestConfig {
            slippage: 0.0003,
            ..BacktestConfig::default()
        };
        let r = zigzag_strategie(&b, false);
        let m = modeles(&b, &r).unwrap();
        assert_eq!(m.len(), r.trades.len());
        let idx = |t: i64| b.iter().position(|x| x.time == t).unwrap();
        let mut alea = Alea::new(3);
        for _ in 0..50 {
            let pf = un_tirage(&b, &cfg, &m, &mut alea);
            assert_eq!(pf.history.len(), r.trades.len(), "même nombre de trades");
            for (t, &(side, duree, valeur)) in pf.history.iter().zip(&m) {
                assert_eq!(t.side, side);
                assert_eq!(idx(t.closed_at) - idx(t.opened_at), duree, "même durée en bougies");
                assert!((t.qty * t.entry - valeur).abs() < 1e-6, "même valeur engagée");
            }
            // 💰 aucune dérive : le solde est la somme exacte des P&L nets, au centime
            let net: Money = pf.history.iter().map(|t| t.pnl - t.fees).sum();
            assert_eq!(pf.balance, Money::from_f64(cfg.start) + net);
        }
    }

    #[test]
    fn contre_hasard_cas_vides() {
        let b = zigzag(100);
        let cfg = BacktestConfig::default();
        let vide = run("BTC", &b, &cfg, |_| None);
        assert_eq!(contre_hasard(&b, &cfg, &vide, TIRAGES, 1), None, "aucun trade");
        let r = zigzag_strategie(&b, true);
        assert_eq!(contre_hasard(&b, &cfg, &r, 0, 1), None, "aucun tirage");
        assert_eq!(contre_hasard(&b[..10], &cfg, &r, 10, 1), None, "trades hors des bougies données");
        assert_eq!(contre_hasard(&[], &cfg, &r, 10, 1), None);
    }
}
