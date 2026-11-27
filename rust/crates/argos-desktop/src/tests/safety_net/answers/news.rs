//! 🕸 📰 📅 The news and the calendar: the Actualités with a headline in each tab, the ⚡ of the chart's crypto and
//! their card (PC) or sheet (phone), the releases of the Calendrier with their « Réel » (`UserEvent::Reel`) and the
//! reaction of the chart's crypto (`UserEvent::Minutes`), the card of a release.
//!
//! All of it is made up: the sources are « Essai », « Exemple » and « Essai FR », the links go nowhere, the numbers
//! are no release's.

use super::super::furniture::{deliver, COIN};
use super::super::scenes::{laid_out, press, tool_open, List};
use super::super::universe::id;
use super::super::windows::panel;
use super::super::{Form, Next};
use crate::*;
use argos_data::eco::{Event, News};
use argos_render::calendrier::CarteClick;
use argos_render::eclair::EclairClick;
use argos_render::text::Text;

/// Every scene of the news and of the calendar.
pub(super) fn scenes(l: &mut List) {
    news(l);
    calendar(l);
}

/// A made-up headline of `minutes` ago (a source whose name ends with « FR » writes in French).
fn headline(title: &str, source: &str, minutes: i64) -> News {
    News {
        title: title.into(),
        link: format!("https://example.invalid/news/{minutes}"),
        ts: now_s() - minutes * 60,
        source: source.into(),
        lang: if source.ends_with(" FR") { "fr" } else { "en" }.into(),
    }
}

/// 📰 News arrive for each tab of the Actualités: two name the chart's crypto (a ⚡ on the chart at the candle of
/// each), one names another crypto, one is in French.
pub(super) fn news_arrive(a: &mut App) {
    let made_up = vec![
        headline("Bitcoin holds its range (made up)", "Essai", 12),
        headline("A made-up ETF headline: the flows turn positive", "Exemple", 47),
        headline("Le bitcoin repasse un seuil (titre inventé)", "Essai FR", 95),
        headline("A made-up law on stablecoins is voted", "Essai", 130),
        headline("Ethereum network upgrade goes live (made up)", "Exemple", 215),
    ];
    deliver(a, UserEvent::News(Ok(made_up)));
}

/// ⚡ The newest ⚡ of the chart pressed: its card (PC) or its sheet (phone).
fn lightning(a: &mut App) {
    news_arrive(a);
    laid_out(a);
    let (row, wide) = (argos_render::econ::rangee_cy(), a.chart.viewport.plot_w());
    let newest = (0..wide as i32)
        .rev()
        .step_by(2)
        .find_map(|x| argos_render::econ::eclair_sous(f64::from(x), row));
    let (x, _) = newest.expect("a ⚡ on the chart");
    press(a, a.rw() + x, a.top_h() + row);
}

/// The point nearest the middle of those, tried every few points of `place` (window), where `asks` says yes.
fn within(place: Rect, asks: impl Fn(f64, f64) -> bool) -> (f64, f64) {
    const APART: f64 = 8.0;
    let each = |from: f64, length: f64| (0..(length / APART) as usize).map(move |k| from + (k as f64 + 0.5) * APART);
    let found: Vec<(f64, f64)> = each(place.y, place.h)
        .flat_map(|y| each(place.x, place.w).map(move |x| (x, y)))
        .filter(|(x, y)| asks(*x, *y))
        .collect();
    let middle = |of: fn(&(f64, f64)) -> f64| found.iter().map(of).sum::<f64>() / found.len().max(1) as f64;
    let (mx, my) = (middle(|p| p.0), middle(|p| p.1));
    let far = |(x, y): &(f64, f64)| (x - mx).hypot(y - my);
    *found.iter().min_by(|p, q| far(p).total_cmp(&far(q))).expect("such a place in the card")
}

