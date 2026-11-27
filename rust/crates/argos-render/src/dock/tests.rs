fn serie(n: usize, f: impl Fn(usize) -> f64) -> Vec<argos_core::Bar> {
    (0..n)
        .map(|i| {
            let c = f(i);
            argos_core::Bar {
                time: i as i64 * 3600,
                open: c,
                high: c * 1.01,
                low: c * 0.99,
                close: c,
                volume: 1.0,
            }
        })
        .collect()
}

#[test]
fn indicateurs_cles_hausse_et_baisse() {
    let up = super::key_stats(&serie(120, |i| 100.0 + i as f64), 3600);
    assert_eq!(up.len(), 4, "{up:?}");
    assert!(up[0].1.contains("surachat"), "{up:?}");
    assert!(up[1].1.starts_with("Golden cross"), "{up:?}");
    assert!(up[2].1.ends_with('%') && up[3].1.ends_with('%'));
    let down = super::key_stats(&serie(120, |i| 300.0 - i as f64), 3600);
    assert!(down[0].1.contains("survente") && down[1].1.starts_with("Death cross"), "{down:?}");
}

#[test]
fn surveillance_prix_24h_et_carnet_compact() {
    use argos_data::book::{Book, Level};
    let bars = serie(48, |i| 100.0 + i as f64);
    let book = Book {
        coin: "BTC".into(),
        time_ms: 0,
        bids: (0..5)
            .map(|k| Level {
                px: 146.0 - k as f64,
                sz: 1.0,
                n: 1,
            })
            .collect(),
        asks: (0..5)
            .map(|k| Level {
                px: 148.0 + k as f64,
                sz: 2.0,
                n: 1,
            })
            .collect(),
    };
    let r = super::watch_rows(&bars, 3600, None, Some(&book), 1);
    let labels: Vec<&str> = r.iter().map(|x| x.0.as_str()).collect();
    assert_eq!(
        labels[..5],
        ["Prix", "Variation 24 h", "Plus haut 24 h", "Plus bas 24 h", "Volume 24 h"],
        "{labels:?}"
    );
    // the French decimal comma, as everywhere else in the app (Jee, 22 Nov: « +1.59 % »)
    let var = &r[1].1;
    assert!(var.ends_with(" %") && var.contains(',') && !var.contains('.'), "{var}");
    assert_eq!(r.len(), 5 + 2 * super::WATCH_DEPTH + 1);
    assert!(
        labels[5].starts_with("Vente") && labels[8] == "Écart" && labels[9].starts_with("Achat"),
        "{labels:?}"
    );
    // 💬 the French comma everywhere in the card (Jee, captures 202611221633: « 3.04 G$ », « 0.002 % »)
    let ecart = &r[8].1;
    assert!(ecart.ends_with(" %") && ecart.contains(',') && !ecart.contains('.'), "{ecart}");
    // la meilleure vente juste au-dessus de l'écart
    assert!(labels[7].contains("148"), "{labels:?}");
    // prix de l'exchange en priorité
    let live = super::Live {
        mark: 150.0,
        prev: 120.0,
        volume: 2.5e9,
    };
    let r = super::watch_rows(&bars, 3600, Some(live), None, 1);
    assert!(r[1].1 == "+25,00 %" && r[4].1 == "2,50 G$", "{r:?}");
    assert!(super::watch_rows(&[], 3600, None, None, 1).is_empty());
}

#[test]
fn scanner_alignes_en_premier() {
    let l = |c: &str, dir: i8, rr: Option<f64>| super::ScanLine {
        coin: c.into(),
        dir,
        rr,
        ex: "Binance".into(),
        also: if c == "BTC" { vec!["OKX".into()] } else { vec![] },
    };
    let r = super::scan_rows(&[l("ADA", 0, None), l("ETH", 2, None), l("BTC", 1, Some(2.4)), l("SOL", -1, None)]);
    let coins: Vec<&str> = r.iter().map(|x| x.0.as_str()).collect();
    assert_eq!(coins, ["BTC · Binance (+ OKX)", "SOL · Binance", "ETH · Binance", "ADA · Binance"]);
    assert_eq!(r[0].1, "▲ LONG · R:R 2.4");
    assert_eq!(r[1].1, "▼ SHORT");
}

