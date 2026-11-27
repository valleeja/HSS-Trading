//! ₿ paires-btc-1, second round (23 Nov 2026): the PC server's exchanges relay in the page, the corrections seen on
//! the captures (order by volume, the phone's market sheet scrolls, the Bitfinex perpetual's badge, one size format
//! in the book, Bitfinex's real trades and its perpetual's figures), and the defects found by the safety net (n° 33
//! alerts, n° 41 drawing bar under a menu, n° 43 the DÉMO marker on tablets). N° 39 is tested in
//! `argos_render::navbar`.

use super::*;

fn inst(ex: Exchange, symbol: &str, base: &str, quote: &str) -> argos_core::Instrument {
    argos_core::Instrument {
        exchange: ex.id().into(),
        symbol: symbol.into(),
        base: base.into(),
        quote: quote.into(),
        kind: argos_core::MarketKind::Spot,
        group: "Crypto".into(),
        price_decimals: 8,
        size_decimals: 8,
        growth: false,
        active: true,
    }
}

fn quote(coin: &str, volume: f64) -> hl_watch::Quote {
    hl_watch::Quote {
        coin: coin.into(),
        mark: "1".into(),
        prev: 1.0,
        volume,
        growth: false,
        oracle: 1.0,
        oi: 0.0,
        funding: 0.0,
    }
}

/// 🔁 In a page, Bitfinex and KuCoin show only with the PC server's relay; Bitstamp (CORS open) always; the PC app
/// shows them all.
#[test]
fn bourses_du_web_avec_le_relais() {
    use crate::actions::bloquee_si as b;
    for e in [Exchange::Bitfinex, Exchange::BitfinexSpot, Exchange::Kucoin] {
        assert!(b(&e, true, false), "{e:?}: hidden without the relay");
        assert!(!b(&e, true, true), "{e:?}: shown with the relay");
        assert!(!b(&e, false, false), "{e:?}: the PC app");
    }
    assert!(!b(&Exchange::Bitstamp, true, false), "Bitstamp: CORS open, no longer hidden");
    assert!(b(&Exchange::Mexc, true, true) && b(&Exchange::Bitunix, true, true), "not carried by the relay");
}

/// 📊 « CONTRE BTC » and a search about BTC: the most traded first (ETH, XRP… before BMN2, CALCPB, CH100), with or
/// without Hyperliquid's volumes; Bitfinex stays first for one pair.
#[test]
fn contre_btc_par_volume() {
    let mut a = app();
    let mut st = Store::in_memory().unwrap();
    let spot: Vec<_> = [
        ("BMN2BTC", "BMN2"),
        ("CALCPB:BTC", "CALCPB"),
        ("CH100:BTC", "CH100"),
        ("ETHBTC", "ETH"),
        ("XRPBTC", "XRP"),
    ]
    .iter()
    .map(|(s, b)| inst(Exchange::BitfinexSpot, s, b, "BTC"))
    .collect();
    st.save_catalog(Exchange::BitfinexSpot.id(), &spot, 0).unwrap();
    st.save_catalog(Exchange::BinanceSpot.id(), &[inst(Exchange::BinanceSpot, "XRPBTC", "XRP", "BTC")], 0)
        .unwrap();
    a.store = Some(st);
    let ordre = |a: &App| a.marches_contre_btc();
    let v = ordre(&a);
    assert_eq!(&v[..3], ["bitfinex-spot:ETHBTC", "bitfinex-spot:XRPBTC", "binance-spot:XRPBTC"], "{v:?}");
    assert!(v[3..].iter().all(|id| ["BMN2", "CALCPB", "CH100"].iter().any(|t| id.contains(t))), "{v:?}");
    // Hyperliquid's 24 h volumes: XRP traded more than ETH today
    a.feeds.quotes.insert("XRP".into(), quote("XRP", 9e9));
    a.feeds.quotes.insert("ETH".into(), quote("ETH", 5e9));
    let v = ordre(&a);
    assert_eq!(&v[..3], ["bitfinex-spot:XRPBTC", "binance-spot:XRPBTC", "bitfinex-spot:ETHBTC"], "{v:?}");
    // the search results: by volume, the search's order kept for one coin
    let mut found: Vec<argos_render::wlmenu::Found> = ["bitfinex-spot:BMN2BTC", "bitfinex-spot:XRPBTC", "binance-spot:XRPBTC", "bitfinex-spot:ETHBTC"]
        .iter()
        .map(|id| argos_render::wlmenu::Found {
            id: id.to_string(),
            name: id.to_string(),
            detail: String::new(),
            group: "Crypto".into(),
            have: false,
        })
        .collect();
    a.trier_par_volume(&mut found);
    let ids: Vec<&str> = found.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(
        ids,
        ["bitfinex-spot:XRPBTC", "binance-spot:XRPBTC", "bitfinex-spot:ETHBTC", "bitfinex-spot:BMN2BTC"]
    );
}

