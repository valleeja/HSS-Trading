//! Symboles permis (liste fermée), contrôle des bougies reçues et erreurs de Lens.

use argos_core::Bar;
use std::fmt;

/// Symboles que Lens accepte. Un nom reçu de l'extérieur n'est jamais recopié : il est reconnu dans cette liste ou refusé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Symbol {
    XauUsd,
    XagUsd,
    EurUsd,
    GbpUsd,
    UsdJpy,
    BtcUsdt,
    EthUsdt,
    SolUsdt,
    Nas100,
    Spx500,
    Us30,
}

impl Symbol {
    pub const ALL: [Symbol; 11] = [
        Self::XauUsd,
        Self::XagUsd,
        Self::EurUsd,
        Self::GbpUsd,
        Self::UsdJpy,
        Self::BtcUsdt,
        Self::EthUsdt,
        Self::SolUsdt,
        Self::Nas100,
        Self::Spx500,
        Self::Us30,
    ];

    /// Identifiant écrit dans le paquet.
    pub fn id(self) -> &'static str {
        match self {
            Self::XauUsd => "XAUUSD",
            Self::XagUsd => "XAGUSD",
            Self::EurUsd => "EURUSD",
            Self::GbpUsd => "GBPUSD",
            Self::UsdJpy => "USDJPY",
            Self::BtcUsdt => "BTCUSDT",
            Self::EthUsdt => "ETHUSDT",
            Self::SolUsdt => "SOLUSDT",
            Self::Nas100 => "NAS100",
            Self::Spx500 => "SPX500",
            Self::Us30 => "US30",
        }
    }

    /// Reconnaît un identifiant exact de la liste ; tout autre nom est refusé.
    pub fn parse(s: &str) -> Result<Self, LensError> {
        Self::ALL.into_iter().find(|x| x.id() == s).ok_or(LensError::SymbolNotAllowed)
    }
}

/// Défaut d'une bougie reçue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultKind {
    /// Prix ou volume NaN ou infini.
    NotFinite,
    /// Prix nul ou négatif.
    NotPositive,
    NegativeVolume,
    /// Haut plus bas que l'ouverture, la clôture ou le bas (ou bas plus haut qu'eux).
    Inconsistent,
    /// Temps pas strictement croissant (désordre ou doublon).
    NotSorted,
}

/// Bougie refusée : son rang dans la liste reçue et le défaut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarFault {
    pub index: usize,
    pub kind: FaultKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LensError {
    SymbolNotAllowed,
    /// La somme des poids ne fait pas 100.
    Weights {
        sum: u16,
    },
    Bars(BarFault),
    HtfBars(BarFault),
    /// Aucune bougie clôturée dans ce qui a été reçu.
    NoClosedBar,
    /// Unité de temps nulle ou négative.
    Interval,
}

impl fmt::Display for LensError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fault = |b: &BarFault| match b.kind {
            FaultKind::NotFinite => "valeur NaN ou infinie",
            FaultKind::NotPositive => "prix nul ou négatif",
            FaultKind::NegativeVolume => "volume négatif",
            FaultKind::Inconsistent => "haut / bas incohérents",
            FaultKind::NotSorted => "bougies dans le désordre ou en double",
        };
        match self {
            Self::SymbolNotAllowed => f.write_str("Lens : symbole hors de la liste permise"),
            Self::Weights { sum } => write!(f, "Lens : la somme des poids fait {sum} au lieu de 100"),
            Self::Bars(b) => write!(f, "Lens : bougie n° {} refusée ({})", b.index, fault(b)),
            Self::HtfBars(b) => write!(f, "Lens : bougie n° {} de l'unité supérieure refusée ({})", b.index, fault(b)),
            Self::NoClosedBar => f.write_str("Lens : aucune bougie clôturée"),
            Self::Interval => f.write_str("Lens : unité de temps invalide"),
        }
    }
}

impl std::error::Error for LensError {}

/// Toutes les bougies doivent être finies, à prix positifs, cohérentes et triées strictement par temps.
pub(crate) fn check(bars: &[Bar]) -> Result<(), BarFault> {
    for (index, b) in bars.iter().enumerate() {
        let prices = [b.open, b.high, b.low, b.close];
        let kind = if prices.iter().chain([&b.volume]).any(|v| !v.is_finite()) {
            Some(FaultKind::NotFinite)
        } else if prices.iter().any(|&v| v <= 0.0) {
            Some(FaultKind::NotPositive)
        } else if b.volume < 0.0 {
            Some(FaultKind::NegativeVolume)
        } else if b.high < b.low || b.high < b.open.max(b.close) || b.low > b.open.min(b.close) {
            Some(FaultKind::Inconsistent)
        } else if index > 0 && b.time <= bars[index - 1].time {
            Some(FaultKind::NotSorted)
        } else {
            None
        };
        if let Some(kind) = kind {
            return Err(BarFault { index, kind });
        }
    }
    Ok(())
}
