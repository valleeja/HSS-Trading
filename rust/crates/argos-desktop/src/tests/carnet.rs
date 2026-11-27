//! 📖 The order-book panel in the app (carnet-1, mock-up v1 approved by Jee on 22 Nov 2026): its volet, the book
//! shared with the heatmap, Delta's moves and trades, the step that follows the book read, and its menus.

use super::*;
use crate::actions::carnet::CarnetMenu;
use argos_render::carnet::{CarnetClick, Onglet};
use argos_render::indicators::Ind;

/// A book of `coin` around `mid` in 1 $ steps, 20 levels a side, every size `q` (the first bid `q0`).
fn livre(coin: &str, mid: f64, t_ms: i64, q: f64, q0: f64) -> argos_data::book::Book {
    let lv = |px: f64, sz: f64| argos_data::book::Level { px, sz, n: 1 };
    argos_data::book::Book {
        coin: coin.into(),
        time_ms: t_ms,
        bids: (0..20).map(|k| lv(mid - 0.5 - k as f64, if k == 0 { q0 } else { q })).collect(),
        asks: (0..20).map(|k| lv(mid + 0.5 + k as f64, q)).collect(),
    }
}

/// An app on Hyperliquid's BTC at about 60 000 $, the order-book volet open.
fn app_carnet() -> App {
    let mut a = app();
    for b in a.chart.raw.iter_mut().chain(a.chart.bars.iter_mut()) {
        (b.open, b.high, b.low, b.close) = (b.open + 59_900.0, b.high + 59_900.0, b.low + 59_900.0, b.close + 59_900.0);
    }
    a.chart.source = Source::Live {
        exchange: Exchange::Hyperliquid,
        base: "BTC".into(),
    };
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Carnet);
    a
}

/// 📖 Hyperliquid's live book in the volet: grouped by the price's own step (5 $ on BTC), said live, its tabs; Delta
/// counts what was placed and pulled between two books, and the trades of the chart's market as « rempli ».
#[test]
fn carnet_volet_lit_le_carnet_partage() {
    let mut a = app_carnet();
    assert!(!wl::Tool::Carnet.action(), "📖 opens its volet (it said « bientôt »)");
    assert!(a.carnet_montre());
    a.carnet_livre();
    assert_eq!(a.watch.carnet.message.as_deref(), Some("Lecture du carnet…"));
    a.feeds.book = Some(livre("BTC", 60_000.0, 1_000, 1.0, 1.0));
    a.carnet_livre();
    let v = a.watch.carnet.clone();
    assert_eq!((v.bourse.as_str(), v.source.as_str()), ("Hyperliquid", "Hyperliquid USDC"));
    assert!(v.en_direct && !v.meme_flux && v.message.is_none());
    assert_eq!((v.pas, v.mid, v.ecart, v.unite.as_str()), (5.0, 60_000.0, 1.0, "₿"));
    assert_eq!(v.bids[0], (59_995.0, 5.0), "59 999,5 … 59 995,5 in the bid bucket of 59 995");
    assert_eq!(v.asks[0], (60_005.0, 5.0));
    // the best bid grows by 3, then a trade of 1 sells into it: placed 3, filled 1
    a.feeds.book = Some(livre("BTC", 60_000.0, 2_000, 1.0, 4.0));
    a.carnet_livre();
    a.carnet_trades(&[(2_500, 59_999.5, 1.0, false)]);
    let s = &a.feeds.carnet;
    let d = s.delta.as_ref().expect("Delta counts on the panel's step");
    let m = d.fenetre(3_000, 60_000);
    let k = argos_core::carnet::case(59_999.5, 5.0, true);
    assert_eq!((m[&(true, k)].pose, m[&(true, k)].rempli), (3.0, 1.0));
    // the tabs and Delta's window are kept
    a.carnet_click(CarnetClick::Onglet(Onglet::Delta));
    a.carnet_click(CarnetClick::Fenetre(argos_render::carnet::Fenetre::M5));
    assert_eq!(
        (a.watch.prefs.carnet.onglet, a.watch.prefs.carnet.fenetre),
        (Onglet::Delta, argos_render::carnet::Fenetre::M5)
    );
}

