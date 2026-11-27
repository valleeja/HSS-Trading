//! ₿ Pairs against BTC and Bitfinex in the app (Jee's mock-up of 23 Nov 2026): the market panel, the « btc » chip, the
//! phone's sheet, the Bitfinex book, the demo ticket; and the captures for Jee.

use super::*;

/// ₿ A screen with the catalogues of the exchanges that have XRP against BTC (recorded real answers of 23 Nov 2026 in
/// the folder `bfx`), Bitfinex's prices, and a chart of Bitfinex's XRP/BTC in 4 h made from its real 1 h candles.
fn scene_paires_btc(mut a: App, bfx: &str) -> App {
    use argos_data::exchanges::{bitfinex, bitstamp, kraken, kucoin, spot};
    let lire = |f: &str| std::fs::read_to_string(format!("{bfx}/{f}")).unwrap();
    let mut st = Store::in_memory().unwrap();
    let mut save = |ex: Exchange, list: Vec<argos_core::Instrument>| st.save_catalog(ex.id(), &list, 0).unwrap();
    let all = bitfinex::parse_instruments(&lire("spot.json"), &lire("fut.json"), &lire("tickers.json")).unwrap();
    for ex in [Exchange::Bitfinex, Exchange::BitfinexSpot] {
        save(ex, all.iter().filter(|i| i.exchange == ex.id()).cloned().collect());
    }
    save(
        Exchange::BinanceSpot,
        spot::parse_instruments(spot::Venue::Binance, &lire("cat/binance-spot.json")).unwrap(),
    );
    save(
        Exchange::OkxSpot,
        spot::parse_instruments(spot::Venue::Okx, &lire("cat/okx-spot.json")).unwrap(),
    );
    save(Exchange::Kraken, kraken::parse_instruments(&lire("cat/kraken.json")).unwrap());
    save(Exchange::Kucoin, kucoin::parse_instruments(&lire("cat/kucoin.json")).unwrap());
    save(Exchange::Bitstamp, bitstamp::parse_instruments(&lire("cat/bitstamp.json")).unwrap());
    a.store = Some(st);
    a.ui.catalogues_btc = true;
    for ex in [Exchange::Bitfinex, Exchange::BitfinexSpot] {
        let m = ex.parse_markets(&lire("tickers.json")).unwrap();
        a.feeds.ex_prices.insert(ex, m.into_iter().map(|m| (m.base.clone(), m)).collect());
    }
    let h1 = bitfinex::parse_candles(&lire("xrpbtc-1h.json")).unwrap();
    a.chart.raw = argos_core::aggregate(&h1, 14_400);
    a.chart.source = Source::Live {
        exchange: Exchange::BitfinexSpot,
        base: "XRP/BTC".into(),
    };
    a.watch.selected = "bitfinex-spot:XRPBTC".into();
    (a.chart.interval, a.chart.file_interval) = (Interval(14_400), Interval(14_400));
    a.title = Exchange::BitfinexSpot.chart_title("XRP/BTC", "4h");
    a.ind.on.clear();
    a.ui.frames = 5;
    a.refresh_indicators();
    a.rebuild();
    // (the price scale fitted to these bars)
    a.start_free();
    a.refresh_top();
    a
}

/// 🖼 An image of the screen saved as a PNG.
fn png(img: &[u32], w: u32, h: u32, chemin: &str) {
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    for (px, v) in pm.pixels_mut().iter_mut().zip(img) {
        *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
    }
    pm.save_png(chemin).unwrap();
}

