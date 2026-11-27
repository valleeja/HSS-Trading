//! 📱 ⚡ The Funding pill of a perpetual on the phone (Jee, 23 Nov 2026, choice A « pilule Funding »): there on a
//! perpetual only, its card says what the PC's row says for the same answers, a touch opens and closes it, the finger
//! on the card does not move the chart, and it never lies on « ⌄ N », the legend or the price scale.

use super::*;

/// Bitfinex's real answer for the XRPUSDT perpetual (`status/deriv`, 24 Nov 2026 01:10 UTC).
const BFX_STATUS: &str = r#"[["tXRPF0:USTF0",1795482639000,null,0.949065,0.94913,null,59333363.65504643,null,1795507200000,-0.00001273,1352,null,0,null,null,0.9487834,null,null,2265770.17060856,null,null,null,0.0005,0.0025]]"#;

/// Hyperliquid's real answer (`metaAndAssetCtxs`, 24 Nov 2026 01:10 UTC), BTC's market only.
const HL_META: &str = r#"[{"universe":[{"szDecimals":5,"name":"BTC","maxLeverage":40,"marginTableId":56}]},[{"funding":"0.0000081741","openInterest":"37150.22674","prevDayPx":"61076.0","dayNtlVlm":"4810062158.0648260117","premium":"-0.0004985466","oraclePx":"61579.0","markPx":"61548.0","midPx":"61545.5","impactPxs":["61545.0","61548.3"],"dayBaseVlm":"77444.45106"}]]"#;

/// The time of these answers, the clock of these tests (2026-11-24 01:10:40 UTC).
const NOW: i64 = 1_795_482_640;

/// 📱 The phone at `largeur` × `hauteur` points (402 × 874: an iPhone in the browser; 502 × 972), its chart on
/// `exchange` / `base`, the answers above read.
fn cell(largeur: u32, hauteur: u32, exchange: Exchange, base: &str) -> App {
    crate::types::NOW_FIGE.with(|n| n.set(Some(NOW)));
    let mut a = cell_390();
    a.watch.phone = Some(largeur);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(largeur, hauteur));
    a.ui.frames = 5;
    a.chart.source = Source::Live { exchange, base: base.into() };
    lire_les_reponses(&mut a);
    // (one image drawn: the screen is laid out for the phone, as before a first touch)
    a.draw();
    assert!(a.top.phone, "the phone's layout");
    a
}

/// The answers above, read as the app reads them; the PC's row (`refresh_top`) made from them.
fn lire_les_reponses(a: &mut App) {
    let s = argos_data::exchanges::bitfinex::parse_status(BFX_STATUS).unwrap();
    a.feeds.bitfinex_statut = Some(crate::actions::carnets::StatutBitfinex {
        base: "XRP".into(),
        statut: s,
        ..Default::default()
    });
    for q in argos_data::watchlist::parse_quotes(HL_META, "").unwrap() {
        a.feeds.quotes.insert(q.coin.clone(), q);
    }
    a.refresh_top();
}

/// The value of the PC's row `label` for the same chart and the same answers.
fn ligne_du_pc(a: &App, label: &str) -> String {
    let mut pc = app();
    pc.chart.source = a.chart.source.clone();
    lire_les_reponses(&mut pc);
    pc.top
        .stats
        .iter()
        .find(|s| s.label == label)
        .unwrap_or_else(|| panic!("no « {label} » in {:?}", pc.top.stats))
        .value
        .clone()
}

/// The value of the card's row `nom`.
fn carte(f: &crate::actions::pilule::FundingCell, nom: &str) -> String {
    f.lignes
        .iter()
        .find(|l| l.0 == nom)
        .unwrap_or_else(|| panic!("no « {nom} » in {:?}", f.lignes))
        .1
        .clone()
}

