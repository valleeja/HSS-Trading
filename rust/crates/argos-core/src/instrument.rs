//! 📇 Catalogue des symboles : un `Instrument` = un marché d'un exchange, avec ses infos fixes (pas de prix).
//! Identifiant unique partout dans Argos : `exchange:symbole`, ex. `hl:BTC`, `binance:BTCUSDT`, `hl:xyz:NVDA`.

/// Type de marché.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MarketKind {
    /// Contrat perpétuel.
    Perp,
    /// Achat / vente au comptant.
    Spot,
}

impl MarketKind {
    pub fn id(self) -> &'static str {
        match self {
            MarketKind::Perp => "perp",
            MarketKind::Spot => "spot",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "perp" => Some(MarketKind::Perp),
            "spot" => Some(MarketKind::Spot),
            _ => None,
        }
    }
}

/// Un marché du catalogue.
#[derive(Debug, Clone, PartialEq)]
pub struct Instrument {
    /// Exchange (son id Argos : « hl », « binance »…).
    pub exchange: String,
    /// Symbole exact pour l'API de l'exchange (« BTCUSDT », « BTC-USDT-SWAP », « xyz:NVDA », « btcusd »).
    pub symbol: String,
    /// Actif (« BTC », « NVDA ») et monnaie de cotation (« USDT », « USDC », « USD »).
    pub base: String,
    pub quote: String,
    pub kind: MarketKind,
    /// Groupe affiché : « Crypto », « HIP-3 xyz », « TradFi »…
    pub group: String,
    /// Décimales du prix et de la taille d'un ordre.
    pub price_decimals: u32,
    pub size_decimals: u32,
    /// HIP-3 en growth mode (frais réduits de 90 %).
    pub growth: bool,
    /// Se négocie en ce moment (faux : suspendu ou retiré).
    pub active: bool,
}

impl Instrument {
    /// Identifiant unique : `exchange:symbole`.
    pub fn id(&self) -> String {
        format!("{}:{}", self.exchange, self.symbol)
    }
}

/// Sépare un identifiant `exchange:symbole` (le symbole peut lui-même contenir « : », ex. `hl:xyz:NVDA`).
pub fn split_id(id: &str) -> Option<(&str, &str)> {
    id.split_once(':').filter(|(e, s)| !e.is_empty() && !s.is_empty())
}

/// Décimales d'un pas de prix ou de taille écrit en texte : « 0.10 » → 1, « 0.0001 » → 4, « 1 » → 0.
pub fn decimals_of(step: &str) -> u32 {
    step.trim().split_once('.').map_or(0, |(_, d)| d.trim_end_matches('0').len() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiant_et_decimales() {
        let i = Instrument {
            exchange: "hl".into(),
            symbol: "xyz:NVDA".into(),
            base: "NVDA".into(),
            quote: "USDC".into(),
            kind: MarketKind::Perp,
            group: "HIP-3 xyz".into(),
            price_decimals: 2,
            size_decimals: 3,
            growth: true,
            active: true,
        };
        assert_eq!(i.id(), "hl:xyz:NVDA");
        assert_eq!(split_id("hl:xyz:NVDA"), Some(("hl", "xyz:NVDA")));
        assert_eq!(split_id("BTC"), None);
        assert_eq!((decimals_of("0.10"), decimals_of("0.0001"), decimals_of("1"), decimals_of("5")), (1, 4, 0, 0));
        assert_eq!(MarketKind::parse(MarketKind::Spot.id()), Some(MarketKind::Spot));
    }
}
