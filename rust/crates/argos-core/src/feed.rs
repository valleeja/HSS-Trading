//! Contrat de lecture d'API de bougies. Chaque exchange de la couche 2A (`argos-data`) l'implémente.
//! Le Core ne connaît aucun exchange : il connaît seulement ce contrat.

use crate::types::{Bar, Result, Timeframe};

/// Un marché tel qu'affiché dans la recherche de symbole.
#[derive(Debug, Clone, PartialEq)]
pub struct Market {
    pub base: String,
    pub price: f64,
    /// The price 24 h ago (for the change); 0 when the exchange does not give it: the change is then not known.
    pub prev: f64,
}

impl Market {
    /// The change over 24 h in %; `None` without a price or without the price 24 h ago (shown as « — »).
    pub fn change_pct(&self) -> Option<f64> {
        (self.prev > 0.0 && self.price > 0.0).then(|| (self.price / self.prev - 1.0) * 100.0)
    }
}

/// Lecture seule : bougies historiques et liste des marchés. Aucune clé, aucun ordre.
/// Le direct (WebSocket) dépend de la plateforme (desktop ou navigateur) : il est défini dans la couche 2A.
#[allow(async_fn_in_trait)]
pub trait Feed {
    /// Identifiant court de l'exchange (« hl », « okx », « binance »…).
    fn id(&self) -> &'static str;
    /// Nom affiché (« Hyperliquid », « OKX »…).
    fn name(&self) -> &'static str;
    /// Bougies du plus ancien au plus récent, sans doublon de temps.
    async fn candles(&self, base: &str, tf: Timeframe, limit: usize) -> Result<Vec<Bar>>;
    /// Tous les perps USDT/USD de l'exchange.
    async fn markets(&self) -> Result<Vec<Market>>;
}

/// Bougie cohérente : heure > 0, prix finis et positifs, haut ≥ ouverture et clôture, bas ≤ ouverture et clôture,
/// volume fini et ≥ 0. Une bougie incohérente (réponse abîmée ou folle) est refusée partout : exchanges, fichiers,
/// cache, direct.
pub fn bar_ok(b: &Bar) -> bool {
    defaut(b).is_none()
}

/// Pourquoi une bougie est refusée (texte court pour le journal), ou `None` si elle est cohérente.
pub fn defaut(b: &Bar) -> Option<&'static str> {
    let prix = [b.open, b.high, b.low, b.close];
    if b.time <= 0 {
        Some("heure absente")
    } else if !prix.iter().all(|p| p.is_finite() && *p > 0.0) {
        Some("prix nul, négatif ou illisible")
    } else if b.high < b.open.max(b.close) {
        Some("haut sous l'ouverture ou la clôture")
    } else if b.low > b.open.min(b.close) {
        Some("bas au-dessus de l'ouverture ou de la clôture")
    } else if !(b.volume.is_finite() && b.volume >= 0.0) {
        Some("volume négatif ou illisible")
    } else {
        None
    }
}

/// Comme `clean_bars`, et rend aussi les bougies refusées : (heure, raison), dans l'ordre reçu.
pub fn clean_bars_detail(bars: Vec<Bar>) -> (Vec<Bar>, Vec<(i64, &'static str)>) {
    let refus: Vec<(i64, &'static str)> = bars.iter().filter_map(|b| defaut(b).map(|d| (b.time, d))).collect();
    (clean_bars(bars), refus)
}

/// Remet des bougies dans l'ordre, enlève les doublons de temps (garde la dernière reçue) et les bougies
/// incohérentes (`bar_ok`). Toutes les implémentations de `Feed` passent leurs bougies par ici avant de les renvoyer.
pub fn clean_bars(mut bars: Vec<Bar>) -> Vec<Bar> {
    bars.retain(bar_ok);
    bars.sort_by_key(|b| b.time);
    let mut out: Vec<Bar> = Vec::with_capacity(bars.len());
    for b in bars {
        match out.last_mut() {
            Some(last) if last.time == b.time => *last = b,
            _ => out.push(b),
        }
    }
    out
}

/// 🧹 A price and a size that can be: two numbers, both above zero. The rule of what is not a candle (a trade, a
/// level of an order book); a candle has its own, [`bar_ok`]. What does not pass is left out where it is read, as an
/// incoherent candle is by [`clean_bars`]: it never reaches a chart, a book, a list or a sum.
pub fn price_size_ok(price: f64, size: f64) -> bool {
    price.is_finite() && price > 0.0 && size.is_finite() && size > 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            time: t,
            open: c,
            high: c,
            low: c,
            close: c,
            volume: 1.0,
        }
    }

    #[test]
    fn nettoie_ordre_et_doublons() {
        let out = clean_bars(vec![bar(3, 3.0), bar(1, 1.0), bar(2, 2.0), bar(2, 2.5), bar(0, 9.0)]);
        assert_eq!(out.iter().map(|b| b.time).collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(out[1].close, 2.5);
    }

    #[test]
    fn bougies_incoherentes_refusees() {
        let ok = bar(10, 5.0);
        assert!(bar_ok(&ok));
        let mal = |f: fn(&mut Bar)| {
            let mut b = ok;
            f(&mut b);
            b
        };
        let cas = [
            mal(|b| b.high = 4.0),             // haut sous la clôture
            mal(|b| b.low = 6.0),              // bas au-dessus de l'ouverture
            mal(|b| b.close = -1.0),           // prix négatif
            mal(|b| b.open = 0.0),             // prix nul
            mal(|b| b.high = f64::NAN),        // NaN
            mal(|b| b.volume = -2.0),          // volume négatif
            mal(|b| b.volume = f64::INFINITY), // volume infini
            mal(|b| b.time = 0),               // heure absente
        ];
        for b in cas {
            assert!(!bar_ok(&b), "{b:?}");
        }
        // une seule mauvaise au milieu : les autres restent, dans l'ordre, sans doublon
        let out = clean_bars(vec![bar(1, 1.0), cas[0], bar(3, 3.0), bar(3, 3.5), bar(2, 2.0)]);
        assert_eq!(out.iter().map(|b| (b.time, b.close)).collect::<Vec<_>>(), vec![(1, 1.0), (2, 2.0), (3, 3.5)]);
    }

    #[test]
    fn price_and_size_that_can_be() {
        assert!(price_size_ok(97_123.5, 0.000_16) && price_size_ok(f64::MIN_POSITIVE, f64::MAX));
        for no in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -0.0, -1.0] {
            assert!(!price_size_ok(no, 1.0) && !price_size_ok(1.0, no), "{no}");
        }
    }

    #[test]
    fn variation_24h() {
        let market = |prev: f64| Market {
            base: "BTC".into(),
            price: 110.0,
            prev,
        };
        assert!((market(100.0).change_pct().unwrap() - 10.0).abs() < 1e-9);
        assert_eq!(market(0.0).change_pct(), None, "no price 24 h ago: no change made up");
    }
}