/// 📱 iPhone (502 × 972 and 390 × 844 points): the market sheet's list scrolls by the finger, so its last row (the
/// CALCULÉE pair) comes above « ⚙ Tous les filtres », nothing over it.
#[test]
fn feuille_des_marches_defile_au_doigt() {
    use argos_render::marches::Panneau;
    for (w, h) in [(502.0, 972.0), (390.0, 844.0)] {
        let mut p = Panneau {
            rect: Rect::new(0.0, h * 0.25, w, h * 0.75),
            feuille: true,
            doigt: true,
            ..Default::default()
        };
        p.resultats = (0..14)
            .map(|k| argos_render::marches::Ligne {
                id: format!("ex{k}:XRPBTC"),
                base: "XRP".into(),
                quote: "BTC".into(),
                bourse: format!("Bourse{k}"),
                groupe: "Crypto".into(),
                ..Default::default()
            })
            .collect();
        p.resultats.push(argos_render::marches::Ligne {
            id: "ratio:binance:XRP:BTC".into(),
            base: "XRP".into(),
            quote: "BTC".into(),
            bourse: "Binance".into(),
            groupe: "Crypto".into(),
            calculee: true,
            ..Default::default()
        });
        p.texte = "xrpbtc".into();
        let n = p.resultats.len();
        let pied = p.filtres_rect();
        let derniere = p.ligne_rect(n - 1).unwrap();
        assert!(derniere.y + derniere.h > pied.y, "{w}: the CALCULÉE row starts under the foot");
        p.wheel(10_000.0);
        let derniere = p.ligne_rect(n - 1).unwrap();
        assert!(
            derniere.y + derniere.h <= pied.y,
            "{w}: scrolled, the CALCULÉE row above the foot: {derniere:?} {pied:?}"
        );
    }
    // the app: the finger on the sheet scrolls it (not the chart)
    let mut a = cell_390();
    a.ui.marches = Some(Panneau {
        rect: Rect::new(0.0, 200.0, 390.0, 600.0),
        feuille: true,
        doigt: true,
        ..Default::default()
    });
    a.mouse_moved(PhysicalPosition::new(200.0, 400.0));
    assert!(a.touch_scrolls(), "the finger scrolls the market sheet");
}

/// ₿ « xrpbtc » typed whole asks the catalogues with pairs against BTC (in a fresh browser only Bitfinex answered).
#[test]
fn xrpbtc_demande_les_catalogues() {
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.ui.marches = Some(argos_render::marches::Panneau {
        texte: "xrpbtc".into(),
        ..Default::default()
    });
    assert!(!a.ui.catalogues_btc);
    a.marches_remplir();
    assert!(a.ui.catalogues_btc, "asked once");
}

/// 🪙 The badge of a Bitfinex perpetual says its coin (XRP), not « U » (« USTF0 »).
#[test]
fn pastille_du_perp_bitfinex() {
    assert_eq!(argos_data::exchanges::bitfinex::pastille("bitfinex:XRPF0:USTF0").as_deref(), Some("XRP"));
}

/// 📖 One size format for a whole book: a book of a cheap coin never mixes « 170 500 » and « 165,000 ».
#[test]
fn tailles_du_carnet_un_seul_format() {
    use argos_render::dock::decimales_taille;
    assert_eq!(decimales_taille(170_500.0), 0, "whole numbers for the whole book");
    let f = |v: f64| argos_core::nombres::format_number_in(v, decimales_taille(170_500.0), 1);
    assert!(!f(165.0).contains(','), "165, not « 165,000 »: {}", f(165.0));
    assert_eq!(decimales_taille(12.0), 2);
    assert_eq!(decimales_taille(0.035), 4);
}