/// 📸 ₿ Paires contre BTC et Bitfinex (Jee's mock-up of 23 Nov): PC search « xrpbtc », PC ⚙ Tous les filtres with
/// « btc », iPhone 390 × 844 search « xrpbtc », Bitfinex's XRP/BTC chart in 4 h, its order book, a Bitfinex perpetual.
/// Real answers recorded from the PC (`ARGOS_BFX`), the app's own drawing: only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_paires_btc_pour_jee() {
    use argos_render::marches::Filtre;
    let (Ok(dir), Ok(bfx)) = (std::env::var("ARGOS_CAPTURES"), std::env::var("ARGOS_BFX")) else {
        return;
    };
    // 🖥 PC 1440 × 900: the search « xrpbtc »
    dossier_a_soi("captures_paires_btc_pc");
    let mut a = scene_paires_btc(pc_avec_volet(wl::Tool::Watchlist), &bfx);
    a.watch.prefs.open = false;
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let p = a.pastille_symbole().expect("pastille");
    a.marches_ouvrir(p, false);
    let m = a.ui.marches.as_mut().unwrap();
    m.epingle = true;
    let c = m.taper("xrpbtc");
    a.marches_faire(c);
    let (img, _) = image_ecran(&mut a, 1440, 900);
    png(&img, 1440, 900, &format!("{dir}/1-pc-recherche-xrpbtc.png"));
    // 🖥 ⚙ Tous les filtres, the « btc » chip
    let m = a.ui.marches.as_mut().unwrap();
    m.texte.clear();
    m.focus = false;
    m.filtres = true;
    m.jetons = vec![(Filtre::Quote, "btc".into())];
    a.marches_faire(argos_render::marches::MarcheClick::Chercher);
    let (img, _) = image_ecran(&mut a, 1440, 900);
    png(&img, 1440, 900, &format!("{dir}/2-pc-tous-les-filtres-btc.png"));
    // 🖥 the chart: Bitfinex XRP/BTC 4 h, then its order book in the column
    a.ui.marches = None;
    let (img, _) = image_ecran(&mut a, 1440, 900);
    png(&img, 1440, 900, &format!("{dir}/4-pc-chart-xrpbtc-bitfinex-4h.png"));
    let b = argos_data::exchanges::bitfinex::parse_book(&std::fs::read_to_string(format!("{bfx}/book-xrpbtc.json")).unwrap(), "XRP/BTC", 0).unwrap();
    a.watch.dock.on = true;
    a.watch.dock.pos = argos_render::dock::DockPos::Right;
    for i in a.watch.dock.items.iter_mut() {
        i.on = i.module == argos_render::dock::Module::Book;
    }
    a.ind.on = vec![argos_render::indicators::Ind::BookHeat];
    a.refresh_indicators();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    a.feeds.carnets = Some(crate::actions::carnets::CarnetsState {
        key: "bitfinex|false|XRP/BTC".into(),
        ..Default::default()
    });
    a.on_user_event(UserEvent::CarnetBitfinex("bitfinex|false|XRP/BTC".into(), Ok(b)));
    let (img, _) = image_ecran(&mut a, 1440, 900);
    png(&img, 1440, 900, &format!("{dir}/5-pc-carnet-xrpbtc-bitfinex.png"));
    // 🖥 a Bitfinex perpetual: XRP/USDT perpetual, 1 h
    let h1 = argos_data::exchanges::bitfinex::parse_candles(&std::fs::read_to_string(format!("{bfx}/xrpperp-1h.json")).unwrap()).unwrap();
    a.chart.raw = h1;
    a.chart.source = Source::Live {
        exchange: Exchange::Bitfinex,
        base: "XRP".into(),
    };
    a.watch.selected = "bitfinex:XRPF0:USTF0".into();
    (a.chart.interval, a.chart.file_interval) = (Interval(3_600), Interval(3_600));
    a.title = Exchange::Bitfinex.chart_title("XRP", "1h");
    (a.feeds.book, a.feeds.heat, a.feeds.carnets) = (None, None, None);
    a.watch.dock.on = false;
    a.ind.on.clear();
    a.refresh_indicators();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    a.rebuild();
    a.start_free();
    a.refresh_top();
    let (img, _) = image_ecran(&mut a, 1440, 900);
    png(&img, 1440, 900, &format!("{dir}/6-pc-perp-xrpusdt-bitfinex.png"));
    // 📱 iPhone 390 × 844: the bottom sheet, « xrpbtc »
    dossier_a_soi("captures_paires_btc_cell");
    let mut a = cell_390();
    a.watch.phone = Some(390);
    let mut a = scene_paires_btc(a, &bfx);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.depart_vu = true;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(390, 844));
    argos_render::actions::set_tactile(true);
    a.marches_ouvrir(Rect::new(0.0, 0.0, 0.0, 0.0), true);
    let c = a.ui.marches.as_mut().unwrap().taper("xrpbtc");
    a.marches_faire(c);
    let (img, _) = image_ecran(&mut a, 390, 844);
    argos_render::actions::set_tactile(false);
    png(&img, 390, 844, &format!("{dir}/3-iphone-recherche-xrpbtc.png"));
}

