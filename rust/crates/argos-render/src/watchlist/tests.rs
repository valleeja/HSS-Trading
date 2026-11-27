#[test]
fn variation_sans_trop_de_decimales() {
    assert_eq!(super::chg_decimals(2745.38, 3), 0, "2 745 et pas 2,745.380");
    assert_eq!(super::chg_decimals(143.8, 2), 1);
    assert_eq!(super::chg_decimals(2.012, 4), 2);
    assert_eq!(super::chg_decimals(-0.0123, 4), 4, "petites variations : toutes les décimales du prix");
    assert_eq!(super::chg_decimals(5000.0, 0), 0);
}
use super::*;

#[test]
fn barre_d_icones_reordonnee_masquee_et_remise() {
    let mut w = Watchlist::default();
    // l'ordre par défaut de la maquette (capture de Jee, 2026-11-09), Indicateurs masqué (ƒx est en haut)
    let defaut: Vec<Tool> = Tool::ALL.iter().copied().filter(|t| !Tool::CACHES.contains(t)).collect();
    assert_eq!(w.bar_tools(), defaut, "ordre par défaut");
    assert_eq!(&defaut[..4], &[Tool::Watchlist, Tool::Trading, Tool::Demo, Tool::Gardien]);
    assert_eq!(defaut.last(), Some(&Tool::Reglages), "⚙ tout en bas");
    // ⚙ (dernier) glissé tout en haut
    let last = w.bar_tools().len() - 1;
    w.move_tool(last, 0);
    assert_eq!(w.bar_tools()[0], Tool::Reglages);
    assert_eq!(w.bar_tools()[1], Tool::Watchlist);
    // la watchlist descendue d'un cran
    w.move_tool(1, 2);
    assert_eq!(&w.bar_tools()[..3], &[Tool::Reglages, Tool::Trading, Tool::Watchlist]);
    // masquer : le bouton disparaît ; la watchlist ne se masque jamais
    w.set_tool_hidden(Tool::News, true);
    w.set_tool_hidden(Tool::Watchlist, true);
    assert!(!w.bar_tools().contains(&Tool::News) && w.bar_tools().contains(&Tool::Watchlist));
    assert_eq!(w.bar_tools().len(), defaut.len() - 1);
    // gardé dans les réglages (JSON), puis remis par défaut
    let back: Prefs = serde_json::from_str(&serde_json::to_string(&w.prefs).unwrap()).unwrap();
    assert_eq!(
        (back.order.first(), back.hidden_tools.clone()),
        (Some(&Tool::Reglages), vec![Tool::Indicators, Tool::Dessin, Tool::News])
    );
    w.reset_tools();
    assert_eq!(w.bar_tools(), defaut);
    // ancien fichier de réglages, barre jamais réordonnée : la barre de la maquette, une seule fois
    let mut old: Prefs = serde_json::from_str(r#"{"open":true}"#).unwrap();
    assert!(old.passer_barre_v2() && !old.passer_barre_v2());
    assert!(old.order.is_empty() && old.hidden_tools == Tool::CACHES.to_vec());
    // réglages déjà en barre v2 (✏ visible) : le ✏ se cache une fois
    let mut v2: Prefs = serde_json::from_str(r#"{"barre_v2":true,"hidden_tools":["Indicators"]}"#).unwrap();
    assert!(v2.passer_barre_v2() && v2.hidden_tools == Tool::CACHES.to_vec());
    // barre déjà réordonnée : son ordre et ses boutons masqués sont gardés
    let mut perso: Prefs = serde_json::from_str(r#"{"order":["Journal","Watchlist"],"hidden_tools":["News"]}"#).unwrap();
    perso.passer_barre_v2();
    assert_eq!(
        (perso.order.clone(), perso.hidden_tools.clone()),
        (vec![Tool::Journal, Tool::Watchlist], vec![Tool::News])
    );
}

fn wl() -> Watchlist {
    let mut w = Watchlist {
        prefs: Prefs {
            open: true,
            ..Prefs::default()
        },
        selected: "BTC".into(),
        ..Watchlist::default()
    };
    w.update(vec![
        Section {
            name: "Crypto".into(),
            rows: vec![
                Row::new("BTC", "BTC-USDC", "80261.3", 80848.6),
                Row::new("ETH", "ETH-USDC", "2300.5", 2250.0),
                Row::new("HYPE", "HYPE-USDC", "38.12", 38.0),
            ],
        },
        Section {
            name: "HIP-3".into(),
            rows: vec![Row::new("xyz:NVDA", "xyz:NVDA", "180.1", 178.0)],
        },
    ]);
    w
}

#[test]
fn ordre_et_fleches() {
    let mut w = wl();
    assert_eq!(w.order(), vec!["BTC", "ETH", "HYPE", "xyz:NVDA"]);
    assert_eq!(w.step(1).as_deref(), Some("ETH"));
    assert_eq!(w.step(-1), None, "début de la liste");
    w.selected = "HYPE".into();
    assert_eq!(w.step(1).as_deref(), Some("xyz:NVDA"), "passe à la section suivante");
    // symbole ouvert noté avec le préfixe de l'exchange (tes listes) : il est retrouvé quand même
    w.selected = "hl:HYPE".into();
    assert_eq!(w.step(-1).as_deref(), Some("ETH"));
    // tri par Chg% décroissant : ETH (+2,2 %) en premier
    w.prefs.sort = Some((Col::Pct, false));
    assert_eq!(w.order()[0], "ETH");
    // section repliée : ses symboles sortent de l'ordre
    w.prefs.collapsed.push("HIP-3".into());
    assert!(!w.order().contains(&"xyz:NVDA".to_string()));
}

#[test]
fn clics() {
    let mut w = wl();
    let h = 800.0;
    // ligne 2 (ETH) : en-tête de section puis BTC puis ETH
    let y_eth = TOP + HEAD_H + SEC_H + ROW_H + 10.0;
    assert_eq!(w.click(100.0, y_eth, h), Click::Open("ETH".into()));
    assert_eq!(w.selected, "ETH");
    // drapeau à gauche : ouvre le choix de couleur, puis la pastille bleue (2e)
    assert_eq!(w.click(8.0, y_eth, h), Click::Changed);
    let p = w.picker_rect("ETH").expect("choix de couleur ouvert");
    w.click(p.x + 6.0 + 1.5 * DOT, p.cy(), h);
    assert_eq!(w.flag("ETH"), Some(1));
    // même couleur encore : le drapeau s'enlève
    w.click(8.0, y_eth, h);
    w.click(p.x + 6.0 + 1.5 * DOT, p.cy(), h);
    assert_eq!(w.flag("ETH"), None);
    // clic ailleurs : le choix se ferme sans rien changer
    w.click(8.0, y_eth, h);
    w.click(300.0, y_eth, h);
    assert_eq!((w.flag("ETH"), w.picker.is_none()), (None, true));
    // tri : ↑ puis ↓ puis aucun
    for want in [Some((Col::Last, true)), Some((Col::Last, false)), None] {
        w.click(w.pw() - 170.0, TOP + 10.0, h); // colonne Dernier (comptée du bord droit)
        assert_eq!(w.prefs.sort, want);
    }
    // replier la section Crypto
    w.click(100.0, TOP + HEAD_H + 10.0, h);
    assert_eq!(w.prefs.collapsed, vec!["Crypto"]);
    // fermer le panneau avec le bouton de la barre d'icônes
    let ir = w.tool_rect(0);
    assert_eq!(w.click(ir.cx(), ir.cy(), h), Click::Changed);
    assert!(!w.prefs.open);
    assert_eq!(w.width(), ICON_BAR);
}

/// 🧩 Watchlist compacte (module de la colonne) : rangées dans l'ordre, clic = symbole.
#[test]
fn watchlist_compacte() {
    let w = wl();
    let order = w.order();
    assert!(!order.is_empty());
    assert_eq!(w.mini_click(MINI_ROW * 0.5), order.first().cloned());
    assert_eq!(w.mini_h(), order.len() as f64 * MINI_ROW + 8.0);
    let mut pm = crate::px::new(300, w.mini_h() as u32).unwrap();
    w.draw_mini(&mut pm, &Text::default(), 300.0, Some((10.0, 5.0)), &Theme::default());
}

#[test]
fn barre_d_icones() {
    let mut w = wl();
    w.prefs.open = false;
    let h = 800.0;
    // Trading démo (2e) : son panneau remplace la watchlist ; encore = fermé
    let r = w.tool_rect(1);
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Changed);
    assert_eq!((w.prefs.open, w.prefs.panel), (true, Tool::Trading));
    let ai = crate::panels::Panels::ai_rect(w.pw());
    assert_eq!(w.click(ai.cx(), ai.cy(), h), Click::Ai);
    let r = w.tool_rect(1);
    w.click(r.cx(), r.cy(), h);
    assert!(!w.prefs.open);
    // 🧪 Démo (3e) : une action de l'app (le Simulateur), sous le bouton
    let r = w.tool_rect(2);
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Outil(Tool::Demo, r));
    // 🛒 la Marketplace ; ⚙ tout en bas : une action ; 🐋 : le panneau Baleines
    let km = w.bar_tools().iter().position(|t| *t == Tool::Marketplace).unwrap();
    let r = w.tool_rect(km);
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Marketplace);
    let r = w.tool_rect(w.bar_tools().len() - 1);
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Outil(Tool::Reglages, r));
    let k = w.bar_tools().iter().position(|t| *t == Tool::Whales).unwrap();
    let r = w.tool_rect(k);
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Changed);
    assert_eq!((w.prefs.open, w.prefs.panel), (true, Tool::Whales));
    w.click(r.cx(), r.cy(), h);
    // 🔌 à côté du ＋ : le panneau des API
    w.toggle_tool(Tool::Watchlist);
    let r = w.api_rect();
    assert_eq!(w.click(r.cx(), r.cy(), h), Click::Changed);
    assert_eq!(w.prefs.panel, Tool::Exchanges);
}