/// 🧮 ⚡ A Bitfinex chart (a pair against BTC too) follows its own real trades: the orderflow, the Trades module and
/// the bubbles read Bitfinex, not an estimate nor Hyperliquid.
#[test]
fn vrais_trades_bitfinex() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::BitfinexSpot,
        base: "XRP/BTC".into(),
    };
    let cle = a.orderflow_cle().expect("Bitfinex: real orderflow");
    assert_eq!((cle.exchange.as_str(), cle.symbole.as_str()), ("bitfinex-spot", "XRP/BTC"));
    a.orderflow_suivre_a(1_795_424_100_000);
    assert_eq!(a.tape_source(), (Exchange::BitfinexSpot, None), "the Trades module reads Bitfinex");
    let t = argos_data::exchanges::bitfinex::parse_trades("[[12,1795424162520,25.5,0.0000153],[11,1795424162000,-3,0.0000152]]", "XRP/BTC").unwrap();
    a.orderflow_evenement(Exchange::BitfinexSpot, LiveEvent::Whales(t));
    assert_eq!(a.tape_vue().len(), 2, "its trades in the Trades module");
    let f = a.feeds.orderflow.iter().find(|f| f.cle == cle).unwrap();
    assert_eq!(f.recus, (2, 2), "and in its orderflow");
    // a Binance pair against BTC keeps the estimate
    a.chart.source = Source::Live {
        exchange: Exchange::BinanceSpot,
        base: "XRP/BTC".into(),
    };
    assert!(a.orderflow_cle().is_none());
}

/// ⚡ A Bitfinex perpetual's header: mark, open interest and funding with the time left (`status/deriv`).
#[test]
fn entete_du_perp_bitfinex() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Bitfinex,
        base: "XRP".into(),
    };
    let s = argos_data::exchanges::bitfinex::parse_status(
        r#"[["tXRPF0:USTF0",1795424162000,null,0.941285,0.941205,null,59608312.47,null,1795449600000,-0.00000696,1072,null,0.0001,null,null,0.9411826,null,null,2254849.38]]"#,
    )
    .unwrap()
    .unwrap();
    a.feeds.bitfinex_statut = Some(crate::actions::carnets::StatutBitfinex {
        base: "XRP".into(),
        statut: Some(s),
        ..Default::default()
    });
    a.refresh_top();
    let l: Vec<&str> = a.top.stats.iter().map(|s| s.label.as_str()).collect();
    assert!(
        l.contains(&"Mark") && l.contains(&"Intérêt ouvert") && l.contains(&"Funding / compte à rebours"),
        "{l:?}"
    );
    let f = a.top.stats.iter().find(|s| s.label.starts_with("Funding")).unwrap();
    assert!(f.value.starts_with("0.0100 % · "), "{}", f.value);
}

/// 🔔 N° 33: an alert made on another exchange than Hyperliquid reads « SOL/USDT · Binance Spot », « XRP/BTC ·
/// Bitfinex Spot » (it keeps the chart's id, not the catalogue's).
#[test]
fn alertes_de_toutes_les_bourses() {
    use crate::actions::alerte_marche_de_la_chart as m;
    let t = |b: &str, q: &str, x: &str| Some((b.to_string(), q.to_string(), x.to_string()));
    assert_eq!(m("binance-spot:SOL"), t("SOL", "USDT", "Binance Spot"));
    assert_eq!(m("binance:SOL"), t("SOL", "USDT", "Binance Perp"));
    assert_eq!(m("bitfinex-spot:XRP/BTC"), t("XRP", "BTC", "Bitfinex Spot"));
    assert_eq!(m("bitfinex:XRP"), t("XRP", "USDT", "Bitfinex Perp"));
    assert_eq!(m("okx:ETH"), t("ETH", "USDT", "OKX Perp"));
    assert_eq!(m("kraken:BTC"), t("BTC", "USD", "Kraken Spot"));
    assert_eq!(m("bybit:SOLUSDT"), None, "a catalogue symbol: read as before");
    // the panel itself
    dossier_a_soi("filet-2-alertes");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::BinanceSpot,
        base: "SOL".into(),
    };
    a.add_alert(
        "BINANCE:SOLUSDT",
        argos_core::alerts::Kind::Price {
            price: 1.0,
            dir: argos_core::alerts::Dir::Both,
        },
        None,
        None,
        "",
    );
    a.chart.source = Source::Live {
        exchange: Exchange::BitfinexSpot,
        base: "XRP/BTC".into(),
    };
    a.add_alert(
        "BITFINEX:XRPBTC",
        argos_core::alerts::Kind::Price {
            price: 1.0,
            dir: argos_core::alerts::Dir::Both,
        },
        None,
        None,
        "",
    );
    let d = a.alertes_donnees();
    let mut v: Vec<(String, String, String)> = d.actives.iter().map(|l| (l.base.clone(), l.quote.clone(), l.bourse.clone())).collect();
    v.sort();
    assert_eq!(
        v,
        [
            ("SOL".to_string(), "USDT".to_string(), "Binance Spot".to_string()),
            ("XRP".to_string(), "BTC".to_string(), "Bitfinex Spot".to_string())
        ]
    );
}

