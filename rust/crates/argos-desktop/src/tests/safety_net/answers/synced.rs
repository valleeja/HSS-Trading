//! 🕸 The settings changed on another device (`UserEvent::Reglages`). The app is told the names of the files of its
//! settings folder that changed elsewhere (the watcher of the desktop app, `app/synchro.js` in the page); it reads
//! them again and the screen follows, without restarting (`App::recharger`). One scene per file the app follows: the
//! file is written as the other device would have written it, then its name arrives.

use super::super::furniture::{deliver, COIN};
use super::super::scenes::{corner, laid_out, middle, point, press, tool_open, used, List};
use super::super::universe::{self, id, id_of};
use super::super::{candles, Form};
use super::chart::next_bar;
use super::{cells, indicators_page, lit, loaded, on, stream};
use crate::actions::synchro::{Derniere, SESSION};
use crate::run::Candles;
use crate::*;
use argos_render::dock::{DockPos, Module};
use argos_render::indicators::Ind;

/// Every scene of the settings changed elsewhere.
pub(super) fn scenes(l: &mut List) {
    market(l);
    screen(l);
    indicators(l);
    drawings(l);
    lists(l);
}

/// 🔄 A setting changed on another device: its file written in the app's folder, then its name told to the app.
fn synced<T: serde::Serialize>(a: &mut App, name: &str, value: &T) {
    App::write_prefs(name, value);
    deliver(a, UserEvent::Reglages(vec![name.into()]));
}

/// ⚡ The next price of the chart arrives. What the indicators draw is computed again then, not before: a legend, a
/// text, the favourites or a script changed elsewhere only show from there (`App::recharger` asks for a new
/// computation for the indicators lit and for their settings only).
fn next_price(a: &mut App) {
    let bar = next_bar(a);
    deliver(a, UserEvent::Live(stream(a), LiveEvent::Bar(bar)));
}

/// A market as a device notes it (`session.json`, a layout): the exchange, the symbol, the unit of time in seconds.
fn noted(exchange: Exchange, base: &str, interval: i64) -> Derniere {
    Derniere {
        ex: exchange.id().into(),
        base: base.into(),
        interval,
    }
}

/// 🔄 The market: the last one opened elsewhere (another market; the same in another unit of time), a layout opened
/// elsewhere (its market, its split screen, its indicators), the save of the market shown (drawings made elsewhere).
/// The candles the app then asks for arrive.
fn market(l: &mut List) {
    l.both("answer/Reglages/session", &[], |a| {
        let there = noted(Exchange::Binance, "ETH", a.chart.interval.secs());
        synced(a, SESSION, &there);
        let answer = loaded(a, Exchange::Binance, "ETH");
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
    });
    l.both("answer/Reglages/session/interval", &[], |a| {
        synced(a, SESSION, &noted(Exchange::Hyperliquid, COIN, 3600));
        let hours = Candles {
            bars: argos_core::aggregate(&candles(), 3600),
            ..loaded(a, Exchange::Hyperliquid, COIN)
        };
        deliver(a, UserEvent::Loaded(stream(a), Ok(hours)));
    });
    l.both("answer/Reglages/layouts", &[id("app", "banner"), id_of("ind", &Ind::Rsi)], |a| {
        use crate::actions::layouts::{Layout, Layouts, FICHIER};
        let here = a.photo_layout(&a.top.prefs.layout_name);
        let mut split = a.split.clone();
        split.layout = "2c".into();
        let there = Layout {
            nom: "Essai".into(),
            marche: noted(Exchange::Binance, "ETH", a.chart.interval.secs()),
            split: serde_json::to_value(&split).unwrap_or_default(),
            indicateurs: serde_json::to_value([Ind::Rsi]).unwrap_or_default(),
            ..a.photo_layout("Essai")
        };
        let all = Layouts {
            ouverte: "Essai".into(),
            liste: vec![here, there],
        };
        synced(a, FICHIER, &all);
        let answer = loaded(a, Exchange::Binance, "ETH");
        deliver(a, UserEvent::Loaded(stream(a), Ok(answer)));
        cells(a);
    });
    l.both("answer/Reglages/saved", &[], |a| {
        on(a, Exchange::Binance, COIN);
        let file = a.chart.save_path.clone().expect("the chart of a market has its save file");
        let mut there = a.to_saved();
        there.drawings = vec![Drawing::hline(180.0), Drawing::new("trend", vec![point(165, 150.0), point(190, 205.0)])];
        crate::persist::saved::write(&file, &there).expect("the save of the market is written");
        deliver(a, UserEvent::Reglages(vec![file]));
    });
}