#[test]
fn carte_dex() {
    use argos_data::dex::{PoolInfo, Trade};
    let info = PoolInfo {
        name: "PEPE / WETH".into(),
        base_addr: String::new(),
        price_usd: Some(0.00000505),
        liq_usd: Some(33e6),
        vol_24h: Some(4.2e6),
        chg_24h: Some(1.5),
        buys: 647,
        sells: 787,
        buyers: 274,
        sellers: 264,
        ..Default::default()
    };
    let t = |s: i64, buy: bool| Trade {
        time_s: s,
        buy,
        usd: 1500.0,
        price_usd: 0.000005,
    };
    let r = super::dex_rows("PEPE/WETH", Some(&info), &[t(3 * 3600 + 5 * 60 + 23, false), t(10, true)], 0);
    let labels: Vec<&str> = r.iter().map(|x| x.0.as_str()).collect();
    assert_eq!(
        labels[..7],
        [
            "Paire",
            "Prix",
            "Variation 24 h",
            "Liquidité",
            "Volume 24 h",
            "Achats / ventes 24 h",
            "Acheteurs / vendeurs"
        ]
    );
    assert_eq!(r[6].1, "274 / 264");
    assert_eq!(labels[7], "▼ 03:05:23");
    assert_eq!(super::dex_rows("X", None, &[], 0).len(), 1);
}

#[test]
fn indicateurs_cles_vides_sans_bougies() {
    assert!(super::key_stats(&[], 3600).is_empty());
    assert!(super::key_stats(&serie(5, |_| 10.0), 3600).len() <= 1);
}

use super::*;