/// 📖 The step menu offers the price's steps; a coarser book than the step chosen shows its own (no empty rows).
#[test]
fn carnet_pas_choisi_et_carnet_plus_grossier() {
    let mut a = app_carnet();
    a.feeds.book = Some(livre("BTC", 60_000.0, 1_000, 1.0, 1.0));
    a.carnet_livre();
    let g = a.carnet_menu(CarnetMenu::Pas);
    let noms: Vec<&str> = g[0].items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(noms, ["1 $", "2 $", "5 $", "10 $", "25 $", "50 $"]);
    assert!(g[0].items[2].checked);
    a.carnet_pick(CarnetMenu::Pas, 3);
    assert_eq!((a.watch.prefs.carnet.pas, a.watch.carnet.pas), (10.0, 10.0));
    // a book grouped by 25 $: 25 $ shown
    let mut b = livre("BTC", 60_000.0, 2_000, 1.0, 1.0);
    for (k, l) in b.bids.iter_mut().enumerate() {
        l.px = 59_975.0 - 25.0 * k as f64;
    }
    for (k, l) in b.asks.iter_mut().enumerate() {
        l.px = 60_025.0 + 25.0 * k as f64;
    }
    a.feeds.book = Some(b);
    a.carnet_livre();
    assert_eq!(a.watch.carnet.pas, 25.0);
}

/// 📖 The exchange menu is the heatmap's choice (one book read for both): Binance alone, then the chart's again.
#[test]
fn carnet_bourse_est_celle_de_la_heatmap() {
    let mut a = app_carnet();
    let g = a.carnet_menu(CarnetMenu::Bourse);
    assert!(g[0].items[0].checked, "the chart's exchange by default");
    a.carnet_pick(CarnetMenu::Bourse, 1);
    let v = argos_render::indicators::values(&a.ind.set, Ind::BookHeat);
    let p = argos_script::inputs::Inputs {
        params: &argos_render::bookheat::PARAMS,
        vals: &v,
    };
    assert_eq!(p.pick("source"), 1);
    assert_eq!(
        ["cBinance", "cBybit", "cOkx", "cBitunix", "cHyperliquid"].map(|k| p.on(k)),
        [true, false, false, false, false]
    );
    assert!(a.carnets_instantanes(), "Binance read as snapshots for the panel (the heatmap is off)");
    assert_eq!(a.watch.carnet.bourse, "Binance");
    assert!(a.watch.carnet.source.starts_with("Binance USDC 0/1"), "{}", a.watch.carnet.source);
    assert!(a.carnet_menu(CarnetMenu::Bourse)[0].items[1].checked);
    a.carnet_pick(CarnetMenu::Bourse, 0);
    assert_eq!(a.watch.carnet.bourse, "Hyperliquid");
    // ⚙: dollars, levels, colours, the walls' sound
    a.carnet_pick(CarnetMenu::Reglages, 0);
    a.carnet_pick(CarnetMenu::Reglages, 12);
    a.carnet_pick(CarnetMenu::Reglages, 21);
    a.carnet_pick(CarnetMenu::Reglages, 30);
    let p = &a.watch.prefs.carnet;
    assert_eq!((p.usd, p.niveaux, p.palette, p.son_murs), (true, 15, 1, true));
}

/// 📖 Closed, the panel forgets what it counted (Delta starts again when it opens).
#[test]
fn carnet_ferme_oublie() {
    let mut a = app_carnet();
    a.feeds.book = Some(livre("BTC", 60_000.0, 1_000, 1.0, 1.0));
    a.carnet_livre();
    assert!(a.feeds.carnet.delta.is_some());
    a.carnet_click(CarnetClick::Fermer);
    assert!(!a.watch.prefs.open);
    a.carnet_suivre();
    assert!(a.feeds.carnet.delta.is_none() && a.feeds.carnet.cle.is_empty());
}

/// 🧱 Argos Murs already reads Binance's book (its photos every 10 s): the panel and the heatmap take its photos
/// instead of reading the same book again, and the panel says so.
#[test]
fn carnet_reprend_les_photos_d_argos_murs() {
    use argos_data::murs::Carnet;
    dossier_a_soi("carnet_photos_murs");
    let mut a = app_carnet();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.ind.on.push(Ind::ArgosMurs);
    a.murs_refresh();
    let px = a.chart.bars.last().unwrap().close;
    a.feeds.murs.as_mut().unwrap().asked = Carnet::PERP.to_vec();
    a.murs_recu(
        "BTC".into(),
        Carnet::PERP.iter().map(|c| (*c, Ok(carnet_murs(px, 10.0, &[])))).collect(),
        None,
        None,
    );
    a.carnets_refresh();
    let s = a.feeds.carnets.as_ref().expect("the panel's books");
    assert!(s.murs_vu.is_some() && !s.busy && s.ok == 1, "Binance USDT: Argos Murs' photo, nothing asked");
    assert!(a.feeds.book.is_some());
    let v = &a.watch.carnet;
    assert!(v.meme_flux && !v.en_direct);
    assert_eq!(v.source, "Binance USDT 1/1 · photo toutes les 10 s");
    assert_eq!(v.pas, 10.0, "its photo is in 10 $ steps: 10 $ shown");
    // USDC: a book Argos Murs does not read, read here
    a.carnet_pick(CarnetMenu::Bourse, 12);
    assert!(a.feeds.carnets.as_ref().is_none_or(|s| s.murs_vu.is_none()));
}