/// 🔌 de l'en-tête, de la largeur la plus petite à la plus grande : le clic ouvre toujours le panneau des API (en
/// volet étroit, le nom de la liste couvrait 🔌 et le clic ouvrait le menu des listes).
#[test]
fn prise_de_l_en_tete_a_toute_largeur() {
    for width in [MIN_W, 340, 380, 400, MAX_W] {
        let mut w = wl();
        w.prefs.width = width;
        (w.prefs.open, w.prefs.panel) = (true, Tool::Watchlist);
        let r = w.api_rect();
        let t = w.title_rect();
        assert!(t.x + t.w <= r.x, "{width} : le nom s'arrête avant 🔌");
        for x in [r.x + 1.0, r.cx(), r.x + r.w - 1.0] {
            let mut w = w.clone();
            assert_eq!(w.click(x, r.cy(), 800.0), Click::Changed, "{width}");
            assert!(w.prefs.open && w.prefs.panel == Tool::Exchanges, "{width} : 🔌 ouvre les API");
        }
        assert_eq!(
            w.click(t.x + 10.0, t.cy(), 800.0),
            Click::Lists(t),
            "{width} : le nom ouvre toujours les listes"
        );
    }
}

#[test]
fn listes_ajout_retrait_et_filtre() {
    let mut w = wl();
    let h = 800.0;
    let t = w.title_rect();
    assert_eq!(w.click(t.cx(), t.cy(), h), Click::Lists(t));
    let a = w.add_rect();
    assert_eq!(w.click(a.cx(), a.cy(), h), Click::Add(a));
    // ✕ : seulement dans une liste de l'utilisateur
    let y_btc = TOP + HEAD_H + SEC_H + 10.0;
    let x = w.remove_rect(0.0).cx();
    assert_eq!(w.click(x, y_btc, h), Click::Open("BTC".into()));
    w.editable = true;
    assert_eq!(w.click(x, y_btc, h), Click::Remove("BTC".into()));
    assert_eq!(w.click(x, TOP + HEAD_H + 15.0, h), Click::RemoveSection("Crypto".into()));
    // 🚩 voir seulement le bleu
    w.prefs.flags.push(("ETH".into(), 1));
    w.prefs.flag_filter = Some(1);
    assert_eq!(w.order(), vec!["ETH"]);
    w.prefs.flag_filter = None;
    assert_eq!(w.order().len(), 4);
    // sans prix (autre exchange) : dessin sans planter
    w.update(vec![Section {
        name: String::new(),
        rows: vec![Row::new("binance:BTCUSDT", "BTCUSDT", "", 0.0)],
    }]);
    w.draw(&Text::default(), 600, Some((100.0, TOP + HEAD_H + SEC_H + 5.0)), &Theme::default());
}

