//! 🐋 Baleines gardées sur l'appareil (`whales.json`) : les réglages du panneau (seuil, pièces, bulles, période) et
//! les grosses transactions reçues (100 k$ et plus, 7 jours au plus, 5 000 au plus). Les périodes longues (24 h, 7 j)
//! couvrent ainsi plus que la session en cours. Données publiques seulement (aucune clé).

use crate::*;

/// Transactions gardées au plus, et seuil gardé.
const KEEP: usize = 5_000;
const MIN_USD: f64 = 100_000.0;
/// Écriture au plus toutes les 2 minutes.
const EVERY_S: u64 = 120;

#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct WhaleStore {
    pub(crate) level: usize,
    pub(crate) only: bool,
    pub(crate) bubbles: bool,
    pub(crate) period_min: u32,
    pub(crate) exchanges: Vec<String>,
    pub(crate) kind: u8,
    /// (pièce, ms, prix, taille, achat, source, portefeuille)
    pub(crate) list: Vec<(String, i64, f64, f64, bool, String, String)>,
}

impl WhaleStore {
    pub(crate) fn of(w: &argos_render::whales::Whales) -> Self {
        Self {
            level: w.level,
            only: w.only,
            bubbles: w.bubbles,
            period_min: w.period_min,
            exchanges: w.exchanges.clone(),
            kind: w.kind,
            list: w
                .list
                .iter()
                .filter(|x| x.usd() >= MIN_USD)
                .take(KEEP)
                .map(|x| (x.coin.clone(), x.time_ms, x.price, x.size, x.buy, x.src.to_string(), x.wallet.clone()))
                .collect(),
        }
    }

    pub(crate) fn apply(self, w: &mut argos_render::whales::Whales) {
        (w.level, w.only, w.bubbles, w.period_min) = (self.level.min(3), self.only, self.bubbles, self.period_min);
        w.kind = self.kind.min(2);
        w.exchanges = self
            .exchanges
            .into_iter()
            .filter(|e| argos_data::whales::EXCHANGES.contains(&e.as_str()))
            .collect();
        let list = self
            .list
            .into_iter()
            .map(|(coin, time_ms, price, size, buy, src, wallet)| argos_data::whales::Whale {
                coin,
                time_ms,
                price,
                size,
                buy,
                src: argos_data::whales::EXCHANGES.iter().copied().find(|e| *e == src).unwrap_or("Binance"),
                id: None,
                wallet,
            })
            .collect();
        w.now_ms = now_s() * 1000;
        w.push(list);
    }
}

