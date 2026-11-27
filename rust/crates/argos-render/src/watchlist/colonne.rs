use super::*;

/// 🖥 Jee (19 Nov): every button of the PC icon bar has its help bubble — the tool's own name, whatever tools Jee
/// put there (all of them here) — and none elsewhere.
#[test]
fn bulle_de_chaque_outil_de_la_colonne() {
    let w = Watchlist {
        colonne: Some(138.0),
        col_h: 676.0,
        col_outils: Some(Tool::ALL.to_vec()),
        ..Default::default()
    };
    for (i, t) in w.bar_tools().into_iter().enumerate() {
        let r = w.tool_rect(i);
        let (nom, place) = w.bulle_outil(r.cx(), r.cy()).unwrap_or_else(|| panic!("{t:?}: a bubble"));
        assert!(!nom.is_empty() && nom == t.name() && place == r, "{t:?}: « {nom} »");
    }
    let r0 = w.tool_rect(0);
    assert!(w.bulle_outil(r0.x - 30.0, r0.cy()).is_none(), "beside the bar: nothing");
}

/// 🫧 Jee (19 Nov, « quand on ouvre, garder la même forme »): the right column opened — unfolded, or shown on
/// hover over the chart — has exactly its folded band's shape: one long pill (radius half its width: the corners
/// show what is behind), the same glass fill beside the icons and the same light outline, top and sides.
#[test]
fn colonne_ouverte_comme_sa_bande() {
    let t = Text::default();
    let base = Watchlist {
        colonne: Some(138.0),
        col_h: 676.0,
        col_repliee: true,
        ..Default::default()
    };
    let (bande, b) = base.draw_colonne(&t, None, &Theme::default()).unwrap();
    assert_eq!(b.w, 12.0, "folded: the 12-wide band");
    let px = |pm: &Pixmap, x: f64, y: f64| {
        let p = pm.pixel(x as u32, y as u32).unwrap();
        (p.red(), p.green(), p.blue(), p.alpha())
    };
    for (quoi, survol, repliee) in [("shown on hover over the chart", true, true), ("unfolded", false, false)] {
        let w = Watchlist {
            col_survol: survol,
            col_repliee: repliee,
            ..base.clone()
        };
        let (pm, c) = w.draw_colonne(&t, None, &Theme::default()).unwrap();
        assert!(c.w > 40.0, "{quoi}: the opened column");
        let (lw, lh) = (pm.width() as f64, pm.height() as f64);
        let k = lw.min(lh) / 2.0;
        // round ends (half the width), like the band: the four corners show what is behind
        for (x, y) in [
            (0.2 * k, 0.2 * k),
            (lw - 0.2 * k, 0.2 * k),
            (0.2 * k, lh - 0.2 * k),
            (lw - 0.2 * k, lh - 0.2 * k),
        ] {
            assert_eq!(px(&pm, x, y).3, 0, "{quoi}: round end at ({x}, {y})");
        }
        // the same outline (left side, right side, top) and the same glass fill (beside the icons)
        let (bw, bh) = (bande.width() as f64, bande.height() as f64);
        assert_eq!(px(&pm, 0.0, lh / 2.0), px(&bande, 0.0, bh / 2.0), "{quoi}: left outline");
        assert_eq!(px(&pm, lw - 1.0, lh / 2.0), px(&bande, bw - 1.0, bh / 2.0), "{quoi}: right outline");
        // (top: the arc's antialiasing differs with the width; the same outline, within a few levels)
        let (a, b) = (px(&pm, lw / 2.0, 0.0), px(&bande, bw / 2.0, 0.0));
        let pres = |u: u8, v: u8| u.abs_diff(v) <= 16;
        assert!(
            pres(a.0, b.0) && pres(a.1, b.1) && pres(a.2, b.2) && pres(a.3, b.3),
            "{quoi}: top outline {a:?} {b:?}"
        );
        assert_eq!(px(&pm, 2.0, lh / 2.0), px(&bande, 2.0, bh / 4.0), "{quoi}: glass fill");
        assert!(pm.pixels().iter().any(|p| p.alpha() > 200 && p.red() > 150), "{quoi}: the icons are drawn");
    }
}

/// 🖥 Colonne PC de la maquette : boutons de 36 au pas de 40 à partir de 9 sous son haut, › et ⚙ en bas ; ⚙ demande
/// le menu de la barre, › la replie ; l'image de la colonne fait 44 de large et toute la hauteur moins 10 en bas.
#[test]
fn colonne_haut_bas_et_boutons_du_bas() {
    let mut w = Watchlist {
        colonne: Some(138.0),
        col_h: 676.0,
        ..Default::default()
    };
    let r0 = w.tool_rect(0);
    // 1er bouton à 19 du haut de la fenêtre, centré dans la colonne ; pas de 40 au plus (serré si tout ne tient pas)
    assert_eq!(r0.y, -129.0);
    assert!((r0.cx() - 28.0).abs() < 0.01 && r0.w <= COL_BTN);
    let pas = w.tool_rect(1).y - r0.y;
    assert!(pas > COL_BTN && pas <= COL_PAS, "pas {pas}");
    let c = w.col_rect(676.0).unwrap();
    assert_eq!((c.x, c.y, c.w, c.h), (6.0, -138.0, 44.0, 138.0 + 666.0));
    let [(plier, _), (perso, _)] = w.col_boutons(676.0).unwrap();
    assert_eq!(perso.y + perso.h, 676.0 - 10.0 - 7.0);
    assert!(matches!(w.click(perso.cx(), perso.cy(), 676.0), Click::PersonnaliserBarre(_)));
    // chaque bouton se touche dans la colonne
    for (i, t) in w.bar_tools().into_iter().enumerate() {
        let r = w.tool_rect(i);
        assert!(r.y + r.h < plier.y, "{t:?} au-dessus de ›");
        assert_eq!(w.tool_at(r.cx(), r.cy()).map(|x| x.1), Some(t));
    }
    let (pm, _) = w.draw_colonne(&Text::default(), Some((r0.cx(), r0.cy())), &Theme::default()).unwrap();
    assert_eq!((pm.width(), pm.height()), (crate::px::dev(44.0) as u32, crate::px::dev(804.0) as u32));
    assert_eq!(w.click(plier.cx(), plier.cy(), 676.0), Click::ReplierBarre);
    assert!(w.prefs.folded && !w.prefs.open);
    // sans colonne : la barre de toujours
    w.colonne = None;
    assert!(w.col_boutons(676.0).is_none() && w.draw_colonne(&Text::default(), None, &Theme::default()).is_none());
}

/// 🫧 Jee's permanent rule (20 Nov): what lies on the chart is its own glass pill (round ends, translucent glass, thin
/// outline); « sans cadre » never means « sans fond », and never a dark rectangle. The folded bar's band (rail on the
/// left, column on the right) is that glass pill: the chart shows through its middle, its corners are empty.
#[test]
fn bande_repliee_en_pilule_verre() {
    let th = crate::Theme::default();
    let (w, h) = (12.0, 400.0);
    let mut pm = crate::px::new(w as u32, h as u32).unwrap();
    dessiner_bande(&mut pm, w, h, &th);
    // the middle, away from the grip: translucent glass (no opaque dark fill)
    let p = pm.pixel(6, 60).unwrap();
    assert!(p.alpha() > 0 && p.alpha() < 200, "glass, not a dark rectangle: alpha {}", p.alpha());
    // round ends: the corners show what is behind
    assert_eq!(pm.pixel(0, 0).unwrap().alpha(), 0, "round end");
    assert_eq!(pm.pixel(11, 399).unwrap().alpha(), 0, "round end");
}