#[test]
fn colonne_de_modules() {
    let mut p = DockPrefs::default();
    assert!(!p.active(1600), "masquée au départ");
    p.on = true;
    assert!(p.active(1600) && !p.active(1000), "grand écran seulement");
    // carnet (Remplir) + trades (Moyen) + funding (Étroit) dans 800 px
    let l = p.layout(320.0, 800.0);
    assert_eq!(l.iter().map(|(m, _)| *m).collect::<Vec<_>>(), [Module::Book, Module::Trades, Module::Funding]);
    assert_eq!(l[1].1.h, 240.0);
    assert_eq!(l[0].1.h, 800.0 - 240.0 - 140.0 - 8.0, "Remplir prend le reste");
    assert!(l.windows(2).all(|w| w[1].1.y > w[0].1.y + w[0].1.h - 0.1), "empilés sans se chevaucher");
    // trop petit : tout est réduit pareil, rien ne dépasse
    let l = p.layout(320.0, 300.0);
    assert!(l.last().map(|(_, r)| r.y + r.h).unwrap() <= 300.0 + 0.1);
    // en bas : côte à côte
    p.pos = DockPos::Bottom;
    let l = p.layout(1200.0, 260.0);
    assert!(l.iter().all(|(_, r)| r.h == 260.0) && l[1].1.x > l[0].1.x);
    // réglages : cocher, taille, ordre
    let t = Text::default();
    let (r, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| *z == Zone::Check(3)).unwrap();
    assert!(click(&mut p, &t, 390.0, r.cx(), r.cy()));
    assert!(p.items[3].on, "watchlist cochée");
    // (défaut de Jee : colonne « Masquée », cocher ne montrait rien) : cocher un module ou choisir un côté l'allume
    p.on = false;
    let (r, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| *z == Zone::Check(4)).unwrap();
    click(&mut p, &t, 390.0, r.cx(), r.cy());
    assert!(p.items[4].on && p.on, "cocher allume la colonne");
    p.on = false;
    let (r, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| matches!(z, Zone::Pos(_))).unwrap();
    click(&mut p, &t, 390.0, r.cx(), r.cy());
    assert!(p.on, "choisir un côté l'allume");
    // les noms des modules se lisent en entier dans un volet de 360 (sur deux lignes au besoin)
    for it in &p.items {
        let (_, nom) = it.module.face();
        let lignes = nom_module(&t, nom, 360.0);
        let tout: Vec<&str> = lignes.iter().map(|l| l.1.as_str()).collect();
        assert_eq!(tout.join(" "), nom, "« {nom} » coupé");
    }
    click(&mut p, &t, 390.0, r.cx(), r.cy());
    // (on remet le module 4 comme avant pour la suite du test)
    let (r4, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| *z == Zone::Check(4)).unwrap();
    click(&mut p, &t, 390.0, r4.cx(), r4.cy());
    let (r, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| *z == Zone::Up(3)).unwrap();
    click(&mut p, &t, 390.0, r.cx(), r.cy());
    assert_eq!(p.items[2].module, Module::Watchlist, "montée d'un rang");
    let (r, _) = zones(&p, &t, 390.0).into_iter().find(|(_, z)| *z == Zone::ModSize(0)).unwrap();
    click(&mut p, &t, 390.0, r.cx(), r.cy());
    assert_eq!(p.items[0].size, ModSize::Small, "Remplir → Étroit (on fait le tour)");
    for (r, _) in zones(&p, &t, 390.0) {
        assert!(r.x + r.w <= 390.0 - PAD + 0.1, "{r:?}");
    }
    // vieux fichier : module manquant ajouté, doublon enlevé
    let mut old: DockPrefs = serde_json::from_str(
        r#"{"on":true,"pos":"Left","size":9,"items":[{"module":"Trades","on":true,"size":"Fill"},{"module":"Trades","on":false,"size":"Small"}]}"#,
    )
    .unwrap();
    old.fix();
    assert_eq!((old.items.len(), old.size, old.items[0].size), (Module::ALL.len(), 3, ModSize::Fill));
    // dessin
    let (pm, bodies) = frame(&p, &t, 1200, 260, None, &Theme::default()).unwrap();
    assert_eq!((pm.width(), bodies.len()), (1200, 4));
    // 🌍 😱 💥 nouveaux modules
    let tiles: Vec<argos_data::world::Tile> = (0..8)
        .map(|k| argos_data::world::Tile {
            name: format!("Actif {k}"),
            price: 1000.0 + k as f64,
            chg_pct: k as f64 - 4.0,
            spark: vec![1.0, 2.0, 1.5, 3.0],
        })
        .collect();
    let mut pm = crate::px::new(320, world_h(8, 320.0) as u32).unwrap();
    draw_world(&mut pm, &t, 320.0, &tiles, "", &Theme::default());
    draw_world(&mut pm, &t, 320.0, &[], "", &Theme::default());
    let mi = argos_data::market::MarketInfo {
        fng: Some(argos_data::market::Fng {
            value: 72,
            label: "Greed".into(),
            yesterday: Some(60),
            week_ago: Some(40),
        }),
        ..Default::default()
    };
    let mut pm = crate::px::new(320, sentiment_h() as u32).unwrap();
    draw_sentiment(&mut pm, &t, 320.0, &mi, &Theme::default());
    let liqs = vec![argos_data::liq::Liq {
        coin: "ETH".into(),
        time_ms: 1000,
        price: 4000.0,
        qty: 30.0,
        long: true,
        venue: "Binance",
    }];
    let mut pm = crate::px::new(320, liqs_h(1) as u32).unwrap();
    draw_liqs(&mut pm, &t, 320.0, &liqs, 2000, 0, &Theme::default());
    let mut s = crate::px::new(390, content_h(&p) as u32).unwrap();
    draw_settings(&p, &mut s, &t, 390.0, false, None, &Theme::default());
}

