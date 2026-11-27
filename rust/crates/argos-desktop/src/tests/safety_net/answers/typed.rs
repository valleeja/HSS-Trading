//! 🕸 The keyboard and the fingers of the page (`UserEvent::Paste`, `UserEvent::SoftKey`, `UserEvent::Finger`). In a
//! browser, what is typed or pasted arrives through a hidden field of the page (the phone's keyboard, the composed
//! accents) with its keys (erase, Enter, Escape, the arrows), and the page reads the fingers. The app sends the text
//! and the keys where one is typing: a scene for each of those places, in the order the app looks at them
//! (`App::paste_text`, `App::soft_key`), then for each key that does more than erase, then for each gesture of a
//! finger (`App::touch`).
//!
//! The gestures have a clock no test can freeze (`touch::now_ms`): a quick swipe (it closes the open page), a long
//! press (the menu of what is under the finger) and the speed a list keeps after a finger leaves it are told with
//! it. No scene lifts a finger far from where it was put down or holds it for a long press: a finger taps, or stays
//! down, or leaves the chart near where it took it (the chart keeps no speed).

use super::super::furniture::deliver;
use super::super::scenes::{doc_open, finger, laid_out, middle, press, tool_open, traded, used, List, Phase};
use super::super::universe::{self, id, id_of};
use super::super::Form;
use super::{indicators_page, lit, plus_panel};
use crate::actions::pilule::PKind;
use crate::*;
use argos_render::indicators::Ind;
use argos_render::text::Text;

/// Every scene of the page's keyboard and fingers.
pub(super) fn scenes(l: &mut List) {
    pasted_text(l);
    keys(l);
    fingers(l);
}

/// ⌨ Text typed or pasted in the page arrives.
fn pasted(a: &mut App, text: &str) {
    deliver(a, UserEvent::Paste(text.into()));
}

/// ⌨ A key of the page's hidden field arrives: « Backspace », « Enter », « Escape », « ArrowUp », « ArrowDown ».
fn key(a: &mut App, name: &'static str) {
    deliver(a, UserEvent::SoftKey(name));
}

/// ⌨ `text` typed with a letter too many, which the erase key takes back.
fn typed(a: &mut App, text: &str) {
    pasted(a, &format!("{text}s"));
    key(a, "Backspace");
}