#[test]
fn nom_long_coupe_et_note() {
    let t = Text::default();
    assert_eq!(t.fit("BTC", 13.5, 200.0), "BTC");
    let cut = t.fit("UN-NOM-TRES-TRES-LONG-QUI-DEBORDE", 13.5, 80.0);
    assert!(cut.ends_with('…') && t.width(&cut, 13.5) as f64 <= 80.0);
    let mut w = wl();
    let mut r = Row::new("binance:XAUUSDT", "XAUUSDT", "3999.80", 3997.0);
    r.note = "Binance".into();
    w.update(vec![Section {
        name: String::new(),
        rows: vec![r],
    }]);
    w.draw(&t, 600, None, &Theme::default());
}

#[test]
fn panneau_qui_defile() {
    let mut w = wl();
    w.toggle_tool(Tool::Trading);
    let h = 450.0;
    assert_eq!(w.panel_off(h), 0.0);
    w.wheel(-50.0, h);
    assert!(w.glide() && w.panel_off(h) > 0.0 && w.panel_off(h) < 700.0 - h, "glisse, pas de saut");
    let mut n = 0;
    while w.glide() {
        n += 1;
        assert!(n < 60, "le glissement finit");
    }
    let off = w.panel_off(h);
    assert_eq!(off, 700.0 - h, "en bas : l'ordre et les onglets du bas sont visibles");
    // « Passer l'ordre » est à 700 − 112 dans le contenu : à l'écran, 700 − 112 − off
    assert_eq!(w.click(40.0, 700.0 - 112.0 + 20.0 - off, h), Click::Order, "ordre démo");
    w.wheel(50.0, h);
    while w.glide() {}
    assert_eq!(w.panel_off(h), 0.0);
    assert!(w.draw(&Text::default(), h as u32, None, &Theme::default()).width() > 0);
}

