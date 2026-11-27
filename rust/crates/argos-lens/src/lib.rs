//! # Argos Lens
//! Lecture du marché pour un agent : à chaque bougie **clôturée**, les indicateurs (EMA 20 / 50 / 200, RSI 14, MACD,
//! +DI / −DI / ADX 14, ATR 14 et son percentile, volume relatif, VWAP, haut / bas de la veille, biais de l'unité de
//! temps supérieure, BOS, motif de bougie), le régime, deux scores (long, short) et un setup éventuel, écrits en
//! paquets `argos-packet/v1` chaînés par empreintes SHA-384.
//!
//! Portage en Rust de l'indicateur public « AI Agent Lens » (licence MIT, roan-maverick) : voir `LICENCES-TIERS.md`.
//!
//! Calcul pur, **aucun droit** : ni réseau, ni fichier, ni horloge. L'hôte fournit les bougies et l'heure
//! (`as_of`). Aucune taille de position, aucun montant de risque, aucun solde ni secret dans les paquets.

#![forbid(unsafe_code)]

mod affichage;
mod analyse;
pub mod explain;
mod market;
mod num;
mod packet;
pub mod reglages;
mod verify;

#[cfg(test)]
mod tests;

pub use affichage::{decimales_prix, nombre_fr};
pub use analyse::{Bias, Components, Direction, Pattern, Regime, Setup, Snapshot};
pub use market::{BarFault, FaultKind, LensError, Symbol};
pub use packet::{Digest, EventKind, Packet, GENESIS, VERSION};
pub use rust_decimal::Decimal;
pub use verify::{ChainError, Verifier};

use argos_core::{Bar, Timeframe};

/// Poids des 5 facteurs du score (tendance, momentum, force, volume, structure). Leur somme doit faire 100.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Weights {
    pub trend: u8,
    pub momentum: u8,
    pub strength: u8,
    pub volume: u8,
    pub structure: u8,
}

impl Default for Weights {
    fn default() -> Self {
        Self {
            trend: 20,
            momentum: 20,
            strength: 20,
            volume: 20,
            structure: 20,
        }
    }
}

impl Weights {
    fn total(self) -> u16 {
        [self.trend, self.momentum, self.strength, self.volume, self.structure]
            .iter()
            .map(|&w| u16::from(w))
            .sum()
    }

    /// La somme des poids doit faire 100.
    fn check(self) -> Result<(), LensError> {
        match self.total() {
            100 => Ok(()),
            sum => Err(LensError::Weights { sum }),
        }
    }
}

/// Photos de toutes les bougies de `bars` (déjà clôturées, triées), pour une unité de `secs` secondes : ce que
/// l'app affiche (carte, repères) et ce que rejoue le backtest. Une photo par bougie, dans l'ordre.
pub fn photos(bars: &[Bar], secs: i64, config: &Config, htf_bars: Option<&[Bar]>) -> Result<Vec<Snapshot>, LensError> {
    config.weights.check()?;
    if secs <= 0 {
        return Err(LensError::Interval);
    }
    market::check(bars).map_err(LensError::Bars)?;
    if let Some(h) = htf_bars {
        market::check(h).map_err(LensError::HtfBars)?;
    }
    let series = analyse::Series::new(bars, secs, config.htf, htf_bars);
    Ok((0..bars.len()).map(|i| series.snapshot(i, config.weights)).collect())
}

/// Setups qui apparaissent (ou changent de sens) à chaque bougie, comme les paquets `SETUP` ; `None` sinon.
pub fn nouveaux_setups(photos: &[Snapshot]) -> Vec<Option<Setup>> {
    let mut avant = None;
    photos
        .iter()
        .map(|p| {
            let side = p.setup.map(|s| s.side);
            let nouveau = p.setup.filter(|_| side != avant);
            avant = side;
            nouveau
        })
        .collect()
}

