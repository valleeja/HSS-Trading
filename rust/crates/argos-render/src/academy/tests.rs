use super::*;

/// Clique au milieu de la zone `hit` (défile d'abord pour qu'elle soit visible).
fn cliquer(p: &mut AcademyPage, t: &Text, w: f64, hit: Hit) -> AcClick {
    let (_, zones, _) = p.build(t, w);
    let r = zones.iter().find(|(_, z)| *z == hit).unwrap_or_else(|| panic!("zone {hit:?}")).0;
    p.scroll = (r.y - 100.0).max(0.0);
    p.click(t, w, 800.0, r.x + 5.0, r.y - p.scroll + HEAD + 5.0)
}

#[test]
fn accueil_parcours_et_bientot() {
    let t = Text::default();
    let mut p = AcademyPage::new(Progress::default());
    let (_, zones, _) = p.build(&t, 1200.0);
    let ouverts: Vec<u8> = zones
        .iter()
        .filter_map(|(_, z)| match z {
            Hit::Parcours(n) => Some(*n),
            _ => None,
        })
        .collect();
    assert_eq!(ouverts, [0, 1, 2, 5, 6], "les autres parcours sont « bientôt »");
    assert!(zones.iter().any(|(_, z)| *z == Hit::Lecon(0)), "▶ Reprendre : la première leçon");
    assert_eq!(cliquer(&mut p, &t, 1200.0, Hit::Parcours(2)), AcClick::None);
    assert_eq!(p.view, View::Parcours(2));
    let (_, zones, _) = p.build(&t, 1200.0);
    assert_eq!(zones.iter().filter(|(_, z)| matches!(z, Hit::Lecon(_))).count(), 8);
    assert!(p.draw(&t, 1200, 800, (0.0, 0.0)).is_some());
    assert!(!p.escape() && p.view == View::Home && p.escape());
}

#[test]
fn lecon_quiz_score_et_progression() {
    let t = Text::default();
    let w = 900.0;
    let mut p = AcademyPage::new(Progress::default());
    let li = argos_academy::index("2-02-bos").unwrap();
    assert_eq!(p.open_guide(li), AcClick::Changed, "lue");
    let l = &lecons()[li];
    // 3 bonnes, 2 fausses : pas terminée, mais le score est gardé
    let mut last = AcClick::None;
    for (qi, q) in l.quiz.iter().enumerate() {
        let c = if qi < 3 { q.bonne } else { (q.bonne + 1) % q.choix.len() };
        last = cliquer(&mut p, &t, w, Hit::Choice(qi, c));
    }
    assert_eq!(last, AcClick::Changed);
    assert_eq!(p.progress.scores[&l.id], 3);
    assert!(!p.progress.is_done(l));
    // 🔁 recommencer puis tout juste : terminée
    cliquer(&mut p, &t, w, Hit::Recommencer);
    assert!(p.answers.iter().all(Option::is_none));
    for (qi, q) in l.quiz.iter().enumerate() {
        last = cliquer(&mut p, &t, w, Hit::Choice(qi, q.bonne));
    }
    assert_eq!(last, AcClick::Changed);
    assert!(p.progress.is_done(l) && p.progress.scores[&l.id] == 5);
    assert!(p.draw(&t, 900, 700, (0.0, 0.0)).is_some());
    // Échap : leçon → parcours → accueil → fermer
    assert!(!p.escape());
    assert_eq!(p.view, View::Parcours(2));
}

#[test]
fn blocs_repliables_et_fuseau() {
    let t = Text::default();
    let mut p = AcademyPage::new(Progress::default());
    p.open_guide(argos_academy::index("1-01-tete-et-epaules").unwrap());
    let (_, zones, h0) = p.build(&t, 1000.0);
    let d = zones
        .iter()
        .find_map(|(_, z)| matches!(z, Hit::Details(..)).then_some(*z))
        .expect("bloc repliable");
    cliquer(&mut p, &t, 1000.0, d);
    assert!(p.build(&t, 1000.0).2 > h0, "ouvert : plus haut");
    cliquer(&mut p, &t, 1000.0, d);
    assert_eq!(p.build(&t, 1000.0).2, h0);
    // 🕘 le bouton du fuseau fait le tour et se garde
    let f = bouton_fuseau(1000.0);
    assert_eq!(p.click(&t, 1000.0, 800.0, f.cx(), f.cy()), AcClick::Changed);
    assert_eq!(p.progress.fuseau, Fuseau::Montreal);
    // une leçon avec des heures affiche la phrase du fuseau
    p.open_guide(argos_academy::index("6-08-range-asiatique").unwrap());
    let (ops, _, _) = p.build(&t, 1000.0);
    assert!(ops.iter().any(|o| matches!(o, Op::Text(s, ..) if s.starts_with("🕘 Heures de New York"))));
}

