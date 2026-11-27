//! Paquet `argos-packet/v1` : ordre des champs fixe, énumérations fermées, aucun texte libre, `null` pour une donnée
//! absente, prix en chaînes décimales à nombre de décimales fixe. Chaîne d'empreintes SHA-384 : `prev` = empreinte du
//! paquet d'avant (zéros pour le premier), `hash` = SHA-384 de `argos-packet/v1` + saut de ligne + corps JSON sans `hash`.

use crate::analyse::{Regime, Snapshot};
use crate::market::Symbol;
use argos_core::Timeframe;
use rust_decimal::Decimal;
use sha2::{Digest as _, Sha384};

pub const VERSION: &str = "argos-packet/v1";

/// Empreinte SHA-384.
pub type Digest = [u8; 48];

/// `prev` du premier paquet d'un flux.
pub const GENESIS: Digest = [0; 48];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    BarClose,
    RegimeChange,
    Setup,
}

impl EventKind {
    pub fn id(self) -> &'static str {
        match self {
            Self::BarClose => "BAR_CLOSE",
            Self::RegimeChange => "REGIME_CHANGE",
            Self::Setup => "SETUP",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Packet {
    pub seq: u64,
    pub prev: Digest,
    pub event: EventKind,
    pub symbol: Symbol,
    pub interval: Timeframe,
    /// Début de la bougie clôturée (secondes UTC).
    pub bar_time: i64,
    /// Régime d'avant (seulement pour `REGIME_CHANGE`).
    pub prev_regime: Option<Regime>,
    pub data: Snapshot,
    pub hash: Digest,
}

fn hex(d: &Digest) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// Objet JSON écrit à la main, champ après champ (ordre fixe).
struct Obj(String);

impl Obj {
    fn new() -> Self {
        Self(String::from("{"))
    }
    fn raw(&mut self, key: &str, value: &str) {
        if self.0.len() > 1 {
            self.0.push(',');
        }
        self.0.push('"');
        self.0.push_str(key);
        self.0.push_str("\":");
        self.0.push_str(value);
    }
    fn str(&mut self, key: &str, value: Option<&str>) {
        match value {
            Some(v) => self.raw(key, &format!("\"{v}\"")),
            None => self.raw(key, "null"),
        }
    }
    fn dec(&mut self, key: &str, value: Option<Decimal>) {
        self.str(key, value.map(|d| d.to_string()).as_deref());
    }
    fn end(mut self) -> String {
        self.0.push('}');
        self.0
    }
}

fn snapshot_json(s: &Snapshot) -> String {
    let mut o = Obj::new();
    for (k, v) in [
        ("close", s.close),
        ("ema20", s.ema20),
        ("ema50", s.ema50),
        ("ema200", s.ema200),
        ("rsi14", s.rsi14),
        ("macd", s.macd),
        ("macd_signal", s.macd_signal),
        ("macd_hist", s.macd_hist),
        ("plus_di", s.plus_di),
        ("minus_di", s.minus_di),
        ("adx14", s.adx14),
        ("atr14", s.atr14),
        ("atr_pct", s.atr_pct),
        ("rel_volume", s.rel_volume),
        ("vwap", s.vwap),
        ("prev_day_high", s.prev_day_high),
        ("prev_day_low", s.prev_day_low),
    ] {
        o.dec(k, v);
    }
    o.str("htf_bias", s.htf_bias.map(|b| b.id()));
    o.str("structure", s.structure.map(|d| d.id()));
    o.str("bos", s.bos.map(|d| d.id()));
    o.str("pattern", s.pattern.map(|p| p.id()));
    o.str("regime", s.regime.map(|r| r.id()));
    o.raw("score_long", &s.score_long.to_string());
    o.raw("score_short", &s.score_short.to_string());
    match s.setup {
        Some(x) => {
            let mut st = Obj::new();
            st.str("side", Some(x.side.id()));
            st.dec("entry", Some(x.entry));
            st.dec("stop", Some(x.stop));
            st.dec("tp1", Some(x.tp1));
            st.dec("tp2", Some(x.tp2));
            o.raw("setup", &st.end());
        }
        None => o.raw("setup", "null"),
    }
    o.end()
}

impl Packet {
    fn body(&self) -> Obj {
        let mut o = Obj::new();
        o.str("v", Some(VERSION));
        o.raw("seq", &self.seq.to_string());
        o.str("prev", Some(&hex(&self.prev)));
        o.str("event", Some(self.event.id()));
        o.str("symbol", Some(self.symbol.id()));
        o.str("interval", Some(self.interval.id()));
        o.raw("bar_time", &self.bar_time.to_string());
        o.str("prev_regime", self.prev_regime.map(|r| r.id()));
        o.raw("data", &snapshot_json(&self.data));
        o
    }

    /// Empreinte du paquet (tout sauf `hash`), avec le préfixe de domaine.
    pub fn digest(&self) -> Digest {
        let mut h = Sha384::new();
        h.update(VERSION.as_bytes());
        h.update(b"\n");
        h.update(self.body().end().as_bytes());
        h.finalize().into()
    }

    /// Le paquet en JSON (une ligne), `hash` en dernier.
    #[cfg(test)]
    pub fn to_json(&self) -> String {
        let mut o = self.body();
        o.str("hash", Some(&hex(&self.hash)));
        o.end()
    }
}