/// ⚡ The pill is on a perpetual's chart (Bitfinex, Hyperliquid, and any other perpetual: « Funding — » when its rate
/// is not read), never on a spot market nor on a pair against BTC, never on the PC.
#[test]
fn the_pill_is_on_a_perpetual_only() {
    for (ex, base) in [(Exchange::Bitfinex, "XRP"), (Exchange::Hyperliquid, "BTC"), (Exchange::Binance, "SOL")] {
        assert!(cell(402, 874, ex, base).funding_cell().is_some(), "{ex:?} {base}: a perpetual");
    }
    assert_eq!(cell(402, 874, Exchange::Binance, "SOL").funding_cell().unwrap().pilule, "Funding —");
    for (ex, base) in [
        (Exchange::BitfinexSpot, "XRP"),
        (Exchange::BitfinexSpot, "XRP/BTC"),
        (Exchange::BinanceSpot, "XRP/BTC"),
        (Exchange::Bitfinex, "XRP/BTC"),
        (Exchange::Kraken, "BTC"),
    ] {
        assert!(cell(402, 874, ex, base).funding_cell().is_none(), "{ex:?} {base}: no pill");
    }
    let mut pc = cell(402, 874, Exchange::Bitfinex, "XRP");
    pc.top.phone = false;
    assert!(pc.funding_cell().is_none(), "the PC has its row");
}

/// 🔒 Reliable data: for the same recorded answers, the pill and its card write what the PC's row writes (Bitfinex's
/// XRPUSDT perpetual, Hyperliquid's BTC-USDC), nothing worked out again.
#[test]
fn the_card_says_what_the_pc_row_says() {
    // Bitfinex: mark 0.9487834, funding 0 (index 12), open interest 2 265 770 XRP, next payment 1795507200000
    let a = cell(402, 874, Exchange::Bitfinex, "XRP");
    let f = a.funding_cell().unwrap();
    let funding = ligne_du_pc(&a, "Funding / compte à rebours");
    assert_eq!(funding, "0.0000 % · 06:49:20");
    assert_eq!(f.pilule, "Funding 0.0000 % 06:49:20");
    assert_eq!(f.titre, "XRPUSDT Perpétuel · Bitfinex");
    assert_eq!(carte(&f, "Funding"), "0.0000 %");
    assert_eq!(carte(&f, "Prochain paiement"), "dans 06:49:20");
    for nom in ["Mark", "Intérêt ouvert"] {
        assert_eq!(carte(&f, nom), ligne_du_pc(&a, nom), "{nom}");
    }
    assert_eq!(carte(&f, "Source"), "Bitfinex Perpétuel");
    // Hyperliquid: mark 61548.0 (its row's « Prix »), funding 0.0000081741 an hour, open interest 37 150.22674 BTC
    let a = cell(402, 874, Exchange::Hyperliquid, "BTC");
    let f = a.funding_cell().unwrap();
    let funding = ligne_du_pc(&a, "Funding / compte à rebours");
    let (taux, reste) = funding.split_once(" · ").unwrap();
    assert_eq!(taux, "0.0008 %");
    assert_eq!(f.pilule, format!("Funding {taux} {reste}"));
    assert_eq!(f.titre, format!("{} Perpétuel · Hyperliquid", Exchange::Hyperliquid.ticker("BTC")));
    assert_eq!(carte(&f, "Funding"), taux);
    assert_eq!(carte(&f, "Prochain paiement"), format!("dans {reste}"));
    for nom in ["Prix", "Variation 24 h", "Intérêt ouvert"] {
        assert_eq!(carte(&f, nom), ligne_du_pc(&a, nom), "{nom}");
    }
    assert_eq!(carte(&f, "Mark"), ligne_du_pc(&a, "Prix"), "Hyperliquid's price is its mark");
}

