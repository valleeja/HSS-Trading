//! 🤖 Argos AI level 1: rules, no model and no network. It explains a Lens snapshot in simple French, in three
//! blocks (« Ce que dit la fiche », « Pourquoi », « Ce qui annulerait l'idée ») plus a fixed reminder, and lists the
//! drawings that support the text.
//!
//! Every sentence is built from fixed templates and from what Lens really computed: a missing value is said, never
//! guessed. Every drawing points to the sentence that justifies it, and only uses prices Lens computed (setup and
//! context levels) and the bar's own time. It is a displayed analysis, never an order: position size stays with the
//! PropFirm Guardian.

use crate::{decimales_prix, nombre_fr, Bias, Components, Config, Direction, Regime, Snapshot};
use rust_decimal::Decimal;

/// The reminder shown under every explanation.
pub const REMINDER: &str = "Analyse affichée, pas un ordre. Taille de position : Gardien PropFirm.";

/// Most drawings for one explanation.
pub const MAX_DRAWINGS: usize = 12;

/// Bars the setup arrow spans to the right of the setup bar.
const ARROW_BARS: i64 = 5;

/// One sentence, with a stable id the drawings refer to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    pub id: &'static str,
    pub text: String,
}

/// A titled block of sentences.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    pub title: &'static str,
    pub sentences: Vec<Sentence>,
}

/// The colour role of a drawn level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Entry,
    Stop,
    Target,
    Context,
}

/// The closed list of drawing commands. `sentence` is the id of the sentence that justifies the drawing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Horizontal level with a label.
    Level {
        price: Decimal,
        label: String,
        tone: Tone,
        sentence: &'static str,
    },
    /// Price band between two levels.
    Zone {
        top: Decimal,
        bottom: Decimal,
        label: String,
        tone: Tone,
        sentence: &'static str,
    },
    /// Vertical time marker.
    TimeMark { time: i64, label: String, sentence: &'static str },
    /// Segment with an arrow head from (time1, price1) to (time2, price2).
    Arrow {
        time1: i64,
        price1: Decimal,
        time2: i64,
        price2: Decimal,
        sentence: &'static str,
    },
    /// Text label at (time, price).
    Label {
        time: i64,
        price: Decimal,
        text: String,
        sentence: &'static str,
    },
}

impl Command {
    /// The id of the sentence that justifies this drawing.
    pub fn sentence(&self) -> &'static str {
        match self {
            Command::Level { sentence, .. }
            | Command::Zone { sentence, .. }
            | Command::TimeMark { sentence, .. }
            | Command::Arrow { sentence, .. }
            | Command::Label { sentence, .. } => sentence,
        }
    }

    /// Every price the drawing uses.
    fn prices(&self) -> Vec<Decimal> {
        match self {
            Command::Level { price, .. } | Command::Label { price, .. } => vec![*price],
            Command::Zone { top, bottom, .. } => vec![*top, *bottom],
            Command::TimeMark { .. } => vec![],
            Command::Arrow { price1, price2, .. } => vec![*price1, *price2],
        }
    }

    /// Every time the drawing uses.
    fn times(&self) -> Vec<i64> {
        match self {
            Command::TimeMark { time, .. } | Command::Label { time, .. } => vec![*time],
            Command::Arrow { time1, time2, .. } => vec![*time1, *time2],
            Command::Level { .. } | Command::Zone { .. } => vec![],
        }
    }
}

/// The explanation of one snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Explanation {
    pub blocks: Vec<Block>,
    pub reminder: &'static str,
    pub drawings: Vec<Command>,
}

/// The visible window of the chart: drawings outside it are dropped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub time_from: i64,
    pub time_to: i64,
    pub price_low: Decimal,
    pub price_high: Decimal,
}

/// A price as shown on screen (« 64 812,50 »).
fn px(d: Decimal) -> String {
    nombre_fr(d, decimales_prix(d))
}

/// An oscillator value with `dp` decimals, or « absent ».
fn val(d: Option<Decimal>, dp: u32) -> String {
    d.map_or_else(|| "absent".to_string(), |v| nombre_fr(v, dp))
}

fn tick(ok: bool) -> &'static str {
    if ok {
        "✓"
    } else {
        "✗"
    }
}

