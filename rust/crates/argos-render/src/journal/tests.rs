use super::*;

/// 🆘 The rescue code button: in the desktop app only, never in the browser page (WASM build), and its click.
#[test]
fn bouton_code_de_secours_hors_navigateur() {
    let foot = |secours| {
        let mut out = vec![];
        pied(&mut out, 100.0, 300.0, secours);
        out.into_iter().map(|(_, it)| it).collect::<Vec<_>>()
    };
    assert!(foot(true).contains(&It::Foot(3)));
    assert!(!foot(false).contains(&It::Foot(3)), "browser page: no rescue code button");
    assert_eq!(SECOURS_ICI, !cfg!(target_arch = "wasm32"));
    // native build of the tests: the button is in the journal, and its click
    let mut j = Journal::default();
    let v = j.view(now_ms());
    let (r, _) = j.items(&v, &Text::default(), 340.0).into_iter().find(|(_, it)| *it == It::Foot(3)).unwrap();
    assert_eq!(j.click(r.cx(), r.cy(), 340.0, now_ms()), JClick::Secours);
    // 📸 ARGOS_SECOURS_PNG=folder: the journal panel with its buttons, to look at it
    if let Ok(dir) = std::env::var("ARGOS_SECOURS_PNG") {
        let p = crate::panels::Panels::default();
        let mut pm = crate::px::new(340, 420).unwrap();
        p.draw(
            crate::panels::Kind::Journal,
            &mut pm,
            &Text::default(),
            (340.0, 420.0),
            None,
            &crate::Theme::default(),
        );
        pm.save_png(format!("{dir}/secours-journal.png")).unwrap();
    }
}

#[test]
fn gardien_propfirm() {
    let mut j = Journal::default();
    // compte 100K : perte max du jour 5 % = 5 000 $, 4 trades par jour
    j.cfg.risk_pct = 0.0; // ce test regarde les pertes du jour : pas de plafond par trade
    assert_eq!(j.guard(500.0, now_ms(), 0, true, None), None, "un trade de 500 $ passe");
    assert!(
        j.guard(6_000.0, now_ms(), 0, true, None).unwrap().contains("perte max du jour"),
        "6 000 $ de risque : bloqué"
    );
    let t = |id: &str, risk| JTrade {
        id: id.into(),
        sym: "BTC".into(),
        side: "long".into(),
        entry: Some(100.0),
        sl: Some(99.0),
        risk: Some(risk),
        entry_ts: now_ms(),
        ..Default::default()
    };
    j.trades = vec![t("a", 2_000.0), t("b", 2_000.0)];
    assert!(j.guard(1_500.0, now_ms(), 0, true, None).is_some(), "4 000 $ déjà ouverts + 1 500 $ > 5 000 $");
    j.trades = (0..4).map(|k| t(&k.to_string(), 10.0)).collect();
    assert!(j.guard(10.0, now_ms(), 0, true, None).unwrap().contains("trades aujourd'hui"));
}

#[test]
fn gardien_risque_positions_et_verrou() {
    let mut j = Journal::default();
    // 1 % de 100K = 1 000 $ par trade au plus
    assert_eq!(j.guard(1_000.0, now_ms(), 0, true, None), None);
    assert!(
        j.guard(1_200.0, now_ms(), 0, true, None).unwrap().contains("1 000 $ permis"),
        "risque trop gros : bloqué"
    );
    // 2 positions ouvertes au plus
    assert_eq!(j.guard(100.0, now_ms(), 1, true, None), None);
    assert!(j.guard(100.0, now_ms(), 2, true, None).unwrap().contains("2 positions ouvertes"));
    // journée verrouillée : tout est bloqué, et le verrou d'un autre jour ne compte plus
    j.cfg.lock = j.propfirm_day(now_ms());
    assert!(j.locked(now_ms()));
    assert!(j.guard(10.0, now_ms(), 0, true, None).unwrap().contains("verrouillée"));
    j.cfg.lock = "2020-01-01".into();
    assert!(!j.locked(now_ms()));
    assert_eq!(j.guard(10.0, now_ms(), 0, true, None), None);
}

