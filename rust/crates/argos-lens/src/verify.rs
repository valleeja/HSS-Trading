//! Vérificateur d'un flux de paquets : `seq` suivi de 1 en 1 depuis 1, `prev` = empreinte du paquet d'avant,
//! `hash` recalculé. À la première erreur, le flux s'arrête : tout paquet suivant est refusé.

use crate::packet::{Digest, Packet, GENESIS, VERSION};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// Un ou plusieurs paquets manquent.
    Gap { expected: u64, got: u64 },
    /// Paquet déjà reçu (ou plus ancien).
    Duplicate { seq: u64 },
    /// `prev` ne correspond pas au paquet d'avant.
    Broken { seq: u64 },
    /// Le contenu ne correspond plus à son empreinte.
    BadHash { seq: u64 },
    /// Le flux est déjà arrêté par une erreur.
    Stopped,
}

impl fmt::Display for ChainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gap { expected, got } => write!(f, "{VERSION} : trou dans la suite (attendu {expected}, reçu {got}) ; flux arrêté"),
            Self::Duplicate { seq } => write!(f, "{VERSION} : paquet {seq} en double ; flux arrêté"),
            Self::Broken { seq } => write!(f, "{VERSION} : chaîne cassée au paquet {seq} ; flux arrêté"),
            Self::BadHash { seq } => write!(f, "{VERSION} : paquet {seq} modifié (empreinte fausse) ; flux arrêté"),
            Self::Stopped => write!(f, "{VERSION} : flux arrêté après une erreur"),
        }
    }
}

impl std::error::Error for ChainError {}

#[derive(Debug, Clone)]
pub struct Verifier {
    next: u64,
    last: Digest,
    stopped: bool,
}

impl Default for Verifier {
    fn default() -> Self {
        Self::new()
    }
}

impl Verifier {
    /// Vérificateur d'un flux qui commence (premier paquet attendu : `seq` 1, `prev` à zéro).
    pub fn new() -> Self {
        Self {
            next: 1,
            last: GENESIS,
            stopped: false,
        }
    }

    /// Vérifie le paquet suivant du flux.
    pub fn check(&mut self, p: &Packet) -> Result<(), ChainError> {
        if self.stopped {
            return Err(ChainError::Stopped);
        }
        let err = if p.seq < self.next {
            Some(ChainError::Duplicate { seq: p.seq })
        } else if p.seq > self.next {
            Some(ChainError::Gap {
                expected: self.next,
                got: p.seq,
            })
        } else if p.prev != self.last {
            Some(ChainError::Broken { seq: p.seq })
        } else if p.digest() != p.hash {
            Some(ChainError::BadHash { seq: p.seq })
        } else {
            None
        };
        if let Some(e) = err {
            self.stopped = true;
            return Err(e);
        }
        self.next += 1;
        self.last = p.hash;
        Ok(())
    }
}