/// 👆 A touch on the pill opens its card, a touch on the card leaves it open and the chart where it was, a touch on
/// the pill or elsewhere closes it; an open sheet of the pill passes over it.
#[test]
fn the_card_opens_and_closes_by_a_touch() {
    let mut a = cell(402, 874, Exchange::Bitfinex, "XRP");
    let (_, z) = a.funding_zones().unwrap();
    assert!(z.carte.is_none(), "closed at rest");
    toucher(&mut a, z.pilule.cx(), z.pilule.cy());
    assert!(a.ui.funding_carte, "the touch on the pill opens it");
    let c = a.funding_zones().unwrap().1.carte.expect("the card");
    assert!(c.y >= z.pilule.y + z.pilule.h, "just under the pill");
    // the finger on the card, dragged: the chart does not move
    let vp = a.chart.viewport;
    {
        use winit::event::TouchPhase;
        a.touch(7, TouchPhase::Started, (c.cx(), c.cy()));
        for k in 1..=6 {
            a.touch(7, TouchPhase::Moved, (c.cx() - 12.0 * k as f64, c.cy() + 4.0 * k as f64));
        }
        a.touch(7, TouchPhase::Ended, (c.cx() - 72.0, c.cy() + 24.0));
    }
    assert_eq!(
        (a.chart.viewport.right_offset, a.chart.viewport.bar_spacing),
        (vp.right_offset, vp.bar_spacing),
        "the chart did not move"
    );
    assert!(a.ui.funding_carte, "a touch on the card leaves it open");
    toucher(&mut a, z.pilule.cx(), z.pilule.cy());
    assert!(!a.ui.funding_carte, "the pill again: closed");
    toucher(&mut a, z.pilule.cx(), z.pilule.cy());
    let (x, y) = (a.rw() + a.chart.viewport.plot_w() / 2.0, a.top_h() + a.chart.viewport.plot_h() - 40.0);
    toucher(&mut a, x, y);
    assert!(!a.ui.funding_carte, "a touch elsewhere closes it");
    // a sheet of the pill over it: the pill is not touched, its card not drawn
    toucher(&mut a, z.pilule.cx(), z.pilule.cy());
    a.pfeuille_ouvrir(crate::actions::pilule::PKind::Argos);
    assert!(!a.funding_sous(z.pilule.cx(), z.pilule.cy()), "the sheet passes over it");
    assert_eq!(
        a.funding_images(&argos_render::text::Text::default()).len(),
        1,
        "the pill only, under the sheet"
    );
}

/// 📐 At 402 and 502 points wide, the pill and its card lie right of « ⌄ N » / under it, never on the legend's lines
/// (title, O H L C, Vol), « ⌄ N » or the price scale, and the banner keeps clear of the pill.
#[test]
fn nothing_overlaps() {
    let croise = |a: &Rect, b: &Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
    for (w, h) in [(402, 874), (502, 972)] {
        let mut a = cell(w, h, Exchange::Bitfinex, "XRP");
        a.banner = Some("📴 Hors ligne · bougies du cache, reconnexion automatique".into());
        a.ui.funding_carte = true;
        // the screen drawn once more: the legend's and the banner's places are known
        a.draw();
        assert!(a.chart.viewport.width <= w, "{w}: the phone's chart");
        let (_, z) = a.funding_zones().unwrap();
        let (ox, oy) = (a.rw(), a.top_h());
        let chart = |r: Rect| Rect::new(r.x - ox, r.y - oy, r.w, r.h);
        let (p, c) = (chart(z.pilule), chart(z.carte.unwrap()));
        let pw = a.chart.viewport.plot_w();
        assert!(p.x + p.w <= pw && c.x + c.w <= pw, "{w}: before the price scale ({p:?}, {c:?}, {pw})");
        assert!(!croise(&p, &argos_render::indleg::bouton_cell()), "{w}: not on « ⌄ N »");
        // the legend's lines above « ⌄ N » (title, O H L C, Vol): « ⌄ N »'s own line is the pill's, checked above
        let n = argos_render::indleg::bouton_cell();
        let legende: Vec<Rect> = argos_render::legende_zones(&a.chart_with(false))
            .into_iter()
            .filter(|l| l.y + l.h <= n.y)
            .collect();
        assert!(!legende.is_empty(), "{w}: the legend is drawn");
        for l in &legende {
            assert!(!croise(&p, l) && !croise(&c, l), "{w}: on the legend's line {l:?}");
        }
        // the banner keeps clear of the pill (the card lies over the chart a moment, as a menu does)
        let b = argos_render::banner_rect(&a.chart.viewport, &argos_render::text::Text::default(), a.banner.as_deref().unwrap());
        assert!(!croise(&p, &b), "{w}: the banner {b:?} on the pill {p:?}");
    }
}

/// 🧱 Nothing goes while the card is open: the places the chart keeps clear are the pill's only, the same open or
/// closed (a table under the card stays drawn, under it).
#[test]
fn the_card_pushes_nothing_away() {
    let mut a = cell(402, 874, Exchange::Bitfinex, "XRP");
    let ferme = a.funding_garder();
    a.ui.funding_carte = true;
    assert_eq!(a.funding_garder(), ferme);
    assert_eq!(ferme.len(), 1, "the pill");
}