/// 📖 ⚡ 💸 Carnet, trades et funding se dessinent (avec et sans données).
#[test]
fn carnet_trades_funding() {
    use argos_data::book::{Book, Funding, Level};
    let t = Text::default();
    let th = Theme::default();
    let lv = |px: f64, sz: f64| Level { px, sz, n: 1 };
    let b = Book {
        coin: "BTC".into(),
        time_ms: 0,
        bids: (0..30).map(|k| lv(80000.0 - k as f64, 0.5 + k as f64 * 0.1)).collect(),
        asks: (0..30).map(|k| lv(80001.0 + k as f64, 0.3 + k as f64 * 0.2)).collect(),
    };
    let mut pm = crate::px::new(320, 400).unwrap();
    for src in [
        CarnetSource::Hyperliquid,
        CarnetSource::MemePiece,
        CarnetSource::Indisponible("Carnet indisponible pour KuCoin".into()),
    ] {
        draw_book(&mut pm, &t, 320.0, 400.0, Some(&b), 1, &th, &src);
        draw_book(&mut pm, &t, 320.0, 400.0, None, 1, &th, &src);
    }
    let trades: Vec<argos_data::whales::Whale> = (0..20)
        .map(|k| argos_data::whales::Whale {
            coin: "BTC".into(),
            time_ms: 1_793_050_000_000 - k * 900,
            price: 80000.0 + k as f64,
            size: 0.01 * (k + 1) as f64,
            buy: k % 3 != 0,
            src: "Hyperliquid",
            id: None,
            wallet: String::new(),
        })
        .collect();
    let mut pm = crate::px::new(320, tape_h(trades.len(), false) as u32).unwrap();
    draw_tape(&mut pm, &t, 320.0, &trades, 1, -4 * 3600, Some(false), ("Binance", None), &th);
    draw_tape(&mut pm, &t, 320.0, &trades, 1, -4 * 3600, Some(true), ("Binance", None), &th);
    draw_tape(&mut pm, &t, 320.0, &[], 1, -4 * 3600, None, ("Hyperliquid", Some("OANDA")), &th);
    // 📸 ARGOS_TRADES_PNG=folder: the module on Binance, then the announced fallback, to look at them
    if let Ok(dir) = std::env::var("ARGOS_TRADES_PNG") {
        for (nom, source, liste) in [
            ("bourse", ("Binance", None), &trades[..6]),
            ("repli", ("Hyperliquid", Some("BloFin")), &trades[..6]),
        ] {
            let mut pm = crate::px::new(320, tape_h(6, source.1.is_some()) as u32).unwrap();
            pm.fill(tiny_skia::Color::from_rgba8(0x14, 0x16, 0x1c, 255));
            draw_tape(&mut pm, &t, 320.0, liste, 1, -4 * 3600, Some(false), source, &th);
            pm.save_png(format!("{dir}/trades-{nom}.png")).unwrap();
        }
    }
    assert!(
        tape_rec(320.0).y + tape_rec(320.0).h <= tape_h(0, false) - 24.0,
        "le bouton tient dans sa ligne"
    );
    assert_eq!(tape_h(0, true), tape_h(0, false) + REPLI_H, "the fallback line has its own place");
    assert_eq!(tape_repli_txt("BloFin"), "BloFin : pas de flux de trades pour cette pièce");
    let hist: Vec<Funding> = (0..24)
        .map(|k| Funding {
            time_ms: k * 3_600_000,
            rate: (k as f64 - 8.0) * 1e-6,
        })
        .collect();
    let mut pm = crate::px::new(320, 260).unwrap();
    draw_funding_bars(&mut pm, &t, 320.0, 160.0, 86.0, &hist, &th);
}

