use super::*;

#[test]
fn chaque_lecon_se_charge() {
    let ls = lecons();
    assert_eq!(ls.len(), fichiers::FICHIERS.len());
    assert_eq!(ls.len(), 49, "4 + 20 + 8 + 1 + 16 leçons");
    for l in ls {
        assert!(!l.titre.is_empty() && !l.titre.starts_with('#'), "{} : titre", l.id);
        assert!(l.minutes > 0, "{} : durée", l.id);
        assert!(!l.sections.is_empty(), "{} : sections", l.id);
        assert!(l.sections.iter().all(|s| !s.titre.is_empty() && !s.blocs.is_empty()), "{} : section vide", l.id);
        assert!(!l.texte().contains("**"), "{} : gras resté dans le texte", l.id);
        assert!(!l.texte().contains("<details") && !l.texte().contains("</"), "{} : balise restée", l.id);
    }
    // ids uniques
    let mut ids: Vec<&str> = ls.iter().map(|l| l.id.as_str()).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), ls.len());
}

#[test]
fn chaque_quiz_a_5_questions_et_une_bonne_reponse() {
    let sans_quiz: Vec<&str> = lecons().iter().filter(|l| l.quiz.is_empty()).map(|l| l.id.as_str()).collect();
    assert_eq!(sans_quiz, ["0-04-tableau-recap-figures-et-smc", "5-01-news-dxy-xau"], "fiches de référence");
    for l in lecons().iter().filter(|l| !l.quiz.is_empty()) {
        assert_eq!(l.quiz.len(), QUESTIONS, "{}", l.id);
        for q in &l.quiz {
            assert!(!q.q.is_empty(), "{}", l.id);
            assert_eq!(q.choix.len(), 3, "{} : {}", l.id, q.q);
            assert!(q.bonne < q.choix.len());
            let bonne = &q.choix[q.bonne];
            assert_eq!(q.choix.iter().filter(|c| *c == bonne).count(), 1, "{} : une seule bonne réponse", l.id);
        }
        // la bonne réponse ne tombe pas toujours à la même place
        let places: std::collections::BTreeSet<usize> = l.quiz.iter().map(|q| q.bonne).collect();
        assert!(places.len() > 1, "{}", l.id);
    }
}

#[test]
fn aucun_nom_de_societe_dans_le_contenu() {
    for (_, nom, src) in fichiers::FICHIERS {
        assert!(!src.to_lowercase().contains("ftmo"), "{nom}");
    }
    assert!(lecons().iter().all(|l| !simple(&l.texte()).contains("ftmo")));
}

#[test]
fn index_complets() {
    // chaque parcours ouvert a ses leçons, les autres sont vides (« bientôt »)
    for p in &PARCOURS {
        assert_eq!(!lecons_du(p.num).is_empty(), p.ouvert, "parcours {}", p.num);
    }
    let n = |p| lecons_du(p).len();
    assert_eq!((n(0), n(1), n(2), n(5), n(6)), (4, 20, 8, 1, 16));
    // toutes les leçons sont dans le chemin, une seule fois, dans l'ordre des parcours
    let c = chemin();
    assert_eq!(c.len(), lecons().len());
    assert!(c.windows(2).all(|w| lecons()[w[0]].parcours <= lecons()[w[1]].parcours));
    // l'ordre des index du contenu : BOS après HH/HL, CHoCH après BOS ; Double Structure en tête des stratégies
    let titres: Vec<&str> = lecons_du(2).iter().map(|&i| lecons()[i].titre.as_str()).collect();
    assert!(titres[1].starts_with("BOS") && titres[2].starts_with("CHoCH"), "{titres:?}");
    assert!(lecons()[lecons_du(6)[0]].titre.contains("Double Structure"));
    // chaque leçon se retrouve par son id
    assert!(lecons().iter().enumerate().all(|(i, l)| index(&l.id) == Some(i)));
}

#[test]
fn fiche_d_une_lecon() {
    let l = &lecons()[index("2-02-bos").unwrap()];
    assert_eq!(l.titre, "BOS : Break of Structure");
    assert_eq!((l.groupe.as_str(), l.minutes), ("Structure SMC", 4));
    assert_eq!(l.note, "Signal : la tendance continue.");
    assert_eq!(l.sections[0].titre, "⚡ En 30 secondes");
    assert_eq!(l.checklist.len(), 4);
    assert_eq!(l.quiz[0].choix[l.quiz[0].bonne], "dans le sens de la tendance");
    // tableau et bloc repliable
    let te = &lecons()[index("1-01-tete-et-epaules").unwrap()];
    assert!(te.sections.iter().flat_map(|s| &s.blocs).any(|b| matches!(b, Bloc::Details { .. })));
    let news = &lecons()[index("5-01-news-dxy-xau").unwrap()];
    assert_eq!(news.tableau(1).unwrap().len(), 15, "en-tête + 14 annonces");
    assert!(news.horaires && !l.horaires);
    assert!(news
        .texte()
        .contains("| 🆘 Jobless Claims | Nouvelles demandes de chômage | chaque jeudi, 8h30 | ⬇ | ⬆ |"));
    // Double Structure : 7 questions dans le fichier, 5 dans le quiz
    assert_eq!(lecons()[index("6-01-double-structure").unwrap()].quiz.len(), 5);
}