#[test]
fn barre_de_defilement() {
    let mut w = wl();
    let rows: Vec<Row> = (0..60).map(|i| Row::new(&format!("C{i}"), &format!("C{i}"), "1.0", 1.0)).collect();
    w.update(vec![Section { name: "Crypto".into(), rows }]);
    let h = 600.0;
    let (track, thumb) = w.scrollbar(h).expect("barre visible : 60 symboles");
    // clic en bas du rail : saute à la fin
    w.click(track.cx(), track.y + track.h - 2.0, h);
    assert!(w.dragging());
    assert!((w.scroll - w.max_scroll(h)).abs() < 1.0, "en bas de la liste");
    w.release();
    // attraper le curseur et remonter tout en haut
    let (_, t) = w.scrollbar(h).unwrap();
    w.click(t.cx(), t.cy(), h);
    w.drag_to(0.0, h);
    assert_eq!(w.scroll, 0.0);
    w.release();
    assert!(!w.dragging());
    assert!(thumb.h >= 30.0);
    // peu de symboles : pas de barre
    assert!(wl().scrollbar(h).is_none());
}

#[test]
fn chiffres_qui_changent() {
    let mut w = wl();
    let mut next = w.sections.clone();
    next[0].rows[0] = Row::new("BTC", "BTC-USDC", "80258.1", 80848.6);
    w.update(next);
    let btc = &w.sections[0].rows[0];
    assert_eq!((btc.last_mark.as_str(), btc.up), ("80261.3", false), "baisse : chiffres changés en rouge");
    // même prix au prochain rafraîchissement : la couleur reste
    let same = w.sections.clone();
    w.update(same);
    assert_eq!(w.sections[0].rows[0].last_mark, "80261.3");
    let pm = w.draw(&Text::default(), 600, None, &Theme::default());
    assert_eq!(pm.width(), PANEL + ICON_BAR);
    // logo : remplace la pastille
    let mut logo = Pixmap::new(64, 64).unwrap();
    logo.fill(tiny_skia::Color::from_rgba8(255, 0, 255, 255));
    w.logos.insert("BTC".into(), logo);
    let with = w.draw(&Text::default(), 600, None, &Theme::default());
    let y = (TOP + HEAD_H + SEC_H + ROW_H / 2.0) as u32;
    let p = with.pixel(32, y).unwrap();
    assert_eq!((p.red(), p.green(), p.blue()), (255, 0, 255), "logo de BTC dessiné");
}

/// 📱 Feuille watchlist du cell : les lignes sont dessinées (Jee, 2026-11-09 : feuille vide).
#[test]
fn cell_feuille_a_ses_lignes() {
    let mut w = wl();
    w.phone = Some(390);
    w.title = "Crypto".into();
    w.listes = vec![(0, "Hyperliquid".into()), (1, "Crypto".into()), (2, "Métaux".into()), (3, "ISO 20022".into())];
    w.liste_active = 1;
    let pm = w.draw(&Text::default(), 434, None, &Theme::default());
    // l'en-tête du cell : ⋯ ouvre les listes, 🔎 cherche / ajoute, une pilule choisit sa liste
    let (dots, _, loupe) = w.entete_cell();
    assert!(matches!(w.click(dots.cx(), dots.cy(), 434.0), Click::Lists(_)));
    assert!(matches!(w.click(loupe.cx(), loupe.cy(), 434.0), Click::Add(_)));
    let p = w.pilules_cell(&Text::default());
    assert_eq!(w.click(p[2].0.cx(), p[2].0.cy(), 434.0), Click::Liste(2));
    // la mini-pilule du symbole ouvert : referme la feuille
    let m = w.mini_cell(434.0);
    assert_eq!(w.click(m.cx(), m.cy(), 434.0), Click::FermerFeuille);
    // le nom d'une section de ta liste : son menu ; ⌄ à droite : la replier
    w.editable = true;
    let ys = w.haut_liste() + 10.0;
    assert!(matches!(w.click(60.0, ys, 434.0), Click::SectionMenu(n, _) if n == "Crypto"));
    assert_eq!(w.click(w.pw() - 20.0, ys, 434.0), Click::Changed);
    w.prefs.collapsed.clear();
    w.editable = false;
    if let Ok(dir) = std::env::var("ARGOS_CAPTURES") {
        pm.save_png(format!("{dir}/cell-feuille-watchlist.png")).unwrap();
    }
    // une ligne de symbole (y ≈ TOP + HEAD_H + …) : des pixels plus clairs que le fond
    let fond = Theme::default().chrome;
    let clair = (130..300u32)
        .flat_map(|y| (10..380u32).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let p = pm.pixel(crate::px::dev(x as f64) as u32, crate::px::dev(y as f64) as u32).unwrap();
            (p.red() as i32 - fond.0 as i32).abs() + (p.green() as i32 - fond.1 as i32).abs() > 120
        })
        .count();
    assert!(clair > 200, "lignes dessinées : {clair} pixels de texte");
}