/// 📱 N° 41: a menu opened over the bar of a selected drawing takes the touch first (touching its row never locks
/// nor deletes the drawing under it).
#[test]
fn menu_ouvert_avant_la_barre_du_dessin() {
    use argos_render::drawbar::DbBtn;
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.selected = Some(0);
    let (_, bar) = a.drawbar().expect("its bar");
    let (rw, th) = (a.rw(), a.top_h());
    let lock = bar.at(DbBtn::Lock).unwrap();
    let (wx, wy) = (lock.cx() + rw, lock.cy() + th);
    // a menu over the bar (its rows cover the padlock)
    a.open_list(ListKind::Intervals, Rect::new(wx - 20.0, wy - 60.0, 40.0, 24.0));
    let menu = a.ui.list.as_ref().map(|(_, m)| m.rect).unwrap();
    assert!(menu.contains(wx, wy), "the menu covers the padlock: {menu:?} ({wx}, {wy})");
    toucher(&mut a, wx, wy);
    assert!(
        a.chart.drawings.len() == 1 && !a.chart.drawings[0].locked(),
        "the drawing under the menu is untouched"
    );
}

/// 🟠 N° 43: the DÉMO marker is always there: PC 1440 × 900 (first row, as today), tablet lying 1092 × 734 and
/// standing 819 × 1008 by the finger, phone 502 × 972; never over the legends nor out of the window.
#[test]
fn repere_demo_toujours_visible() {
    use argos_render::formfactor::{choose, Screen};
    for (w, h, coarse) in [(1440.0, 900.0, false), (1092.0, 734.0, true), (819.0, 1008.0, true), (502.0, 972.0, true)] {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        // (a real chart's long title, as in the browser)
        a.title = "XRPBTC Spot · 4h · Bitfinex Spot".into();
        a.ui.form = choose(&Screen {
            w,
            h,
            scale: 2.0,
            coarse,
            ..Default::default()
        });
        a.placer(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        a.draw_frame();
        if a.top.phone {
            assert!(a.demo_en_bas() && a.demo_pastille().0.starts_with("DÉMO"), "{w} × {h}: the phone's pill");
            continue;
        }
        if a.demo_en_bas() {
            assert!(a.chart_demo().is_some(), "{w} × {h}: the pill of the bar under the chart");
            continue;
        }
        let text = argos_render::text::Text::default();
        let (r, _) = a.bandeau_place();
        assert!(r.w > 0.0 && a.bandeau_draw(&text).is_some(), "{w} × {h}: the DÉMO band is drawn");
        assert!(r.x >= 0.0 && r.x + r.w <= w && r.y >= 0.0 && r.y + r.h <= h, "{w} × {h}: inside {r:?}");
        let croise = |q: Rect| r.x < q.x + q.w && q.x < r.x + r.w && r.y < q.y + q.h && q.y < r.y + r.h;
        for (n, q) in a.legendes() {
            assert!(!croise(q), "{w} × {h}: the band {r:?} over « {n} » {q:?}");
        }
        // nor over the legend's own lines (the chart's title is longer than the legend's box)
        let (rw, th) = (a.rw(), a.top_h());
        for q in argos_render::legende_zones(&a.to_chart()) {
            let q = Rect::new(rw + q.x, th + q.y, q.w, q.h);
            assert!(!croise(q), "{w} × {h}: the band {r:?} over a legend line {q:?}");
        }
        for t in argos_render::indicators::tableaux() {
            assert!(!croise(Rect::new(rw + t.x, th + t.y, t.w, t.h)), "{w} × {h}: the band {r:?} over a table {t:?}");
        }
        // the title as drawn (bold, 16): never under the band
        let nom = a.to_chart().title.unwrap_or_default();
        let tw = argos_render::text::Text::graisse(argos_render::text::Graisse::Bold).width(&nom, 16.0) as f64;
        assert!(
            !croise(Rect::new(rw + 12.0, th, tw + 26.0, 30.0)),
            "{w} × {h}: the band {r:?} over the title ({tw})"
        );
        if w >= 1440.0 {
            assert!(r.y + r.h <= a.top_h(), "PC: in the first row, as today ({r:?})");
        }
    }
}

/// 🟠 N° 43, tablet standing 819 × 1008 by the finger with a wide panel open (~150 points of chart left): the DÉMO
/// marker is still drawn (its short form if need be), inside the chart, over neither the legends nor a table.
#[test]
fn repere_demo_tablette_debout_volet_large() {
    use argos_render::formfactor::{choose, Screen};
    let (w, h) = (819.0, 1008.0);
    for outil in [
        wl::Tool::Alertes,
        wl::Tool::Indicators,
        wl::Tool::Reglages,
        wl::Tool::Support,
        wl::Tool::Market,
        wl::Tool::Watchlist,
    ] {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.title = "XRPBTC Spot · 4h · Bitfinex Spot".into();
        a.ui.form = choose(&Screen {
            w: 1024.0,
            h: 1260.0,
            scale: 2.0,
            coarse: true,
            ..Default::default()
        });
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, outil);
        a.placer(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        a.draw_frame();
        assert!(!a.demo_en_bas(), "{outil:?}: the band, at the top");
        let text = argos_render::text::Text::default();
        let (r, _) = a.bandeau_place();
        assert!(r.w > 0.0 && a.bandeau_draw(&text).is_some(), "{outil:?}: the DÉMO band is drawn");
        let (rw, th) = (a.rw(), a.top_h());
        let fin = rw + a.chart.viewport.plot_w();
        assert!(r.x >= rw && r.x + r.w <= fin && r.y >= th, "{outil:?}: inside the chart {r:?} ({rw}..{fin})");
        let croise = |q: Rect| r.x < q.x + q.w && q.x < r.x + r.w && r.y < q.y + q.h && q.y < r.y + r.h;
        for (n, q) in a.legendes() {
            assert!(!croise(q), "{outil:?}: the band {r:?} over « {n} » {q:?}");
        }
        for t in argos_render::indicators::tableaux() {
            assert!(!croise(Rect::new(rw + t.x, th + t.y, t.w, t.h)), "{outil:?}: the band {r:?} over a table {t:?}");
        }
    }
}

/// ⌨ 📱 The phone's keyboard (the page's hidden field: its text, its erase key and its Enter) in every search: the
/// erase key erases in « Chercher une alerte », « Chercher dans les nouvelles », the Explorer's address and the
/// Indicators panel (n° 32, 15: it did nothing), the text reaches the Actualités' search (n° 18), and Enter keeps
/// the search of the Indicators page, the Symbols panel and the emojis (it emptied them). The cause, common to all:
/// the page's keys had their own list of places, written apart from the computer keyboard's (`Focus`).
#[test]
fn clavier_du_cell_dans_chaque_recherche() {
    let page = |a: &mut App, t: &str| a.on_user_event(UserEvent::Paste(t.into()));
    let key = |a: &mut App, k: &'static str| a.on_user_event(UserEvent::SoftKey(k));
    // 🔔 the alerts panel's search
    let mut a = app();
    a.notifs_pop(Rect::new(270.0, 10.0, 30.0, 30.0));
    if let Some(n) = a.ui.notifs.as_mut() {
        n.fil = false;
        n.alertes.recherche = Some(String::new());
    }
    assert!(a.open_now().alertes_recherche);
    page(&mut a, "sol");
    key(&mut a, "Backspace");
    assert_eq!(a.ui.notifs.as_ref().unwrap().alertes.recherche.as_deref(), Some("so"), "alerts: erased");
    // 📰 the Actualités' search: the text arrives, the erase key erases, Enter keeps it
    let mut a = app();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::News);
    a.watch.eco.recherche = Some(String::new());
    page(&mut a, "bitcoin");
    assert_eq!(a.watch.eco.recherche.as_deref(), Some("bitcoin"), "news: the text arrives");
    key(&mut a, "Backspace");
    key(&mut a, "Enter");
    assert_eq!(a.watch.eco.recherche.as_deref(), Some("bitcoi"), "news: erased, kept by Enter");
    // 🧭 the Explorer's address
    let mut a = app();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Explorer);
    a.watch.explorer.saisie = Some(String::new());
    page(&mut a, "0xe55a1e");
    key(&mut a, "Backspace");
    assert_eq!(a.watch.explorer.saisie.as_deref(), Some("0xe55a1"), "Explorer: erased");
    // 🔎 the Indicators panel's search
    let mut a = app();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Indicators);
    a.watch.panels.search = Some(String::new());
    page(&mut a, "rsi");
    key(&mut a, "Backspace");
    assert_eq!(a.watch.panels.search.as_deref(), Some("rs"), "Indicators panel: erased");
    // ƒx the Indicators page, 🔎 the Symbols panel, 😀 the emojis: Enter keeps what is typed
    let mut a = app();
    a.fx_volet();
    a.ui.fx_page.as_mut().unwrap().recherche = Some(String::new());
    page(&mut a, "rsi");
    key(&mut a, "Enter");
    assert_eq!(a.ui.fx_page.as_ref().unwrap().recherche.as_deref(), Some("rsi"), "Indicators page: kept");
    let mut a = app();
    a.symboles_volet();
    a.ui.symboles.as_mut().unwrap().recherche = Some(String::new());
    page(&mut a, "sol");
    key(&mut a, "Enter");
    assert_eq!(a.ui.symboles.as_ref().unwrap().recherche.as_deref(), Some("sol"), "Symbols: kept");
    let mut a = app();
    a.pick_tool("icon");
    a.ui.pop.as_mut().and_then(|p| p.pick.as_mut()).unwrap().recherche = Some(String::new());
    page(&mut a, "rocket");
    key(&mut a, "Enter");
    assert_eq!(a.emoji_pick().unwrap().recherche.as_deref(), Some("rocket"), "emojis: kept");
    // (Escape still leaves a search, as the computer's Escape does)
    key(&mut a, "Escape");
    assert_eq!(a.emoji_pick().unwrap().recherche, None);
}