/// « au-dessus de l'EMA 50 » (`above`) or « sous l'EMA 50 »; « de le » becomes « du ».
fn side_of(above: bool, noun: &str) -> String {
    match (above, noun.strip_prefix("le ")) {
        (false, _) => format!("sous {noun}"),
        (true, Some(rest)) => format!("au-dessus du {rest}"),
        (true, None) => format!("au-dessus de {noun}"),
    }
}

fn side_name(d: Direction) -> &'static str {
    match d {
        Direction::Long => "Long",
        Direction::Short => "Short",
    }
}

/// A context level the text may cite.
#[derive(Clone, Copy)]
struct ContextLevel {
    id: &'static str,
    /// Short label on the chart.
    label: &'static str,
    /// Name with its article, inside a sentence.
    name: &'static str,
    price: Decimal,
}

/// The context levels Lens computed for this snapshot.
fn context_levels(s: &Snapshot) -> Vec<ContextLevel> {
    [
        ("ema200", "EMA 200", "l'EMA 200", s.ema200),
        ("vwap", "VWAP", "le VWAP", s.vwap),
        ("pdh", "Haut veille", "le haut de la veille", s.prev_day_high),
        ("pdl", "Bas veille", "le bas de la veille", s.prev_day_low),
    ]
    .into_iter()
    .filter_map(|(id, label, name, p)| p.map(|price| ContextLevel { id, label, name, price }))
    .collect()
}

/// « Ce que dit la fiche »: regime, scores, setup (or why there is none).
fn what_it_says(s: &Snapshot) -> Block {
    let adx = val(s.adx14, 1);
    let regime = match s.regime {
        Some(Regime::TrendUp) => {
            format!("Le marché est en tendance haussière : ADX {adx} (25 ou plus), EMA 20 au-dessus de l'EMA 50 et prix au-dessus de l'EMA 50.")
        }
        Some(Regime::TrendDown) => format!("Le marché est en tendance baissière : ADX {adx} (25 ou plus), EMA 20 sous l'EMA 50 et prix sous l'EMA 50."),
        Some(Regime::HighVolatility) => format!(
            "Le marché est très agité : l'ATR est au centile {} des 100 dernières bougies (90 ou plus).",
            val(s.atr_pct, 0)
        ),
        Some(Regime::Range) => match s.adx14 {
            Some(a) if a < Decimal::from(25) => format!("Le marché est sans tendance nette (range) : ADX {adx}, sous 25."),
            _ => format!("Le marché est sans tendance nette (range) : ADX {adx}, mais moyennes et prix pas alignés."),
        },
        None => "Régime inconnu : pas encore assez de bougies pour le calculer.".to_string(),
    };
    let mut sentences = vec![
        Sentence { id: "regime", text: regime },
        Sentence {
            id: "scores",
            text: format!("Score Long {} sur 100, score Short {} sur 100.", s.score_long, s.score_short),
        },
    ];
    sentences.push(match s.setup {
        Some(st) => Sentence {
            id: "setup",
            text: format!(
                "Setup {} sur la dernière bougie clôturée : entrée {} (la clôture), stop {} (1,5 fois l'ATR), cible 1 {} (1,5 R), cible 2 {} (3 R).",
                side_name(st.side),
                px(st.entry),
                px(st.stop),
                px(st.tp1),
                px(st.tp2)
            ),
        },
        None => Sentence {
            id: "no_setup",
            text: format!(
                "Pas de setup : il faut un score d'au moins 70, plus haut que l'autre côté (ici Long {}, Short {}).",
                s.score_long, s.score_short
            ),
        },
    });
    Block {
        title: "Ce que dit la fiche",
        sentences,
    }
}

