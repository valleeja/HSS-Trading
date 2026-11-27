//! # Argos AI, level 1: readings by rules
//! The Argos AI button of an indicator's legend explains that indicator, without a model, a key or the network. [`read`] computes
//! the indicator again on the closed bars with the settings it receives, through the same Argos Script functions as
//! the chart (so the numbers are the chart's numbers), and reads its state from the last values, in simple French:
//! « Ce qu'il montre maintenant », « Comment le lire », « Ce qui changerait la lecture », then the role of each
//! setting and the same reminder as Argos Lens. An indicator without rules gets its settings only, never another
//! indicator's reading.
//!
//! The indicators are named by their stable text id (the name of their `Ind` variant: « Rsi », « Macd »…), and the
//! settings by their key (« len », « haut »…). Pure computation: no network, no file, no clock. Never an order.

#![forbid(unsafe_code)]

mod day_levels;
mod divergence;
mod format;
mod month;
mod rules;
mod timeframes;
mod together;
mod trade;
mod volatility;

#[cfg(test)]
mod tests;

use argos_core::Bar;
pub use argos_lens::explain::{Block, Sentence, REMINDER};
pub use day_levels::{read_day_levels, DayLevels};
pub use month::{month_review, MonthReview};
pub use timeframes::{higher_timeframe, read_timeframes, timeframe_label};
pub use together::{read_together, read_together_with, Item, Overview};
pub use trade::{r_text, read_for_trade, review, trade_parts, Ecart, Part, Tone, TradeReview, Verdict, LECON_CAPITAL, TRADE_REMINDER};
pub use volatility::read_volatility;

/// How much of the indicator Argos AI reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coverage {
    /// Rules read its state (« règles »).
    Rules,
    /// No rules for it yet: its settings only (« réglages seulement »).
    SettingsOnly,
}

/// Which way a reading leans.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lean {
    /// « haussier »
    Up,
    /// « baissier »
    Down,
    /// « neutre »: the indicator gives a direction, but none now.
    Neutral,
    /// The indicator measures something without a direction (agitation, volume).
    NoDirection,
}

/// The sense of a reading by rules: its lean, a short note (« en surachat », « pic de volume »…) and its key value
/// (« 41,2 », « écart −1,30 % »), from the same computation as its text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sense {
    pub lean: Lean,
    pub note: Option<&'static str>,
    pub value: Option<String>,
}

/// One setting as received, with its role when Argos AI knows it (otherwise the screen shows the setting's own help).
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub key: String,
    pub value: f64,
    pub role: Option<&'static str>,
}

/// The reading of one indicator.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    /// The indicator's text id, as received.
    pub id: String,
    /// « Ce qu'il montre maintenant », « Comment le lire », « Ce qui changerait la lecture » (rules), or one block
    /// « Lecture des réglages seulement » (settings only).
    pub blocks: Vec<Block>,
    /// The settings received, in their order.
    pub settings: Vec<Setting>,
    /// « Analyse affichée, pas un ordre. Taille de position : Gardien PropFirm. » (the constant of Argos Lens).
    pub reminder: &'static str,
    pub coverage: Coverage,
    /// The sense of the reading, from the same computation as its text; `None` without rules or with too few bars.
    pub sense: Option<Sense>,
}

/// The text ids Argos AI reads by rules.
pub const READ_BY_RULES: [&str; 51] = [
    "Rsi",
    "Macd",
    "EmaCross",
    "EmaMulti",
    "Vwap",
    "StochRsi",
    "Mfi",
    "Stochastic",
    "Bollinger",
    "Atr",
    "RelVolume",
    "Dmi",
    "Supertrend",
    "Cci",
    "WilliamsR",
    "Obv",
    "Keltner",
    "Donchian",
    "Psar",
    "Aroon",
    "Cipher",
    "AiTrend",
    "Tbo",
    "Tvd",
    "VectorZone",
    "Smc",
    "SmcPro",
    "SpringTension",
    "PriceAction",
    "BandsOsc",
    "Bands",
    "Ichimoku",
    "Patterns",
    "SwingVwap",
    "SmartSwingVwap",
    "AnchoredVwap",
    "Heatmap",
    "Profile",
    "Sma",
    "Wma",
    "Hma",
    "Vwma",
    "Smma",
    "MaRibbon",
    "Bubbles",
    "Cvd",
    "MtfTable",
    "Tape",
    "BookHeat",
    "LiqMap",
    "Footprint",
];

