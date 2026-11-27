//! 🕸 The messages of the app itself: what is said of its keys and their tests, Face ID, a language pack, the sources
//! that fail at a first start, the funding of a demo perpetual, OANDA's catalogue, the tickets of the Support, and
//! the messages that change nothing on screen.

use super::super::furniture::{deliver, COIN};
use super::super::scenes::{only, tool_open, List};
use super::super::universe::{id, id_of};
use super::super::Form;
use super::{on, plus_panel, stream};
use crate::*;
use argos_core::retours::{date_iso, Billet, Changement, Etat, Genre, Reponse, FORMAT};
use argos_render::dock::{DockPos, Module};
use argos_render::support::Onglet;

/// Every scene of the app's own messages.
pub(super) fn scenes(l: &mut List) {
    keys(l);
    face_id(l);
    language(l);
    first_start(l);
    funding(l);
    oanda(l);
    support(l);
    relay(l);
    quiet(l);
}

/// What a source says when it refuses.
fn refused() -> String {
    "refusé (essai)".into()
}

/// 🔑 The keys of the sources: what the PC's server says of the CoinMarketCap key (kept, then tested; forgotten; a
/// failure; never the key itself), and the tests of that key and of the OANDA token.
fn keys(l: &mut List) {
    let said: [(&str, fn() -> UserEvent); 7] = [
        ("CleCmc/kept", || UserEvent::CleCmc(Ok(true), true)),
        ("CleCmc/forgotten", || UserEvent::CleCmc(Ok(false), true)),
        ("CleCmc-err", || UserEvent::CleCmc(Err(refused()), true)),
        ("CmcTest", || UserEvent::CmcTest(Ok(2.5e12))),
        ("CmcTest-err", || UserEvent::CmcTest(Err(refused()))),
        ("OandaTest", || UserEvent::OandaTest(Ok(2))),
        ("OandaTest-err", || UserEvent::OandaTest(Err(refused()))),
    ];
    for (name, answer) in said {
        l.both(&format!("answer/{name}"), &[id("app", "banner")], move |a| deliver(a, answer()));
    }
}

/// 🔐 Face ID (the page's): turned on, a login that brings the public part of an account, a packet the app cannot
/// read, a failure.
fn face_id(l: &mut List) {
    let banner = [id("app", "banner")];
    l.both("answer/Passkey", &banner, |a| {
        deliver(a, UserEvent::Passkey(Ok("🔐 Face ID activé sur cet appareil (essai)".into()), false));
    });
    l.both("answer/Passkey/login", &banner, |a| {
        // (the public part of an account that is nobody's: « essai » written with the figures of a key)
        let account = serde_json::json!({
            "account": {
                "id": "e55a1e55a1e55a1e",
                "handle": "essai",
                "pub_hex": &"e55a1".repeat(13)[..64],
                "created": now_s() - 86_400,
            }
        });
        deliver(a, UserEvent::Passkey(Ok(account.to_string()), true));
    });
    l.both("answer/Passkey/unreadable", &banner, |a| deliver(a, UserEvent::Passkey(Ok("{}".into()), true)));
    l.both("answer/Passkey-err", &banner, |a| deliver(a, UserEvent::Passkey(Err(refused()), false)));
}

/// 🌐 The language of the made-up pack: one the code does not have (`i18n::DONE`), written in Latin letters.
fn pack_language() -> usize {
    argos_render::i18n::CATALOG.iter().position(|language| language.0 == "de").unwrap_or(2)
}

/// 🌐 A language pack, as its file: these phrases of the top of the window, each with a made-up translation.
fn pack(phrases: &[(&str, &str)]) -> String {
    let phrases: std::collections::BTreeMap<&str, &str> = phrases.iter().copied().collect();
    serde_json::to_string(&phrases).unwrap_or_default()
}