impl App {
    /// Garde les baleines (au plus toutes les 2 min, ou tout de suite avec `force`).
    pub(crate) fn save_whales(&mut self, force: bool) {
        if !force && self.feeds.whales_saved.is_some_and(|t| t.elapsed().as_secs() < EVERY_S) {
            return;
        }
        self.feeds.whales_saved = Some(Instant::now());
        Self::write_prefs("whales.json", &WhaleStore::of(&self.watch.whales));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baleines_gardees_et_relues() {
        let mut w = argos_render::whales::Whales::default();
        let t = 1_800_000_000_000;
        let wh = |usd: f64, dt: i64, buy: bool| argos_data::whales::Whale {
            coin: "BTC".into(),
            time_ms: t - dt,
            price: 100_000.0,
            size: usd / 100_000.0,
            buy,
            src: "Hyperliquid",
            id: None,
            wallet: String::new(),
        };
        w.push(vec![wh(500e3, 0, true), wh(60e3, 1000, false), wh(200e3, 3 * 3_600_000, false)]);
        w.period_min = 240;
        let s = WhaleStore::of(&w);
        assert_eq!(s.list.len(), 2, "sous 100 k$ : pas gardé");
        let back: WhaleStore = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let mut w2 = argos_render::whales::Whales::default();
        back.apply(&mut w2);
        assert_eq!((w2.period_min, w2.list.len()), (240, 2));
        // période : 4 h = les deux ; 1 h = seulement la récente
        w2.now_ms = t;
        assert_eq!(w2.pressure(t), (500e3, 200e3));
        w2.period_min = 60;
        assert_eq!(w2.pressure(t), (500e3, 0.0));
        w2.period_min = 0;
        assert_eq!(w2.period_label(), "15m", "15 min par défaut");
        // ⏳ données sur 3 h : 4 h et plus sont « au-delà », 1 h non ; la mention a l'heure de départ
        assert!(w2.beyond(240, t) && w2.beyond(10_080, t) && !w2.beyond(60, t));
        w2.period_min = 10_080;
        let note = w2.coverage_note(t).expect("7 j : mention de la durée couverte");
        assert!(note.contains("depuis") && note.contains("3 h 00"), "{note}");
        w2.period_min = 60;
        assert_eq!(w2.coverage_note(t), None, "1 h : tout est couvert");
        // relu après un redémarrage, avec la limite de 7 jours
        let mut old = s.clone();
        old.list
            .push(("ETH".into(), t - 8 * 86_400_000, 3000.0, 100.0, true, "Binance".into(), String::new()));
        let mut w3 = argos_render::whales::Whales::default();
        old.apply(&mut w3);
        assert_eq!(w3.list.len(), 2, "plus de 7 jours : oublié");
        // 🏦 filtre par exchange : les transactions Hyperliquid disparaissent avec « OKX seulement »
        w3.now_ms = t;
        w3.period_min = 10_080;
        assert_eq!(w3.whale_stats().0, 2);
        w3.exchanges = vec!["OKX".into()];
        assert_eq!(w3.whale_stats().0, 0);
        assert_eq!(w3.exchanges_on(), vec!["OKX"]);
        w3.exchanges.clear();
        assert_eq!(w3.exchanges_on().len(), argos_data::whales::DEFAULT_ON, "aucun choix : les 5 plus gros");
        // 📊 flux de tous les ordres et moyenne 7 j
        let f = |h: i64, b: f64, s: f64| argos_data::whales::Flow {
            t_ms: t - (h + 1) * 3_600_000,
            buy_usd: b,
            sell_usd: s,
        };
        w3.flow_h = (0..168).rev().map(|h| f(h, 10e6, 10e6)).collect();
        w3.period_min = 60;
        assert_eq!(w3.flow_sum(t), Some((10e6, 10e6)));
        let ((ab, as_), r) = w3.flow_avg(t).unwrap();
        assert_eq!((ab, as_), (10e6, 10e6));
        assert!((r - 1.0).abs() < 1e-9, "période = la moyenne : 1×");
        // 🔁 pas de doublon : la même transaction reçue deux fois (reconnexion) ne compte qu'une fois ; le même ordre
        // vu sur deux exchanges reste deux lignes (deux exchanges différents)
        let mut w4 = argos_render::whales::Whales::default();
        let one = wh(500e3, 0, true);
        w4.push(vec![one.clone()]);
        w4.push(vec![one.clone(), one.clone()]);
        assert_eq!(w4.list.len(), 1, "même transaction : une seule fois");
        w4.push(vec![argos_data::whales::Whale { src: "OKX", ..one.clone() }]);
        assert_eq!(w4.list.len(), 2, "autre exchange : gardée");
        // 🏛️ tradfi HIP-3 (xyz:GOLD) : filtre Crypto / Tradfi / Tout
        w4.push(vec![argos_data::whales::Whale {
            coin: "xyz:GOLD".into(),
            ..one.clone()
        }]);
        (w4.now_ms, w4.period_min) = (t, 60);
        assert_eq!(w4.whale_stats().0, 3, "Tout");
        w4.kind = 1;
        assert_eq!(w4.whale_stats().0, 2, "Crypto");
        w4.kind = 2;
        assert_eq!(w4.whale_stats().0, 1, "Tradfi");
        assert!(argos_render::whales::is_tradfi("xyz:GOLD") && !argos_render::whales::is_tradfi("BTC"));
    }
}