/// 🖥 Colonne PC : 5 puis 10 modules à 900 de haut, rien n'est écrasé (chaque module ≥ titre + une ligne, les fiches
/// gardent tout leur contenu), le total dépasse la colonne et elle défile jusqu'au dernier ; Remplir prend le reste
/// quand il y en a ; une hauteur réglée à la main l'emporte (jamais sous le minimum).
#[test]
fn colonne_pc_defile_sans_rien_ecraser() {
    use super::*;
    let contenu = |m: Module| match m {
        Module::Book => 0.0,
        Module::Funding => 246.0,
        Module::Positions => 36.0,
        Module::Trades | Module::News | Module::Whales => 900.0,
        _ => 200.0,
    };
    let sans = |_: Module| None;
    for n in [5usize, 10] {
        let mut p = DockPrefs::default();
        for (k, i) in p.items.iter_mut().enumerate() {
            i.on = k < n;
        }
        let (places, total) = p.layout_pc(320.0, 900.0, &contenu, &sans);
        assert_eq!(places.len(), n);
        for (m, r) in &places {
            assert!(r.h >= hauteur_min(), "{m:?} au moins titre + une ligne");
            if !m.liste() && contenu(*m) > 0.0 {
                assert!(r.h >= HEAD_H + contenu(*m), "{m:?} : tout son contenu");
            }
        }
        for w in places.windows(2) {
            assert!(w[1].1.y >= w[0].1.y + w[0].1.h + POIGNEE - 0.01, "pas de chevauchement");
        }
        assert!(total > 900.0, "{n} modules : plus haut que la colonne ({total})");
        let max = defil_max(900.0, total);
        let (_, der) = places.last().unwrap();
        assert!((der.y + der.h - max - 900.0).abs() < 0.01, "le dernier module arrive en bas");
        assert!(barre_defil(DockPos::Right, 320.0, 900.0, total, max).is_some_and(|b| b.y + b.h <= 900.01));
    }
    // 2 modules : Remplir (le carnet) prend le reste, rien ne défile
    let mut p = DockPrefs::default();
    for i in p.items.iter_mut() {
        i.on = matches!(i.module, Module::Book | Module::Positions);
    }
    let (places, total) = p.layout_pc(320.0, 900.0, &contenu, &sans);
    assert!((total - 900.0).abs() <= 2.0, "toute la colonne ({total})");
    assert_eq!(places[1].1.h, HEAD_H + 36.0, "Positions démo : la hauteur de son contenu, pas plus");
    assert!(barre_defil(DockPos::Right, 320.0, 900.0, total, 0.0).is_none());
    // hauteur à la main : prise, jamais sous le minimum
    let (pl, _) = p.layout_pc(320.0, 900.0, &contenu, &|m| (m == Module::Positions).then_some(300.0));
    assert_eq!(pl[1].1.h, 300.0);
    let (pl, _) = p.layout_pc(320.0, 900.0, &contenu, &|m| (m == Module::Positions).then_some(5.0));
    assert_eq!(pl[1].1.h, hauteur_min());
}

/// 🖥 Glisser un titre : la place où il tombe, et l'ordre changé comme les ↑ ↓ (les modules décochés restent).
#[test]
fn glisser_un_titre_change_l_ordre() {
    use super::*;
    let mut p = DockPrefs::default();
    let avant: Vec<Module> = p.shown().iter().map(|i| i.module).collect();
    assert_eq!(avant, vec![Module::Book, Module::Trades, Module::Funding]);
    let contenu = |_: Module| 100.0;
    let (places, _) = p.layout_pc(320.0, 900.0, &contenu, &|_| None);
    // le carnet glissé sous le funding : 3e place
    let fin = places[2].1.y + places[2].1.h - 2.0;
    let (i, ligne) = depot(&places, Module::Book, 0.0, fin);
    assert_eq!(i, 2);
    assert!(ligne > places[2].1.y);
    p.deplacer(Module::Book, i);
    let apres: Vec<Module> = p.shown().iter().map(|i| i.module).collect();
    assert_eq!(apres, vec![Module::Trades, Module::Funding, Module::Book]);
    assert_eq!(p.items.len(), Module::ALL.len(), "les modules décochés restent");
    // retour en tête
    p.deplacer(Module::Book, 0);
    assert_eq!(p.shown()[0].module, Module::Book);
    // poignée et titre se trouvent
    let (places, _) = p.layout_pc(320.0, 900.0, &contenu, &|_| None);
    let r = places[0].1;
    assert_eq!(poignee_a(&places, 0.0, r.y + r.h + 3.0), Some(Module::Book));
    assert_eq!(titre_a(&places, 0.0, 50.0, r.y + 10.0), Some(Module::Book));
    assert_eq!(titre_a(&places, 0.0, 50.0, r.y + 60.0), None, "le contenu n'est pas le titre");
}

/// 🧩 Volet Modules : les 3 côtés (Gauche, Droite, Sous la chart) sont toujours là, à la largeur la plus petite du volet
/// comme à la plus grande (passés à la ligne s'il le faut), rien ne se croise, et un clic sur Droite met la colonne à
/// droite.
#[test]
fn boutons_de_cote_toujours_la() {
    use super::*;
    let t = Text::default();
    let croise = |a: &Rect, b: &Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
    for pw in [crate::watchlist::MIN_W as f64, 360.0, 480.0, 720.0] {
        for on in [true, false] {
            let mut p = DockPrefs {
                on,
                pos: DockPos::Left,
                ..DockPrefs::default()
            };
            let z = zones(&p, &t, pw);
            let haut: Vec<(Rect, Zone)> = z.iter().copied().filter(|(_, z)| matches!(z, Zone::On | Zone::Pos(_))).collect();
            assert_eq!(haut.len(), 4, "{pw} : les 4 boutons");
            for (r, k) in &haut {
                assert!(r.x >= PAD - 0.1 && r.x + r.w <= pw - PAD + 0.1, "{pw} : {k:?} dans le volet");
            }
            for (i, (a, ka)) in z.iter().enumerate() {
                for (b, kb) in z.iter().skip(i + 1) {
                    assert!(!croise(a, b), "{pw} : {ka:?} croise {kb:?}");
                }
            }
            let (r, _) = haut.iter().find(|(_, k)| *k == Zone::Pos(DockPos::Right)).unwrap();
            assert!(click(&mut p, &t, pw, r.cx(), r.cy()));
            assert_eq!((p.pos, p.on), (DockPos::Right, true), "{pw} : Droite");
        }
    }
}