#[test]
fn gardien_surveille_la_perte_en_cours() {
    let j = Journal::default();
    // limite du jour 5 000 $, totale 10 000 $ (compte 100K)
    assert_eq!(j.breach(-4_999.0, now_ms()), None, "juste sous la limite");
    assert_eq!(j.breach(-5_000.0, now_ms()), Some("perte du jour"));
    assert_eq!(j.breach(2_000.0, now_ms()), None, "en gain : rien");
}

#[test]
fn gardien_sl_obligatoire_et_news() {
    let mut j = Journal::default();
    assert_eq!(j.guard(100.0, now_ms(), 0, false, None).as_deref(), Some("stop loss obligatoire"));
    j.cfg.need_sl = false;
    assert_eq!(j.guard(100.0, now_ms(), 0, false, None), None, "SL facultatif : l'ordre passe");
    let why = j.guard(100.0, now_ms(), 0, true, Some("USD CPI m/m")).unwrap();
    assert!(why.contains("USD CPI m/m") && why.contains("2 min"), "{why}");
    j.cfg.news = false;
    assert_eq!(j.guard(100.0, now_ms(), 0, true, Some("USD CPI m/m")), None, "blocage news coupé");
}

/// 🛡 Pilule de la barre du haut (maquette verre « 0,8 % / 5 % ») : perte du jour / limite du jour en % du compte.
#[test]
fn gardien_pilule_en_pourcent() {
    let j = Journal::default();
    let sep = &crate::viewport::format_number(0.5, 1)[1..2];
    assert_eq!(j.pilule(-800.0, 1, now_ms()), (format!("0{sep}8 % / 5 %"), GuardLevel::Ok));
    assert_eq!(j.pilule(0.0, 0, now_ms()).0, "0 % / 5 %", "rien de perdu");
    assert_eq!(j.pilule(500.0, 1, now_ms()).0, "0 % / 5 %", "en gain : 0");
    assert_eq!(j.pilule(-4_000.0, 1, now_ms()), ("4 % / 5 %".into(), GuardLevel::Warn));
    assert_eq!(j.pilule(-6_000.0, 1, now_ms()).1, GuardLevel::Bad);
}

/// 🛡 Paliers 50 / 80 / 100 % : sur la limite du jour (5 000 $) puis sur la limite totale (10 000 $).
#[test]
fn gardien_paliers_50_80_100() {
    let mut j = Journal::default();
    let niveau = |j: &Journal, perte: f64| j.guard_card(-perte, 1, now_ms()).level();
    for (lim, nom) in [(5_000.0, "jour"), (10_000.0, "total")] {
        if nom == "total" {
            j.cfg.daily = 50.0; // la limite du jour (50 000 $) ne gêne plus : seule la totale compte
        }
        let p = |pct: f64| lim * pct / 100.0;
        assert_eq!(niveau(&j, p(49.9)), GuardLevel::Ok, "{nom} 49,9 %");
        assert_eq!(niveau(&j, p(50.0)), GuardLevel::Half, "{nom} 50 %");
        assert_eq!(niveau(&j, p(79.9)), GuardLevel::Half, "{nom} 79,9 %");
        assert_eq!(niveau(&j, p(80.0)), GuardLevel::Warn, "{nom} 80 %");
        assert_eq!(niveau(&j, p(100.0)), GuardLevel::Bad, "{nom} 100 %");
    }
    // alerte réglée à 40 % (≤ 50) : orange dès 40 %, jamais de jaune
    j.cfg.warn = 40.0;
    assert_eq!(niveau(&j, 3_000.0), GuardLevel::Ok, "30 %");
    assert_eq!(niveau(&j, 4_500.0), GuardLevel::Warn, "45 %");
    assert_eq!(niveau(&j, 6_000.0), GuardLevel::Warn, "60 %");
}

/// 🧊 Pendant la pause anti-tilt, tout nouvel ordre est refusé (minutes restantes arrondies vers le haut), puis
/// accepté de nouveau à la fin.
#[test]
fn gardien_refuse_pendant_la_pause_anti_tilt() {
    let mut j = Journal::default();
    let now = now_ms();
    j.cfg.pause_until = now + 12 * 60_000 + 1;
    assert_eq!(j.guard(100.0, now, 0, true, None).as_deref(), Some("pause anti-tilt : encore 13 min"));
    assert_eq!(
        j.guard(100.0, now + 12 * 60_000, 0, true, None).as_deref(),
        Some("pause anti-tilt : encore 1 min")
    );
    assert_eq!(j.guard(100.0, now + 12 * 60_000 + 1, 0, true, None), None, "pause finie : l'ordre passe");
}