/// 💲 The Symboles panel writes the prices like the watchlist (Jee, captures 202611221633: « 61373.0 », « 1723.2 »):
/// the number format's comma and thousands space, the exchange's decimals; and the 24 h change of the row.
#[test]
fn prix_comme_la_watchlist() {
    let r = Row::new("BTC", "BTC", "61373.0", 60_000.0);
    let p = r.prix_affiche();
    assert!(p.contains(',') && !p.contains('.') && p.starts_with("61"), "{p}");
    assert_eq!(p, format_price(61_373.0, 1));
    assert!((r.variation().unwrap() - 2.288_333).abs() < 1e-4);
    assert_eq!(Row::new("ETH", "ETH", "", 1.0).prix_affiche(), "");
    assert_eq!(Row::new("ETH", "ETH", "1723.2", 0.0).variation(), None, "no price 24 h ago");
}

/// 📶📱 Stats Hyperliquid by touch: the bottom of the sheet keeps the round Argos AI button's room at every scroll
/// (capture 202611222137-15: it covered the last row's « 24 h »): a band of the window's colour, no row clicked
/// under it; nothing on the PC.
#[test]
fn stats_bas_libre_pour_le_rond_ai() {
    let mut w = wl();
    w.prefs.open = true;
    w.prefs.panel = Tool::Stats;
    assert_eq!(w.bande_ai(800.0).h, 0.0, "PC: nothing kept");
    crate::actions::set_tactile(true);
    let b = w.bande_ai(800.0);
    assert_eq!((b.y, b.h), (800.0 - crate::eco::marge_ai(), crate::eco::marge_ai()));
    assert_eq!(w.click(b.x + 20.0, b.cy(), 800.0), Click::None);
    w.prefs.panel = Tool::News;
    assert_eq!(w.bande_ai(800.0).h, 0.0, "the Stats sheet only");
    // 📖 the Carnet: on the phone, its sheet's footer and ⚙ above the round Argos AI; not in the PC's volet
    w.prefs.panel = Tool::Carnet;
    assert_eq!(w.bande_ai(800.0).h, 0.0, "a volet, not the phone");
    w.phone = Some(390);
    assert_eq!(w.bande_ai(800.0).h, b.h, "the Stats sheet's room");
    // 📖 and, by touch, the sheet is never scrolled, even lying down (see `le_carnet_ne_defile_jamais`)
    w.panel_scroll = 10_000.0;
    for h in [130.0, 208.0, 242.0, 400.0] {
        assert_eq!(w.panel_off(h), 0.0, "by touch, {h}");
    }
    crate::actions::set_tactile(false);
}

/// 📖 The Carnet's volet and the phone's sheet are never scrolled, upright or lying down: the panel takes the height
/// it is given, its footer and ⚙ at its bottom (the control's review, 25 Nov: lying down, a sheet of about 210 points
/// scrolled and its footer and ⚙ went below the scroll, B-020 again).
#[test]
fn le_carnet_ne_defile_jamais() {
    let mut w = wl();
    w.prefs.open = true;
    w.prefs.panel = Tool::Carnet;
    w.panel_scroll = 10_000.0;
    for phone in [None, Some(390)] {
        w.phone = phone;
        for h in [130.0, 208.0, 242.0, 400.0, 622.0, 900.0] {
            assert_eq!(w.panel_off(h), 0.0, "{phone:?} {h}");
        }
    }
}