/// ₿ 📱 iPhone 390 × 844, « xrpbtc » in the bottom sheet: Bitfinex first, the computed pair last in CALCULÉE, every row
/// inside the sheet, none over another, none under the Argos AI round; the « btc » chip: « CONTRE BTC », only BTC.
#[test]
fn paires_btc_au_doigt_rien_ne_chevauche() {
    dossier_a_soi("paires-btc-cell");
    let mut a = cell_390();
    a.watch.phone = Some(390);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(390, 844));
    let mut st = Store::in_memory().unwrap();
    let inst = |ex: &str, sym: &str, quote: &str| argos_core::Instrument {
        exchange: ex.into(),
        symbol: sym.into(),
        base: "XRP".into(),
        quote: quote.into(),
        kind: argos_core::MarketKind::Spot,
        group: "Crypto".into(),
        price_decimals: 8,
        size_decimals: 2,
        growth: false,
        active: true,
    };
    for (ex, sym) in [
        ("bitfinex-spot", "XRPBTC"),
        ("binance-spot", "XRPBTC"),
        ("kraken", "XRP/BTC"),
        ("okx-spot", "XRP-BTC"),
    ] {
        st.save_catalog(ex, &[inst(ex, sym, "BTC")], 0).unwrap();
    }
    st.save_catalog("kucoin", &[inst("kucoin", "XRP-BTC", "BTC"), inst("kucoin", "XRP-USDT", "USDT")], 0)
        .unwrap();
    a.store = Some(st);
    a.ui.catalogues_btc = true;
    argos_render::actions::set_tactile(true);
    a.marches_ouvrir(Rect::new(0.0, 0.0, 0.0, 0.0), true);
    let c = a.ui.marches.as_mut().unwrap().taper("xrpbtc");
    a.marches_faire(c);
    let m = a.ui.marches.clone().unwrap();
    argos_render::actions::set_tactile(false);
    let s = m.sections();
    let titres: Vec<&str> = s.iter().map(|x| x.0).collect();
    assert_eq!(titres, ["RÉSULTATS", "CALCULÉE"]);
    assert_eq!(
        s[0].1.iter().map(|l| l.bourse.as_str()).collect::<Vec<_>>(),
        ["Bitfinex", "Binance", "Kraken", "OKX", "KuCoin"]
    );
    assert!(s[0].1.iter().all(|l| l.quote == "BTC" && !l.calculee) && s[1].1[0].calculee);
    let n = m.lignes().len();
    let rects: Vec<Rect> = (0..n).filter_map(|k| m.ligne_rect(k)).collect();
    assert_eq!(rects.len(), n);
    for (k, r) in rects.iter().enumerate() {
        assert!(
            r.x >= m.rect.x && r.x + r.w <= m.rect.x + m.rect.w + 0.5 && r.w <= 390.0,
            "{k}: inside the sheet {r:?} {:?}",
            m.rect
        );
        if let Some(q) = rects.get(k + 1) {
            assert!(r.y + r.h <= q.y + 0.5, "{k}: no row over the next");
        }
    }
    // the rows in view stay above « Tous les filtres », which stays above the Argos AI round's place
    let f = m.filtres_rect();
    let ai = a.ai_bouton_rect();
    assert!(f.y + f.h <= 844.0);
    if ai.w > 0.0 {
        assert!(
            f.y + f.h <= ai.y || f.x + f.w <= ai.x,
            "« Tous les filtres » not under the Argos AI round {f:?} {ai:?}"
        );
    }
    // the « btc » chip: CONTRE BTC, every market against BTC (the USDT one left out)
    let m = a.ui.marches.as_mut().unwrap();
    m.texte.clear();
    m.jetons = vec![(argos_render::marches::Filtre::Quote, "btc".into())];
    a.marches_faire(argos_render::marches::MarcheClick::Chercher);
    let m = a.ui.marches.as_ref().unwrap();
    let s = m.sections();
    assert_eq!(s[0].0, "CONTRE BTC");
    assert_eq!(s[0].1.len(), 5);
    assert!(s[0].1.iter().all(|l| l.quote == "BTC"));
    // a row opens its market: Bitfinex's XRP/BTC
    let id = s[0].1[0].id.clone();
    a.marches_faire(argos_render::marches::MarcheClick::Ouvrir(id));
    assert!(matches!(&a.chart.source, Source::Live { exchange: Exchange::BitfinexSpot, base } if base == "XRP/BTC"));
}