/// 🔄 The screen: the favourite units of time of the bar, the theme, the drawings hidden, the split screen, the panel
/// of the right column, the column of modules (PC).
fn screen(l: &mut List) {
    l.both("answer/Reglages/toolbar", &[], |a| {
        let mut there = a.tools.clone();
        there.favorites = vec![60, 3600];
        synced(a, "toolbar.json", &there);
    });
    const THEME: usize = 1;
    l.both("answer/Reglages/topbar", &[id("theme", argos_render::Theme::NAMES[THEME])], |a| {
        let mut there = a.top.prefs.clone();
        there.theme = THEME;
        synced(a, "topbar.json", &there);
    });
    l.both("answer/Reglages/rail", &[], |a| {
        used(a);
        let mut there = a.rail.clone();
        there.hide_draw = true;
        synced(a, "rail.json", &there);
    });
    l.both("answer/Reglages/split", &[], |a| {
        let mut there = a.split.clone();
        there.layout = "2c".into();
        synced(a, "split.json", &there);
        cells(a);
    });
    l.both("answer/Reglages/watchlist", &[id_of("panel", &wl::Tool::News)], |a| {
        let mut there = a.watch.prefs.clone();
        (there.open, there.panel) = (true, wl::Tool::News);
        synced(a, "watchlist.json", &there);
    });
    const MODULES: [Module; 2] = [Module::Book, Module::Trades];
    l.add(Form::Pc, "answer/Reglages/dock", &MODULES.map(|m| id_of("module", &m)), |a| {
        let mut there = a.watch.dock.clone();
        (there.on, there.pos) = (true, DockPos::Right);
        for item in there.items.iter_mut() {
            item.on = MODULES.contains(&item.module);
        }
        synced(a, "dock.json", &there);
    });
}

/// 📜 A script written on another device: an average of ten candles, on the price.
const SCRIPT: &str = "//@version=6\nindicator(\"Essai\", overlay = true)\nplot(ta.sma(close, 10), \"Moyenne\")\n";

/// 🔄 The indicators: the ones lit elsewhere, a setting changed; then what only shows at the next price: a text
/// changed, the legend (unfolded, an indicator hidden), the favourites (the Indicators page counts them at once;
/// the stars of the PC's legend follow, the phone says it in the ⋯ menu of an indicator), a script written elsewhere.
fn indicators(l: &mut List) {
    const LIT: [Ind; 2] = [Ind::Rsi, Ind::Cvd];
    l.both("answer/Reglages/indicators", &LIT.map(|i| id_of("ind", &i)), |a| {
        synced(a, "indicators.json", &LIT)
    });
    l.both("answer/Reglages/indicator-settings", &[id_of("ind", &Ind::Sma)], |a| {
        use argos_render::catalogue::ReglagesParNom;
        lit(a, Ind::Sma);
        // (the line of the average: four wide)
        let width = Ind::Sma.params().1.iter().position(|p| p.key == "MA · épaisseur");
        let mut values = argos_render::indicators::values(&a.ind.set, Ind::Sma);
        values[width.expect("the width of the average is one of its settings")] = 4.0;
        let mut set = a.ind.set.clone();
        set.insert(Ind::Sma, values);
        synced(a, crate::actions::indicators::REGLAGES_NOMS, &ReglagesParNom::default().avec(&set));
    });
    l.both("answer/Reglages/indicator-texts", &[id_of("ind", &Ind::VectorZone)], |a| {
        // (Argos Vector Zone draws a Fibonacci channel on these candles once it reads the chart's own unit of time,
        // « 30m », with pivots of three candles)
        let params = Ind::VectorZone.params().1;
        let mut values = argos_render::indicators::values(&a.ind.set, Ind::VectorZone);
        for (key, value) in [("anchorTF", 2.0), ("fibPiv", 3.0)] {
            values[params.iter().position(|p| p.key == key).expect("a setting of Argos Vector Zone")] = value;
        }
        a.ind.set.insert(Ind::VectorZone, values);
        lit(a, Ind::VectorZone);
        laid_out(a);
        assert!(
            a.ind.out.vz.as_ref().is_some_and(|zones| zones.fib.is_some()),
            "a Fibonacci channel is drawn on the made-up candles"
        );
        // (the levels of the channel: three of them only, instead of thirteen)
        let mut there = a.ind.texts.clone();
        there.entry(Ind::VectorZone).or_default().insert("fibLevels".into(), "0,0.5,1".into());
        synced(a, "indicator_texts.json", &there);
        next_price(a);
    });
    let shown = |a: &mut App| {
        LIT.into_iter().for_each(|i| lit(a, i));
        laid_out(a);
    };
    l.both("answer/Reglages/indicator-legend", &LIT.map(|i| id_of("ind", &i)), move |a| {
        shown(a);
        synced(a, "indicator_legend.json", &(true, vec![Ind::Cvd]));
        next_price(a);
    });
    l.both("answer/Reglages/market-fav", &[id("ui", "fx_page")], |a| {
        indicators_page(a);
        synced(a, "market_fav.json", &LIT);
    });
    l.both("answer/Reglages/market-fav/legend", &LIT.map(|i| id_of("ind", &i)), move |a| {
        // (the legend unfolded: one line per indicator, with its star on the PC)
        a.ind.leg.0 = true;
        shown(a);
        synced(a, "market_fav.json", &LIT);
        next_price(a);
        // 📱 (the legend of the phone has no star: the ⋯ menu of an indicator says it is a favourite)
        if a.top.phone {
            a.ind.sel = Some(Ok(Ind::Rsi));
            a.legend_click(argos_render::indleg::LegClick::Plus(Ind::Rsi));
        }
    });
    l.both("answer/Reglages/scripts", &[], |a| {
        let there = [crate::types::SavedScript {
            id: "e55a1e55a1e55a1e".into(),
            name: "Essai".into(),
            src: SCRIPT.into(),
            on: true,
            ..Default::default()
        }];
        synced(a, "scripts.json", &there);
        next_price(a);
    });
}