/// 🛡 Un avis par palier et par jour : 50 % puis 80 %, chacun une seule fois ; le lendemain tout recommence.
#[test]
fn gardien_previent_une_fois_par_palier() {
    let mut j = Journal::default();
    let now = now_ms();
    let proche = |v: Option<f64>, pct: f64| v.is_some_and(|p| (p - pct).abs() < 1e-9);
    assert_eq!(j.warn_once(-2_000.0, 1, now), None, "40 % : rien");
    assert!(proche(j.warn_once(-2_500.0, 1, now), 50.0), "palier 50 %");
    assert_eq!(j.warn_once(-3_000.0, 1, now), None, "50 % déjà annoncé");
    assert!(proche(j.warn_once(-4_000.0, 1, now), 80.0), "palier 80 %");
    assert_eq!(j.warn_once(-4_500.0, 1, now), None, "80 % déjà annoncé");
    assert_eq!(j.warn_once(-2_600.0, 1, now), None, "redescendu à 52 % : pas de nouvel avis");
    // le lendemain : de nouveau un avis par palier
    let demain = now + 86_400_000;
    assert!(proche(j.warn_once(-2_500.0, 1, demain), 50.0));
    // un saut direct à 80 % ne donne qu'un avis
    let apres = demain + 86_400_000;
    assert!(proche(j.warn_once(-4_000.0, 1, apres), 80.0));
    assert_eq!(j.warn_once(-3_000.0, 1, apres), None);
}

#[test]
fn gardien_carte_avertissement_et_deverrouillage() {
    let mut j = Journal::default();
    // limite du jour 5 000 $ : 1 000 $ de perte flottante = 20 %, dans les règles
    let c = j.guard_card(-1_000.0, 1, now_ms());
    assert_eq!(c.level(), GuardLevel::Ok);
    assert_eq!(c.rows()[2].1, "1 / 2");
    assert!(c.rows()[0].1.contains("1 000 $"), "{}", c.rows()[0].1);
    // 100 % ou journée verrouillée : rouge
    assert_eq!(j.guard_card(-5_000.0, 1, now_ms()).level(), GuardLevel::Bad);
    j.cfg.lock = j.propfirm_day(now_ms());
    assert_eq!(j.guard_card(0.0, 0, now_ms()).level(), GuardLevel::Bad);
    assert!(j.unlock("", now_ms()).is_err() && j.locked(now_ms()), "sans raison : toujours verrouillée");
    j.unlock("Erreur de saisie, je reprends demain", now_ms()).unwrap();
    assert!(!j.locked(now_ms()));
    assert_eq!(j.guard_card(0.0, 0, now_ms()).level(), GuardLevel::Ok);
}