/// ₿ Trading démo on a pair against BTC: the demo account counts in dollars, its prices are in BTC: no ticket, the
/// message instead (no buy / sell to touch); back on a dollar market, the ticket.
#[test]
fn ticket_pas_encore_pour_les_paires_btc() {
    let (mut a, w, h) = scene_ticket(false);
    a.chart.source = Source::Live {
        exchange: Exchange::BinanceSpot,
        base: "XRP/BTC".into(),
    };
    a.ticket_sync();
    assert!(a.watch.panels.trading.pas_btc);
    // the sheet says « pas encore »: no line of the ticket on the chart (+2 % / −1 %) and the price scale not widened
    let _ = image_ecran(&mut a, w, h);
    assert!(a.vue_ordres().ticket.is_none(), "no ticket line on a BTC pair");
    a.ticket_echelle();
    assert!(a.ui.ticket_zoom.is_none(), "no widened scale on a BTC pair");
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    a.ticket_sync();
    assert!(!a.watch.panels.trading.pas_btc);
}

/// 📖 A Bitfinex chart reads Bitfinex's own book (every 5 s while the panel is shown), never Argos Murs' books nor
/// Hyperliquid's stream; the panel names it.
#[test]
fn carnet_bitfinex() {
    use argos_render::dock::CarnetSource;
    let mut a = ecran_avec("carnet-bitfinex", vec![argos_render::indicators::Ind::BookHeat]);
    a.chart.source = Source::Live {
        exchange: Exchange::BitfinexSpot,
        base: "XRP/BTC".into(),
    };
    assert!(a.carnets_choix().is_none() && !a.carnets_instantanes(), "not Argos Murs' books");
    a.carnets_refresh();
    let key = a.feeds.carnets.as_ref().unwrap().key.clone();
    assert_eq!(key, "bitfinex|false|XRP/BTC");
    let b = argos_data::exchanges::bitfinex::parse_book("[[0.0000152,2,100],[0.0000153,1,-50]]", "XRP/BTC", 1).unwrap();
    a.on_user_event(UserEvent::CarnetBitfinex(key.clone(), Ok(b)));
    assert_eq!(a.feeds.book.as_ref().unwrap().bids[0].px, 0.0000152);
    assert_eq!(a.carnet_source(), CarnetSource::Choisis("Bitfinex Spot 1/1".into()));
    assert_eq!(a.book_coin(), None, "no other market's book (nor Argos Murs) under prices in BTC");
    // the next reading is refused: the panel says the book cannot be read, the old book goes
    {
        let s = a.feeds.carnets.as_mut().unwrap();
        (s.busy, s.at) = (true, Some(std::time::Instant::now()));
    }
    a.on_user_event(UserEvent::CarnetBitfinex(key, Err("HTTP 429".into())));
    assert!(a.feeds.book.is_none(), "the old book is not shown any more");
    assert_eq!(a.carnet_source(), CarnetSource::Indisponible("Carnet indisponible pour Bitfinex Spot".into()));
}