#[test]
fn fiche_news_tableaux_pc_et_telephone() {
    let li = argos_academy::index(NEWS_GUIDE).expect("fiche news");
    let l = &lecons()[li];
    assert_eq!(l.tableau(1).unwrap().len(), 15, "en-tête + 14 annonces");
    assert!(l.tableau(1).unwrap().iter().all(|r| r.len() == 5));
    assert_eq!(l.tableau(2).unwrap().len(), 4, "en-tête + 3 tiers");
    let t = Text::default();
    for w in [1200.0, 390.0] {
        let mut p = AcademyPage::new(Progress::default());
        p.open_at(NEWS_GUIDE, 1, 6, &t, w);
        let (_, zones, full) = p.build(&t, w);
        let rows: Vec<Rect> = zones.iter().filter(|(_, z)| matches!(z, Hit::Row(1, _))).map(|(r, _)| *r).collect();
        assert_eq!(rows.len(), 14, "largeur {w}");
        assert!(rows.windows(2).all(|r| r[1].y >= r[0].y + r[0].h - 0.1), "lignes sans chevauchement ({w})");
        assert!(rows.iter().all(|r| r.x >= PAD - 0.1 && r.x + r.w <= w - PAD + 0.1), "dans la page ({w})");
        assert!(p.scroll > 0.0 && p.scroll < full, "défilé jusqu'à la ligne ({w})");
        assert!(p.draw(&t, w as u32, 800, (0.0, 0.0)).is_some());
    }
    let r = guide_text(NEWS_GUIDE).unwrap();
    assert!(r.contains("| 🆘 Jobless Claims | Nouvelles demandes de chômage | chaque jeudi, 8h30 | ⬇ | ⬆ |"));
    assert!(r.contains("Pas un conseil financier."));
}

#[test]
fn chaque_lecon_se_dessine() {
    let t = Text::default();
    for i in 0..lecons().len() {
        for w in [1200.0, 390.0] {
            let mut p = AcademyPage::new(Progress::default());
            p.open_guide(i);
            let (_, zones, full) = p.build(&t, w);
            assert!(full > 300.0, "{}", lecons()[i].id);
            assert_eq!(zones.iter().filter(|(_, z)| matches!(z, Hit::Choice(..))).count(), lecons()[i].quiz.len() * 3);
        }
    }
}

#[test]
fn annonces_du_calendrier_vers_la_bonne_ligne() {
    let cases = [
        ("Core CPI m/m", 0),
        ("PPI m/m", 1),
        ("Core PCE Price Index m/m", 2),
        ("Non-Farm Employment Change", 3),
        ("Unemployment Rate", 4),
        ("Average Hourly Earnings m/m", 5),
        ("Unemployment Claims", 6),
        ("JOLTS Job Openings", 7),
        ("Retail Sales m/m", 8),
        ("ISM Services PMI", 9),
        ("Prelim UoM Consumer Sentiment", 10),
        ("Prelim UoM Inflation Expectations", 11),
        ("Advance GDP q/q", 12),
        ("Federal Funds Rate", 13),
        ("FOMC Press Conference", 13),
    ];
    let t = lecons()[argos_academy::index(NEWS_GUIDE).unwrap()].tableau(1).unwrap();
    for (title, row) in cases {
        assert_eq!(news_row(title), Some(row), "{title}");
        assert!(t.get(row + 1).is_some());
    }
    assert_eq!(news_row("Crude Oil Inventories"), None);
}

