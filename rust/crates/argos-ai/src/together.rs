//! 🤖 « Ce que disent ensemble tes indicateurs » (idea 1): the readings of the indicators on the chart side by side,
//! by the sense each reading by rules gives ([`crate::Sense`], from the same computation as its text). A plain
//! account: how many lean up, down or neither, which ones disagree, what the indicators without a direction say,
//! which ones are not read. No global score, no conclusion to act on.

use crate::{read_with, Block, Coverage, Lean, ScreenData, Sentence, REMINDER};
use argos_core::Bar;

/// One indicator on the chart: its text id, the name shown on the chart (« RSI 14 »), its settings (key, value).
#[derive(Debug, Clone, Copy)]
pub struct Item<'a> {
    pub id: &'a str,
    pub name: &'a str,
    pub settings: &'a [(&'a str, f64)],
}

/// The reading of the indicators together.
#[derive(Debug, Clone, PartialEq)]
pub struct Overview {
    /// « Ce que disent ensemble tes indicateurs », « Comment le lire ».
    pub blocks: Vec<Block>,
    /// « Analyse affichée, pas un ordre. Taille de position : Gardien PropFirm. » (the constant of Argos Lens).
    pub reminder: &'static str,
}

/// `one` for 1 (or 0), `many` from 2.
pub(crate) fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n >= 2 {
        many
    } else {
        one
    }
}

/// « A, B et C ».
pub(crate) fn list(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} et {last}", rest.join(", ")),
    }
}

/// « RSI 14 (en surachat) ».
fn named(name: &str, note: Option<&str>) -> String {
    note.map_or_else(|| name.to_string(), |n| format!("{name} ({n})"))
}

fn s(id: &'static str, text: String) -> Sentence {
    Sentence { id, text }
}

/// Reads the `items` (the indicators on the chart) together on `closed_bars` (closed bars only, oldest first).
pub fn read_together(items: &[Item], closed_bars: &[Bar]) -> Overview {
    read_together_with(items, closed_bars, &[])
}

/// `read_together` with what the screen has for each item besides the bars: `data[k]` for `items[k]`. When `data` is
/// shorter, the items without an entry are read with no screen data (`ScreenData::default()`); when it is longer, the
/// extra entries are ignored. Never a panic.
pub fn read_together_with(items: &[Item], closed_bars: &[Bar], data: &[ScreenData]) -> Overview {
    let (mut up, mut down, mut neutral, mut no_dir, mut too_few, mut not_read) = (vec![], vec![], vec![], vec![], vec![], vec![]);
    for (k, it) in items.iter().enumerate() {
        let r = read_with(it.id, closed_bars, it.settings, data.get(k).unwrap_or(&ScreenData::default()));
        match (r.coverage, r.sense) {
            (Coverage::SettingsOnly, _) => not_read.push(it.name.to_string()),
            (Coverage::Rules, None) => too_few.push(it.name.to_string()),
            (Coverage::Rules, Some(x)) => match x.lean {
                Lean::Up => up.push(named(it.name, x.note)),
                Lean::Down => down.push(named(it.name, x.note)),
                Lean::Neutral => neutral.push(named(it.name, x.note)),
                Lean::NoDirection => no_dir.push(named(it.name, x.note)),
            },
        }
    }
    let with_dir = up.len() + down.len() + neutral.len();
    let mut now = vec![];
    if items.is_empty() {
        now.push(s("empty", "Aucun indicateur sur la chart : rien à lire ensemble.".into()));
    } else if with_dir == 0 {
        now.push(s(
            "no_direction",
            "Aucun des indicateurs affichés ne donne de sens à la hausse ou à la baisse.".into(),
        ));
    } else {
        let mut parts = vec![];
        if !up.is_empty() {
            parts.push(format!("{} à la hausse ({})", up.len(), list(&up)));
        }
        if !down.is_empty() {
            parts.push(format!("{} à la baisse ({})", down.len(), list(&down)));
        }
        if !neutral.is_empty() {
            parts.push(format!("{} sans sens net ({})", neutral.len(), list(&neutral)));
        }
        if with_dir == 1 {
            let (who, way) = match (up.first(), down.first(), neutral.first()) {
                (Some(n), _, _) => (n, "à la hausse"),
                (_, Some(n), _) => (n, "à la baisse"),
                (_, _, n) => (n.unwrap_or(&parts[0]), "sans sens net"),
            };
            now.push(s(
                "alone",
                format!("Un seul indicateur donne un sens : {who}, {way}. Il n'y a rien à comparer."),
            ));
        } else {
            now.push(s("count", format!("Sur {with_dir} indicateurs qui donnent un sens : {}.", parts.join(", "))));
        }
        // (with a single indicator, nothing to compare)
        if with_dir >= 2 && !up.is_empty() && !down.is_empty() {
            now.push(s(
                "disagree",
                format!(
                    "Ils ne sont pas d'accord : {} {} à la hausse, mais {} {} à la baisse.",
                    list(&up),
                    if up.len() == 1 { "penche" } else { "penchent" },
                    list(&down),
                    if down.len() == 1 { "penche" } else { "penchent" }
                ),
            ));
        } else if with_dir >= 2 && neutral.is_empty() {
            now.push(s(
                "agree",
                format!("Ils vont tous dans le même sens : à la {}.", if up.is_empty() { "baisse" } else { "hausse" }),
            ));
        }
    }
    if !no_dir.is_empty() {
        now.push(s(
            "measures",
            format!("Sans direction, ils mesurent l'agitation ou le volume : {}.", list(&no_dir)),
        ));
    }
    if !too_few.is_empty() {
        now.push(s("too_few", format!("Pas assez de bougies clôturées pour lire : {}.", list(&too_few))));
    }
    if !not_read.is_empty() {
        now.push(s("not_read", format!("Pas encore lus par Argos AI : {}.", list(&not_read))));
    }
    let how = vec![
        s(
            "how_agree",
            "Des indicateurs d'accord décrivent un marché qui va dans un sens ; des indicateurs qui se contredisent décrivent un marché indécis.".into(),
        ),
        s(
            "how_family",
            "Plusieurs indicateurs de la même famille (RSI, RSI stochastique, MFI) lisent le même élan : leur accord compte comme un seul avis.".into(),
        ),
        s(
            "how_each",
            "Le rond Argos AI de chaque indicateur explique sa lecture et ce qui la changerait.".into(),
        ),
    ];
    Overview {
        blocks: vec![
            Block {
                title: "Ce que disent ensemble tes indicateurs",
                sentences: now,
            },
            Block {
                title: "Comment le lire",
                sentences: how,
            },
        ],
        reminder: REMINDER,
    }
}