/// A value too long for its row is cut (…), never written over its name (the 🧪 card's « Réglages » on a narrow dock).
#[test]
fn valeur_trop_longue_coupee_sans_couvrir_le_nom() {
    let t = crate::text::Text::default();
    let mut pm = crate::px::new(340, 60).unwrap();
    let long = "capital 10 000 $ · 10 % du compte · frais 0.05 % · glissement 2 ticks · pyramiding 3 · ordres à la clôture";
    let rows = vec![("Réglages".to_string(), long.to_string(), None), ("Trades".to_string(), "12".to_string(), None)];
    let _ = crate::text::ecrits();
    super::draw_kv(&mut pm, &t, 340.0, &rows, "");
    let e = crate::text::ecrits();
    assert_eq!(e[0], "Réglages", "the name whole: {e:?}");
    assert!(e[1].ends_with('…') && e[1].starts_with("capital 10 000 $"), "{e:?}");
    let place = 340.0 - 26.0 - t.width("Réglages", 12.0) as f64;
    assert!(t.width(&e[1], 12.5) as f64 <= place, "the value stays right of its name");
    assert_eq!((e[2].as_str(), e[3].as_str()), ("Trades", "12"), "a short value as before");
}

/// 📜 A strategy's card in the dock's medium column (320 px): its own rows are whole, none cut by « … » (the
/// balance curve and the last trades may be, as for every strategy).
#[test]
fn carte_d_une_strategie_lisible_sans_couper() {
    use argos_core::backtest::{rapport, Hasard};
    use argos_core::Money;
    let v = super::BtView {
        name: "📜 Exemple EMA 9 / 21".into(),
        settings: "10 000 $ · 10 % du compte".into(),
        frais: "0.05 % · glissement 2 ticks · pyramiding 3".into(),
        symbol: "BTC".into(),
        bars: 5000,
        start: 10_000.0,
        report: rapport(10_000.0, Money::from_f64(10_000.0), vec![], &[]),
        buy_hold_pct: Some(-21.79),
        hasard: Some(Hasard {
            tirages: 1000,
            rang: 464,
            battus_pct: 53.6,
            mediane: Money::ZERO,
            meilleur: Money::ZERO,
            pire: Money::ZERO,
        }),
        script: true,
        detail: Default::default(),
        ouverte: Some(-3.25),
    };
    let rows = super::bt_rows(&v, 0);
    let w = super::COL_W[1];
    let t = crate::text::Text::default();
    let mut pm = crate::px::new(w as u32, super::bt_h() as u32).unwrap();
    let _ = crate::text::ecrits();
    super::draw_kv(&mut pm, &t, w, &rows, "");
    let e = crate::text::ecrits();
    for nom in [
        "Stratégie",
        "Capital · taille",
        "Frais",
        "Position ouverte",
        "Buy & hold",
        "🎲 Contre le hasard",
        "Simulation",
    ] {
        let (_, valeur, _) = rows.iter().find(|r| r.0 == nom).unwrap_or_else(|| panic!("a row « {nom} »"));
        assert!(e.contains(&nom.to_string()) && e.contains(valeur), "« {nom} » · « {valeur} » whole: {e:?}");
    }
    let hasard = rows.iter().find(|r| r.0 == "🎲 Contre le hasard").unwrap();
    assert_eq!(hasard.1, "mieux que 54 % des tirages", "the share only: the rank said it again");
}