/// The setting keys that matter to the reading of `id`: first the ones the rules read by name (thresholds, the
/// lengths they cite, the boxes that decide what is named) in the order of the reading, then the ones that change a
/// number or a mark it cites (in the order of the catalogue). The screen shows only these under a reading. Empty for
/// an id without rules. The first part is the table that gives the role of each setting in `Reading::settings`.
pub fn keys_read(id: &str) -> &'static [&'static str] {
    static KEYS: std::sync::OnceLock<Vec<(&'static str, Vec<&'static str>)>> = std::sync::OnceLock::new();
    let all = KEYS.get_or_init(|| {
        READ_BY_RULES
            .iter()
            .map(|id| {
                let named = rules::roles_of(id).unwrap_or(&[]).iter().map(|(k, _)| *k);
                (*id, named.chain(rules::extra_keys(id).iter().copied()).collect())
            })
            .collect()
    });
    all.iter().find(|(i, _)| *i == id).map_or(&[], |(_, k)| k.as_slice())
}

/// Reads the indicator `id` on `closed_bars` (closed bars only, oldest first) with its `settings` (key, value); a
/// missing setting takes its default, an unknown key is ignored.
/// What the screen has besides the closed bars, for the readings that need it. Every field is optional: a missing one
/// makes the reading say the data is missing, never guessed. Everything is borrowed.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScreenData<'a> {
    /// The chart's bar length, in seconds (otherwise deduced from the bars: their smallest gap).
    pub bar_secs: Option<i64>,
    /// The time of the screen's computation, in Unix seconds (the Multi-UT table's « time left »).
    pub now: Option<i64>,
    /// The real orderflow at the chart's unit, sorted by time; `Some(&[])`: none, the chart estimates from the bars.
    pub flow: Option<&'a [argos_core::orderflow::FlowBar]>,
    /// The bars of the smaller units of the AI Trend's multi-timeframe slots.
    pub lower: Option<&'a argos_script::indicators::ai_trend::LowerBars>,
    /// The series chosen as the Anchored VWAP's source when it is another indicator's output, aligned with the bars.
    pub ext_source: Option<&'a [f64]>,
    /// The tape's state the screen keeps from one update to the next (it includes the bar in progress).
    pub tape: Option<&'a argos_script::indicators::flow::Tape>,
    /// The order-book walls the screen draws, computed as for the drawing (middle price: the last close).
    pub walls: Option<&'a [Wall]>,
    /// The liquidation map the screen draws (Argos's estimate).
    pub liq: Option<LiqData<'a>>,
}

/// One order-book wall: its price, its size in $, and its side (above the price: sellers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Wall {
    pub price: f64,
    pub usd: f64,
    pub sell: bool,
}

/// What the reading of the liquidation map needs: the cumulated amounts by price of the bands still alive (price,
/// longs in $, shorts in $), lowest price first, and the map's messages.
#[derive(Debug, Clone, Copy)]
pub struct LiqData<'a> {
    pub profile: &'a [(f64, f64, f64)],
    pub notes: &'a [String],
}

/// The reading of one indicator from its closed bars and settings only: `read_with` with no screen data.
pub fn read(id: &str, closed_bars: &[Bar], settings: &[(&str, f64)]) -> Reading {
    read_with(id, closed_bars, settings, &ScreenData::default())
}

/// The reading of one indicator, with what the screen has besides the bars (`data`).
pub fn read_with(id: &str, closed_bars: &[Bar], settings: &[(&str, f64)], data: &ScreenData) -> Reading {
    let (blocks, roles, sense, coverage) = match rules::read(id, closed_bars, settings, data) {
        Some((blocks, roles, sense)) => (blocks, roles, sense, Coverage::Rules),
        None => (vec![settings_only()], &[][..], None, Coverage::SettingsOnly),
    };
    let role = |k: &str| roles.iter().find(|(key, _)| *key == k).map(|(_, r)| *r);
    Reading {
        id: id.to_string(),
        blocks,
        settings: settings
            .iter()
            .map(|(k, v)| Setting {
                key: (*k).to_string(),
                value: *v,
                role: role(k),
            })
            .collect(),
        reminder: REMINDER,
        coverage,
        sense,
    }
}

/// The only block of an indicator without rules.
fn settings_only() -> Block {
    Block {
        title: "Lecture des réglages seulement",
        sentences: vec![Sentence {
            id: "settings_only",
            text: "Argos AI n'a pas encore de règles pour lire cet indicateur : voici seulement ses réglages.".to_string(),
        }],
    }
}