/// The middle of the part of `window` where `takes` says yes, asked every few points: the place of a field, found
/// by what a click there does to a copy of its window.
fn field(window: Rect, takes: impl Fn(f64, f64) -> bool) -> Option<(f64, f64)> {
    let mut found: Option<[f64; 4]> = None;
    let mut y = window.y + 2.0;
    while y < window.y + window.h {
        let mut x = window.x + 2.0;
        while x < window.x + window.w {
            if takes(x, y) {
                let b = found.get_or_insert([x, y, x, y]);
                *b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
            }
            x += 6.0;
        }
        y += 4.0;
    }
    found.map(|b| ((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0))
}

/// The part of `window` that is on the screen (the form of the journal is wider than a phone).
fn on_screen(a: &App, window: Rect) -> Rect {
    let (left, top) = (window.x.max(0.0), window.y.max(0.0));
    let right = (window.x + window.w).min(a.win_w());
    let bottom = (window.y + window.h).min(a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    Rect::new(left, top, right - left, bottom - top)
}

/// 🔑 The box of a key (CoinMarketCap's). It only ever receives made-up text, and no scene presses Enter in it (a
/// complete box would keep its key, `App::key_save`).
fn key_box(a: &mut App) {
    a.cmc_key_box();
}

/// 🧭 The Explorer Hyperliquid, its address field taking the keyboard as a click on it does.
fn explorer(a: &mut App) {
    tool_open(a, wl::Tool::Explorer);
    a.watch.explorer.saisie = Some(String::new());
}

/// A made-up address someone has not finished typing: the Explorer reads nothing before it is whole.
const UNFINISHED: &str = "0xe55a1e";

/// 📰 The Actualités with their made-up headlines, their search taking the keyboard as a click on it does.
fn news_search(a: &mut App) {
    super::news::news_arrive(a);
    tool_open(a, wl::Tool::News);
    a.watch.eco.recherche = Some(String::new());
}

/// 📖 The Doc of the editor, its search taking the keyboard: its field pressed, once the app has drawn the Doc (the
/// Doc finds what is typed in the page it last drew).
fn doc_search(a: &mut App) {
    doc_open(a);
    laid_out(a);
    super::super::windows::doc_search(a);
}

/// 🛟 The Support panel (PC), its subject taking the keyboard.
fn support(a: &mut App) {
    a.support_ouvrir();
    if let Some(panel) = a.ui.support.as_mut() {
        panel.focus = Some(argos_render::support::Champ::Objet);
    }
}

/// ✎ The sheet « Dessins » of the phone's pill, its search touched.
fn sheet(a: &mut App) {
    a.pfeuille_ouvrir(PKind::Dessins);
    crate::tests::toucher_feuille(a, argos_render::ecrans::feuille::Toucher::Champ);
}

/// 🔎 The Symboles volet (PC), its search taking the keyboard.
fn symbols(a: &mut App) {
    a.symboles_volet();
    if let Some(volet) = a.ui.symboles.as_mut() {
        volet.recherche = Some(String::new());
    }
}

/// ƒx The Indicators page, its search taking the keyboard.
fn indicators(a: &mut App) {
    indicators_page(a);
    if let Some(page) = a.ui.fx_page.as_mut() {
        page.recherche = Some(String::new());
    }
}

/// 😀 The emoji picker of the icon tool, its search taking the keyboard (the names are Unicode's, in English).
fn emoji(a: &mut App) {
    a.pick_tool("icon");
    a.banner = None;
    if let Some(picker) = a.ui.pop.as_mut().and_then(|pop| pop.pick.as_mut()) {
        picker.recherche = Some(String::new());
    }
}

/// 📜 The editor on a small script.
fn editor(a: &mut App) {
    a.editor_on(None, Some("//@version=6\nindicator(\"Essai\")\nplot(close)\n"));
}

/// 📓 The form of a trade the demo account noted in the journal, its notes clicked.
fn journal(a: &mut App) {
    traded(a);
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Journal);
    // (the volet laid out first: the form takes its place from the width of the window, which counts the volet)
    laid_out(a);
    a.journal_click(argos_render::journal::JClick::Edit(0));
    // (drawn once: the phone lays the form out then)
    laid_out(a);
    let Some(form) = a.ui.j_form.clone() else { return };
    let text = Text::default();
    let notes = field(on_screen(a, form.rect), |x, y| {
        let mut probe = form.clone();
        probe.click(x, y, &text);
        probe.type_char('x');
        probe.trade().notes != form.trade().notes
    });
    if let Some((x, y)) = notes {
        press(a, x, y);
    }
}

/// ⚙ The settings window of the RSI, its length clicked then erased. Gives the length to type: seven more.
fn indicator_settings(a: &mut App) -> String {
    use argos_script::inputs::Kind;
    lit(a, Ind::Rsi);
    a.open_ind_dlg(Ind::Rsi);
    let (Some(window), Some(length)) = (a.ui.ind_dlg.clone(), Ind::Rsi.params().1.iter().position(|p| matches!(p.kind, Kind::Num(..)))) else {
        return String::new();
    };
    let (now, wanted) = (window.vals[length], window.vals[length] + 7.0);
    let text = Text::default();
    let place = field(on_screen(a, window.rect), |x, y| {
        let mut probe = window.clone();
        probe.click(x, y, &text);
        (0..12).for_each(|_| probe.backspace());
        format!("{wanted}").chars().for_each(|c| probe.type_char(c));
        probe.enter();
        probe.vals[length] == wanted
    });
    if let Some((x, y)) = place {
        press(a, x, y);
        (0..format!("{now}").len()).for_each(|_| key(a, "Backspace"));
    }
    format!("{wanted}")
}

/// ✏ The settings window of a drawing on its tab « Texte », its text box clicked.
fn drawing_settings(a: &mut App) {
    used(a);
    a.open_obj_dlg(1, Some(argos_render::objdlg::Tab::Text));
    let (Some(window), Some(drawing)) = (a.ui.obj_dlg.clone(), a.chart.drawings.get(1).cloned()) else {
        return;
    };
    let place = field(on_screen(a, window.rect), |x, y| {
        let (mut probe, mut copy) = (window.clone(), drawing.clone());
        probe.click(x, y, &mut copy);
        probe.type_char('x', &mut copy);
        copy.style.text != drawing.style.text
    });
    if let Some((x, y)) = place {
        press(a, x, y);
    }
}

/// What « retiens que … » asks the assistant to keep: a note it proposes without the network.
const NOTE: &str = "retiens que les essais sont courts (fabriqué)";

/// ⌨ Each place the app types in, in the order `App::paste_text` looks at them: text arrives there, with a letter
/// too many the erase key takes back (the form of the journal and the text of a drawing have no erase key of the
/// page: `App::soft_key` does not know them).
fn pasted_text(l: &mut List) {
    let both: [(&str, String, fn(&mut App), &str); 7] = [
        ("key-box", id("session", "key_box"), key_box, "essai"),
        ("indicators", id("ui", "fx_page"), indicators, "rsi"),
        ("emoji", id("ui", "pop"), emoji, "rocket"),
        ("search", id("ui", "palette"), App::open_palette, "rsi"),
        ("assistant", id("ui", "ai_open"), App::ouvrir_panneau_ai, "Que dit la chart ? (essai)"),
        ("editor", id("ui", "editor"), editor, "// un essai\n// deux essais\n"),
        ("name-box", id("ui", "name_box"), App::guard_unlock_demander, "essai"),
    ];
    for (name, shows, open, text) in both {
        l.both(&format!("answer/Paste/{name}"), &[shows], move |a| {
            open(a);
            typed(a, text);
        });
    }
    // 🧭 (a whole address is read at once: `answer/Hl-Compte`; before that the field shows what is typed)
    l.both("answer/Paste/explorer", &[id_of("panel", &wl::Tool::Explorer)], |a| {
        explorer(a);
        pasted(a, UNFINISHED);
    });
    l.add(Form::Pc, "answer/Paste/support", &[id("ui", "support")], |a| {
        support(a);
        typed(a, "Un essai");
    });
    l.add(Form::Pc, "answer/Paste/symbols", &[id("ui", "symboles")], |a| {
        symbols(a);
        typed(a, "sol");
    });
    let pill = [id("ui", "pfeuille"), id("pill", universe::pill_name(&PKind::Dessins))];
    l.add(Form::Phone, "answer/Paste/sheet", &pill, |a| {
        sheet(a);
        typed(a, "fib");
    });
    l.both("answer/Paste/add", &[id("ui", "add")], |a| {
        plus_panel(a);
        typed(a, "doge");
    });
    l.both("answer/Paste/journal", &[id("ui", "j_form")], |a| {
        journal(a);
        pasted(a, " · une note fabriquée (essai)");
    });
    l.both("answer/Paste/indicator-settings", &[id("ui", "ind_dlg")], |a| {
        let length = indicator_settings(a);
        pasted(a, &length);
    });
    l.both("answer/Paste/drawing-settings", &[id("ui", "obj_dlg")], |a| {
        drawing_settings(a);
        pasted(a, "essai");
    });
    // 📖 (with the Doc in place of the code, what is typed goes to its search and the script stays as it is)
    l.both("answer/Paste/doc", &[id("ui", "editor")], |a| {
        doc_search(a);
        typed(a, "plot");
    });
    // 📰 the search of the Actualités: the headlines that have what is typed
    l.both("answer/Paste/news", &[id_of("panel", &wl::Tool::News)], |a| {
        news_search(a);
        pasted(a, "bitcoin");
    });
    // 🔔 the search of the alerts panel, on its history: the firings of the market typed
    l.both("answer/Paste/alerts", &[id("notifs", "alertes")], |a| {
        super::super::alerts::search(a);
        pasted(a, "sol");
    });
    // (a market no firing has: what the panel says then)
    l.both("answer/Paste/alerts-nothing", &[id("notifs", "alertes")], |a| {
        super::super::alerts::search(a);
        pasted(a, "doge");
    });
}

/// ⌨ Each key of the page that does more than erase, where it does it (`App::soft_key`): Enter and Escape leave a
/// search, Enter goes to the next field of the Support, sends a question, validates a name, adds the first symbol
/// found, applies a setting; the arrows choose a line of the search; Escape wipes the box of a key and closes what
/// asks for text.
fn keys(l: &mut List) {
    let banner = id("app", "banner");
    l.both("answer/SoftKey/key-box-escape", &[], |a| {
        key_box(a);
        typed(a, "essai");
        key(a, "Escape");
    });
    // 🧭 (the Explorer's field is no place `App::soft_key` knows: today the erase key of the page does nothing there)
    l.both("answer/SoftKey/explorer-erase", &[id_of("panel", &wl::Tool::Explorer)], |a| {
        explorer(a);
        pasted(a, UNFINISHED);
        key(a, "Backspace");
    });
    // 📰 🔔 (nor the search of the Actualités, nor the one of the alerts panel: the erase key does nothing there either)
    l.both("answer/SoftKey/news-erase", &[id_of("panel", &wl::Tool::News)], |a| {
        news_search(a);
        pasted(a, "bitcoin");
        key(a, "Backspace");
    });
    l.both("answer/SoftKey/alerts-erase", &[id("notifs", "alertes")], |a| {
        super::super::alerts::search(a);
        pasted(a, "sol");
        key(a, "Backspace");
    });
    l.add(Form::Pc, "answer/SoftKey/support-enter", &[id("ui", "support")], |a| {
        support(a);
        typed(a, "Un essai");
        key(a, "Enter");
        pasted(a, "Une description fabriquée (essai).");
    });
    l.add(Form::Pc, "answer/SoftKey/support-escape", &[id("ui", "support")], |a| {
        support(a);
        typed(a, "Un essai");
        key(a, "Escape");
    });
    l.add(Form::Phone, "answer/SoftKey/sheet-enter", &[id("ui", "pfeuille")], |a| {
        sheet(a);
        typed(a, "fib");
        key(a, "Enter");
    });
    l.add(Form::Pc, "answer/SoftKey/symbols-escape", &[id("ui", "symboles")], |a| {
        symbols(a);
        typed(a, "sol");
        key(a, "Escape");
    });
    // (in the search of a page, Enter does what Escape does: the search is left and its text is lost)
    l.both("answer/SoftKey/indicators-enter", &[id("ui", "fx_page")], |a| {
        indicators(a);
        typed(a, "rsi");
        key(a, "Enter");
    });
    l.both("answer/SoftKey/emoji-escape", &[id("ui", "pop")], |a| {
        emoji(a);
        typed(a, "rocket");
        key(a, "Escape");
    });
    // 🔎 the search: the arrows choose its second line, Enter opens what the line names, any other key closes it
    l.both("answer/SoftKey/search-arrows", &[id("ui", "palette")], |a| {
        a.open_palette();
        typed(a, "rsi");
        ["ArrowDown", "ArrowDown", "ArrowUp"].into_iter().for_each(|arrow| key(a, arrow));
    });
    l.both("answer/SoftKey/search-enter", &[banner.clone(), id_of("ind", &Ind::StochRsi)], |a| {
        a.open_palette();
        typed(a, "rsi");
        key(a, "ArrowDown");
        key(a, "Enter");
    });
    l.both("answer/SoftKey/search-escape", &[], |a| {
        a.open_palette();
        typed(a, "rsi");
        key(a, "Escape");
    });
    l.both("answer/SoftKey/assistant-enter", &[id("ui", "ai_open")], |a| {
        a.ouvrir_panneau_ai();
        pasted(a, NOTE);
        key(a, "Enter");
    });
    l.both("answer/SoftKey/assistant-escape", &[], |a| {
        a.ouvrir_panneau_ai();
        typed(a, "essai");
        key(a, "Escape");
    });
    l.both("answer/SoftKey/editor-enter", &[id("ui", "editor")], |a| {
        editor(a);
        for line in ["// un essai", "// deux essais"] {
            pasted(a, line);
            key(a, "Enter");
        }
    });
    // 📖 the search of the Doc: Enter and ↓ go to the next place found, ↑ to the one before; Escape empties the
    // search (and, once more, goes back to the code)
    l.both("answer/SoftKey/doc-enter", &[id("ui", "editor")], |a| {
        doc_search(a);
        typed(a, "plot");
        key(a, "Enter");
    });
    l.both("answer/SoftKey/doc-arrows", &[id("ui", "editor")], |a| {
        doc_search(a);
        typed(a, "plot");
        ["ArrowDown", "ArrowDown", "ArrowDown", "ArrowUp"].into_iter().for_each(|arrow| key(a, arrow));
    });
    l.both("answer/SoftKey/doc-escape", &[id("ui", "editor")], |a| {
        doc_search(a);
        typed(a, "plot");
        key(a, "Escape");
    });
    l.both("answer/SoftKey/doc-escape/twice", &[id("ui", "editor")], |a| {
        doc_search(a);
        typed(a, "plot");
        key(a, "Escape");
        key(a, "Escape");
    });
    // (the reason to unlock the day, validated: the day is unlocked, the reason noted in the journal)
    l.both("answer/SoftKey/name-box-enter", std::slice::from_ref(&banner), |a| {
        a.guard_unlock_demander();
        typed(a, "essai");
        key(a, "Enter");
    });
    l.both("answer/SoftKey/add-enter", &[id("ui", "add")], |a| {
        plus_panel(a);
        typed(a, "doge");
        key(a, "Enter");
    });
    l.both("answer/SoftKey/add-escape", &[], |a| {
        plus_panel(a);
        typed(a, "doge");
        key(a, "Escape");
    });
    // (the length of the RSI: Enter applies what is typed, Escape closes the window and takes it back)
    l.both("answer/SoftKey/indicator-settings-enter", &[id("ui", "ind_dlg")], |a| {
        let length = indicator_settings(a);
        pasted(a, &length);
        key(a, "Enter");
    });
    l.both("answer/SoftKey/indicator-settings-escape", &[], |a| {
        let length = indicator_settings(a);
        pasted(a, &length);
        key(a, "Enter");
        key(a, "Escape");
    });
}

/// 👆 A finger takes the chart at its middle and drags it `by` points to the right (it stays down).
fn dragged(a: &mut App, by: f64) {
    let (x, y) = middle(a);
    finger(a, 1, Phase::Down, (x, y));
    finger(a, 1, Phase::Moved, (x + by / 2.0, y));
    finger(a, 1, Phase::Moved, (x + by, y));
}

/// 👆 The gestures of the fingers (`App::touch`): a tap lays the point of a drawing, a finger drags the chart (and
/// stays down, leaves it, is taken back by the system), two fingers spread zoom in, a finger makes a page scroll.
fn fingers(l: &mut List) {
    l.both("answer/Finger/tap", &[], |a| {
        a.pick_tool("hline");
        a.banner = None;
        let at = middle(a);
        finger(a, 1, Phase::Down, at);
        finger(a, 1, Phase::Lifted, at);
    });
    l.both("answer/Finger/drag", &[], |a| dragged(a, 160.0));
    l.both("answer/Finger/lifted", &[], |a| {
        dragged(a, 60.0);
        let (x, y) = middle(a);
        finger(a, 1, Phase::Lifted, (x + 60.0, y));
    });
    l.both("answer/Finger/cancelled", &[], |a| {
        dragged(a, 100.0);
        let (x, y) = middle(a);
        finger(a, 1, Phase::Cancelled, (x + 100.0, y));
    });
    l.both("answer/Finger/pinch", &[], |a| {
        let (x, y) = middle(a);
        finger(a, 1, Phase::Down, (x - 60.0, y));
        finger(a, 2, Phase::Down, (x + 60.0, y));
        finger(a, 2, Phase::Moved, (x + 120.0, y));
    });
    // (the Indicators page under the finger, its fourth line « Classiques » unfolded: sixteen indicators, more
    // than the page shows; the finger goes up, the page follows it)
    l.both("answer/Finger/scroll", &[id("ui", "fx_page")], |a| {
        indicators_page(a);
        if let Some(page) = a.ui.fx_page.as_mut() {
            page.ouverte = Some(3);
        }
        let page = a.fx_rect().unwrap_or(Rect::new(0.0, 0.0, a.win_w(), a.top_h() + a.area.1 as f64));
        let (x, y) = (page.cx(), page.cy());
        finger(a, 1, Phase::Down, (x, y));
        finger(a, 1, Phase::Moved, (x, y - 60.0));
        finger(a, 1, Phase::Moved, (x, y - 120.0));
    });
}