/// 🔄 The drawings: a template of style kept elsewhere (the menu of the drawing's bar lists it), the style kept
/// elsewhere for a tool (the next drawing of that tool takes it).
fn drawings(l: &mut List) {
    use argos_render::drawbar::DbBtn;
    const MAGENTA: &str = "#ff00ff";
    let menu = ListKind::DrawBar(1, DbBtn::Template);
    let shows = [id("list", universe::list_name(&menu)), id("ui", "list")];
    for form in Form::ALL.into_iter().filter(|form| universe::has(*form, &shows[0])) {
        l.add(form, "answer/Reglages/templates", &shows, move |a| {
            used(a);
            let mut style = a.chart.drawings[1].style.clone();
            (style.color, style.width) = (MAGENTA.into(), 4.0);
            let mut there = a.templates.clone();
            there.entry("trend".into()).or_default().push(("Essai".into(), style));
            synced(a, "templates.json", &there);
            let at = corner(a);
            a.open_list(menu, at);
        });
    }
    l.both("answer/Reglages/tool-styles", &[], |a| {
        let mut style = Drawing::default_style("hline");
        (style.color, style.width, style.dash) = (MAGENTA.into(), 4.0, true);
        let mut there = a.tool_styles.clone();
        there.insert("hline".into(), style);
        synced(a, "tool_styles.json", &there);
        a.pick_tool("hline");
        let (x, y) = middle(a);
        press(a, x, y);
    });
}

/// 🔄 The accounts and the lists: the demo account (its balance), a price alert laid elsewhere, the watchlists
/// (replaced by the other device's: a symbol out of a section, two in the section « Essai »).
fn lists(l: &mut List) {
    l.both("answer/Reglages/demo-account", &[id("ui", "sim")], |a| {
        a.toggle_simulateur();
        let mut there = a.demo.clone();
        there.balance = argos_core::Money::from_f64(250_000.0);
        synced(a, "demo_account.json", &there);
    });
    l.both("answer/Reglages/alerts", &[], |a| {
        // (the other device laid a price alert: the file has it, this app does not yet)
        let here = a.alerts.clone();
        a.add_price_alert(195.0);
        (a.alerts, a.banner) = (here, None);
        deliver(a, UserEvent::Reglages(vec!["alerts.json".into()]));
    });
    l.both("answer/Reglages/watchlists", &[id_of("panel", &wl::Tool::Watchlist)], |a| {
        tool_open(a, wl::Tool::Watchlist);
        // (the panel on the first of the user's lists, « Mes favoris », instead of Hyperliquid's own)
        a.watch.prefs.list = a.lists.first().map(|list| list.id);
        a.refresh_watch();
        let there = serde_json::json!({
            "v": argos_store::listes::VERSION,
            "lists": [[1, "Mes favoris"]],
            "sections": [[1, "Essai"]],
            "items": [[1, "", "hl:SOL"], [1, "Essai", "hl:HYPE"], [1, "Essai", "hl:ETH"]],
        });
        synced(a, "watchlists.json", &there);
    });
}