/// 🌐 A language pack: downloaded (the app switches to it), updated in silence while it is the language shown (a
/// phrase changed, one more phrase), not published yet, unreadable. (A pack translates whole phrases: « DÉMO » is one
/// on both forms, « Source » and « Variation 24 h » are on the PC's strip.)
fn language(l: &mut List) {
    const FIRST: [(&str, &str); 2] = [("DÉMO", "ESSAI"), ("Source", "Quelle (essai)")];
    let banner = [id("app", "banner")];
    l.both("answer/LangPack", &banner, |a| {
        deliver(a, UserEvent::LangPack(pack_language(), Ok(pack(&FIRST)), false))
    });
    l.both("answer/LangPack/update", &banner, |a| {
        deliver(a, UserEvent::LangPack(pack_language(), Ok(pack(&FIRST)), false));
        let newer = [("DÉMO", "ESSAI 2"), FIRST[1], ("Variation 24 h", "Änderung 24 h (essai)")];
        deliver(a, UserEvent::LangPack(pack_language(), Ok(pack(&newer)), true));
    });
    l.both("answer/LangPack/unreadable", &banner, |a| {
        deliver(a, UserEvent::LangPack(pack_language(), Ok("pas un pack (essai)".into()), false));
    });
    l.both("answer/LangPack-err", &banner, |a| {
        deliver(a, UserEvent::LangPack(pack_language(), Err(refused()), false))
    });
}

/// 📅 📰 🌍 A first start whose sources fail: the calendar (the local server does not have it yet: « bientôt »; or
/// another failure), the news, the macro markets. (A list already received stays on screen: its note only shows in
/// place of an empty list.)
fn first_start(l: &mut List) {
    let failures: [(&str, wl::Tool, fn() -> UserEvent); 3] = [
        ("Agenda-err", wl::Tool::Agenda, || UserEvent::Agenda(Err("HTTP 404 (essai)".into()))),
        ("Agenda-err/other", wl::Tool::Agenda, || UserEvent::Agenda(Err(refused()))),
        ("News-err", wl::Tool::News, || UserEvent::News(Err(refused()))),
    ];
    for (name, tool, answer) in failures {
        for form in Form::ALL {
            l.add(form, &format!("answer/{name}"), &[id_of("panel", &tool)], move |a| {
                tool_open(a, tool);
                deliver(a, answer());
            })
            .empty = true;
        }
    }
    l.add(Form::Pc, "answer/World-err", &[id_of("module", &Module::World)], |a| {
        only(a, &[Module::World], DockPos::Right);
        deliver(a, UserEvent::World(Err(refused())));
    })
    .empty = true;
}

/// ⚡ The real funding rate of a demo perpetual, read at the exchange of the chart: the Simulateur says it (on the
/// phone, in its « Détail », unfolded here).
fn funding(l: &mut List) {
    l.both("answer/FundingDemo", &[id("ui", "sim")], |a| {
        on(a, Exchange::Binance, COIN);
        a.toggle_simulateur();
        let phone = a.top.phone;
        if let Some(simulateur) = a.ui.sim.as_mut().filter(|_| phone) {
            simulateur.ouvertes.push(argos_render::simulateur::Rub::Detail);
        }
        // (what the app notes when it asks: the exchange, the market, and when)
        a.feeds.funding_demo_at = Some((Exchange::Binance, COIN.into(), web_time::Instant::now()));
        deliver(a, UserEvent::FundingDemo(Exchange::Binance, COIN.into(), Some(0.00045)));
    });
}

/// 🏦 OANDA's whole catalogue (read with the user's token): the app keeps it in its base, so the search of the ＋
/// panel finds its instruments.
fn oanda(l: &mut List) {
    l.both("answer/OandaCatalog", &[id("ui", "add")], |a| {
        let instrument = |symbol: &str, group: &str| argos_core::instrument::Instrument {
            exchange: "oanda".into(),
            symbol: symbol.into(),
            base: symbol.split('_').next().unwrap_or(symbol).into(),
            quote: symbol.split('_').nth(1).unwrap_or_default().into(),
            kind: argos_core::instrument::MarketKind::Spot,
            group: group.into(),
            price_decimals: 5,
            size_decimals: 0,
            growth: false,
            active: true,
        };
        let catalogue = vec![
            instrument("ESSAI_USD", "Forex"),
            instrument("ESSAI_EUR", "Forex"),
            instrument("ESSAI30_USD", "Indices"),
        ];
        deliver(a, UserEvent::OandaCatalog(Ok(catalogue)));
        plus_panel(a);
        deliver(a, UserEvent::Paste("essai".into()));
    });
}