/// 🔓 Every unlock is kept (500 at most, the oldest go), and the PropFirm tab shows them week by week with their
/// reasons; no unlock: a calm line.
#[test]
fn bilan_de_discipline_dans_le_journal() {
    assert_eq!(discipline(&[])[0].0, "Aucun déverrouillage : les règles ont tenu.");
    let mut j = Journal::default();
    let now = now_ms();
    j.unlock("Erreur de saisie", now).unwrap();
    j.unlock("News manquée", now + 1).unwrap();
    assert_eq!(j.cfg.deverrouillages.len(), 2);
    assert_eq!(j.cfg.deverrouillage.as_ref().unwrap().raison, "News manquée", "the last one stays for the card");
    let l = discipline(&j.cfg.deverrouillages);
    assert!(l[0].0.starts_with("Semaine du ") && l[0].0.ends_with(" : 2 déverrouillages"), "{:?}", l[0]);
    assert_eq!((l[1].0.as_str(), l[2].0.as_str()), ("• « Erreur de saisie »", "• « News manquée »"));
    j.tab = 2;
    let v = j.view(now);
    let textes: Vec<String> = j
        .items(&v, &Text::default(), 340.0)
        .into_iter()
        .filter_map(|(_, it)| match it {
            It::Text(t, _) | It::Title(t) => Some(t),
            _ => None,
        })
        .collect();
    assert!(textes.iter().any(|t| t == "🔓 Discipline : déverrouillages du Gardien"));
    assert!(textes.iter().any(|t| t == "• « News manquée »"));
    for k in 0..argos_core::journal::DEVERROUILLAGES_MAX {
        j.unlock(&format!("n° {k}"), now + 2 + k as i64).unwrap();
    }
    assert_eq!(j.cfg.deverrouillages.len(), argos_core::journal::DEVERROUILLAGES_MAX);
    assert_eq!(j.cfg.deverrouillages[0].raison, "n° 0", "the oldest went first");
    // 📸 ARGOS_DISCIPLINE_PNG=folder: the PropFirm tab with its review, to look at it
    if let Ok(dir) = std::env::var("ARGOS_DISCIPLINE_PNG") {
        let mut p = crate::panels::Panels::default();
        p.journal.tab = 2;
        p.journal.unlock("Erreur de saisie corrigée", now).unwrap();
        p.journal.unlock("News manquée, je voulais finir mon plan", now + 1).unwrap();
        let mut pm = crate::px::new(340, 1300).unwrap();
        p.draw(
            crate::panels::Kind::Journal,
            &mut pm,
            &Text::default(),
            (340.0, 1300.0),
            None,
            &crate::Theme::default(),
        );
        pm.save_png(format!("{dir}/discipline.png")).unwrap();
    }
}