/// Réglages de Lens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub weights: Weights,
    /// Unité de temps supérieure du biais (`None` = pas de biais). Ignorée si elle n'est pas plus grande que celle de la chart.
    pub htf: Option<Timeframe>,
    /// Un paquet `BAR_CLOSE` à chaque bougie clôturée (éteint par défaut).
    pub bar_close: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            weights: Weights::default(),
            htf: Some(Timeframe::H4),
            bar_close: false,
        }
    }
}

/// Ce que l'hôte donne à chaque appel : tout l'historique connu, triée par temps.
#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub bars: &'a [Bar],
    /// Bougies de l'unité de temps supérieure, si l'hôte les a (sinon Lens les construit à partir de `bars`).
    pub htf_bars: Option<&'a [Bar]>,
    /// Heure de l'hôte (secondes UTC) : une bougie est clôturée quand `time + durée ≤ as_of`. Les autres sont ignorées.
    pub as_of: i64,
}

/// Un flux Lens : un symbole, une unité de temps, une chaîne de paquets.
#[derive(Debug, Clone)]
pub struct Lens {
    symbol: Symbol,
    interval: Timeframe,
    config: Config,
    seq: u64,
    prev: Digest,
    last_time: Option<i64>,
    last_regime: Option<Regime>,
    last_setup: Option<Direction>,
}

impl Lens {
    /// Nouveau flux. Erreur si la somme des poids ne fait pas 100.
    pub fn new(symbol: Symbol, interval: Timeframe, config: Config) -> Result<Self, LensError> {
        config.weights.check()?;
        Ok(Self {
            symbol,
            interval,
            config,
            seq: 0,
            prev: GENESIS,
            last_time: None,
            last_regime: None,
            last_setup: None,
        })
    }

    /// Analyse les bougies clôturées pas encore vues et renvoie les paquets à émettre, dans l'ordre.
    /// En cas d'erreur, rien ne change dans le flux (on peut rappeler avec des données corrigées).
    pub fn update(&mut self, input: &Input) -> Result<Vec<Packet>, LensError> {
        market::check(input.bars).map_err(LensError::Bars)?;
        if let Some(h) = input.htf_bars {
            market::check(h).map_err(LensError::HtfBars)?;
        }
        let secs = self.interval.seconds();
        let closed = input.bars.partition_point(|b| b.time.checked_add(secs).is_some_and(|end| end <= input.as_of));
        if closed == 0 {
            return Err(LensError::NoClosedBar);
        }
        let bars = &input.bars[..closed];
        let first = self.last_time.map_or(0, |t| bars.partition_point(|b| b.time <= t));
        if first == closed {
            return Ok(Vec::new());
        }
        let series = analyse::Series::new(bars, secs, self.config.htf, input.htf_bars);
        let mut next = self.clone();
        let mut out = Vec::new();
        for (i, bar) in bars.iter().enumerate().skip(first) {
            let snap = series.snapshot(i, self.config.weights);
            if self.config.bar_close {
                out.push(next.seal(EventKind::BarClose, bar.time, None, &snap));
            }
            if snap.regime.is_some() && snap.regime != next.last_regime {
                let before = next.last_regime;
                next.last_regime = snap.regime;
                out.push(next.seal(EventKind::RegimeChange, bar.time, before, &snap));
            }
            let side = snap.setup.map(|s| s.side);
            if side.is_some() && side != next.last_setup {
                out.push(next.seal(EventKind::Setup, bar.time, None, &snap));
            }
            next.last_setup = side;
            next.last_time = Some(bar.time);
        }
        *self = next;
        Ok(out)
    }

    fn seal(&mut self, event: EventKind, bar_time: i64, prev_regime: Option<Regime>, snap: &Snapshot) -> Packet {
        self.seq += 1;
        let mut p = Packet {
            seq: self.seq,
            prev: self.prev,
            event,
            symbol: self.symbol,
            interval: self.interval,
            bar_time,
            prev_regime,
            data: snap.clone(),
            hash: GENESIS,
        };
        p.hash = p.digest();
        self.prev = p.hash;
        p
    }
}