/// 🛟 The routes of the PC's server the Support of the page asks (`actions/support.rs`): the tickets (asked, or one
/// sent), the replies seen, a reply sent, a state changed.
const LIST: &str = "/api/retours";
const SEEN: &str = "/api/retours/lu";
const REPLY: &str = "/api/retours/repondre";
const STATE: &str = "/api/retours/etat";

/// 🛟 The id of the made-up ticket of this number: the shape of a real one, « essai » written with the figures.
fn ticket_id(number: u32) -> String {
    format!("e55a1e55-a1e5-4a1e-95a1-e55a1e55a1{number:02}")
}

/// 🛟 A made-up ticket of the Support, opened a day ago: its number, kind, subject, state, and its replies (author,
/// text), one an hour.
fn ticket(number: u32, kind: Genre, subject: &str, state: Etat, replies: &[(&str, &str)]) -> Billet {
    Billet {
        format: FORMAT,
        id: ticket_id(number),
        numero: number,
        genre: kind,
        objet: subject.into(),
        description: "Une description fabriquée, sur une ligne.".into(),
        contexte: Default::default(),
        appareil: "essai".into(),
        etat: state,
        historique: vec![Changement {
            date: date_iso(now_s() - 90_000),
            etat: Etat::Ouvert,
            auteur: "admin".into(),
        }],
        reponses: replies
            .iter()
            .enumerate()
            .map(|(k, (author, said))| Reponse {
                date: date_iso(now_s() - 3_600 * (replies.len() - k) as i64),
                auteur: author.to_string(),
                texte: said.to_string(),
            })
            .collect(),
        vues: 0,
        capture: None,
    }
}

/// 🛟 The ticket in progress: Argos replied, the reply is not seen yet.
fn waiting() -> Billet {
    ticket(
        2,
        Genre::Probleme,
        "Un bouton ne répond pas (essai)",
        Etat::EnCours,
        &[("argos", "Une réponse fabriquée d'Argos.")],
    )
}

/// 🛟 The tickets as the PC's server lists them: the one in progress, and a solved one whose replies are seen.
fn tickets() -> String {
    let solved = Billet {
        vues: 1,
        ..ticket(
            1,
            Genre::Idee,
            "Une idée (essai)",
            Etat::Resolu,
            &[("argos", "C'est fait (essai)."), ("admin", "Merci (essai).")],
        )
    };
    serde_json::to_string(&[waiting(), solved]).unwrap_or_default()
}

/// 🛟 A ticket as the PC's server answers it (sent, or changed).
fn said(ticket: &Billet) -> Result<String, String> {
    Ok(serde_json::to_string(ticket).unwrap_or_default())
}

/// 🛟 The Support open on « Mes billets » (the press on the tab asks for the tickets), this ticket unfolded, the
/// tickets arrived.
fn mine(a: &mut App, unfolded: Option<u32>) {
    a.support_ouvrir();
    if let Some(panel) = a.ui.support.as_mut() {
        (panel.onglet, panel.ouvert) = (Onglet::Mes, unfolded.map(ticket_id));
    }
    deliver(a, UserEvent::Support(LIST, false, Ok(tickets())));
}

/// 💡 The Support open on « Nouveau billet », its tickets arrived, an idea typed in the card and gone (what the app
/// notes when it sends it).
fn idea_gone(a: &mut App) {
    a.support_ouvrir();
    deliver(a, UserEvent::Support(LIST, false, Ok(tickets())));
    if let Some(panel) = a.ui.support.as_mut() {
        panel.idee = "Une idée fabriquée (essai)".into();
        (panel.envoi, panel.idee_envoi) = (true, true);
    }
}