/// « Pourquoi »: the 5 factors of the leading side, argument by argument.
fn why(s: &Snapshot, config: &Config) -> Block {
    let side = s
        .setup
        .map(|st| st.side)
        .unwrap_or(if s.score_short > s.score_long { Direction::Short } else { Direction::Long });
    let (c, long): (Components, bool) = match side {
        Direction::Long => (s.for_long, true),
        Direction::Short => (s.for_short, false),
    };
    let (up, up_f, sign) = if long {
        ("haussier", "haussière", "positif")
    } else {
        ("baissier", "baissière", "négatif")
    };
    let at = |noun: &str| side_of(long, noun);
    let q = c.quarters();
    let w = config.weights;
    let head = |name: &str, weight: u8, quarters: u16| format!("{name} (poids {weight}) : {quarters} sur 4.");
    let miss = |ok: bool, d: Option<Decimal>| if d.is_some() { tick(ok) } else { "(donnée absente)" };
    let sentences = vec![
        Sentence {
            id: "lead",
            text: format!(
                "Côté {} (score {} sur 100), facteur par facteur :",
                side_name(side),
                if long { s.score_long } else { s.score_short }
            ),
        },
        Sentence {
            id: "trend",
            text: format!(
                "{} Prix {} {}, EMA 20 {} {}, prix {} {}, biais de l'unité supérieure {up} {}.",
                head("Tendance", w.trend, q[0]),
                at("l'EMA 50"),
                miss(c.ema50, s.ema50),
                at("l'EMA 50"),
                miss(c.ema20_ema50, s.ema20.and(s.ema50)),
                at("l'EMA 200"),
                miss(c.ema200, s.ema200),
                match s.htf_bias {
                    Some(Bias::Neutral) => "✗ (neutre)",
                    Some(_) => tick(c.bias),
                    None => "(donnée absente)",
                }
            ),
        },
        Sentence {
            id: "momentum",
            text: format!(
                "{} RSI {} {} {}, histogramme du MACD {sign} {}.",
                head("Momentum", w.momentum, q[1]),
                val(s.rsi14, 1),
                at("50"),
                miss(c.rsi, s.rsi14),
                miss(c.macd, s.macd_hist)
            ),
        },
        Sentence {
            id: "strength",
            text: format!(
                "{} ADX {} (20 ou plus) {}, +DI {} {}.",
                head("Force", w.strength, q[2]),
                val(s.adx14, 1),
                miss(c.adx, s.adx14),
                at("−DI"),
                miss(c.di, s.plus_di.and(s.minus_di))
            ),
        },
        Sentence {
            id: "volume",
            text: format!(
                "{} Volume relatif {} (1 ou plus) {}, prix {} {}.",
                head("Volume", w.volume, q[3]),
                val(s.rel_volume, 2),
                miss(c.volume, s.rel_volume),
                at("le VWAP"),
                miss(c.vwap, s.vwap)
            ),
        },
        Sentence {
            id: "structure",
            text: format!(
                "{} Dernière cassure de structure {up_f} {}, BOS ou motif {up} sur cette bougie {}.",
                head("Structure", w.structure, q[4]),
                tick(c.structure),
                tick(c.trigger)
            ),
        },
    ];
    Block { title: "Pourquoi", sentences }
}

/// « Ce qui annulerait l'idée »: the stop and the context levels that would weaken the setup, or, without a setup,
/// the levels to watch.
fn invalidation(s: &Snapshot) -> Block {
    let ctx = context_levels(s);
    let mut sentences = vec![];
    match s.setup {
        Some(st) => {
            let long = st.side == Direction::Long;
            sentences.push(Sentence {
                id: "stop",
                text: format!("Une clôture au-delà du stop {} annule le setup.", px(st.stop)),
            });
            for l in &ctx {
                // only the levels between the stop and the entry: they would be crossed before the stop
                let between = if long {
                    l.price < st.entry && l.price > st.stop
                } else {
                    l.price > st.entry && l.price < st.stop
                };
                if between {
                    sentences.push(Sentence {
                        id: l.id,
                        text: format!("Repasser {} ({}) affaiblirait l'idée avant même le stop.", side_of(!long, l.name), px(l.price)),
                    });
                }
            }
            if s.regime == Some(Regime::HighVolatility) {
                sentences.push(Sentence {
                    id: "volatility",
                    text: "Volatilité très haute : le stop est plus souvent touché par une simple mèche.".to_string(),
                });
            }
        }
        None => {
            sentences.push(Sentence {
                id: "nothing",
                text: "Pas de setup, donc pas d'idée à annuler.".to_string(),
            });
            for l in &ctx {
                sentences.push(Sentence {
                    id: l.id,
                    text: format!("Niveau à surveiller : {} à {}.", l.name, px(l.price)),
                });
            }
        }
    }
    Block {
        title: "Ce qui annulerait l'idée",
        sentences,
    }
}