/// ⚡ The open card (PC) or sheet (phone) of a ⚡, pressed where a press asks this.
fn lightning_pressed(a: &mut App, asks: fn(&EclairClick) -> bool) {
    let text = Text::default();
    let (card, (_, place)) = (
        a.ui.eclair.clone().expect("the card of a ⚡"),
        a.eclair_draw(&text).expect("the card of a ⚡ drawn"),
    );
    let (w, h) = a.taille_points();
    // (the sheet counts from the window's corner, the card from the chart's)
    let (dx, dy) = if card.feuille { (0.0, 0.0) } else { (a.rw(), a.top_h()) };
    let (x, y) = within(place, |x, y| asks(&card.click(&text, x - dx, y - dy, w, h)));
    press(a, x, y);
}

/// Of the presses of the open volet, the ones named after one of these: each control of the volet is pressed on the
/// data of every scene already (`traded/tool/…`), these alone show something else on the data of these scenes.
fn only(a: &mut App, named: &[&str]) -> Next {
    panel(a)
        .into_iter()
        .filter(|(name, _)| named.iter().any(|start| name.starts_with(start)))
        .collect()
}

/// 📰 Each tab of the Actualités (and, on the phone, their row slid).
fn tabs(a: &mut App) -> Next {
    only(a, &["onglet"])
}

/// 📰 In the ⚙ of the Actualités: « ⚡ sur la chart » (off: the chart loses its ⚡) and each language.
fn settings(a: &mut App) -> Next {
    only(a, &["eclairs", "langue"])
}

/// 📅 The card of each release of the Calendrier.
fn cards(a: &mut App) -> Next {
    only(a, &["carte"])
}

/// 📰 The Actualités with something in each tab and each tab pressed; their ⚙, « ⚡ sur la chart » and each language
/// pressed; the ⚡ of the chart's crypto, the card (PC) or sheet (phone) of one, the card's « Voir tout › » (the
/// Actualités on that crypto alone), and their ✕.
fn news(l: &mut List) {
    l.both_followed(
        "news/tabs",
        &[],
        |a| {
            news_arrive(a);
            tool_open(a, wl::Tool::News);
        },
        Some(tabs),
    );
    // (📱 the ⚙ is taller than the sheet and the wheel brings the rest of it: open before the presses, as its
    // button opens it, `traded/tool/News`)
    l.both_followed(
        "news/settings",
        &[],
        |a| {
            news_arrive(a);
            tool_open(a, wl::Tool::News);
            a.watch.eco.reglages_news = true;
        },
        Some(settings),
    );
    l.both("news/lightning", &[], news_arrive);
    l.both("news/lightning/card", &[id("ui", "eclair")], lightning);
    // (the phone's sheet lists every headline of the crypto: it has no « Voir tout › »)
    l.add(Form::Pc, "news/lightning/see-all", &[], |a| {
        lightning(a);
        lightning_pressed(a, |asks| matches!(asks, EclairClick::VoirTout(_)));
    });
    l.both("news/lightning/closed", &[], |a| {
        lightning(a);
        lightning_pressed(a, |asks| *asks == EclairClick::Fermer);
    });
}

/// A made-up release of the calendar, `minutes` from now: what was forecast, the number before, the « Réel ».
fn release(title: &str, currency: &str, impact: &str, minutes: i64, numbers: [&str; 3]) -> Event {
    Event {
        title: title.into(),
        country: currency.into(),
        impact: impact.into(),
        ts: now_s() + minutes * 60,
        forecast: numbers[0].into(),
        previous: numbers[1].into(),
        actual: numbers[2].into(),
    }
}

/// The release whose « Réel » and reaction the scenes read: 90 minutes ago, of high impact.
const READ: &str = "CPI m/m";

