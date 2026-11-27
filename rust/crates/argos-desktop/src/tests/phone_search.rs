//! 🔎 The phone's symbol search (bug B-017, Jee, 25 Nov 2026, IMG_3900: « la boîte est coupée et l'information de
//! symbole aussi »): the panel opened by the 🔎 of the watchlist sheet.

use super::*;

/// 📱 The phone at 402 × 874 by the finger, its watchlist sheet open, laid out as on the screen.
fn phone_with_sheet() -> App {
    let mut a = cell_390();
    a.watch.phone = Some(402);
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(402, 874));
    a.ui.frames = 5;
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    argos_render::actions::set_tactile(true);
    // (the first image lays the phone out and closes what was open on the PC: the sheet opened after it)
    a.draw();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Watchlist);
    a.refresh_watch();
    a.draw();
    assert!(a.top.phone && a.ui.rows.is_some(), "the phone's layout, its sheet open");
    a
}

/// A found market of the test (40 of them: more than the panel shows).
fn found(i: usize) -> argos_render::wlmenu::Found {
    argos_render::wlmenu::Found {
        id: format!("binance:C{i}USDT"),
        name: format!("C{i}USDT"),
        detail: "Binance".into(),
        group: "Crypto".into(),
        have: false,
    }
}

/// 📱 B-017: the 🔎 of the phone's watchlist sheet opens the search from under the top bar down to the bottom of the
/// sheet's list: it had the sheet's height but stood at the top of the screen, so it stopped half way down the sheet
/// and its last result was cut over the list behind it. (The results inside the card: `argos-render`
/// `wlmenu::tests::the_phone_search_keeps_its_last_result_inside_the_card`.)
#[test]
fn the_phone_search_reaches_the_bottom_of_the_sheet_list() {
    let mut a = phone_with_sheet();
    let (_, bot) = a.ui.rows.unwrap();
    let (_, _, loupe) = a.watch.entete_cell();
    let (x, y) = (a.side_x() + loupe.cx(), a.top_h() + bot + loupe.cy());
    toucher(&mut a, x, y);
    let p = a.ui.add.as_ref().expect("🔎 opens the symbol search");
    let liste = a.watch.list_rect(a.side_h(0.0));
    let bas = a.top_h() + bot + liste.y + liste.h;
    assert!((p.rect.y + p.rect.h - bas).abs() < 0.5, "down to the sheet list's bottom ({bas}): {:?}", p.rect);
    assert!(
        p.rect.y <= a.top_h() + liste.y + 0.5,
        "from under the top bar (above the keyboard): {:?}",
        p.rect
    );
    assert!(p.rect.x >= 0.0 && p.rect.x + p.rect.w <= a.win_w() + 0.5, "in the window: {:?}", p.rect);
    // 🖼 the screen, for Jee (ARGOS_CAPTURES=<folder>): 40 markets found, scrolled to the end
    if let Ok(dir) = std::env::var("ARGOS_CAPTURES") {
        if let Some(p) = a.ui.add.as_mut() {
            p.set_results((0..40).map(found).collect());
            p.wheel(-100.0);
        }
        let (img, _) = image_ecran(&mut a, 402, 874);
        let mut pm = tiny_skia::Pixmap::new(402, 874).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/recherche-cell-apres.png")).unwrap();
    }
    argos_render::actions::set_tactile(false);
}

/// 🖥 B-017 on the PC: the ＋ of the floating volet opens the search in the place of the volet's list, under its header:
/// it was laid from under the top bar, 12 points past the volet's bottom and 2 past the window's (its bottom edge cut).
#[test]
fn the_pc_search_stays_inside_its_volet() {
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Watchlist);
    a.refresh_watch();
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(1440, 900));
    a.draw();
    let v = a.volet_rect().expect("the PC's floating volet");
    let r = a.watch.add_rect();
    clic(&mut a, v.x + r.cx(), v.y + r.cy());
    let p = a.ui.add.as_ref().expect("＋ opens the symbol search").rect;
    let l = a.watch.list_rect(v.h);
    assert_eq!(p, Rect::new(v.x + l.x, v.y + l.y, l.w, l.h), "the volet's list, under its header");
    assert!(p.y + p.h <= v.y + v.h && p.y + p.h <= 900.0, "inside the volet and the window: {p:?} in {v:?}");
}