/// The drawings for the sentences of `blocks`: the setup (entry, stop, targets, risk and reward zones, setup bar,
/// arrow, score label) and only the context levels a sentence cites.
fn drawings(s: &Snapshot, time: i64, bar_secs: i64, blocks: &[Block]) -> Vec<Command> {
    let cited = |id: &str| blocks.iter().flat_map(|b| &b.sentences).any(|x| x.id == id);
    let mut out = vec![];
    if let Some(st) = s.setup {
        let name = side_name(st.side);
        let score = if st.side == Direction::Long { s.score_long } else { s.score_short };
        out.extend([
            Command::Level {
                price: st.entry,
                label: format!("Entrée {}", px(st.entry)),
                tone: Tone::Entry,
                sentence: "setup",
            },
            Command::Level {
                price: st.stop,
                label: format!("Stop {}", px(st.stop)),
                tone: Tone::Stop,
                sentence: "stop",
            },
            Command::Level {
                price: st.tp1,
                label: format!("Cible 1 {}", px(st.tp1)),
                tone: Tone::Target,
                sentence: "setup",
            },
            Command::Level {
                price: st.tp2,
                label: format!("Cible 2 {}", px(st.tp2)),
                tone: Tone::Target,
                sentence: "setup",
            },
            Command::Zone {
                top: st.entry.max(st.stop),
                bottom: st.entry.min(st.stop),
                label: "Risque".to_string(),
                tone: Tone::Stop,
                sentence: "stop",
            },
            Command::Zone {
                top: st.entry.max(st.tp2),
                bottom: st.entry.min(st.tp2),
                label: "Objectif".to_string(),
                tone: Tone::Target,
                sentence: "setup",
            },
            Command::TimeMark {
                time,
                label: format!("Setup {name}"),
                sentence: "setup",
            },
            Command::Arrow {
                time1: time,
                price1: st.entry,
                time2: time.saturating_add(bar_secs.saturating_mul(ARROW_BARS)),
                price2: st.tp1,
                sentence: "setup",
            },
            Command::Label {
                time,
                price: st.entry,
                text: format!("{name} {score}/100"),
                sentence: "scores",
            },
        ]);
    }
    for l in context_levels(s) {
        if cited(l.id) {
            out.push(Command::Level {
                price: l.price,
                label: format!("{} {}", l.label, px(l.price)),
                tone: Tone::Context,
                sentence: l.id,
            });
        }
    }
    out
}

/// Explains the snapshot of the bar opening at `time` (bars of `bar_secs` seconds), with the Lens settings it was
/// computed with. The drawings are checked against no window yet: see [`check`].
pub fn explain(s: &Snapshot, config: &Config, time: i64, bar_secs: i64) -> Explanation {
    let blocks = vec![what_it_says(s), why(s, config), invalidation(s)];
    let drawings = drawings(s, time, bar_secs, &blocks);
    Explanation {
        blocks,
        reminder: REMINDER,
        drawings,
    }
}

/// Keeps only the drawings that are safe to show: each one cites an existing sentence, uses only prices Lens
/// computed for this snapshot, stays inside the window, and there are at most [`MAX_DRAWINGS`].
pub fn check(e: &Explanation, s: &Snapshot, window: &Window) -> Vec<Command> {
    let ids: Vec<&str> = e.blocks.iter().flat_map(|b| &b.sentences).map(|x| x.id).collect();
    let mut known: Vec<Decimal> = context_levels(s).into_iter().map(|l| l.price).collect();
    if let Some(st) = s.setup {
        known.extend([st.entry, st.stop, st.tp1, st.tp2]);
    }
    e.drawings
        .iter()
        .filter(|c| ids.contains(&c.sentence()))
        .filter(|c| {
            c.prices()
                .iter()
                .all(|p| known.contains(p) && (window.price_low..=window.price_high).contains(p))
        })
        .filter(|c| c.times().iter().all(|t| (window.time_from..=window.time_to).contains(t)))
        .take(MAX_DRAWINGS)
        .cloned()
        .collect()
}