/// 🧩 N° 19 (PC 1440 × 900): a panel open and the column of modules on the left: the column stays, image after image
/// (it was there one image, gone the next: its own width was counted against the room of the next image).
#[test]
fn colonne_de_modules_a_gauche_ne_clignote_pas() {
    use argos_render::dock::{DockPos, Module};
    for outil in [wl::Tool::Watchlist, wl::Tool::Indicators, wl::Tool::Alertes] {
        let mut a = app();
        (a.watch.dock.on, a.watch.dock.pos) = (true, DockPos::Left);
        for item in a.watch.dock.items.iter_mut() {
            item.on = item.module == Module::Book;
        }
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, outil);
        let vus: Vec<Option<Rect>> = (0..4)
            .map(|_| {
                a.placer(winit::dpi::PhysicalSize::new(1440, 900));
                a.dock.rect
            })
            .collect();
        assert!(vus.windows(2).all(|w| w[0] == w[1]), "{outil:?}: the column blinks {vus:?}");
        assert!(vus[0].is_some(), "{outil:?}: the column is there");
    }
}

/// 👆 The texts by the finger (phone, tablet) name no key nor click: the failed loading's banner (« F5 pour
/// réessayer » stays on the PC), and the Replay's « Touche la bougie de départ ».
#[test]
fn textes_au_doigt_sans_touche() {
    for doigt in [false, true] {
        let mut a = app();
        argos_render::actions::set_tactile(doigt);
        let g = a.gen.load(std::sync::atomic::Ordering::SeqCst);
        a.on_user_event(UserEvent::Loaded(g, Err("HTTP 500".into())));
        // (the banner is drawn through `sans_touches`, as every banner)
        let b = argos_render::actions::sans_touches(&a.banner.clone().unwrap_or_default());
        assert_eq!(b.contains("F5"), !doigt, "{b}");
        assert!(b.contains("HTTP 500"), "{b}");
        let t = argos_render::i18n::tr(if doigt {
            "Touche la bougie de départ"
        } else {
            "Clique la bougie de départ"
        });
        assert!(!t.is_empty());
    }
    argos_render::actions::set_tactile(false);
}