/// 🔓 Déverrouiller la journée demande une raison : sans elle (ou trop longue), refus et verrou gardé ; avec elle,
/// la trace (jour du reset, heure, raison) est gardée dans le journal, relue après rechargement ; celle d'hier ne
/// vaut pas pour aujourd'hui ; un reverrouillage la fait taire ; un ancien journal sans trace se lit toujours.
#[test]
fn gardien_deverrouiller_demande_une_raison() {
    let mut j = Journal::default();
    let maintenant = 1_794_772_800_000; // 2026-11-15 20:00 UTC (21:00 à Prague)
    j.cfg.lock = j.propfirm_day(maintenant);
    for vide in ["", "   ", "\n\t"] {
        assert_eq!(j.unlock(vide, maintenant), Err("Écris une raison pour déverrouiller (obligatoire)".into()));
    }
    let longue = "x".repeat(RAISON_MAX + 1);
    assert_eq!(j.unlock(&longue, maintenant), Err("🔒 Raison trop longue (200 caractères au plus)".into()));
    assert!(
        j.locked(maintenant) && j.cfg.deverrouillage.is_none() && j.deverrouillee(maintenant).is_none(),
        "rien n'a changé"
    );
    assert_eq!(j.unlock("  Erreur de saisie corrigée  ", maintenant), Ok(()));
    assert!(!j.locked(maintenant));
    assert_eq!(j.deverrouillee(maintenant), Some("Erreur de saisie corrigée"));
    let d = j.cfg.deverrouillage.clone().unwrap();
    assert_eq!((d.jour.as_str(), d.ms), ("2026-11-15", maintenant));
    // rechargement : le réglage passe par le JSON du journal (chiffré sur le disque)
    let relu: JCfg = serde_json::from_str(&serde_json::to_string(&j.cfg).unwrap()).unwrap();
    let j2 = Journal {
        cfg: relu,
        ..Journal::default()
    };
    assert_eq!(j2.deverrouillee(maintenant), Some("Erreur de saisie corrigée"));
    // le lendemain (jour du reset suivant) : plus rien
    assert_eq!(j2.deverrouillee(maintenant + 86_400_000), None);
    // reverrouillée le même jour par le Gardien : plus « déverrouillée »
    let mut j3 = j2.clone();
    j3.cfg.lock = j3.propfirm_day(maintenant);
    assert_eq!(j3.deverrouillee(maintenant), None);
    // ancien journal (sans le champ) : lisible, jamais déverrouillé
    let ancien: JCfg = serde_json::from_str(r#"{"lock":"2026-11-15","tilt_n":2}"#).unwrap();
    assert_eq!((ancien.deverrouillage, ancien.lock.as_str(), ancien.tilt_n), (None, "2026-11-15", 2));
}

#[test]
fn gardien_reset_dans_le_fuseau_choisi() {
    let mut j = Journal::default();
    // 2026-10-31 23:30 UTC : déjà le 1er novembre à Prague, encore le 31 à New York
    let ms = 1_793_489_400_000;
    assert_eq!(j.propfirm_day(ms), "2026-11-01");
    j.cfg.reset_tz = "America/New_York".into();
    assert_eq!(j.propfirm_day(ms), "2026-10-31");
}

fn trade(id: &str, side: &str, entry: f64, sl: f64, exit: Option<f64>, ts: i64) -> JTrade {
    JTrade {
        id: id.into(),
        sym: "BTC".into(),
        side: side.into(),
        strat: "Scalp".into(),
        entry: Some(entry),
        sl: Some(sl),
        exit,
        risk: Some(100.0),
        entry_ts: ts,
        exit_ts: exit.map(|_| ts + 60_000),
        emo: "calme".into(),
        ..Default::default()
    }
}

#[test]
fn formats_comme_v1() {
    assert_eq!(money(Some(-1234.4)), "−1 234 $");
    assert_eq!(money(Some(100_000.0)), "100 000 $");
    assert_eq!(money(None), "—");
    assert_eq!(r_txt(Some(-0.5)), "−0.50 R");
    let ms = parse_date("2026-10-26 14:30").unwrap();
    assert_eq!(date_txt(ms), "2026-10-26 14:30");
    assert_eq!(ms, 1_793_039_400_000, "14:30 à New York (UTC−4) = 18:30 UTC");
    assert_eq!(sess_of(ms), "New York");
    assert_eq!(day_key(ms, "Europe/Prague"), "2026-10-26");
}

#[test]
fn sans_chevauchement() {
    let t = Text::default();
    let mut j = Journal {
        trades: vec![trade("a", "long", 100.0, 90.0, Some(120.0), 0)],
        ..Default::default()
    };
    for tab in 0..3 {
        j.tab = tab;
        for pw in [300.0, 340.0, 420.0, 640.0] {
            let v = j.view(1);
            let z: Vec<Rect> = j
                .items(&v, &t, pw)
                .into_iter()
                .filter(|(_, it)| !matches!(it, It::Cfg(_) | It::Step(..)))
                .map(|(r, _)| r)
                .collect();
            for (i, a) in z.iter().enumerate() {
                assert!(a.x >= 0.0 && a.x + a.w <= pw + 0.01, "dans le panneau ({pw}) : {a:?}");
                for b in &z[i + 1..] {
                    let over = a.x < b.x + b.w - 0.01 && b.x < a.x + a.w - 0.01 && a.y < b.y + b.h - 0.01 && b.y < a.y + a.h - 0.01;
                    assert!(!over, "chevauchement onglet {tab} à {pw} : {a:?} / {b:?}");
                }
            }
        }
    }
}

#[test]
fn panneau_et_fiche() {
    let now = parse_date("2026-10-26 12:00").unwrap();
    let mut j = Journal {
        trades: vec![
            trade("a", "long", 100.0, 90.0, Some(120.0), now - 3 * DAY_MS),
            trade("b", "short", 100.0, 110.0, Some(105.0), now - 40 * DAY_MS),
            trade("c", "long", 50.0, 45.0, None, now),
        ],
        ..Default::default()
    };
    let t = Text::default();
    let pw = 340.0;
    let at = |j: &Journal, it: It| {
        let v = j.view(now);
        j.items(&v, &t, pw).into_iter().find(|(_, x)| *x == it).map(|(r, _)| (r.cx(), r.cy())).unwrap()
    };
    // Tout : 3 lignes, la plus récente en premier ; clic sur une ligne = modifier ce trade
    let v = j.view(now);
    assert_eq!(v.rows, vec![2, 0, 1]);
    let (x, y) = at(&j, It::Row(0));
    assert_eq!(j.click(x, y, pw, now), JClick::Edit(0));
    // 30 j : le trade de 40 jours sort
    let (x, y) = at(&j, It::Period(1));
    j.click(x, y, pw, now);
    assert_eq!(j.view(now).rows, vec![2, 0]);
    // filtre symbole : liste ouverte, puis « BTC »
    let (x, y) = at(&j, It::Pick(1));
    j.click(x, y, pw, now);
    let (x, y) = at(&j, It::Opt(1, 1));
    j.click(x, y, pw, now);
    assert_eq!(j.sym.as_deref(), Some("BTC"));
    // stats et PropFirm
    j.period = 3;
    j.tab = 1;
    let v = j.view(now);
    assert_eq!((v.stats.n, v.stats.open, v.stats.total), (2, 1, 150.0));
    assert_eq!(v.groups[0].1[0].key, "Scalp");
    j.tab = 2;
    let (x, y) = at(&j, It::Step(1, 1));
    j.click(x, y, pw, now);
    assert_eq!(j.cfg.phase, 1);
    let (x, y) = at(&j, It::Step(7, 1));
    j.click(x, y, pw, now);
    assert_eq!(j.cfg.start, "2026-10-26", "vide → aujourd'hui");
    j.click(x, y, pw, now);
    assert_eq!(j.cfg.start, "", "après aujourd'hui → depuis toujours");
    for tab in 0..3 {
        j.tab = tab;
        let mut pm = crate::px::new(340, j.content_h(pw) as u32).unwrap();
        j.draw(&mut pm, &t, pw, None, pal::edge(), now);
    }
    // fiche : taper, sens, émotion, enregistrer
    let mut f = JForm::new(
        JTrade {
            id: "n".into(),
            sym: "ETH".into(),
            side: "long".into(),
            entry_ts: now,
            ..Default::default()
        },
        true,
        j.cfg.strats.clone(),
        1600.0,
        900.0,
    );
    assert_eq!(f.enter(), FormClick::Warn("Il faut au moins l'entrée et le stop loss"));
    let (z, ..) = f.zones(&t);
    let c = |h: FHit| z.iter().find(|(_, x)| *x == h).map(|(r, _)| (r.cx(), r.cy())).unwrap();
    let (x, y) = c(FHit::Field(4));
    f.click(x, y, &t);
    "2000".chars().for_each(|ch| f.type_char(ch));
    f.tab();
    "1900".chars().for_each(|ch| f.type_char(ch));
    let (x, y) = c(FHit::Field(1));
    f.click(x, y, &t);
    let (x, y) = c(FHit::Emo(2));
    f.click(x, y, &t);
    let (x, y) = c(FHit::NextStrat);
    f.click(x, y, &t);
    assert!(f.draw(&t, (0.0, 0.0)).is_some());
    match f.enter() {
        FormClick::Save(s) => {
            assert_eq!((s.entry, s.sl, s.side.as_str(), s.emo.as_str()), (Some(2000.0), Some(1900.0), "short", "fomo"));
            assert_eq!(s.strat, "Double structure 4H");
        }
        o => panic!("{o:?}"),
    }
    // modifier : Supprimer demande une confirmation
    let mut f = JForm::new(j.trades[0].clone(), false, vec![], 1600.0, 900.0);
    let (z, ..) = f.zones(&t);
    let d = z.iter().find(|(_, x)| *x == FHit::Delete).unwrap().0;
    assert_eq!(f.click(d.cx(), d.cy(), &t), FormClick::Changed);
    assert_eq!(f.click(d.x + 20.0, d.cy(), &t), FormClick::Delete("a".into()));
}

/// 📱 Journal vide au doigt : « Touche « + Trade » » (le PC garde « Clique »).
#[test]
fn journal_vide_dit_touche_au_doigt() {
    let j = Journal::default();
    let v = j.view(now_ms());
    let phrase = |j: &Journal| -> String {
        j.items(&v, &Text::default(), 340.0)
            .into_iter()
            .filter_map(|(_, it)| match it {
                It::Text(t, _) => Some(t),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    crate::actions::set_tactile(false);
    let pc = phrase(&j);
    crate::actions::set_tactile(true);
    let doigt = phrase(&j);
    crate::actions::set_tactile(false);
    assert!(pc.contains("Clique « + Trade »"), "{pc}");
    assert!(doigt.contains("Touche « + Trade »") && !doigt.contains("Clique"), "{doigt}");
}