/// 📅 A calendar arrives with two past releases of high impact (the older one has its « Réel », better than
/// forecast; the other one has none yet) and two to come.
fn calendar_arrives(a: &mut App) {
    let week = vec![
        release("Non-Farm Employment Change", "USD", "High", -26 * 60, ["180K", "150K", "210K"]),
        release(READ, "USD", "High", -90, ["0.3%", "0.2%", ""]),
        release("Federal Funds Rate", "USD", "High", 4 * 60, ["4.50%", "4.50%", ""]),
        release("German ZEW", "EUR", "Low", 26 * 60, ["12.0", "10.3", ""]),
    ];
    deliver(a, UserEvent::Agenda(Ok(week)));
    a.watch.eco.semaine = true;
}

/// 🔢 The « Réel » of the release read arrives (worse than forecast: prices rose more), with the calendar it was read
/// for.
fn real_arrives(a: &mut App) {
    let mut week = a.watch.eco.agenda.clone();
    let mut read = argos_data::reel::Reel::default();
    for release in week.iter_mut().filter(|release| release.title == READ) {
        release.actual = "0.4%".into();
        read.values.insert((release.title.clone(), release.ts), release.actual.clone());
    }
    deliver(a, UserEvent::Reel(read, week));
}

/// 📈 The 1-minute candles of the chart's crypto arrive, from 30 minutes before the release read to an hour after:
/// flat, then down 1 % in 5 minutes, 1.5 % in 15 minutes, and back up to +0.5 % in an hour.
fn minutes_arrive(a: &mut App) {
    let at = now_s() - 90 * 60;
    let minute = at - at.rem_euclid(60);
    let price = |k: i64| match k {
        k if k < 0 => 220.0,
        k if k < 5 => 220.0 - 0.44 * (k + 1) as f64,
        k if k < 15 => 217.8 - 0.11 * (k - 4) as f64,
        k => 216.7 + 4.4 * (k - 14) as f64 / 45.0,
    };
    let candles = (-30..60)
        .map(|k| {
            let (open, close) = (price(k - 1), price(k));
            argos_core::Bar {
                time: minute + k * 60,
                open,
                high: open.max(close) + 0.05,
                low: open.min(close) - 0.05,
                close,
                volume: 12.0,
            }
        })
        .collect();
    deliver(a, UserEvent::Minutes(COIN.into(), candles));
}

/// 📅 The card of the release read, opened as a touch on its row does.
fn card(a: &mut App) {
    calendar_arrives(a);
    real_arrives(a);
    minutes_arrive(a);
    tool_open(a, wl::Tool::Agenda);
    a.watch.eco.carte = a.watch.eco.agenda.iter().position(|release| release.title == READ);
    laid_out(a);
}

/// 📅 The open card of a release, pressed where a press asks this.
fn card_pressed(a: &mut App, asks: fn(&CarteClick) -> bool) {
    let text = Text::default();
    let (card, (_, place)) = (
        a.cal_carte().expect("the card of a release"),
        a.cal_carte_draw(&text).expect("the card of a release drawn"),
    );
    let (x, y) = within(place, |x, y| asks(&card.click(&text, place.w, x - place.x, y - place.y)));
    press(a, x, y);
}

/// 📅 The Calendrier of a week with past releases: a « Réel » that arrives, the reaction of the chart's crypto to a
/// release and the card of each release, opened by a press on its row; the card of the release read (its numbers,
/// the reaction and its curve), its « Voir sur la chart › » and its ✕.
fn calendar(l: &mut List) {
    l.both("answer/Reel", &[], |a| {
        calendar_arrives(a);
        real_arrives(a);
        tool_open(a, wl::Tool::Agenda);
    });
    l.both_followed(
        "answer/Minutes",
        &[],
        |a| {
            calendar_arrives(a);
            real_arrives(a);
            minutes_arrive(a);
            tool_open(a, wl::Tool::Agenda);
        },
        Some(cards),
    );
    l.both("calendar/card", &[], card);
    l.both("calendar/card/on-chart", &[], |a| {
        card(a);
        card_pressed(a, |asks| matches!(asks, CarteClick::VoirChart(_)));
    });
    l.both("calendar/card/closed", &[], |a| {
        card(a);
        card_pressed(a, |asks| *asks == CarteClick::Fermer);
    });
}