/// 📸 Captures de l'Académie pour Jee : seulement si `ARGOS_CAPTURES` donne un dossier (sinon rien).
#[test]
fn captures_pour_jee() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    let t = Text::default();
    let mut prog = Progress::default();
    for id in [
        "0-01-comment-naviguer-dans-hss-trading",
        "0-02-savoir-dans-quoi-tu-t-embarques",
        "2-01-hh-hl-lh-ll",
    ] {
        let l = &lecons()[argos_academy::index(id).unwrap()];
        prog.lire(l);
        prog.noter(l, 5);
    }
    let save = |p: &AcademyPage, nom: &str, w: u32, h: u32| {
        p.draw(&t, w, h, (0.0, 0.0)).unwrap().save_png(format!("{dir}/{nom}.png")).unwrap();
    };
    let mut p = AcademyPage::new(prog.clone());
    save(&p, "01-accueil", 1280, 860);
    p.view = View::Parcours(1);
    save(&p, "02-parcours-1-figures", 1280, 860);
    p.view = View::Parcours(6);
    save(&p, "03-parcours-6-strategies", 1280, 860);
    p.open_guide(argos_academy::index("2-02-bos").unwrap());
    save(&p, "04-lecon-bos", 1280, 860);
    let l = &lecons()[argos_academy::index("2-02-bos").unwrap()];
    for (qi, q) in l.quiz.iter().enumerate() {
        cliquer(&mut p, &t, 1280.0, Hit::Choice(qi, if qi == 2 { (q.bonne + 1) % 3 } else { q.bonne }));
    }
    let full = p.build(&t, 1280.0).2;
    p.scroll = (full - 800.0).max(0.0);
    save(&p, "05-quiz-score-4-sur-5", 1280, 860);
    p.open_guide(argos_academy::index("1-01-tete-et-epaules").unwrap());
    let d = p
        .build(&t, 1280.0)
        .1
        .iter()
        .find_map(|(_, z)| matches!(z, Hit::Details(..)).then_some(*z))
        .unwrap();
    cliquer(&mut p, &t, 1280.0, d);
    save(&p, "06-bloc-repliable-ouvert", 1280, 860);
    p.progress.fuseau = Fuseau::Paris;
    p.open_guide(argos_academy::index("6-08-range-asiatique").unwrap());
    save(&p, "07-horaires-fuseau-paris", 1280, 860);
    let mut n = AcademyPage::new(prog.clone());
    n.open_at(NEWS_GUIDE, 1, 6, &t, 1280.0);
    save(&n, "08-fiche-news-agenda", 1280, 860);
    let mut m = AcademyPage::new(prog);
    save(&m, "09-telephone-accueil", 390, 844);
    m.open_guide(argos_academy::index("2-02-bos").unwrap());
    save(&m, "10-telephone-lecon", 390, 844);
}

/// 📱 Au doigt : aucune carte de parcours « bientôt », et de la place en bas pour le rond Argos AI ; le PC garde tout.
#[test]
fn au_doigt_pas_de_parcours_bientot() {
    let t = Text::default();
    let p = AcademyPage::new(Progress::default());
    let bientot = |ops: &[Op]| ops.iter().filter(|o| matches!(o, Op::Text(s, ..) if s == "bientôt")).count();
    crate::actions::set_tactile(false);
    let (ops_pc, zones_pc, h_pc) = p.build(&t, 402.0);
    crate::actions::set_tactile(true);
    let (ops_doigt, zones_doigt, h_doigt) = p.build(&t, 402.0);
    crate::actions::set_tactile(false);
    let fermes = PARCOURS.iter().filter(|p| !p.ouvert).count();
    assert!(fermes > 0 && bientot(&ops_pc) == fermes, "PC : les cartes « bientôt » restent");
    assert_eq!(bientot(&ops_doigt), 0, "au doigt : plus de carte « bientôt »");
    let parcours = |z: &[(Rect, Hit)]| z.iter().filter(|(_, h)| matches!(h, Hit::Parcours(_))).count();
    assert_eq!(parcours(&zones_doigt), parcours(&zones_pc), "les parcours ouverts sont tous là");
    assert!(h_doigt >= h_pc - (fermes as f64 * 86.0) + 50.0, "marge en bas pour le rond Argos AI");
}