/// 👆 N° 35 (phone): a finger scrolling the Alertes sheet is the sheet's: the crosshair of the chart behind does not
/// follow it.
#[test]
fn feuille_alertes_prend_le_doigt() {
    use argos_render::ecrans::CellHit;
    use winit::event::TouchPhase as P;
    let mut a = cell_390();
    a.ui.frames = 5;
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let cloche = z.iter().find(|(_, c)| *c == CellHit::Alertes).unwrap().0;
    toucher(&mut a, cloche.cx(), cloche.cy());
    a.draw();
    let n = a.ui.notifs.clone().expect("🔔 opens the sheet");
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let r = n.alertes_place(w, h);
    let (x, y) = (r.cx(), r.y + r.h * 0.6);
    a.touch(1, P::Started, (x, y));
    for k in 1..=4 {
        a.touch(1, P::Moved, (x, y - 30.0 * k as f64));
        assert!(a.to_chart().crosshair.is_none(), "move {k}: no crosshair behind the sheet");
    }
    a.touch(1, P::Ended, (x, y - 120.0));
    assert!(a.to_chart().crosshair.is_none(), "lifted: still none");
}

/// 📱 N° 40 (phone): a wide banner (« Footprint : bientôt… ») goes under the « ⌄ 0 » pill, never over it; n° 21: the
/// card of a release of the Calendrier ends above the Argos AI round (it covered « Voir sur la chart › »).
#[test]
fn cell_banniere_et_carte_libres() {
    let mut a = cell_390();
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.frames = 5;
    let b = "Footprint : bientôt dans la version Rust (comme dans Argos v1)";
    a.banner = Some(b.into());
    let _ = image_ecran(&mut a, 502, 972);
    let text = argos_render::text::Text::default();
    let r = argos_render::banner_rect(&a.chart.viewport, &text, b);
    // the « ⌄ N » pill: 72 to 100 points under the legend's top
    assert!(r.y >= 100.0, "the banner {r:?} under « ⌄ 0 »");
    let ai = a.ai_bouton_rect();
    a.watch.eco.agenda = vec![argos_data::eco::Event {
        title: "CPI m/m".into(),
        country: "USD".into(),
        impact: "High".into(),
        ts: now_s() - 3_600,
        forecast: "0.3%".into(),
        previous: "0.2%".into(),
        actual: "0.4%".into(),
    }];
    a.watch.eco.carte = Some(0);
    let (_, c) = a.cal_carte_draw(&text).expect("the card");
    assert!(ai.w > 0.0 && c.y + c.h <= ai.y, "the card {c:?} above the round {ai:?}");
}

/// 🏠 N° 44 (PC and tablet): a drawing selected, then ⌂ Accueil: the drawing's bar is neither drawn over the page nor
/// given its clicks; the page closed, the bar comes back, the drawing still selected.
#[test]
fn barre_du_dessin_cachee_sous_une_page() {
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.track_drawings();
    a.selected = Some(0);
    let (_, bar) = a.drawbar().expect("the bar of the selected drawing");
    let (x, y) = (bar.rect.cx(), bar.rect.cy());
    a.open_accueil();
    assert!(a.ui.accueil.is_some(), "the Accueil is open");
    assert!(a.drawbar().is_none(), "no bar over the Accueil");
    assert!(!a.drawbar_click(x, y), "its buttons take no click");
    a.ui.accueil = None;
    assert_eq!(a.selected, Some(0), "still selected");
    assert!(a.drawbar().is_some(), "the bar is back with the chart");
}