#[test]
fn pieges_ecrits_dans_le_fichier() {
    let l = lire(
        9,
        "essai",
        "# Essai\n> Parcours 9 · Test · 1 min\n## ⚡ A\nTexte\n## ❓ Quiz\n1. Q → **oui** ~ non ~ peut-être\n",
    );
    assert_eq!(l.quiz[0].choix.len(), 3);
    assert_eq!(l.quiz[0].choix[l.quiz[0].bonne], "oui");
    assert!(l.quiz[0].choix.contains(&"peut-être".to_string()));
}

/// 🎓 Les vrais pièges : chaque question a 3 choix distincts, ses 2 mauvais choix sont les pièges écrits dans le
/// fichier, et le mélange est stable (deux lectures donnent le même ordre).
#[test]
fn quiz_vrais_pieges_et_trois_choix_distincts() {
    let (mut avec_pieges, mut ecrites_tout) = (0, 0);
    for (parcours, nom, texte) in crate::fichiers::FICHIERS {
        let l = lire(*parcours, nom, texte);
        assert_eq!(l.quiz, lire(*parcours, nom, texte).quiz, "{nom} : mélange stable");
        let ecrites: Vec<_> = texte
            .lines()
            .skip_while(|x| !(x.starts_with("## ") && x.contains("Quiz")))
            .filter_map(question)
            .collect();
        ecrites_tout += ecrites.iter().filter(|e| e.2.len() >= 2).count();
        for (q, (_, bonne, pieges)) in l.quiz.iter().zip(&ecrites) {
            let distincts: std::collections::BTreeSet<&String> = q.choix.iter().collect();
            assert_eq!(distincts.len(), 3, "{nom} : {} → {:?}", q.q, q.choix);
            assert_eq!(&q.choix[q.bonne], bonne, "{nom} : {}", q.q);
            if pieges.len() >= 2 {
                avec_pieges += 1;
                for p in &pieges[..2] {
                    assert!(q.choix.contains(p), "{nom} : piège « {p} » absent");
                }
            }
        }
    }
    assert_eq!(ecrites_tout, 237, "questions écrites avec 2 pièges");
    assert_eq!(avec_pieges, 235, "questions montrées (5 par quiz ; la Double structure en a 7 écrites)");
}

#[test]
fn quiz_score_et_progression() {
    let l = &lecons()[index("2-02-bos").unwrap()];
    let bonnes: Vec<Option<usize>> = l.quiz.iter().map(|q| Some(q.bonne)).collect();
    assert_eq!(score(l, &bonnes), 5);
    let mut trois = bonnes.clone();
    trois[0] = Some((l.quiz[0].bonne + 1) % 3);
    trois[1] = None;
    assert_eq!(score(l, &trois), 3);
    let mut p = Progress::default();
    assert!(p.lire(l) && !p.lire(l));
    assert!(p.noter(l, 3));
    assert!(!p.is_done(l), "3 / 5 : pas encore terminée");
    assert!(p.noter(l, 4) && p.is_done(l));
    assert!(!p.noter(l, 2), "le meilleur score reste");
    assert_eq!(p.scores[&l.id], 4);
    // une fiche sans quiz est terminée dès qu'elle est lue
    let fiche = &lecons()[index("0-04-tableau-recap-figures-et-smc").unwrap()];
    p.lire(fiche);
    assert!(p.is_done(fiche));
    assert_eq!(p.globale(), (2, lecons().len()));
    assert_eq!(p.du_parcours(2), (1, 8));
    assert_eq!(p.prochaine(), Some(0));
    // format du fichier : l'ancien (read / done) se lit toujours ; aller-retour sans perte
    let ancien: Progress = serde_json::from_str(r#"{"read":["ta-1"],"done":[]}"#).unwrap();
    assert_eq!((ancien.read.len(), ancien.fuseau), (1, Fuseau::NewYork));
    p.fuseau = Fuseau::Paris;
    let json = serde_json::to_string(&p).unwrap();
    assert_eq!(serde_json::from_str::<Progress>(&json).unwrap(), p);
}

#[test]
fn recherche() {
    let t = |q: &str| chercher(q).into_iter().map(|i| lecons()[i].id.clone()).collect::<Vec<_>>();
    assert_eq!(t("bos").first().map(String::as_str), Some("2-02-bos"));
    assert_eq!(t("tete epaules").first().map(String::as_str), Some("1-01-tete-et-epaules"));
    assert!(t("silver bullet").contains(&"6-12-silver-bullet".to_string()));
    assert!(t("jobless claims").contains(&"5-01-news-dxy-xau".to_string()), "trouvée dans le texte");
    assert!(t("").is_empty() && t("zzzzqx").is_empty());
    assert!(mots_cles(&lecons()[0]).contains("ctrl+k"));
}