/// 🛟 The Support of the page: what the PC's server answers to its requests. Its tickets (one with a reply of Argos
/// not seen yet), a ticket just sent, an idea of the card just sent, the replies marked as seen, a reply sent, a state
/// changed, a failure (of the tickets, of the idea); and, its panel closed, the badge of its button. (PC: the phone
/// never asks, `App::support_tick`.)
fn support(l: &mut List) {
    let panel = [id("ui", "support")];
    let mut pc = |name: &str, shows: &[String], open: fn(&mut App)| {
        l.add(Form::Pc, &format!("answer/{name}"), shows, open);
    };
    pc("Support", &panel, |a| mine(a, None));
    pc("Support/sent", &panel, |a| {
        a.support_ouvrir();
        deliver(a, UserEvent::Support(LIST, false, Ok(tickets())));
        let sent = ticket(3, Genre::Question, "Une question (essai)", Etat::Ouvert, &[]);
        deliver(a, UserEvent::Support(LIST, true, said(&sent)));
    });
    pc("Support/idea", &panel, |a| {
        idea_gone(a);
        let sent = ticket(3, Genre::Idee, "Une idée fabriquée (essai)", Etat::Ouvert, &[]);
        deliver(a, UserEvent::Support(LIST, true, said(&sent)));
    });
    pc("Support/seen", &panel, |a| {
        mine(a, Some(2));
        deliver(a, UserEvent::Support(SEEN, true, Ok(r#"{"ok":true}"#.into())));
    });
    pc("Support/reply", &panel, |a| {
        mine(a, Some(2));
        let mut answered = waiting();
        let _ = answered.repondre("admin", "Une réponse fabriquée (essai).", &date_iso(now_s() - 60));
        deliver(a, UserEvent::Support(REPLY, true, said(&answered)));
    });
    pc("Support/state", &panel, |a| {
        mine(a, Some(2));
        let mut solved = waiting();
        let _ = solved.changer_etat(Etat::Resolu, "admin", &date_iso(now_s() - 60));
        deliver(a, UserEvent::Support(STATE, true, said(&solved)));
    });
    pc("Support-err", &panel, |a| {
        a.support_ouvrir();
        deliver(a, UserEvent::Support(LIST, false, Err(refused())));
    });
    pc("Support-err/idea", &panel, |a| {
        idea_gone(a);
        deliver(a, UserEvent::Support(LIST, true, Err(refused())));
    });
    pc("Support/badge", &[], |a| deliver(a, UserEvent::Support(LIST, false, Ok(tickets()))));
}

/// 🔁 The exchanges relay of the PC's server, as a page learns it is there (the market panel that is open fills
/// again) or that it is not.
fn relay(l: &mut List) {
    let markets = [id("ui", "marches")];
    for (form, sheet) in [(Form::Pc, false), (Form::Phone, true)] {
        l.add(form, "answer/RelaisBourse", &markets, move |a| {
            a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), sheet);
            deliver(a, UserEvent::RelaisBourse(true));
        });
        l.add(form, "answer/RelaisBourse-none", &markets, move |a| {
            a.marches_ouvrir(Rect::new(20.0, 86.0, 120.0, 24.0), sheet);
            deliver(a, UserEvent::RelaisBourse(false));
        });
    }
}

/// 🤫 The messages that change nothing on screen: an event a stream carries for another listener, a source without an
/// answer, a failure the app only notes for itself or whose list stays on screen, an update in silence that fails, a
/// file of settings the app does not follow, the small candles of a market left meanwhile, a text and a key of the
/// page when the app is typing nowhere.
fn quiet(l: &mut List) {
    l.both("answer/quiet", &[], |a| {
        let other = || LiveEvent::Liqs(vec![]);
        let silent = [
            UserEvent::Live(stream(a), other()),
            UserEvent::Whales(other()),
            UserEvent::Flow(other()),
            UserEvent::Orderflow(Exchange::Binance, other()),
            UserEvent::Liq(LiveEvent::Connected),
            UserEvent::FundingDemo(Exchange::Binance, COIN.into(), None),
            UserEvent::FundingHist(COIN.into(), Err(refused())),
            UserEvent::Tickers(Exchange::Binance, Err(refused())),
            UserEvent::OandaCatalog(Err(refused())),
            UserEvent::CleCmc(Err(refused()), false),
            UserEvent::LangPack(pack_language(), Err(refused()), true),
            UserEvent::Agenda(Err(refused())),
            UserEvent::News(Err(refused())),
            UserEvent::World(Err(refused())),
            UserEvent::Reglages(vec!["inconnu.json".into()]),
            UserEvent::MtfBars("un autre marché".into(), Default::default()),
            UserEvent::ScriptBars("un autre marché".into(), vec![]),
            UserEvent::Paste("essai".into()),
            UserEvent::SoftKey("Enter"),
        ];
        for message in silent {
            deliver(a, message);
        }
    });
}
