//! 🧪 Tests de l'app entière, sans fenêtre : on fabrique une `App` (bougies d'un faux fichier, réglages par défaut)
//! puis on lui envoie de vrais clics (position de la souris + bouton) et de vraies touches, comme winit le ferait.
//! Ils vérifient ce que l'utilisateur voit changer : l'outil choisi, l'unité de temps, le type de chart, l'échelle,
//! les dessins. Ils protègent les restructurations : si un déplacement de code casse un clic, un test tombe.
//!
//! Rien n'est lu ni écrit dans tes vrais réglages : pendant les tests, le dossier de config est un dossier temporaire
//! (`persist::saved::dir`), et il n'y a ni réseau, ni base, ni presse-papiers.

use crate::*;
use argos_render::settings::ScaleMode;
use winit::dpi::PhysicalPosition;
use winit::keyboard::SmolStr;

/// 🧱 Quality gate, bug B-001: no black band right of the candles (phone, tablet, upright, lying, turned).
mod bande_noire;
mod carnet;
/// 🟠 The DÉMO band over the upright tablet's chart: no wall pill nor high / low pill under it (bug B-018).
mod demo_murs;
/// ✏️ Every drawing tool by the finger: laid, selected, moved, stretched, its settings (bug B-019).
mod dessins_au_doigt;
mod filet_2;
mod funding_cell;
mod paires_btc;
/// 🔎 The phone's symbol search: the panel stays inside its card (bug B-017).
mod phone_search;
/// 🕸 Safety net of the split into modules: a reference for every screen of the app (`tests/safety_net.rs`).
mod safety_net;
mod touch_fixes;
/// 🏎 Speed guard of the cell (fluidité, lot A): dragging the chart and the market list stay as fast as their reference.
mod vitesse;

/// Début des bougies : un multiple de 4 h (les bougies regroupées tombent juste).
const START: i64 = 14_400 * 118_052;

/// 200 bougies de 30 min qui montent doucement (prix entre 100 et 300).
fn bougies() -> Vec<Bar> {
    (0..200)
        .map(|i| {
            let p = 100.0 + i as f64;
            Bar {
                time: START + i * 1800,
                open: p,
                high: p + 2.0,
                low: p - 2.0,
                close: p + 1.0,
                volume: 10.0,
            }
        })
        .collect()
}

/// Une app ouverte sur un fichier de bougies 30 min, comme `argos-desktop essai.json` (sans fenêtre).
fn app() -> App {
    let iv = Interval(1800);
    let mut a = App::blank(Source::File("essai.json".into()), iv, iv, vec![], "essai.json · 30m · fichier".into());
    a.chart.raw = bougies();
    a.rebuild();
    a.start_free();
    a
}

/// 🧪 Un dossier de réglages à ce test seul, vidé (interface-2.json, dock.json… : aucun autre test n'y écrit en même temps).
fn dossier_a_soi(nom: &str) {
    let d = argos_core::dossiers::de_test(nom);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    crate::persist::saved::DOSSIER_TEST.with(|x| *x.borrow_mut() = Some(d));
}

/// Un clic gauche à cet endroit de la fenêtre : la souris y va, on appuie, on relâche.
fn clic(a: &mut App, wx: f64, wy: f64) {
    a.mouse_moved(PhysicalPosition::new(wx, wy));
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
}

/// Un clic au point (x, y) de la chart (coordonnées de la chart, sous la barre du haut, après la barre de gauche),
/// posé « sans se presser » : jamais compté comme la 2e moitié d'un double-clic.
fn clic_chart(a: &mut App, x: f64, y: f64) {
    a.pointer.last_chart_click = None;
    let (wx, wy) = (x + a.rw(), y + a.top_h());
    clic(a, wx, wy);
}

/// Un clic au milieu d'un bouton de la barre de gauche.
fn clic_rail(a: &mut App, b: rail::Btn) {
    let (r, _) = rail::layout(a.area.1 as f64)
        .into_iter()
        .find(|(_, x)| *x == b)
        .expect("bouton absent de la barre de gauche");
    let wy = r.cy() + a.top_h();
    clic(a, r.cx(), wy);
}

/// Une touche (lettre ou touche nommée) envoyée à la chart, avec les touches Ctrl / Alt / Maj données.
fn touche(a: &mut App, k: Key, mods: ModifiersState) {
    a.pointer.mods = mods;
    a.key_chart(&k);
    a.pointer.mods = ModifiersState::empty();
}

fn lettre(c: &str) -> Key {
    Key::Character(SmolStr::new(c))
}

// — ⌨️ clavier —

#[test]
fn t_change_le_type_de_chart_et_maj_t_revient() {
    let mut a = app();
    assert_eq!(a.chart.style, 0);
    touche(&mut a, lettre("t"), ModifiersState::empty());
    assert_eq!(a.chart.style, 1, "T = type suivant");
    touche(&mut a, lettre("T"), ModifiersState::SHIFT);
    assert_eq!(a.chart.style, 0, "Maj + T = type précédent");
    // depuis le premier, Maj + T passe au dernier (on tourne en rond)
    touche(&mut a, lettre("T"), ModifiersState::SHIFT);
    assert_eq!(a.chart.style, a.chart.styles.len() - 1);
}

#[test]
fn alt_l_alt_p_alt_i_changent_l_echelle() {
    let mut a = app();
    let alt = ModifiersState::ALT;
    touche(&mut a, lettre("l"), alt);
    assert_eq!(a.chart.settings.scale.mode, ScaleMode::Log, "Alt + L = logarithmique");
    touche(&mut a, lettre("l"), alt);
    assert_eq!(a.chart.settings.scale.mode, ScaleMode::Normal, "Alt + L encore = retour à normal");
    touche(&mut a, lettre("p"), alt);
    assert_eq!(a.chart.settings.scale.mode, ScaleMode::Percent, "Alt + P = pourcent");
    assert!(!a.chart.settings.scale.invert);
    touche(&mut a, lettre("i"), alt);
    assert!(a.chart.settings.scale.invert, "Alt + I = échelle inversée");
}

#[test]
fn plus_et_moins_zooment() {
    let mut a = app();
    let avant = a.chart.viewport.bar_spacing;
    touche(&mut a, lettre("+"), ModifiersState::empty());
    let zoome = a.chart.viewport.bar_spacing;
    assert!(zoome > avant, "+ grossit les bougies ({avant} → {zoome})");
    touche(&mut a, lettre("-"), ModifiersState::empty());
    assert!(a.chart.viewport.bar_spacing < zoome, "− les réduit");
    // Ctrl + ↑ fait comme +
    let avant = a.chart.viewport.bar_spacing;
    touche(&mut a, Key::Named(NamedKey::ArrowUp), ModifiersState::CONTROL);
    assert!(a.chart.viewport.bar_spacing > avant, "Ctrl + ↑ grossit aussi");
}

#[test]
fn une_touche_sans_action_ne_change_rien() {
    let mut a = app();
    let (style, spacing, mode) = (a.chart.style, a.chart.viewport.bar_spacing, a.chart.settings.scale.mode);
    touche(&mut a, lettre("x"), ModifiersState::empty());
    assert_eq!(
        (a.chart.style, a.chart.viewport.bar_spacing, a.chart.settings.scale.mode),
        (style, spacing, mode)
    );
    assert_eq!(a.tool, "cursor");
}

#[test]
fn echap_quitte_l_outil_puis_deselectionne() {
    let mut a = app();
    a.set_tool("trend");
    touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert_eq!(a.tool, "cursor", "Échap quitte l'outil");
    assert_eq!(a.banner, None, "et son aide disparaît");
    // sans outil ni menu : Échap désélectionne l'objet
    a.chart.drawings.push(Drawing::hline(150.0));
    a.selected = Some(0);
    touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert_eq!(a.selected, None);
    assert_eq!(a.chart.drawings.len(), 1, "Échap n'efface pas l'objet");
}

#[test]
fn echap_ferme_d_abord_le_menu_ouvert() {
    let mut a = app();
    a.open_list(ListKind::Intervals, Rect::new(300.0, 100.0, 40.0, 24.0));
    assert!(a.ui.list.is_some());
    a.selected = None;
    touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert!(a.ui.list.is_none(), "Échap ferme le menu des unités de temps");
}

#[test]
fn suppr_efface_l_objet_choisi_sauf_s_il_est_verrouille() {
    let mut a = app();
    a.chart.drawings.push(Drawing::hline(150.0));
    a.chart.drawings.push(Drawing::hline(160.0));
    a.selected = Some(0);
    touche(&mut a, Key::Named(NamedKey::Delete), ModifiersState::empty());
    assert_eq!(a.chart.drawings.len(), 1, "Suppr efface l'objet sélectionné");
    assert_eq!(a.chart.drawings[0].pts[0].p, 160.0, "c'est bien le bon qui reste");
    assert_eq!(a.selected, None);
    // objet verrouillé 🔒 : Suppr ne l'efface pas
    a.chart.drawings[0].style.locked = true;
    a.selected = Some(0);
    touche(&mut a, Key::Named(NamedKey::Backspace), ModifiersState::empty());
    assert_eq!(a.chart.drawings.len(), 1, "un objet verrouillé reste");
}

#[test]
fn ctrl_z_annule_le_dernier_dessin() {
    let mut a = app();
    // chaque dessin posé passe par un relevé (comme après chaque action de l'app) : un pas d'historique chacun
    a.chart.drawings.push(Drawing::hline(150.0));
    a.track_drawings();
    a.chart.drawings.push(Drawing::hline(160.0));
    a.track_drawings();
    touche(&mut a, lettre("z"), ModifiersState::CONTROL);
    assert_eq!(a.chart.drawings.len(), 1);
    assert_eq!(a.chart.drawings[0].pts[0].p, 150.0);
}

// — 🖱️ barre de gauche : un clic change l'outil —

#[test]
fn clic_sur_un_groupe_de_la_barre_de_gauche_prend_son_outil() {
    let mut a = app();
    assert_eq!(a.tool, "cursor");
    clic_rail(&mut a, rail::Btn::Group("lines"));
    assert_eq!(a.tool, "trend", "le groupe Lignes donne la ligne de tendance (son 1er outil)");
    assert!(
        a.banner.as_deref().is_some_and(|b| b.starts_with("Ligne de tendance")),
        "l'aide de l'outil s'affiche"
    );
    // le groupe se souvient de ton dernier outil
    a.rail.set_last("lines", "hline");
    clic_rail(&mut a, rail::Btn::Group("lines"));
    assert_eq!(a.tool, "hline");
}

#[test]
fn clic_sur_le_curseur_revient_au_curseur() {
    let mut a = app();
    a.set_tool("trend");
    clic_rail(&mut a, rail::Btn::Tool("cursor"));
    assert_eq!(a.tool, "cursor");
    assert_eq!(a.banner, None, "le curseur n'a pas d'aide");
}

#[test]
fn clic_sur_la_gomme_l_allume_puis_l_eteint() {
    let mut a = app();
    clic_rail(&mut a, rail::Btn::Erase);
    assert_eq!(a.tool, "erase");
    clic_rail(&mut a, rail::Btn::Erase);
    assert_eq!(a.tool, "cursor", "2e clic sur la gomme = retour au curseur");
}

#[test]
fn clic_sur_la_petite_fleche_ouvre_la_liste_du_groupe() {
    let mut a = app();
    let (r, _) = rail::layout(a.area.1 as f64)
        .into_iter()
        .find(|(_, b)| *b == rail::Btn::Group("lines"))
        .unwrap();
    let wy = r.y + r.h - 3.0 + a.top_h();
    clic(&mut a, r.x + r.w - 3.0, wy);
    assert!(matches!(a.ui.list, Some((ListKind::RailGroup("lines"), _))), "la liste des lignes s'ouvre");
    assert_eq!(a.tool, "cursor", "sans changer d'outil");
}

#[test]
fn clic_sur_dessin_continu_garde_l_outil() {
    let mut a = app();
    assert!(!a.rail.keep);
    clic_rail(&mut a, rail::Btn::Keep);
    assert!(a.rail.keep, "le bouton ✏️ allume le mode continu");
    clic_rail(&mut a, rail::Btn::Keep);
    assert!(!a.rail.keep);
}

// — ✏️ dessiner sur la chart avec de vrais clics —

#[test]
fn deux_clics_avec_la_ligne_de_tendance_la_posent() {
    let mut a = app();
    a.set_tool("trend");
    clic_chart(&mut a, 300.0, 300.0);
    assert!(a.draft.is_some(), "1er clic : la ligne commence");
    assert!(a.chart.drawings.is_empty());
    clic_chart(&mut a, 600.0, 200.0);
    assert_eq!(a.chart.drawings.len(), 1, "2e clic : la ligne est posée");
    assert_eq!(a.chart.drawings[0].tool, "trend");
    assert_eq!(a.chart.drawings[0].pts.len(), 2);
    assert!(a.draft.is_none());
    assert_eq!(a.selected, Some(0), "la ligne posée est sélectionnée");
    assert_eq!(a.tool, "cursor", "l'outil revient au curseur après le dessin");
}

#[test]
fn en_mode_continu_l_outil_reste_apres_le_dessin() {
    let mut a = app();
    a.rail.keep = true;
    a.set_tool("trend");
    clic_chart(&mut a, 300.0, 300.0);
    clic_chart(&mut a, 600.0, 200.0);
    assert_eq!(a.chart.drawings.len(), 1);
    assert_eq!(a.tool, "trend", "mode continu : on peut tracer la suivante tout de suite");
}

#[test]
fn la_gomme_efface_l_objet_clique() {
    let mut a = app();
    a.set_tool("trend");
    clic_chart(&mut a, 300.0, 300.0);
    clic_chart(&mut a, 600.0, 200.0);
    assert_eq!(a.chart.drawings.len(), 1);
    a.set_tool("erase");
    clic_chart(&mut a, 450.0, 250.0); // au milieu de la ligne
    assert!(a.chart.drawings.is_empty(), "la gomme efface la ligne");
}

#[test]
fn entree_termine_un_dessin_a_points_libres() {
    let mut a = app();
    // la polyligne : un point par clic, jusqu'à Entrée (ou double-clic)
    let libre = "polyline";
    assert_eq!(argos_render::tools::def(libre).map(|t| t.points), Some(99));
    a.set_tool(libre);
    clic_chart(&mut a, 300.0, 300.0);
    clic_chart(&mut a, 400.0, 250.0);
    clic_chart(&mut a, 500.0, 280.0);
    assert!(a.chart.drawings.is_empty(), "{libre} : pas fini tant qu'on n'a pas validé");
    touche(&mut a, Key::Named(NamedKey::Enter), ModifiersState::empty());
    assert_eq!(a.chart.drawings.len(), 1, "Entrée termine le dessin");
    assert_eq!(a.chart.drawings[0].pts.len(), 3, "avec ses 3 points");
    assert!(a.draft.is_none());
}

#[test]
fn double_clic_termine_aussi_la_polyligne() {
    let mut a = app();
    a.set_tool("polyline");
    clic_chart(&mut a, 300.0, 300.0);
    clic_chart(&mut a, 400.0, 250.0);
    assert!(a.chart.drawings.is_empty());
    // 2e clic tout de suite au même endroit = double-clic
    let (wx, wy) = (400.0 + a.rw(), 250.0 + a.top_h());
    clic(&mut a, wx, wy);
    assert_eq!(a.chart.drawings.len(), 1, "le double-clic termine la polyligne");
    assert_eq!(a.chart.drawings[0].pts.len(), 2, "sans garder le point en trop du double-clic");
}

// — 🕐 unité de temps —

#[test]
fn changer_d_unite_de_temps_regroupe_les_bougies() {
    let mut a = app();
    assert_eq!(a.chart.bars.len(), 200);
    a.set_interval(Interval(3600));
    assert_eq!(a.chart.interval, Interval(3600));
    assert_eq!(a.chart.bars.len(), 100, "200 bougies de 30 min = 100 bougies de 1 h");
    assert_eq!(a.tools.interval, 3600, "la barre flottante montre la nouvelle unité");
    assert!(a.title.contains("1h"), "le titre suit : {}", a.title);
    // les dessins restent
    a.chart.drawings.push(Drawing::hline(150.0));
    a.set_interval(Interval(4 * 3600));
    assert_eq!(a.chart.bars.len(), 25);
    assert_eq!(a.chart.drawings.len(), 1, "changer d'unité garde les dessins");
}

#[test]
fn unite_plus_petite_que_le_fichier_refusee() {
    let mut a = app();
    a.set_interval(Interval(900)); // 15 min : on ne peut pas couper des bougies de 30 min
    assert_eq!(a.chart.interval, Interval(1800), "l'unité ne change pas");
    assert_eq!(a.chart.bars.len(), 200);
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("pas disponible")), "un bandeau le dit");
    // pas un multiple de 30 min non plus
    assert!(!a.interval_ok(Interval(45 * 60)));
    assert!(a.interval_ok(Interval(3 * 3600)));
}

#[test]
fn clic_dans_le_menu_des_unites_change_l_unite_de_temps() {
    let mut a = app();
    a.open_list(ListKind::Intervals, Rect::new(300.0, 100.0, 40.0, 24.0));
    let menu = a.ui.list.as_ref().map(|(_, m)| m.clone()).expect("menu des unités ouvert");
    // la ligne « 1 heure » du menu : on cherche où elle est, comme l'œil de l'utilisateur
    let x = menu.rect.x + 20.0;
    let y = (0..(menu.rect.h as i64))
        .map(|dy| menu.rect.y + dy as f64)
        .find(|&y| menu.clone().click(x, y) == MenuClick::Pick(3600))
        .expect("ligne 1 h dans le menu");
    clic(&mut a, x, y);
    assert_eq!(a.chart.interval, Interval(3600), "le clic sur « 1 h » passe la chart en 1 h");
    assert_eq!(a.chart.bars.len(), 100);
    assert!(a.ui.list.is_none(), "le menu se ferme après le choix");
}

#[test]
fn dans_le_menu_des_unites_les_impossibles_sont_grisees() {
    let mut a = app();
    let items: Vec<_> = a.interval_menu().into_iter().flat_map(|g| g.items).collect();
    let ligne = |s: i64| items.iter().find(|i| i.id == s);
    assert!(ligne(900).is_some_and(|i| !i.enabled), "15 min grisé (fichier en 30 min)");
    assert!(ligne(3600).is_some_and(|i| i.enabled), "1 h possible");
    assert!(ligne(1800).is_some_and(|i| i.checked), "30 min cochée (l'unité affichée)");
    a.set_interval(Interval(3600));
    let items: Vec<_> = a.interval_menu().into_iter().flat_map(|g| g.items).collect();
    assert!(items.iter().any(|i| i.id == 3600 && i.checked), "après le choix, 1 h est cochée");
}

// — 🛡️ gardien PropFirm (ticket démo) —

/// Ticket démo prêt : achat, stop à 10 $ sous l'entrée, `risk` $ de risque, gardien allumé.
fn ticket(a: &mut App, risk: f64) {
    let t = &mut a.watch.panels.trading;
    (t.buy, t.sl_pips, t.risk, t.guardian, t.plan) = (true, 10.0, risk, true, None);
}

#[test]
fn gardien_propfirm_bloque_un_risque_trop_gros() {
    let mut a = app();
    // compte 100K : 1 % = 1 000 $ par trade au plus
    ticket(&mut a, 1_500.0);
    a.place_demo_order();
    assert!(a.demo.positions.is_empty(), "l'ordre est bloqué");
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.contains("Gardien PropFirm") && b.contains("1 000 $ permis"), "le bandeau dit pourquoi : {b}");
    // gardien éteint : le même ordre passe
    a.watch.panels.trading.guardian = false;
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1, "sans gardien, rien ne bloque");
}

#[test]
fn gardien_propfirm_limite_les_positions_ouvertes() {
    let mut a = app();
    ticket(&mut a, 500.0);
    a.place_demo_order();
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 2);
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 2, "3e position bloquée (limite 2)");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("2 positions ouvertes")));
}

#[test]
fn gardien_propfirm_ferme_tout_et_verrouille_la_journee() {
    let mut a = app();
    ticket(&mut a, 1_000.0); // 100 unités, stop 10 $ plus bas
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1);
    // le prix s'effondre de 60 $ d'un coup (le stop saute) : −6 000 $ > limite du jour de 5 000 $
    if let Some(b) = a.chart.bars.last_mut() {
        b.close -= 60.0;
    }
    assert!(a.guard_watch(), "le gardien ferme la position");
    assert!(a.demo.positions.is_empty());
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("perte du jour")), "{:?}", a.banner);
    assert!(a.watch.panels.journal.locked(now_s() * 1000), "journée verrouillée");
    // plus aucun ordre aujourd'hui
    ticket(&mut a, 100.0);
    a.place_demo_order();
    assert!(a.demo.positions.is_empty());
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("verrouillée")));
}

#[test]
fn gardien_propfirm_eteint_ne_ferme_rien() {
    let mut a = app();
    ticket(&mut a, 1_000.0);
    a.place_demo_order();
    a.watch.panels.trading.guardian = false;
    if let Some(b) = a.chart.bars.last_mut() {
        b.close -= 60.0;
    }
    assert!(!a.guard_watch());
    assert_eq!(a.demo.positions.len(), 1, "gardien éteint : la position reste");
}

#[test]
fn gardien_propfirm_exige_un_stop_loss() {
    let mut a = app();
    ticket(&mut a, 500.0);
    a.watch.panels.trading.sl_pips = 0.0;
    a.place_demo_order();
    assert!(a.demo.positions.is_empty());
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("stop loss obligatoire")), "{:?}", a.banner);
}

#[test]
fn gardien_propfirm_bloque_pendant_une_news() {
    let mut a = app();
    // l'or : touché par les news en dollars (pas par celles en dollars canadiens)
    a.chart.source = Source::Oanda { instrument: "XAU_USD".into() };
    ticket(&mut a, 500.0);
    let news = |c: &str| argos_data::eco::Event {
        title: "CPI m/m".into(),
        country: c.into(),
        impact: "High".into(),
        ts: now_s(),
        ..Default::default()
    };
    a.watch.eco.agenda = vec![news("USD")];
    a.place_demo_order();
    assert!(a.demo.positions.is_empty(), "news USD en cours : bloqué");
    assert!(
        a.banner.as_deref().is_some_and(|b| b.contains("news à fort impact") && b.contains("USD CPI")),
        "{:?}",
        a.banner
    );
    a.watch.eco.agenda = vec![news("CAD")];
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1, "news CAD : l'or n'est pas touché");
    // sans calendrier chargé : rien ne bloque
    a.watch.eco.agenda.clear();
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 2);
}

#[test]
fn gardien_propfirm_previent_une_fois_puis_deverrouille() {
    let mut a = app();
    ticket(&mut a, 1_000.0); // 100 unités
    a.place_demo_order();
    // −42 $ × 100 = −4 200 $ : 84 % de la limite du jour (5 000 $), sous la limite
    if let Some(b) = a.chart.bars.last_mut() {
        b.close -= 42.0;
    }
    assert!(!a.guard_watch(), "rien ne ferme sous la limite");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("84 %")), "{:?}", a.banner);
    a.banner = None;
    a.guard_watch();
    assert_eq!(a.banner, None, "une seule fois par jour");
    // limite touchée → verrou, puis « Déverrouiller aujourd'hui »
    if let Some(b) = a.chart.bars.last_mut() {
        b.close -= 20.0;
    }
    assert!(a.guard_watch());
    let maintenant = now_s() * 1000;
    assert!(a.watch.panels.journal.locked(maintenant));
    // 🔓 la boîte demande une raison : vide, la journée reste verrouillée
    a.guard_unlock_demander();
    assert_eq!(a.ui.name_box.as_ref().map(|b| b.kind), Some(NameFor::Raison));
    a.name_ok();
    assert!(a.watch.panels.journal.locked(maintenant), "sans raison : toujours verrouillée");
    assert_eq!(a.banner.as_deref(), Some("Écris une raison pour déverrouiller (obligatoire)"));
    a.guard_unlock_demander();
    for c in "Erreur de saisie, je reprends demain".chars() {
        a.ui.name_box.as_mut().unwrap().type_char(c);
    }
    a.name_ok();
    assert!(!a.watch.panels.journal.locked(maintenant));
    assert_eq!(a.banner.as_deref(), Some("🔓 Déverrouillée aujourd'hui · raison notée au Journal"));
    assert_eq!(a.watch.panels.journal.deverrouillee(maintenant), Some("Erreur de saisie, je reprends demain"));
}

#[test]
fn tout_fermer_ferme_les_positions_et_annule_les_ordres() {
    let mut a = app();
    ticket(&mut a, 500.0);
    a.watch.panels.trading.guardian = false;
    a.place_demo_order();
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    (a.watch.panels.trading.tab, a.watch.panels.trading.px) = (1, last - 20.0);
    a.place_demo_order();
    assert_eq!((a.demo.positions.len(), a.demo.pending.len()), (1, 1));
    a.close_demo_positions();
    assert!(a.demo.positions.is_empty() && a.demo.pending.is_empty(), "tout est fermé et annulé");
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.contains("1 position(s) fermée(s)") && b.contains("1 ordre(s) en attente annulé(s)"), "{b}");
    a.close_demo_positions();
    assert_eq!(a.banner.as_deref(), Some("💼 Aucune position ni ordre démo"));
}

/// 🔐 « Tout fermer »: « inscrit au journal » only when the journal write succeeded; when it did not (journal
/// unavailable, write refused), the position is closed all the same and the banner says it is not noted. Made-up key
/// and a folder of the build directory only (`persist::journal`), never the keyring nor the real journal.
#[test]
fn tout_fermer_dit_si_c_est_note_au_journal() {
    let fermer = |a: &mut App| {
        ticket(a, 500.0);
        a.watch.panels.trading.guardian = false;
        a.place_demo_order();
        assert_eq!(a.demo.positions.len(), 1);
    };
    // written: noted
    let mut a = app();
    fermer(&mut a);
    a.close_demo_positions();
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.contains("1 position(s) fermée(s)") && b.ends_with(", inscrit au journal"), "{b}");
    // journal unavailable (read only): closed all the same, not noted
    let mut a = app();
    fermer(&mut a);
    a.jsync.pret = false;
    a.close_demo_positions();
    let b = a.banner.clone().unwrap_or_default();
    assert!(a.demo.positions.is_empty(), "the closing is always done");
    assert!(
        b.contains("1 position(s) fermée(s)") && b.contains("⚠ non noté au journal (journal indisponible)"),
        "{b}"
    );
    assert!(!b.contains("inscrit au journal"), "{b}");
    // write refused (the journal's file cannot be opened): closed all the same, not noted
    let mut a = app();
    fermer(&mut a);
    let dossier = crate::persist::journal::chemin().unwrap().with_file_name("pas-une-base");
    std::fs::create_dir_all(&dossier).unwrap();
    a.jsync.base = Some(dossier.clone());
    a.close_demo_positions();
    let b = a.banner.clone().unwrap_or_default();
    assert!(
        a.demo.positions.is_empty() && b.contains("⚠ non noté au journal (") && !b.contains("inscrit"),
        "{b}"
    );
    std::fs::remove_dir_all(&dossier).unwrap();
}

/// 💼 « Tout fermer » with only a pending order: the order is cancelled, no position is touched.
#[test]
fn tout_fermer_avec_seulement_des_ordres_en_attente() {
    let mut a = app();
    ticket(&mut a, 500.0);
    a.watch.panels.trading.guardian = false;
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    (a.watch.panels.trading.tab, a.watch.panels.trading.px) = (1, last - 20.0);
    a.place_demo_order();
    assert_eq!((a.demo.positions.len(), a.demo.pending.len()), (0, 1));
    a.close_demo_positions();
    assert!(a.demo.pending.is_empty(), "the pending order is cancelled");
    assert_eq!(a.banner.as_deref(), Some("💼 1 ordre(s) en attente annulé(s)"));
}

#[test]
fn pause_anti_tilt_bloque_les_ordres_apres_des_pertes() {
    let mut a = app();
    ticket(&mut a, 500.0);
    a.watch.panels.journal.cfg.tilt_n = 1;
    let perte = |a: &mut App| {
        a.place_demo_order();
        if let Some(b) = a.chart.bars.last_mut() {
            b.close -= 5.0;
        }
        a.close_demo_positions();
    };
    // gardien éteint : la pause ne démarre pas
    a.watch.panels.trading.guardian = false;
    perte(&mut a);
    assert_eq!(a.watch.panels.journal.cfg.pause_until, 0);
    // gardien allumé : une perte (réglage 1) → pause de 30 min, les ordres sont refusés
    a.watch.panels.trading.guardian = true;
    perte(&mut a);
    assert!(a.watch.panels.journal.cfg.pause_until > now_s() * 1000);
    assert_eq!(a.banner.as_deref(), Some("🧊 Pause anti-tilt : 2 pertes de suite, ordres bloqués 30 min"));
    a.place_demo_order();
    assert!(a.demo.positions.is_empty());
    assert!(
        a.banner.as_deref().is_some_and(|b| b.contains("pause anti-tilt : encore 30 min")),
        "{:?}",
        a.banner
    );
    // la pause finie, l'ordre passe
    a.watch.panels.journal.cfg.pause_until = now_s() * 1000;
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1);
}

// — 🔔 alertes —

#[test]
fn alerte_de_prix_creee_glissee_et_declenchee() {
    let mut a = app();
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    a.add_price_alert(last + 5.0);
    assert_eq!(a.alerts.lines(&a.symbol_label()).len(), 1, "une ligne sur la chart");
    assert_eq!(a.chart_with(false).alerts, vec![last + 5.0]);
    // la ligne glissée plus près, puis le prix la franchit
    let id = a.alerts.list[0].id;
    a.alerts.move_line(id, last + 2.0);
    a.alerts_watch(); // prix de départ
    if let Some(b) = a.chart.bars.last_mut() {
        b.close = last + 3.0;
    }
    a.alerts_watch();
    assert!(
        a.banner.as_deref().is_some_and(|b| b.starts_with("🔔") && b.contains("franchit")),
        "{:?}",
        a.banner
    );
    assert_eq!(a.alerts.history.len(), 1);
    assert!(!a.alerts.list[0].on, "une seule fois par défaut");
}

#[test]
fn menu_des_alertes_modeles_et_options() {
    let mut a = app();
    assert!(a.alerts_pick(31), "option « à chaque fois » : le menu reste ouvert");
    assert!(!a.alerts_pick(10), "modèle RSI : créé, le menu se ferme");
    let al = &a.alerts.list[0];
    assert_eq!(al.every, Some(900));
    assert!(al.text().contains("RSI 14 plus petit que 30"), "{}", al.text());
    let titles: Vec<String> = a.alerts_menu().into_iter().map(|g| g.title).collect();
    assert!(titles.iter().any(|t| t.starts_with("MES ALERTES")), "{titles:?}");
    a.alerts_pick(2_000_000);
    assert!(a.alerts.list.is_empty(), "supprimée");
}

// — 💼 ordres Limit / Stop et comptes démo —

#[test]
fn ordre_limit_place_puis_execute_avec_journal() {
    let mut a = app();
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    {
        let t = &mut a.watch.panels.trading;
        (t.buy, t.sl_pips, t.risk, t.guardian, t.plan, t.tab, t.px) = (true, 10.0, 500.0, false, None, 1, last - 20.0);
    }
    a.place_demo_order();
    assert!(a.demo.positions.is_empty(), "pas avant le prix");
    assert_eq!(a.demo.pending.len(), 1);
    let o = &a.chart_with(false).ordres.ordres;
    assert_eq!(
        (o.len(), o[0].entree, o[0].buy, o[0].attente),
        (1, last - 20.0, true, Some("Limite")),
        "ligne sur la chart"
    );
    // mauvais côté : refusé, message clair
    a.watch.panels.trading.px = last + 20.0;
    a.place_demo_order();
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("Buy Limit se place sous")), "{:?}", a.banner);
    assert_eq!(a.demo.pending.len(), 1);
    // le prix descend jusqu'à l'ordre : position ouverte, inscrite au journal
    let n = a.watch.panels.journal.trades.len();
    if let Some(b) = a.chart.bars.last_mut() {
        b.close = last - 21.0;
    }
    a.demo_exits();
    assert!(a.demo.pending.is_empty());
    assert_eq!(a.demo.positions.len(), 1);
    assert_eq!(a.demo.positions[0].entry, last - 20.0);
    assert_eq!(a.watch.panels.journal.trades.len(), n + 1);
}

#[test]
fn ordre_stop_annule_et_gardien_a_l_execution() {
    let mut a = app();
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    {
        let t = &mut a.watch.panels.trading;
        (t.buy, t.sl_pips, t.risk, t.guardian, t.plan, t.tab, t.px) = (true, 10.0, 500.0, false, None, 2, last + 15.0);
    }
    a.place_demo_order();
    let id = a.demo.pending[0].id;
    a.cancel_demo_order(id);
    assert!(a.demo.pending.is_empty(), "annulé");
    // le gardien (allumé après coup) refuse à l'exécution : positions max à 0
    a.place_demo_order();
    a.watch.panels.journal.cfg.max_pos = 0;
    a.watch.panels.journal.cfg.risk_pct = 0.1; // 100 $ par trade au plus sur 100K
    a.watch.panels.trading.guardian = true;
    if let Some(b) = a.chart.bars.last_mut() {
        b.close = last + 16.0;
    }
    a.demo_exits();
    assert!(a.demo.positions.is_empty() && a.demo.pending.is_empty(), "refusé et annulé");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("annulé à l'exécution")), "{:?}", a.banner);
}

#[test]
fn deux_comptes_demo_independants_et_migration() {
    let mut a = app();
    a.accounts_ready();
    assert_eq!(a.accounts.list.len(), 1);
    assert_eq!(a.demo.name, "Démo 1", "l'ancien compte devient Démo 1");
    ticket(&mut a, 500.0);
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1);
    a.new_account(25_000.0);
    assert_eq!(
        (a.demo.name.as_str(), a.demo.balance, a.demo.positions.len()),
        ("Démo 2", argos_core::Money::from_f64(25_000.0), 0),
        "nouveau compte vide"
    );
    assert_eq!(a.watch.panels.journal.cfg.account, 25_000.0, "gardien réglé sur son solde");
    a.switch_account(0);
    assert_eq!(a.demo.positions.len(), 1, "le premier compte a gardé sa position");
    assert!(!a.delete_account() || a.accounts.list.len() == 1);
    assert!(!a.delete_account(), "jamais le dernier compte");
}

// — 🔥 heatmap du carnet et Bid & Ask —

#[test]
fn carnet_indisponible_et_heatmap_remplie() {
    let mut a = app();
    // fichier local : pas de carnet public, un message discret (pas d'erreur)
    a.chart.settings.lines.bid_ask = true;
    assert_eq!(a.book_coin(), None);
    assert_eq!(a.chart_with(false).book_note, Some("📖 Carnet indisponible pour ce symbole"));
    assert_eq!(a.chart_with(false).bid_ask, None);
    // heatmap allumée : chaque photo du carnet remplit la colonne de la bougie en cours
    a.ind.on.push(argos_render::indicators::Ind::BookHeat);
    let px = a.chart.bars.last().unwrap().close;
    let lvl = |p: f64, s: f64| argos_data::book::Level { px: p, sz: s, n: 1 };
    let b = argos_data::book::Book {
        coin: "BTC".into(),
        time_ms: 0,
        bids: vec![lvl(px - 1.0, 5.0), lvl(px - 2.0, 1.0)],
        asks: vec![lvl(px + 1.0, 2.0)],
    };
    a.heat_add(&b);
    let h = a.feeds.heat.as_ref().expect("heatmap créée");
    assert_eq!(h.cols.len(), 1);
    assert!(!h.cols.values().next().unwrap().is_empty());
    // coupée : plus rien n'est gardé
    a.ind.on.clear();
    a.heat_add(&b);
    assert!(a.feeds.heat.is_none());
}

/// 🧪 Jee (20 nov.) : « je ne vois pas jusqu'en bas » : le volet du Simulateur défile au doigt (cell, tablette) comme
/// à la molette, et rien n'est choisi en glissant.
#[test]
fn le_doigt_fait_defiler_le_simulateur() {
    use winit::event::TouchPhase;
    for phone in [true, false] {
        let mut a = if phone { cell_390() } else { app() };
        a.toggle_simulateur();
        let r = a.sim_rect();
        let (x, y0) = (r.x + r.w / 2.0, r.y + r.h - 80.0);
        a.touch(1, TouchPhase::Started, (x, y0));
        for k in 1..=10 {
            a.touch(1, TouchPhase::Moved, (x, y0 - k as f64 * 25.0));
        }
        a.touch(1, TouchPhase::Ended, (x, y0 - 250.0));
        let s = a.ui.sim.as_ref().map_or(0.0, |s| s.scroll);
        assert!(s > 100.0, "phone {phone} : le volet a défilé ({s})");
        assert!(a.demo.positions.is_empty() && a.demo.pending.is_empty(), "glisser n'a rien placé");
        // la molette (PC) aussi, jusqu'au bout et pas plus loin
        a.pointer.win_mouse = (x, r.cy());
        for _ in 0..200 {
            a.sim_wheel(40.0);
        }
        let fin = a.ui.sim.as_ref().unwrap().scroll;
        a.sim_wheel(40.0);
        assert_eq!(a.ui.sim.as_ref().unwrap().scroll, fin, "au bout");
    }
}

/// 🖥 Jee (20 nov.) : sur PC (1440 × 900), le volet du Simulateur défile à la molette jusqu'au bout : Résultat et la
/// dernière ligne de l'Historique s'atteignent, au-dessus du rond Argos AI posé sur le bas du volet, même quand
/// l'examen PropFirm a commencé (ses lignes comptent dans la limite de défilement).
#[test]
fn pc_le_simulateur_defile_jusqu_au_bout() {
    dossier_a_soi("pc_le_simulateur_defile_jusqu_au_bout");
    let mut a = pc_avec_volet(wl::Tool::Trading);
    a.toggle_simulateur();
    for i in 0..12 {
        let id = a
            .demo
            .open(
                argos_core::portfolio::Order {
                    base: "BTC".into(),
                    side: argos_core::Side::Long,
                    qty: 1.0,
                    price: 100.0,
                    stop: Some(90.0),
                    target: None,
                    couts: None,
                    perp: None,
                },
                now_s() - 100 + i,
            )
            .unwrap();
        a.demo.close(id, 101.0, now_s() - 50 + i, "objectif").unwrap();
    }
    assert!(a.sim_examen().is_some(), "l'examen a commencé : ses lignes sont dans le volet");
    let r = a.sim_rect();
    a.pointer.win_mouse = (r.cx(), r.cy());
    for _ in 0..400 {
        a.sim_wheel(40.0);
    }
    let s = a.ui.sim.clone().unwrap();
    let fin = a.sim_vue(|v| s.fin_des_lignes(v, r.w));
    let bas_visible = s.scroll + a.sim_vue(|v| argos_render::simulateur::Simulateur::visible(r, v));
    assert!(fin <= bas_visible, "fin {fin} sous le bas visible {bas_visible} (défilé {})", s.scroll);
}

#[test]
fn le_doigt_fait_defiler_le_menu_des_unites() {
    use winit::event::TouchPhase;
    let mut a = app();
    a.area = (375, 600);
    a.open_list(ListKind::Intervals, Rect::new(60.0, 560.0, 40.0, 24.0));
    let m = a.ui.list.as_ref().map(|(_, m)| m.clone()).unwrap();
    eprintln!("rect {:?} scroll {}", m.rect, m.scroll);
    let (x, y0) = (m.rect.x + 40.0, m.rect.y + m.rect.h - 60.0);
    a.touch(1, TouchPhase::Started, (x, y0));
    for k in 1..=10 {
        a.touch(1, TouchPhase::Moved, (x, y0 - k as f64 * 15.0));
    }
    a.touch(1, TouchPhase::Ended, (x, y0 - 150.0));
    let s = a.ui.list.as_ref().map(|(_, m)| m.scroll);
    eprintln!(
        "after {:?} win_mouse {:?} dock {:?} side {:?}",
        s, a.pointer.win_mouse, a.pointer.dock_mouse, a.pointer.side_mouse
    );
    assert!(s.is_some_and(|s| s != m.scroll), "le menu a défilé");
    assert_eq!(a.chart.interval, Interval(1800), "glisser n'a rien choisi");
    assert!(a.ui.list.is_some(), "le menu reste ouvert");
}

#[test]
fn cell_le_doigt_fait_defiler_la_barre_des_favoris() {
    use winit::event::TouchPhase;
    let mut a = app();
    a.tools.pad = true;
    a.tools.max_w = 300.0;
    (a.tools.x, a.tools.y) = (8.0, 200.0);
    let text = argos_render::text::Text::default();
    assert!(toolbar::max_scroll(&a.tools, &text) > 0.0);
    let (wx, wy) = (200.0 + a.rw(), 225.0 + a.top_h());
    a.touch(7, TouchPhase::Started, (wx, wy));
    for k in 1..=10 {
        a.touch(7, TouchPhase::Moved, (wx - k as f64 * 15.0, wy));
    }
    a.touch(7, TouchPhase::Ended, (wx - 150.0, wy));
    assert!(a.tools.scroll > 100.0, "la barre a défilé ({})", a.tools.scroll);
    assert_eq!(a.chart.interval, Interval(1800), "glisser n'a rien choisi");
}

#[test]
fn double_clic_puis_main_partout() {
    let mut a = app();
    a.run_action(ChartAction::ResetPrice);
    assert!(a.chart.settings.scale.auto());
    let (wx, wy) = (300.0 + a.rw(), 200.0 + a.top_h());
    (a.pointer.last_chart_click, a.pointer.triple) = (None, None); // (a little later: no third click)
    clic(&mut a, wx, wy);
    clic(&mut a, wx, wy);
    assert!(!a.chart.settings.scale.auto(), "double-clic : mode libre");
    let before = a.chart.settings.scale.manual;
    (a.pointer.last_chart_click, a.pointer.triple) = (None, None); // (a little later: no third click)
    a.mouse_moved(PhysicalPosition::new(wx, wy));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(wx + 40.0, wy + 60.0));
    a.left_button(ElementState::Released);
    assert_ne!(a.chart.settings.scale.manual, before, "glisser vers le bas : le prix suit");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("Main libre")), "le bandeau explique le mode");
    // nouveau double-clic : prix automatique ; ↺ RAZ aussi
    (a.pointer.last_chart_click, a.pointer.triple) = (None, None); // (a little later: no third click)
    clic(&mut a, wx, wy);
    clic(&mut a, wx, wy);
    assert!(a.chart.settings.scale.auto(), "2e double-clic : prix automatique");
    (a.pointer.last_chart_click, a.pointer.triple) = (None, None); // (a little later: no third click)
    clic(&mut a, wx, wy);
    clic(&mut a, wx, wy);
    assert!(!a.chart.settings.scale.auto());
    a.run_action(ChartAction::ResetView);
    assert!(a.chart.settings.scale.auto(), "↺ RAZ : prix automatique");
}

#[test]
fn alerte_sur_une_ligne_d_indicateur() {
    let mut a = app();
    a.alerts = Default::default();
    a.ind.on = vec![argos_render::indicators::Ind::Rsi];
    a.ind.key = Default::default();
    a.refresh_indicators();
    let lines = a.alert_lines();
    let rsi = lines.iter().find(|l| l.0.contains("RSI")).expect("ligne RSI proposée").0.clone();
    let pick = |a: &mut App, label: &str| {
        let id = a
            .alerts_menu()
            .iter()
            .flat_map(|g| g.items.clone())
            .find(|m| m.label.contains(label))
            .map(|m| m.id);
        a.alerts_pick(id.unwrap_or_else(|| panic!("ligne « {label} » absente")))
    };
    pick(&mut a, &format!("📈 {rsi}"));
    assert!(
        a.alerts_menu().iter().any(|g| g.title.starts_with("CONDITION")),
        "la condition se choisit ensuite"
    );
    assert!(!pick(&mut a, "sort de la zone"), "zone : la boîte de saisie s'ouvre");
    assert!(a.ui.name_box.is_some());
    a.ui.name_box = None;
    a.alert_value_typed("abc");
    assert!(a.alerts.list.is_empty(), "pas de nombre : rien n'est créé");
    a.alert_value_typed("70 30");
    assert_eq!(a.alerts.list.len(), 1);
    let t = a.alerts.list[0].text();
    assert!(t.contains(&rsi) && t.contains("sort de la zone 30 – 70"), "{t}");
    // « croise le prix » : créée tout de suite, sans saisie
    pick(&mut a, "croise le prix vers le haut");
    assert_eq!(a.alerts.list.len(), 2);
    a.alerts_watch(); // la surveillance passe les lignes de la chart (pas de panique, rien d'obligatoire)
}

#[test]
fn rangee_des_annonces_nouvelles_et_fenetres_propfirm() {
    use argos_data::eco::{Event, News};
    let mut a = app();
    let ev = |impact: &str, ts: i64| Event {
        title: "CPI".into(),
        country: "USD".into(),
        impact: impact.into(),
        ts,
        forecast: String::new(),
        previous: String::new(),
        actual: String::new(),
    };
    a.watch.eco.agenda = vec![ev("High", 1000), ev("Medium", 2000), ev("Low", 3000)];
    let news = |title: &str, ts: i64| News {
        title: title.into(),
        link: format!("https://x.fr/{ts}"),
        ts,
        source: "Agence".into(),
        lang: "fr".into(),
    };
    // (two news of the same 1 h candle, one about the market only)
    a.watch.eco.news = vec![news("Le bitcoin bouge", 7_300), news("Le BTC recule", 9_000), news("Le marché bouge", 8_000)];
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    a.chart.interval = argos_core::Interval(3600);
    a.watch.panels.journal.cfg.news = true;
    // ⚡ panel closed or open: the news of the chart's crypto, one per candle (the market-only one is not there)
    for open in [false, true] {
        (a.watch.prefs.open, a.watch.prefs.panel) = (open, wl::Tool::News);
        let m = a.econ_marks();
        assert_eq!(m.len(), 3, "fort + moyen + un ⚡ (le faible n'est pas montré)");
        let e = m.iter().find(|x| x.currency == argos_render::econ::NEWS).unwrap();
        assert_eq!(e.ts, 7_200, "at its candle's time");
        assert_eq!(e.link, "https://x.fr/9000", "the newest of that candle");
        assert_eq!(m[0].window_s, 120, "fenêtre PropFirm ±2 min sur l'annonce à fort impact");
        assert_eq!(m[1].window_s, 0);
    }
    // not a crypto chart: no ⚡
    a.chart.source = Source::Oanda { instrument: "XAU_USD".into() };
    assert_eq!(a.econ_marks().len(), 2);
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    // interrupteurs du menu de la chart
    a.chart.settings.propfirm_windows = false;
    assert!(a.econ_marks().iter().all(|x| x.window_s == 0));
    a.chart.settings.econ_news = false;
    assert_eq!(a.econ_marks().len(), 2);
    a.chart.settings.econ_events = false;
    assert!(a.econ_marks().is_empty());
    // gardien sans blocage des news : pas de fenêtre
    (a.chart.settings.econ_events, a.chart.settings.propfirm_windows) = (true, true);
    a.watch.panels.journal.cfg.news = false;
    assert!(a.econ_marks().iter().all(|x| x.window_s == 0));
}

#[test]
fn dessins_defaire_refaire() {
    use argos_render::drawings::{Drawing, Point};
    let mut a = app();
    a.chart.drawings.clear();
    a.track_drawings(); // relevé de départ
    let p = |t: i64, px: f64| Point { t, p: px };
    // ajout, déplacement, style, suppression : un pas chacun
    a.chart.drawings.push(Drawing::new("hline", vec![p(100, 10.0)]));
    a.track_drawings();
    a.chart.drawings[0].pts[0].p = 12.0;
    a.track_drawings();
    a.chart.drawings[0].style.width = 3.0;
    a.track_drawings();
    a.chart.drawings.clear();
    a.track_drawings();
    a.run_action(ChartAction::Undo);
    assert_eq!(a.chart.drawings.len(), 1, "suppression défaite");
    assert_eq!(a.chart.drawings[0].style.width, 3.0);
    a.run_action(ChartAction::Undo);
    assert_ne!(a.chart.drawings[0].style.width, 3.0, "style défait");
    a.run_action(ChartAction::Undo);
    assert_eq!(a.chart.drawings[0].pts[0].p, 10.0, "déplacement défait");
    a.run_action(ChartAction::Redo);
    assert_eq!(a.chart.drawings[0].pts[0].p, 12.0, "déplacement refait");
    // un nouveau changement vide « refaire »
    a.chart.drawings.push(Drawing::new("hline", vec![p(200, 20.0)]));
    a.track_drawings();
    a.run_action(ChartAction::Redo);
    assert_eq!(a.chart.drawings.len(), 2, "plus rien à refaire après un nouveau dessin");
    // au plus 100 pas
    for k in 0..150 {
        a.chart.drawings[0].pts[0].p = k as f64;
        a.track_drawings();
    }
    assert_eq!(a.chart.hist.borrow().undo.len(), crate::state::HIST_MAX);
    // raccourcis : Ctrl + Maj + Z et Ctrl + Y = refaire
    use argos_render::actions::{from_key, Key, Mods};
    let m = |ctrl, shift| Mods {
        ctrl,
        shift,
        ..Default::default()
    };
    assert_eq!(from_key(Key::Char('z'), m(true, false)), Some(ChartAction::Undo));
    assert_eq!(from_key(Key::Char('z'), m(true, true)), Some(ChartAction::Redo));
    assert_eq!(from_key(Key::Char('y'), m(true, false)), Some(ChartAction::Redo));
    // autre symbole : historique à part
    a.restore_session();
    assert!(a.chart.hist.borrow().undo.is_empty());
}

#[test]
fn split_reticule_et_temps_suivent_la_chart_principale() {
    let mut a = app();
    a.pointer.mouse = Some((200.0, 150.0));
    assert_eq!(a.split_follow(), (None, None), "synchros éteintes : rien ne suit");
    (a.split.sync_time, a.split.sync_crosshair) = (true, true);
    let (range, cross) = a.split_follow();
    let (t0, t1) = range.expect("plage de temps visible");
    assert!(t0 < t1 && t1 <= a.chart.bars.last().unwrap().time);
    let (pt, rel) = cross.expect("point du réticule");
    assert!(pt.t >= t0 && pt.t <= t1, "heure du réticule dans la plage visible");
    assert!(pt.p.is_finite() && (0.0..=1.0).contains(&rel));
    a.pointer.mouse = Some((-5.0, 10.0));
    assert!(a.split_follow().1.is_none(), "souris hors de la chart : pas de réticule copié");
}

#[test]
fn symboles_calcules_dans_la_recherche_sans_doublon() {
    use crate::actions::synth_results;
    let have = Default::default();
    let r = synth_results("XAUGBP", &have, false);
    assert_eq!(r.len(), 1);
    assert!(r[0].id == "synth:XAUGBP" && r[0].detail.contains("calculé") && r[0].detail.contains("xyz:GOLD"));
    let names: Vec<String> = synth_results("xau", &have, false).into_iter().map(|f| f.name).collect();
    assert_eq!(names, ["XAUGBP", "XAUEUR"]);
    assert!(synth_results("xageur", &have, true)[0].detail.contains("OANDA"), "avec un jeton : OANDA");
    assert!(synth_results("dxy", &have, false)[0].detail.contains("Yahoo"));
    // ouvrir la ligne de watchlist : la source calculée
    let mut a = app();
    a.open_coin("synth:XAUEUR".into());
    assert!(matches!(&a.chart.source, Source::Synth { sym } if sym == "XAUEUR"));
    assert!(a.title.contains("XAUEUR (calculé, approximatif)"));
}

/// 📈 Le module EMA (bulle du kit) donne la même courbe que l'EMA intégrée d'Argos, à l'arrondi de sa sortie près
/// (échelle des prix + 4 décimales) : sur des bougies fixes, longueurs 10 et 20, et sur l'ouverture.
#[test]
fn module_ema_egal_a_l_ema_integree() {
    use argos_module_host::{engine::Engine, input_of, lines_of, sha384, Manifest, RawBar};
    let c: Vec<f64> = (0..300).map(|i| 100.0 + 10.0 * (i as f64 * 0.5).sin() + i as f64 * 0.3).collect();
    let raw: Vec<RawBar> = (0..300)
        .map(|i| {
            let o = if i == 0 { c[0] - 0.5 } else { c[i - 1] };
            (1_700_000_000 + i as i64 * 3600, o, o.max(c[i]) + 1.0, o.min(c[i]) - 1.0, c[i], 1000.0)
        })
        .collect();
    let m = Manifest::parse(crate::actions::modules::EMA_JSON).unwrap();
    m.check().unwrap();
    let wasm = crate::actions::modules::EMA_WASM;
    argos_module_host::scan(wasm).unwrap();
    let e = Engine::new().unwrap();
    let dir = std::env::temp_dir().join(format!("argos-ema-{}", std::process::id()));
    for (len, source) in [(10_i64, 0_i64), (20, 0), (20, 1)] {
        let input = input_of("TEST", 3600, &raw, vec![len, source]);
        // 🎯 les prix entrent en entiers à l'échelle choisie par l'app : l'EMA intégrée calculée sur ces mêmes prix
        let k = 10f64.powi(input.price_scale as i32);
        let src: Vec<f64> = input.bars.iter().map(|b| if source == 1 { b.o } else { b.c } as f64 / k).collect();
        let want = argos_script::ta::ema(&src, len as usize);
        let out = e.run(&m, wasm, &sha384(wasm), &[], &dir, &[], &input).expect("appel dans la bulle");
        let got = &lines_of(&out)[0].1;
        let tol = 0.5 * 10f64.powi(-(out.lines[0].scale as i32)) + 1e-9;
        assert_eq!(got.len(), want.len());
        for (i, (g, w)) in got.iter().zip(&want).enumerate() {
            assert_eq!(g.is_nan(), w.is_nan(), "EMA {len} bougie {i} : vide des deux côtés");
            if w.is_finite() {
                assert!((g - w).abs() <= tol, "EMA {len} bougie {i} : module {g} / intégrée {w}");
            }
        }
    }
}

/// 🎓 Le module de l'Académie tourne dans la bulle du kit : range asiatique d'une journée d'hiver (heure de New York),
/// suivi pendant la fenêtre puis figé ; il est dans le catalogue embarqué.
#[test]
fn module_academie_dans_la_bulle() {
    use argos_module_host::{engine::Engine, input_of, lines_of, sha384, Manifest, RawBar};
    let m = Manifest::parse(crate::actions::modules::ACADEMIE_JSON).unwrap();
    m.check().unwrap();
    let wasm = crate::actions::modules::ACADEMIE_WASM;
    argos_module_host::scan(wasm).unwrap();
    // 15 janvier 2026, 23 h UTC = 18 h à New York ; bougies de 30 min, prix qui monte de 1 à chaque bougie
    let t0 = 1_768_518_000;
    let raw: Vec<RawBar> = (0..20)
        .map(|i| (t0 + i * 1800, 100.0 + i as f64, 100.5 + i as f64, 99.5 + i as f64, 100.0 + i as f64, 1.0))
        .collect();
    let input = input_of("XAUUSD", 1800, &raw, vec![0]);
    let dir = std::env::temp_dir().join(format!("argos-academie-{}", std::process::id()));
    let out = Engine::new()
        .unwrap()
        .run(&m, wasm, &sha384(wasm), &[], &dir, &[], &input)
        .expect("appel dans la bulle");
    let lines = lines_of(&out);
    assert_eq!(lines.len(), 2);
    let (h, l) = (&lines[0].1, &lines[1].1);
    assert!(h[3].is_nan(), "rien avant 20 h");
    assert!((h[4] - 104.5).abs() < 1e-9 && (l[4] - 103.5).abs() < 1e-9, "20 h : première bougie");
    assert!((h[11] - 111.5).abs() < 1e-9 && (l[19] - 103.5).abs() < 1e-9, "suivi jusqu'à minuit puis figé");
    assert_eq!(out.labels.len(), 1);
    assert!(app().catalog().iter().any(|(_, m)| m.path == "trading/academie"), "dans le catalogue embarqué");
}

#[test]
fn catalogue_local_fiche_installer_exporter_importer() {
    let mut a = app();
    (a.mods.list, a.mods.catalog) = (vec![], vec![]);
    let pick = |a: &mut App, label: &str| {
        let id = a
            .modules_menu()
            .iter()
            .flat_map(|g| g.items.clone())
            .find(|m| m.label.contains(label))
            .map(|m| m.id);
        a.modules_pick(id.unwrap_or_else(|| panic!("ligne « {label} » absente")))
    };
    // l'exemple est dans le catalogue ; sa fiche montre les droits avant l'installation
    let g = a.modules_menu();
    assert!(
        g.iter().any(|g| g.title.starts_with("🛒 CATALOGUE LOCAL (4)")),
        "l'exemple du kit, l'EMA d'Argos, les sessions de l'Académie et le pinceau de zone"
    );
    pick(&mut a, "+ Moyenne mobile");
    let sheet = a.modules_menu();
    assert!(sheet[0].items.iter().any(|m| m.label.contains("Aucun droit")));
    assert!(sheet[0].items.iter().any(|m| m.label.contains("Empreinte SHA-384 vérifiée")));
    pick(&mut a, "+ Installer");
    assert_eq!(a.mods.list.len(), 1, "installé depuis le catalogue");
    assert!(a.modules_menu().iter().flat_map(|g| g.items.clone()).any(|m| m.label.starts_with("✅ Moyenne")));
    // exporter mes modules, puis importer une version 2 : la mise à jour est proposée
    let mut file = a.catalog_file();
    assert_eq!(file.modules.len(), 1);
    file.modules[0].manifest = file.modules[0].manifest.replace("\"version\": 1", "\"version\": 2");
    let msg = a.catalog_import(&serde_json::to_string(&file).unwrap());
    assert!(msg.contains("1 module(s) vérifié(s)"), "{msg}");
    assert!(
        a.modules_menu().iter().flat_map(|g| g.items.clone()).any(|m| m.label.starts_with("⬆ v2")),
        "mise à jour proposée"
    );
    // un fichier abîmé (empreinte différente) : refusé, nommé
    file.modules[0].sha384 = "ab".repeat(48);
    file.modules[0].title = "abîmé".into();
    assert!(a.catalog_import(&serde_json::to_string(&file).unwrap()).contains("abîmé : empreinte"));
    assert!(a.catalog_import("{}").contains("pas un catalogue"));
}

#[test]
fn modules_reglages_longueur_source_gardes_a_la_mise_a_jour() {
    let mut a = app();
    a.mods.list.clear();
    let pick = |a: &mut App, label: &str| {
        let id = a
            .modules_menu()
            .iter()
            .flat_map(|g| g.items.clone())
            .find(|m| m.label.contains(label))
            .map(|m| m.id);
        a.modules_pick(id.unwrap_or_else(|| panic!("ligne « {label} » absente")))
    };
    pick(&mut a, "📦 Installer le module d'exemple");
    pick(&mut a, "▶ Démarrer");
    assert_eq!(a.mods.list[0].settings(), [20, 0], "kit : longueur 20, source clôture");
    let last = |a: &mut App| {
        a.ind.key = Default::default();
        a.refresh_indicators();
        let v = a.ind.out.scripts.iter().find(|v| v.name.contains("moyenne")).unwrap().clone();
        assert!(v.error.is_none(), "{:?}", v.error);
        (v.out.plots[0].values.last().copied().unwrap(), v.out.plots[0].title.clone())
    };
    let (v20, t20) = last(&mut a);
    assert!(t20.contains("20") && v20.is_finite());
    // ⚙ longueur 5 (saisie) et source suivante (ouverture) : le module tourne de nouveau
    pick(&mut a, "⚙ Réglages");
    assert!(a.modules_menu()[0].title.contains("RÉGLAGES"));
    assert!(!pick(&mut a, "Longueur : 20"), "nombre : la boîte de saisie s'ouvre");
    a.ui.name_box = None;
    a.mod_number_typed("5");
    a.mods.settings_for = Some(a.mods.list[0].id.clone());
    pick(&mut a, "Source : close");
    let (v5, t5) = last(&mut a);
    assert!(t5.contains("5") && v5 != v20, "longueur 5 sur l'ouverture : autre moyenne");
    a.mods.editing = Some((a.mods.list[0].id.clone(), "length".into()));
    a.mod_number_typed("9999");
    assert_eq!(a.mods.list[0].values["length"], 500.0, "borné au max du manifeste");
    // mise à jour du module (version 2) : valeurs gardées
    let json = crate::actions::modules_example_json().replace("\"version\": 1", "\"version\": 2");
    a.mod_install(&json, crate::actions::modules_example_wasm()).unwrap();
    assert_eq!(
        (a.mods.list[0].version.as_str(), a.mods.list[0].settings()),
        ("2", vec![500, 1]),
        "gardées à la mise à jour"
    );
    // ↺ valeurs par défaut
    a.mods.settings_for = Some(a.mods.list[0].id.clone());
    pick(&mut a, "↺ Valeurs par défaut");
    assert_eq!(a.mods.list[0].settings(), [20, 0]);
}

#[test]
fn modules_installer_droits_demarrer_retirer() {
    let mut a = app();
    a.mods.list.clear(); // un autre test peut avoir laissé modules.json
    let pick_label = |a: &mut App, label: &str| {
        let id = a
            .modules_menu()
            .iter()
            .flat_map(|g| g.items.clone())
            .find(|m| m.label.contains(label))
            .map(|m| m.id);
        a.modules_pick(id.unwrap_or_else(|| panic!("ligne « {label} » absente")))
    };
    // son propre chemin (un autre test installe « trading/moyenne » en même temps dans le même dossier de tests)
    let own = crate::actions::modules_example_json().replace("trading/moyenne", "trading/moyenne-essai");
    a.mod_install(&own, crate::actions::modules_example_wasm()).unwrap();
    assert_eq!(a.mods.list.len(), 1);
    assert!(!a.mods.list[0].on && !a.mods.list[0].pending(), "installé arrêté, aucun droit à valider");
    assert_eq!(a.mods.list[0].sha.len(), 96, "empreinte SHA-384 gardée comme accord");
    pick_label(&mut a, "▶ Démarrer");
    a.ind.key = Default::default();
    a.refresh_indicators();
    let v = a
        .ind
        .out
        .scripts
        .iter()
        .find(|v| v.name.contains("moyenne"))
        .expect("courbe du module sur la chart");
    assert!(v.error.is_none(), "{:?}", v.error);
    assert_eq!(v.out.plots[0].values.len(), a.chart.bars.len());
    assert!(
        v.out.plots[0].values.last().unwrap().is_finite(),
        "moyenne calculée par le module (bulle du kit)"
    );
    // 🔐 nouvelle version qui demande le dossier privé : redemandé, refusé tant qu'il n'est pas accepté
    let json = own
        .replace("\"version\": 1", "\"version\": 2")
        .replace("\"capabilities\": []", "\"capabilities\": [\"ReadFiles\", \"WriteFiles\"]");
    a.mod_install(&json, crate::actions::modules_example_wasm()).unwrap();
    let e = a.mods.list[0].clone();
    assert!(e.on && e.version == "2" && e.pending() && !e.fichiers, "réglages gardés, droit nouveau refusé");
    assert_eq!(a.mods.rights_for.as_deref(), Some(e.id.as_str()), "fenêtre des droits ouverte");
    pick_label(&mut a, "📁 Lire et écrire");
    pick_label(&mut a, "Valider");
    assert!(a.mods.list[0].fichiers && !a.mods.list[0].pending());
    // 🌐 réseau et 🔑 clé : refusés à l'installation
    let net = json.replace("[\"ReadFiles\", \"WriteFiles\"]", "[\"Network\"], \"domains\": [\"api.hyperliquid.xyz\"]");
    assert!(a.mod_install(&net, crate::actions::modules_example_wasm()).unwrap_err().contains("réseau"));
    assert!(a
        .mod_install(&json.replacen('{', "{ \"cles\": \"x\",", 1), crate::actions::modules_example_wasm())
        .is_err());
    // 🗑 retirer : deux touches
    pick_label(&mut a, "🗑 Retirer");
    assert_eq!(a.mods.list.len(), 1, "première touche : seulement armé");
    pick_label(&mut a, "Confirmer");
    assert!(a.mods.list.is_empty());
}

#[test]
fn communaute_idee_avec_image_partagee_et_exportee() {
    let mut a = app();
    a.ui.ideas.clear(); // un autre test peut avoir laissé ideas.json dans le dossier de tests
    let pick = |a: &mut App, label: &str| {
        let id = a
            .community_menu()
            .iter()
            .flat_map(|g| g.items.clone())
            .find(|m| m.label.contains(label))
            .map(|m| m.id);
        a.community_pick(id.unwrap_or_else(|| panic!("ligne « {label} » absente")), Rect::new(300.0, 100.0, 40.0, 24.0))
    };
    pick(&mut a, "📸 Publier avec");
    a.ui.name_box = None;
    a.idea_typed("long 100 sl 95 tp 115 test image");
    let i = a.ui.ideas.last().cloned().unwrap();
    assert!(i.snap && i.image.is_empty(), "image rangée à part, pas dans ideas.json");
    let png = App::idea_image(&i).expect("image relue");
    let pm = tiny_skia::Pixmap::decode_png(&png).unwrap();
    assert!(pm.width() <= 1280 && pm.width() > 100);
    assert!(!serde_json::to_string(&a.ui.ideas).unwrap().contains("iVBOR"), "pas de PNG dans ideas.json");
    // 🔎 le détail : partager et télécharger l'image
    pick(&mut a, "📸 ▲");
    let g = a.community_menu();
    assert!(g[0].title.contains("IDÉE") && g[0].items.iter().any(|m| m.label.contains("Partager")));
    assert!(g[0].items.iter().any(|m| m.label.contains("Télécharger l'image")));
    // export avec l'image, import ailleurs sans doublon
    let mut out = i.clone();
    use base64::Engine;
    out.image = base64::engine::general_purpose::STANDARD.encode(&png);
    out.id = "venue-dailleurs".into();
    out.snap = false;
    assert_eq!(a.merge_ideas(vec![out.clone(), out.clone()]), 1);
    let got = a.ui.ideas.iter().find(|x| x.id == "venue-dailleurs").unwrap().clone();
    assert!(got.snap && got.image.is_empty() && App::idea_image(&got).as_deref() == Some(&png[..]));
}

#[test]
fn communaute_idees_gardees_rouvertes_sans_doublon() {
    let mut a = app();
    a.ui.ideas.clear(); // un autre test peut avoir laissé ideas.json dans le dossier de tests
    a.idea_typed("short 105 sl 110 tp 90 rejet de la zone");
    assert_eq!(a.ui.ideas.len(), 1);
    let i = a.ui.ideas[0].clone();
    assert!(!i.long && i.entry == Some(105.0) && i.text == "rejet de la zone");
    assert_eq!(i.interval, a.chart.interval.secs(), "l'unité de la chart");
    // import : le même identifiant ne s'ajoute pas deux fois
    assert_eq!(a.merge_ideas(vec![i.clone(), i.clone()]), 0);
    let mut j = i.clone();
    j.id = "autre".into();
    assert_eq!(a.merge_ideas(vec![j]), 1);
    assert_eq!(a.ui.ideas.len(), 2);
    // texte sans sens : refusé, rien n'est gardé
    a.idea_typed("juste un texte");
    assert_eq!(a.ui.ideas.len(), 2);
    let g = a.community_menu();
    assert!(g[1].title.contains("IDÉES (2)"));
    assert!(
        g[2].items.iter().any(|m| m.label.contains("Nostr") && !m.enabled),
        "Nostr : emplacement prévu, éteint"
    );
}

/// 📱 Communauté au doigt : plus de doublon des tuiles du menu ⋯ Argos (Académie, Marketplace, Modules), plus de ligne
/// « Bientôt » (Nostr), « Importer des idées » sans le nom du fichier ; le PC garde tout.
#[test]
fn communaute_au_doigt_sans_doublon_ni_bientot() {
    let a = app();
    let lignes = |a: &App| -> Vec<String> {
        a.community_menu()
            .iter()
            .flat_map(|g| g.items.iter().map(|m| m.label.clone()).collect::<Vec<_>>())
            .collect()
    };
    argos_render::actions::set_tactile(false);
    let pc = lignes(&a);
    argos_render::actions::set_tactile(true);
    let doigt = lignes(&a);
    argos_render::actions::set_tactile(false);
    for l in [
        "Académie Argos",
        "Marketplace d'indicateurs",
        "Modules : catalogue",
        "Nostr",
        "argos-idees.json",
    ] {
        assert!(pc.iter().any(|m| m.contains(l)), "PC : « {l} » reste");
        assert!(!doigt.iter().any(|m| m.contains(l)), "au doigt : « {l} » retiré");
    }
    assert!(doigt.iter().any(|m| m.contains("Top gagnants")) && doigt.iter().any(|m| m == "📂 Importer des idées"));
}

/// 💾 Les tests n'écrivent jamais dans le vrai cache des bougies (`~/.cache/argos`) : un dossier temporaire.
#[test]
fn les_tests_n_ecrivent_jamais_dans_le_vrai_cache() {
    let d = cache_dir().unwrap();
    assert!(d.starts_with(argos_core::dossiers::racine_des_tests()), "{}", d.display());
}

/// ⚡ Étape 2b : pendant le chargement, le direct bouge tout de suite sur les bougies du cache, puis il est réappliqué
/// sur les bougies fraîches (rien n'est perdu).
#[test]
fn le_direct_bouge_tout_de_suite_sur_le_cache() {
    let mut a = app();
    let gen = a.gen.load(Ordering::SeqCst);
    a.chart.loading = Some(vec![]);
    let last = *a.chart.raw.last().unwrap();
    let live = Bar {
        close: last.close + 5.0,
        high: last.high + 5.0,
        ..last
    };
    a.on_user_event(UserEvent::Live(gen, LiveEvent::Bar(live)));
    assert_eq!(
        a.chart.raw.last().unwrap().close,
        live.close,
        "affiché tout de suite, avant la fin du chargement"
    );
    assert!(a.chart.loading.as_ref().is_some_and(|w| w.len() == 1), "et gardé pour après le chargement");
    // les bougies fraîches arrivent (la dernière sans le direct) : le direct est réappliqué par-dessus
    a.on_user_event(UserEvent::Loaded(
        gen,
        Ok(crate::run::Candles {
            bars: bougies(),
            title: "essai".into(),
            banner: None,
            primary: false,
            secours: None,
            a_reparer: vec![],
        }),
    ));
    assert_eq!(a.chart.raw.last().unwrap().close, live.close);
    assert!(a.chart.loading.is_none());
}

/// 🏦 Suivi 1 (Jee, 21 Nov: the Coinbase BTC 1-minute chart stayed frozen, it had no live feed): Coinbase has no
/// candle channel, its trades move the candle in progress, then start the next one; in a unit made of smaller candles
/// (1 h 30 = 3 × 30 min) they fill the downloaded candle (they filled a 1 h 30 one, older than it: nothing moved); the
/// demo account follows the price, as with a live candle (a stop crossed closes the position).
#[test]
fn coinbase_la_chart_bouge_avec_ses_trades() {
    use argos_data::live::Trade;
    dossier_a_soi("coinbase_la_chart_bouge");
    let coinbase = |a: &mut App| {
        a.chart.source = Source::Live {
            exchange: argos_data::Exchange::Coinbase,
            base: "BTC".into(),
        };
    };
    let trades = |a: &mut App, l: Vec<Trade>| {
        let gen = a.gen.load(Ordering::SeqCst);
        a.on_user_event(UserEvent::Live(gen, LiveEvent::Trades(l)));
    };
    let mut a = app();
    coinbase(&mut a);
    let last = *a.chart.raw.last().unwrap();
    let t = |s: i64, price: f64| Trade {
        time_ms: (last.time + s) * 1000,
        price,
        size: 0.5,
    };
    // in the candle in progress: it moves
    trades(&mut a, vec![t(60, last.high + 3.0)]);
    let b = *a.chart.bars.last().unwrap();
    assert_eq!(
        (b.time, b.close, b.high, b.volume),
        (last.time, last.high + 3.0, last.high + 3.0, last.volume + 0.5)
    );
    assert_eq!(a.chart.live, Some(true));
    // the next unit: a new candle
    trades(&mut a, vec![t(1805, 250.0), t(1810, 251.0)]);
    let b = *a.chart.bars.last().unwrap();
    assert_eq!((a.chart.bars.len(), b.time, b.open, b.close), (201, last.time + 1800, 250.0, 251.0));
    // 1 h 30 made of 30-minute candles (the last one is the 3rd of its 1 h 30): the displayed candle moves
    let mut a = app();
    coinbase(&mut a);
    a.chart.interval = Interval(5400);
    a.rebuild();
    let avant = *a.chart.bars.last().unwrap();
    assert_eq!(avant.time, last.time - 3600);
    trades(&mut a, vec![t(60, last.high + 7.0)]);
    let b = *a.chart.bars.last().unwrap();
    assert_eq!((b.time, b.open, b.close, b.high), (avant.time, avant.open, last.high + 7.0, last.high + 7.0));
    assert_eq!(a.chart.raw.last().unwrap().time, last.time, "the downloaded candle filled");
    // the demo account: a buy with its stop 1 % under the entry, a trade under the stop closes it
    let mut a = app();
    coinbase(&mut a);
    a.toggle_simulateur();
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    let p = a.demo.positions[0].clone();
    let stop = p.stop.unwrap();
    trades(&mut a, vec![t(120, stop - 1.0)]);
    assert!(a.demo.positions.is_empty(), "the stop crossed by a trade: {:?}", a.banner);
    assert_eq!(a.demo.history.last().map(|c| c.reason.as_str()), Some("stop"));
}

/// 💼 Suivi (22 Nov): some trading banners wrote raw amounts (« risque 1000.00 $ », « BTC fermé (stop) : -1620.00 $ »)
/// beside amounts written like the app (« coûts d'entrée 630,00 $ »): all are written like the app now.
#[test]
fn bandeaux_du_trading_ecrivent_l_argent_comme_l_app() {
    use argos_render::simulateur::argent;
    dossier_a_soi("bandeaux_du_trading_argent");
    let mut a = app();
    a.toggle_simulateur();
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.contains("(risque 1 000,00 $"), "{b}");
    // the stop crossed: the loss
    let stop = a.demo.positions[0].stop.unwrap();
    a.chart.raw.last_mut().unwrap().close = stop - 1.0;
    a.rebuild();
    assert!(a.demo_exits());
    let net = argent(a.demo.history.last().unwrap().net(), true);
    let b = a.banner.clone().unwrap_or_default();
    assert!(net.starts_with("−1 ") && b.contains(&format!("fermé (stop) : {net}")), "{b}");
    // closed from its label (✕)
    a.sim_place();
    let id = a.demo.positions[0].id;
    a.ordre_clic(argos_render::ordres::Prise::Fermer(id));
    let net = argent(a.demo.history.last().unwrap().net(), true);
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.starts_with(&format!("💼 Position fermée : {net}")), "{b}");
    // all of them closed at once
    a.sim_place();
    a.close_demo_positions();
    let net = argent(a.demo.history.last().unwrap().net(), true);
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.contains(&format!("1 position(s) fermée(s) : {net}")), "{b}");
}

/// 🏷️ KuCoin chosen, KuCoin silent, the supervisor fell back to Binance: the legend keeps the chosen exchange, like
/// the symbol bar, and says where the data come from (it said « Binance » under a « KUCOIN » bar).
#[test]
fn repli_la_legende_et_la_barre_ne_se_contredisent_plus() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Kucoin,
        base: "BTC".into(),
    };
    let gen = a.gen.load(Ordering::SeqCst);
    a.on_user_event(UserEvent::Loaded(
        gen,
        Ok(crate::run::Candles {
            bars: bougies(),
            title: crate::run::titre_charge(Exchange::Kucoin, Exchange::Binance, "BTC", "1h"),
            banner: None,
            primary: false,
            secours: None,
            a_reparer: vec![],
        }),
    ));
    a.refresh_top();
    let barre = Exchange::Kucoin.name();
    assert!(a.top.source.starts_with(barre), "bar: the chosen exchange: {}", a.top.source);
    let legende = a.title.split(" (").next().unwrap_or("");
    assert!(legende.ends_with(&format!("· {barre}")), "legend: the same exchange: {}", a.title);
    assert!(a.title.ends_with("(données Binance)"), "and where the data come from: {}", a.title);
    // no fallback: the title as before
    assert_eq!(
        crate::run::titre_charge(Exchange::Binance, Exchange::Binance, "BTC", "1h"),
        Exchange::Binance.chart_title("BTC", "1h")
    );
}

/// 📡 KuCoin in a web page (Suivi, 22 Nov: its chart stayed frozen): KuCoin refuses its token there, so the chart has
/// no direct of its own; its candles come from a fallback exchange, and the chart takes that exchange's direct, so it
/// moves with the data it shows. A chart that has its direct, or a source that is not an exchange, opens nothing more.
#[test]
fn kucoin_sans_direct_prend_celui_de_son_secours() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Kucoin,
        base: "BTC".into(),
    };
    a.chart.interval = Interval(3600);
    a.chart.live = None; // no direct here (web page)
    let (ex, direct) = a.direct_de_secours(Some(Exchange::Binance)).expect("the fallback's direct");
    assert_eq!(ex, Exchange::Binance);
    assert_eq!(Some(direct), argos_data::live::direct_chart(Exchange::Binance, "BTC", Some(Timeframe::H1)));
    assert!(a.direct_de_secours(None).is_none(), "candles of the chosen exchange: nothing more");
    a.chart.live = Some(false);
    assert!(a.direct_de_secours(Some(Exchange::Binance)).is_none(), "its own direct is already open");
    a.chart.live = None;
    a.chart.source = Source::Oanda { instrument: "XAU_USD".into() };
    assert!(a.direct_de_secours(Some(Exchange::Binance)).is_none(), "not an exchange chart");
}

/// 📖 The Book module says where its book comes from: Hyperliquid's own on Hyperliquid; on another exchange,
/// Hyperliquid's book of the same coin (a band says it); « Carnet indisponible pour <exchange> » when the coin is not
/// on Hyperliquid (it showed Hyperliquid's book under a Binance chart without a word).
#[test]
fn carnet_dit_sa_vraie_source() {
    use argos_render::dock::CarnetSource;
    let mut a = app();
    let quote = |coin: &str| argos_data::watchlist::Quote {
        coin: coin.into(),
        mark: "1".into(),
        prev: 1.0,
        volume: 1.0,
        growth: false,
        oracle: 1.0,
        oi: 0.0,
        funding: 0.0,
    };
    a.feeds.quotes = ["BTC", "ETH"].iter().map(|c| (c.to_string(), quote(c))).collect();
    a.chart.source = Source::Live {
        exchange: Exchange::Hyperliquid,
        base: "BTC".into(),
    };
    assert_eq!(a.carnet_source(), CarnetSource::Hyperliquid);
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    assert_eq!(a.carnet_source(), CarnetSource::MemePiece, "Binance BTC: Hyperliquid's BTC book, said");
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "PEPEUSDT".into(),
    };
    assert_eq!(
        a.carnet_source(),
        CarnetSource::Indisponible("Carnet indisponible pour Binance".into()),
        "a coin Hyperliquid does not list"
    );
    // the band and the message are drawn in the module
    let t = argos_render::text::Text::default();
    let th = argos_render::Theme::default();
    let mut pm = argos_render::px::new(320, 400).unwrap();
    for src in [CarnetSource::MemePiece, a.carnet_source()] {
        argos_render::dock::draw_book(&mut pm, &t, 320.0, 400.0, None, 1, &th, &src);
    }
    assert!(argos_render::dock::CARNET_MEME_PIECE.contains("Hyperliquid (même pièce)"));
}

/// ⚡ Étape 2b : le premier écran demande au plus `FIRST_SCREEN` bougies (une page), le reste vient après.
#[test]
fn premier_ecran_court() {
    use crate::actions::FIRST_SCREEN;
    const { assert!(FIRST_SCREEN <= 500 && FIRST_SCREEN < DEFAULT_CANDLES) };
    assert_eq!(
        cache::missing(&[], 0, Timeframe::H1, DEFAULT_CANDLES).min(FIRST_SCREEN),
        FIRST_SCREEN,
        "sans cache"
    );
}

/// 🕳️ Étape 3 : les bougies retrouvées entrent dans la chart ; un trou que l'exchange n'a pas devient « connu ».
#[test]
fn trous_repares_dans_la_chart() {
    let mut a = app();
    let full = bougies();
    a.chart.raw = full.iter().enumerate().filter(|(k, _)| !(50..53).contains(k)).map(|(_, b)| *b).collect();
    a.chart.a_reparer = vec![full[51].time];
    a.chart.trous_connus = ("essai|1800".into(), vec![]);
    let perdu = argos_data::trous::Trou {
        debut: full[120].time,
        fin: full[121].time,
    };
    a.gaps_repaired(
        "essai|1800".into(),
        argos_data::trous::Reparation {
            bars: full[50..53].to_vec(),
            irreparables: vec![perdu],
        },
    );
    assert_eq!(a.chart.raw.len(), full.len(), "les 3 bougies retrouvées sont dans la chart");
    assert!(a.chart.a_reparer.is_empty(), "la bougie refusée est réparée");
    assert_eq!(
        a.chart.trous_connus.1.iter().map(|c| c.trou).collect::<Vec<_>>(),
        vec![perdu],
        "le trou que l'exchange n'a pas n'est plus redemandé (pendant une semaine)"
    );
    assert!(!a.chart.repair_busy);
    // une réponse pour un autre marché (changé entre-temps) : ignorée
    a.gaps_repaired("autre|60".into(), argos_data::trous::Reparation::default());
    assert_eq!(a.chart.trous_connus.1.len(), 1);
}

/// 📴 Étape 6 : coupure du direct = étiquette « hors ligne », la chart garde ses bougies (cache) ; au retour du
/// direct, l'étiquette disparaît et la coupure est notée comme rattrapée.
#[test]
fn hors_ligne_puis_retour() {
    let mut a = app();
    let gen = a.gen.load(Ordering::SeqCst);
    let avant = a.chart.raw.len();
    a.on_user_event(UserEvent::Live(gen, LiveEvent::Disconnected("réseau coupé".into())));
    assert!(a.chart.hors_ligne && a.chart.coupe);
    assert_eq!(a.chart.raw.len(), avant, "la chart reste utilisable avec ses bougies");
    assert!(a.chart_with(false).hors_ligne, "étiquette affichée");
    a.on_user_event(UserEvent::Live(gen, LiveEvent::Connected));
    assert!(!a.chart.hors_ligne && !a.chart.coupe, "retour : étiquette retirée, rattrapage lancé");
    // un chargement en échec sans direct : hors ligne aussi
    a.chart.live = None;
    a.on_user_event(UserEvent::Loaded(gen, Err("réseau injoignable".into())));
    assert!(a.chart.hors_ligne);
    a.on_user_event(UserEvent::Loaded(
        gen,
        Ok(crate::run::Candles {
            bars: bougies(),
            title: "essai".into(),
            banner: None,
            primary: false,
            secours: None,
            a_reparer: vec![],
        }),
    ));
    assert!(!a.chart.hors_ligne, "les bougies reviennent : plus hors ligne");
}

/// 🧾 Revue R&D, point 1 : le passé venu d'un exchange d'appoint (Binance) s'affiche, mais ne va jamais dans le
/// cache de l'exchange de la chart (ni dans la réparation de ses trous).
#[test]
fn le_secours_ne_pollue_pas_le_cache() {
    let mut a = app();
    let propres = bougies();
    let debut = propres[0].time;
    a.chart.older_key = "essai|1800".into();
    a.chart.older_via = Some(argos_data::Exchange::Binance);
    let appoint: Vec<Bar> = (1..=50)
        .map(|k| Bar {
            time: debut - k * 1800,
            ..propres[0]
        })
        .rev()
        .collect();
    a.older_arrived("essai|1800".into(), Ok(appoint));
    assert_eq!(a.chart.raw.len(), propres.len() + 50, "le passé de l'appoint s'affiche");
    assert_eq!(a.chart.debut_propre, Some(debut));
    let pour_le_cache = a.bougies_propres();
    assert_eq!(pour_le_cache.len(), propres.len(), "seulement les bougies de l'exchange de la chart");
    assert_eq!(pour_le_cache[0].time, debut);
    // sans appoint : tout est à l'exchange de la chart
    let b = app();
    assert_eq!(b.bougies_propres().len(), b.chart.raw.len());
}

/// 🧮 Revue R&D, point 5 : les paires calculées sont marquées « approximatives » (titre et bandeau) et ne vont
/// jamais dans le cache comme une donnée d'exchange.
#[test]
fn paires_calculees_marquees_approximatives() {
    use crate::run::{ratio_title, synth_title};
    for s in ["XAUGBP", "XAUEUR", "XAGEUR"] {
        assert!(synth_title(s, "1h").contains("(calculé, approximatif)"), "{s}");
    }
    assert!(ratio_title(argos_data::Exchange::Binance, "xrp", "btc", "1h").contains("(calculée, approximative)"));
    // le DXY calculé l'était déjà (bandeau et titre) ; une paire calculée n'a jamais de fichier de cache
    let mut a = app();
    a.open_synth("XAUGBP".into());
    assert!(a.chart.cache_path.is_none(), "jamais de cache pour une paire calculée");
}

/// 🔄 Un réglage changé ailleurs est relu et appliqué : la colonne, le split ; un nom inconnu ne fait rien.
#[test]
fn recharger_applique_les_reglages() {
    let mut a = app();
    let mut dock = a.watch.dock.clone();
    dock.on = !dock.on;
    App::write_prefs("dock.json", &dock);
    a.recharger(&["dock.json".into(), "inconnu.json".into()]);
    assert_eq!(a.watch.dock.on, dock.on);
    let _ = std::fs::remove_file(crate::persist::saved::dir().unwrap().join("dock.json"));
}

/// 🗂 Mises en page : ➕ nouvelle (l'ancienne est gardée), copie, renommer, ouvrir depuis la liste.
#[test]
fn mises_en_page_nommees() {
    let mut a = app();
    a.top.prefs.layout_name = "Argos".into();
    a.nouvelle_layout(false);
    assert_eq!(a.top.prefs.layout_name, "Mise en page");
    a.nouvelle_layout(true);
    assert_eq!(a.top.prefs.layout_name, "Mise en page (copie)");
    a.renommer_layout("Or 4h".into());
    let noms: Vec<String> = a.liste_layouts().into_iter().map(|x| x.0).collect();
    assert_eq!(noms, ["Argos", "Mise en page", "Or 4h"]);
    // un nom déjà pris : refusé
    a.renommer_layout("Argos".into());
    assert_eq!(a.top.prefs.layout_name, "Or 4h");
    // ouvrir « Argos » depuis la liste : elle devient l'ouverte, partout (layouts.json)
    a.clic_layout(0);
    assert_eq!(a.top.prefs.layout_name, "Argos");
    let all: crate::actions::layouts::Layouts = App::read_prefs(crate::actions::layouts::FICHIER);
    assert_eq!(all.ouverte, "Argos");
    let _ = std::fs::remove_file(crate::persist::saved::dir().unwrap().join(crate::actions::layouts::FICHIER));
}

/// 🪟 Grille de 9 : la molette et le glisser sur la case 5 ne bougent que la case 5 (ni la principale, ni les autres) ;
/// chaque case a sa propre échelle de prix ; double-clic = vue automatique.
#[test]
fn split_la_souris_agit_sur_la_case_survolee() {
    let mut a = app();
    a.area = (1500, 900);
    a.split.layout = "9g".into();
    a.sync_panes();
    assert_eq!(a.panes.len(), 8);
    (a.chart.viewport.width, a.chart.viewport.height) = (500, 300);
    let main = (a.chart.viewport.bar_spacing, a.chart.viewport.right_offset);
    // case 5 de la grille (2e rangée, 3e colonne) = la 5e case en plus de la principale
    let (x, y) = (1250.0, 450.0);
    let (k, ..) = a.pane_at(x, y).unwrap();
    assert_eq!(k, 4);
    a.pointer.win_mouse = (x + a.rw(), y + a.top_h());
    a.pointer.side_mouse = None;
    a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, 2.0));
    assert!(a.panes[4].vue.is_some(), "la case 5 a zoomé");
    assert!(
        a.panes.iter().enumerate().all(|(j, p)| j == 4 || p.vue.is_none()),
        "les autres cases n'ont pas bougé"
    );
    assert_eq!(
        (a.chart.viewport.bar_spacing, a.chart.viewport.right_offset),
        main,
        "la principale n'a pas bougé"
    );
    // glisser dans la case 5
    let avant = a.panes[4].vue;
    assert!(a.pane_click(x, y));
    a.mouse_moved(PhysicalPosition::new(x + 60.0 + a.rw(), y + a.top_h()));
    a.left_button(ElementState::Released);
    assert_ne!(a.panes[4].vue, avant, "la case 5 a défilé");
    assert!(a.panes.iter().enumerate().all(|(j, p)| j == 4 || p.vue.is_none()));
    assert_eq!((a.chart.viewport.bar_spacing, a.chart.viewport.right_offset), main);
    // ✚ le réticule est sur la case survolée seulement
    a.pane_hover(x, y);
    assert!(a.panes[4].croix.is_some() && a.panes.iter().filter(|p| p.croix.is_some()).count() == 1);
    // double-clic (deux vrais clics au même endroit) : vue automatique
    a.mouse_moved(PhysicalPosition::new(x + a.rw(), y + a.top_h()));
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert!(a.panes[4].vue.is_none());
    // 📏 échelle de prix propre à chaque case, même si la principale est figée
    a.chart.settings.scale.manual = Some((70_000.0, 80_000.0));
    assert_eq!(a.pane_settings().scale.manual, None);
}

/// 🪟 Titre d'une case : le symbole et l'unité de temps se choisissent pour cette case seulement ; 🔗 lie la case au
/// symbole de la principale.
#[test]
fn split_choix_du_symbole_et_de_l_unite_par_case() {
    let mut a = app();
    a.area = (1500, 900);
    a.split.layout = "4g".into();
    a.sync_panes();
    (a.chart.viewport.width, a.chart.viewport.height) = (750, 450);
    // clic sur le nom de la case 2 (en haut à droite) : la watchlist s'ouvre pour elle
    assert!(a.pane_click(750.0 + 20.0, 20.0));
    assert_eq!(a.ui.pane_symbole, Some(0));
    a.open_coin("binance:ETHUSDT".into());
    assert!(a.panes[0].coin.starts_with("binance:"), "{}", a.panes[0].coin);
    assert_eq!(a.ui.pane_symbole, None);
    assert!(a.main_coin().starts_with("file:"), "la principale n'a pas changé");
    // l'unité de temps : le menu des intervalles pour cette case seulement
    let iv = a.chart.interval;
    a.pane_set_interval(0, Interval(300));
    assert_eq!((a.panes[0].interval, a.chart.interval), (Interval(300), iv));
    // 🔗 lier la case 3 : elle suit le symbole de la principale
    let (name, title) = a.pane_title(1);
    let (_, _, _, lien) = App::pane_title_zones(&title, &name, false, 0.0);
    assert!(a.pane_click(lien.cx(), 450.0 + lien.cy()));
    assert_eq!(a.split.liees.get(1), Some(&true));
    assert_eq!(a.panes[1].coin, a.main_coin());
}

/// ⏱ Intervalle personnalisé : « ➕ Ajouter » dans le menu des intervalles, on écrit « 90m » : il est choisi, gardé dans
/// le menu (PERSONNALISÉS) et mis en favori (barre des unités de temps) ; une saisie illisible est refusée.
#[test]
fn intervalle_personnalise_ajoute_et_choisi() {
    let mut a = app();
    let menu = a.interval_menu();
    let perso = menu.last().unwrap();
    assert_eq!(perso.title, "PERSONNALISÉS");
    assert_eq!(perso.items.last().map(|i| i.id), Some(crate::actions::ADD_INTERVAL));
    a.ui.name_box = Some(NameBox::new(NameFor::Interval, "90m", Rect::new(0.0, 0.0, 10.0, 10.0), 1200.0));
    a.name_ok();
    assert_eq!(a.chart.interval, Interval(5400));
    assert!(a.tools.custom.contains(&5400) && a.tools.favorites.contains(&5400));
    assert!(a.interval_menu().last().unwrap().items.iter().any(|i| i.id == 5400 && i.star == Some(true)));
    a.ui.name_box = Some(NameBox::new(NameFor::Interval, "beaucoup", Rect::new(0.0, 0.0, 10.0, 10.0), 1200.0));
    a.name_ok();
    assert_eq!(a.chart.interval, Interval(5400), "illisible : rien ne change");
    let _ = std::fs::remove_file(crate::persist::saved::dir().unwrap().join("toolbar.json"));
}

/// ⏱ Favoris dans le titre d'une case du split : un clic sur un favori de la case 5 ne change que la case 5 ; trop
/// étroite, la case n'a plus de raccourcis (ils restent dans son menu « · 1h »).
#[test]
fn split_favoris_par_case() {
    let mut a = app();
    a.area = (2400, 900);
    a.split.layout = "6g".into();
    a.tools.favorites = vec![60, 900, 3600, 14_400];
    a.sync_panes();
    (a.chart.viewport.width, a.chart.viewport.height) = (800, 450);
    // case 5 de la grille 3 × 2 = 2e rangée, 3e colonne
    let (x0, y0) = (1600.0, 450.0);
    let (k, ..) = a.pane_at(x0 + 10.0, y0 + 10.0).unwrap();
    assert_eq!(k, 4);
    let avant: Vec<Interval> = a.panes.iter().map(|p| p.interval).collect();
    let favs = a.pane_favs(k, 800.0);
    assert_eq!(favs.len(), 4, "la place pour les 4 favoris");
    let (r, secs) = favs.iter().find(|(_, s)| Interval(*s) != avant[4]).copied().unwrap();
    let principale = a.chart.interval;
    assert!(a.pane_click(x0 + r.cx(), y0 + r.cy()));
    assert_eq!(a.panes[4].interval, Interval(secs));
    assert!(
        a.panes.iter().enumerate().all(|(j, p)| j == 4 || p.interval == avant[j]),
        "les autres cases n'ont pas changé"
    );
    assert_eq!(a.chart.interval, principale);
    // case étroite : pas de raccourcis
    assert!(a.pane_favs(k, 260.0).is_empty());
}

/// 🪟 PC, top rows folded: their band lies on the main chart and its legend goes down 16 points under it. A pane's
/// title stays on the row of its click zones: « · 1h » opens the pane's unit menu where it is drawn (Jee, 21 Nov:
/// « je ne peux pas choisir de time frame dans les 2 autres colonnes »).
#[test]
fn titre_des_cases_sous_la_bande_du_haut() {
    dossier_a_soi("titre_des_cases_sous_la_bande_du_haut");
    let mut a = app();
    (a.area, a.top.phone, a.top.sans_bandeaux, a.top.survol_bandeaux) = ((1500, 900), false, true, false);
    a.split.layout = "3c".into();
    a.sync_panes();
    (a.chart.viewport.width, a.chart.viewport.height) = (500, 900);
    assert!(a.bande_haut().is_some());
    argos_render::set_haut_legende(a.decalage_legende());
    assert_eq!(argos_render::haut_legende(), 16.0, "the main chart's legend goes under the band");
    let cells = argos_render::splits::cells(&a.split.layout);
    let main = argos_render::px::new(500, 900).unwrap();
    let out = a.compose_split(&main, &cells).unwrap();
    assert_eq!(argos_render::haut_legende(), 16.0, "given back to the main chart");
    // the title's letters in the 2nd pane (x 500…): on the row of ⇄ and 🔗 (8 to 30), none 16 points lower
    let bg = Theme::current().background;
    let encre = |y0: u32, y1: u32| {
        (y0..y1)
            .flat_map(|y| (512..540).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let c = out.pixel(x, y).unwrap();
                let d = |v: u8, b: u8| (v as i32 - b as i32).abs();
                d(c.red(), bg.0) + d(c.green(), bg.1) + d(c.blue(), bg.2) > 240
            })
            .count()
    };
    let (rangee, dessous) = (encre(12, 27), encre(30, 44));
    assert!(rangee > 10 && dessous == 0, "title on its row: {rangee} points on it, {dessous} below");
    // ⏱ a click on « · 1h » opens the unit menu of that pane
    let (name, title) = a.pane_title(0);
    let (fin_nom, fin_ut, ..) = App::pane_title_zones(&title, &name, false, 0.0);
    a.pointer.last_chart_click = None;
    let (wx, wy) = (a.rw() + 500.0 + (fin_nom + fin_ut) / 2.0, a.top_h() + 20.0);
    clic(&mut a, wx, wy);
    assert!(
        matches!(a.ui.list.as_ref().map(|(k, _)| *k), Some(ListKind::PaneIntervals(0))),
        "{:?}",
        a.ui.list.as_ref().map(|(k, _)| *k)
    );
    argos_render::set_haut_legende(0.0);
}

/// ⭐ Listes partagées : une liste changée sur un autre appareil arrive ici (remplacée), et les listes d'ici sont
/// écrites dans watchlists.json (seulement si elles ont changé). Dossier de test, jamais le vrai ~/.config.
#[test]
fn listes_partagees_en_direct() {
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    let ecrit = platform::read("watchlists.json").unwrap();
    assert!(ecrit.contains("Mes favoris"), "les listes d'ici sont écrites");
    // l'autre appareil : « Mes favoris » avec seulement SOL, et une liste « Or »
    let mut autre = argos_store::MemStore::default();
    let f = autre.create_list("Mes favoris").unwrap();
    autre.add_item(f, "", "hl:SOL").unwrap();
    let o = autre.create_list("Or").unwrap();
    autre.add_item(o, "", "oanda:XAUUSD").unwrap();
    platform::write("watchlists.json", &argos_store::listes::vers_json(&autre)).unwrap();
    a.recharger(&["watchlists.json".into()]);
    let st = a.store.as_ref().unwrap();
    let noms: Vec<String> = st.lists().unwrap().into_iter().map(|l| l.name).collect();
    assert!(noms.contains(&"Mes favoris".to_string()) && noms.contains(&"Or".to_string()), "{noms:?}");
    let fav = st.lists().unwrap().into_iter().find(|l| l.name == "Mes favoris").unwrap().id;
    assert_eq!(st.items(fav).unwrap(), vec![(String::new(), "hl:SOL".to_string())]);
    let _ = std::fs::remove_file(crate::persist::saved::dir().unwrap().join("watchlists.json"));
}

/// ⌂ Accueil : le premier lancement se termine une seule fois (drapeau), puis chaque lettre ouvre sa section.
#[test]
fn accueil_premier_lancement_et_lettres() {
    let mut a = app();
    a.top.prefs.bienvenue_vue = false;
    a.open_accueil();
    assert_eq!(a.ui.accueil.as_ref().and_then(|p| p.bienvenue), Some(0), "premier lancement : ses 3 écrans");
    // pendant le premier lancement, les lettres ne font rien
    a.key_accueil(&Key::Character("j".into()));
    assert!(a.ui.accueil.is_some());
    // Échap : terminé, gardé ; l'Accueil reste ouvert
    a.key_accueil(&Key::Named(NamedKey::Escape));
    assert!(a.top.prefs.bienvenue_vue);
    assert!(a.ui.accueil.as_ref().is_some_and(|p| p.bienvenue.is_none()));
    // la lettre J ouvre le Journal (statistiques), F le PropFirm, E l'Agenda, M la watchlist
    for (k, tool, tab) in [
        ("j", wl::Tool::Journal, Some(1)),
        ("F", wl::Tool::Journal, Some(2)),
        ("e", wl::Tool::Agenda, None),
        ("m", wl::Tool::Watchlist, None),
    ] {
        a.open_accueil();
        assert_eq!(a.ui.accueil.as_ref().and_then(|p| p.bienvenue), None, "une seule fois");
        a.key_accueil(&Key::Character(k.into()));
        assert!(a.ui.accueil.is_none(), "{k} ferme l'Accueil");
        assert_eq!((a.watch.prefs.open, a.watch.prefs.panel), (true, tool), "{k}");
        if let Some(t) = tab {
            assert_eq!(a.watch.panels.journal.tab, t);
        }
    }
    // A : l'Académie ; P : le Portefeuille
    a.open_accueil();
    a.key_accueil(&Key::Character("a".into()));
    assert!(a.ui.ac_page.is_some());
    a.open_accueil();
    assert!(a.ui.ac_page.is_none(), "l'Accueil ferme les pages");
    a.key_accueil(&Key::Character("p".into()));
    assert!(a.ui.pf_page.is_some());
    // une lettre sans carte ne fait rien ; Échap ferme
    a.open_accueil();
    a.key_accueil(&Key::Character("x".into()));
    assert!(a.ui.accueil.is_some());
    a.key_accueil(&Key::Named(NamedKey::Escape));
    assert!(a.ui.accueil.is_none());
    // « Revoir l'accueil » : les 3 écrans de nouveau
    a.revoir_accueil();
    assert_eq!(a.ui.accueil.as_ref().and_then(|p| p.bienvenue), Some(0));
    // les cartes se calculent sur les données de l'app (rien d'inventé sans données)
    let c = argos_render::accueil::cartes(&a.accueil_donnees());
    assert_eq!(c.len(), 10);
}

/// 🔎 Ctrl + K : ouvrir / fermer, filtrer, et chaque genre de ligne ouvre la bonne chose.
#[test]
fn recherche_universelle() {
    use argos_render::recherche::{Cible, Reglage};
    let mut a = app();
    a.open_palette();
    assert!(a.ui.palette.is_some());
    a.open_palette();
    assert!(a.ui.palette.is_none(), "2e Ctrl + K : fermée");
    a.open_palette();
    let p = a.ui.palette.as_mut().unwrap();
    p.tape("raccourcis");
    assert_eq!(p.choisie(), Some(Cible::Raccourcis));
    let c = p.choisie().unwrap();
    a.palette_cible(c);
    assert!(a.ui.palette.is_none() && a.ui.raccourcis.is_some(), "Entrée ouvre et ferme la recherche");
    // un indicateur se pose sur la chart
    let i = argos_render::indicators::Ind::ALL[0];
    a.palette_cible(Cible::Indicateur(i));
    assert!(a.ind.on.contains(&i));
    // une section, un réglage
    a.palette_cible(Cible::Section(argos_render::accueil::Section::Calendrier));
    assert_eq!(a.watch.prefs.panel, wl::Tool::Agenda);
    let avant = a.top.prefs.accueil;
    a.palette_cible(Cible::Reglage(Reglage::AccueilAuLancement));
    assert_eq!(a.top.prefs.accueil, !avant);
    a.palette_cible(Cible::Accueil);
    assert!(a.ui.accueil.is_some());
    // une leçon ouvre l'Académie sur ce guide
    let l = &argos_render::academy::lecons()[0];
    a.palette_cible(Cible::Lecon(l.id.clone()));
    assert!(a.ui.accueil.is_none());
    assert!(matches!(a.ui.ac_page.as_ref().map(|p| p.view), Some(argos_render::academy::View::Lecon(0))));
}

/// 🔎 Ctrl + K dans le navigateur : les lettres du champ caché de la page (clavier du cell, accents composés)
/// filtrent la liste, Effacer / ↓ / Entrée / Échap marchent, et Ctrl est suivi dès sa touche.
#[test]
fn recherche_recoit_les_lettres() {
    use winit::keyboard::NamedKey;
    let mut a = app();
    a.open_palette();
    let tout = a.ui.palette.as_ref().unwrap().res.len();
    a.paste_text("raccou".into());
    let p = a.ui.palette.as_ref().unwrap();
    assert_eq!(p.q, "raccou", "le champ reçoit les lettres");
    assert!(p.res.len() < tout, "la liste est filtrée");
    a.soft_key("Backspace");
    assert_eq!(a.ui.palette.as_ref().unwrap().q, "racco");
    a.paste_text("urcis".into());
    a.soft_key("ArrowDown");
    a.soft_key("ArrowUp");
    a.soft_key("Enter");
    assert!(a.ui.palette.is_none() && a.ui.raccourcis.is_some(), "Entrée ouvre la ligne choisie");
    a.open_palette();
    a.soft_key("Escape");
    assert!(a.ui.palette.is_none(), "Échap ferme");
    assert_eq!(App::mod_of(&Key::Named(NamedKey::Control)), Some(ModifiersState::CONTROL));
    assert_eq!(App::mod_of(&Key::Named(NamedKey::Alt)), Some(ModifiersState::ALT));
    assert_eq!(App::mod_of(&lettre("k")), None);
}

/// ‹ › Les deux barres se replient par le même geste (languette), et ⌂ de la barre du haut ouvre l'Accueil.
#[test]
fn barres_repliables_et_bouton_accueil() {
    let mut a = app();
    a.area = (1000, 700);
    let tabs = a.fold_tabs();
    // 🖥 colonne d'icônes PC : pas de languette › (son › en bas replie la barre) ; il reste ‹ à gauche
    assert_eq!(tabs.len(), 1);
    let l = tabs[0].0;
    assert!(a.rw() > 0.0);
    assert!(a.fold_click(l.cx(), l.cy()));
    assert!(a.rail.folded && a.rw() == crate::app::BANDE_W, "barre de gauche repliée");
    assert!(a.fold_tabs().is_empty(), "🖥 à gauche, la bande remplace la languette");
    let w0 = a.watch.width();
    assert!(w0 > 0);
    // › en bas de la colonne : la barre de droite se replie, puis la languette › la rouvre
    (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
    let x = a.win_w() - 28.0;
    let y = a.top_h() + a.watch.col_h - 10.0 - 83.0 + 18.0;
    clic(&mut a, x, y);
    assert!(a.watch.prefs.folded && a.watch.width() == 0, "barre de droite repliée");
    let r = a.fold_tabs().into_iter().find(|(_, left)| !left).expect("languette › pour la rouvrir").0;
    assert!(a.fold_click(r.cx(), r.cy()));
    assert!(!a.watch.prefs.folded && a.watch.width() > 0, "rouverte");
    a.watch.prefs.folded = true;
    // un panneau qui s'ouvre (par une section) se montre quand même
    a.accueil_section(argos_render::accueil::Section::Journal);
    assert!(a.watch.width() > 0);
    // rien à replier hors des languettes
    assert!(!a.fold_click(500.0, 5.0));
    // ⌂ en tête de la barre du haut
    let t = argos_render::text::Text::default();
    assert_eq!(a.top.click(22.0, 20.0, 1280.0, &t, &|_: &str| false), argos_render::topbar::TopClick::Home);
}

/// ⏯ Suivi 9 (Jee, 21 Nov): during a replay the top bar's « Prix » showed the live price. It shows the price of the
/// last bar played, and the 24 h change of the bars played (volume, open interest, funding are today's: not shown);
/// the live price again after the replay.
#[test]
fn replay_la_case_prix_suit_le_replay() {
    use argos_render::viewport::{data_decimals, format_price};
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    let live = argos_core::feed::Market {
        base: "BTC".into(),
        price: 99_999.0,
        prev: 90_000.0,
    };
    a.feeds.ex_prices.entry(Exchange::Binance).or_default().insert("BTC".into(), live);
    let prix = |a: &App| a.top.stats.iter().find(|s| s.label == "Prix").map(|s| s.value.clone());
    let dec = data_decimals(&a.chart.bars);
    a.refresh_top();
    assert_eq!(prix(&a), Some(format_price(99_999.0, dec)), "live: the live price");
    a.toggle_simulateur();
    a.sim_replay_start();
    a.refresh_top();
    let b = &a.chart.bars;
    let (l, avant) = (b[b.len() - 1].close, b[b.len() - 49].close);
    assert_eq!(prix(&a), Some(format_price(l, dec)), "replay: the last bar played");
    let var = a.top.stats.iter().find(|s| s.label == "Variation 24 h").expect("the replay's 24 h change");
    assert!(var.value.starts_with(&format_price(l - avant, dec)), "48 bars of 30 min back: {}", var.value);
    assert_eq!(a.top.stats.len(), 2, "{:?}", a.top.stats);
    a.replay_step(1);
    a.refresh_top();
    assert_eq!(prix(&a), Some(format_price(l + 1.0, dec)), "it follows the replay");
    a.replay_act(argos_render::replay::ReplayClick::Exit);
    a.refresh_top();
    assert_eq!(prix(&a), Some(format_price(99_999.0, dec)), "back to live");
}

/// 🧪 Suivi 3 (Jee, 21 Nov): on PC, with the Simulator open, the orders' labels (entry, SL, TP) and the price scale lay
/// under its panel. The chart now stops at the panel's left edge (mock-up): price scale in sight, the entry's ✕ closes
/// the position with a click; the icon column stays at the window's edge, and the chart takes its width back once the
/// panel is closed.
#[test]
fn simulateur_la_chart_s_arrete_a_son_bord() {
    use argos_render::ordres::Prise;
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    let (w, h) = (1440, 900);
    let _ = image_ecran(&mut a, w, h);
    let large = a.chart.viewport.width;
    a.toggle_simulateur();
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    let _ = image_ecran(&mut a, w, h);
    let (s, vp) = (a.sim_rect(), a.chart.viewport);
    assert!(s.x < 1440.0 - 400.0, "{s:?}");
    assert_eq!(
        a.rw() + vp.width as f64,
        s.x,
        "the chart, its price scale included, ends at the panel's left edge"
    );
    assert_eq!(a.win_w(), w as f64, "the icon column at the window's right edge");
    // the entry's ✕, left of the panel: a click there closes the position
    let p = a.demo.positions[0].clone();
    let y = a.scale_now().unwrap().y_of(p.entry);
    let x = (0..vp.plot_w() as i32)
        .map(f64::from)
        .find(|&x| {
            a.pointer.mouse = Some((x, y));
            a.ordre_prise() == Some(Prise::Fermer(p.id))
        })
        .expect("the entry's ✕");
    let (wx, wy) = (a.rw() + x, a.top_h() + y);
    assert!(wx < s.x);
    clic(&mut a, wx, wy);
    assert!(a.demo.positions.is_empty(), "closed from its label: {:?}", a.banner);
    // the panel closed: the chart takes its width back
    a.toggle_simulateur();
    let _ = image_ecran(&mut a, w, h);
    assert_eq!(a.chart.viewport.width, large);
}

/// 🖥 Seen by the Interface thread (22 Nov): on PC, with the Trading volet open, the Simulator's panel opened over it,
/// shifted (420 px stuck to the window's right edge): the « Trading » header and a strip of its cut rows stayed in sight
/// on its left. The Simulator is now the Trading volet's 🧪 tab (module 31 mock-up): in the ticket's place, under the
/// volet's header and at its width; 🎫 Ticket brings the ticket back, the volet's ✕ closes both, the column's 🧪 opens
/// the volet on that tab (and closes it again), another volet takes its place.
#[test]
fn pc_le_simulateur_est_l_onglet_du_volet_trading() {
    dossier_a_soi("pc_le_simulateur_est_l_onglet_du_volet_trading");
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    let mut a = pc_avec_volet(wl::Tool::Trading);
    let v = a.volet_rect().expect("the Trading volet floats");
    let haut = a.watch.panels.entete_fixe();
    assert!(haut > 0.0, "the volet shows the mock-up's sheet");
    // 🧪 its tab (the header's places: 🎫 at 13, 🧪 at 93.5, 80 down)
    clic(&mut a, v.x + 93.5 + 50.0, v.y + 93.0);
    a.placer(taille);
    assert!(a.ui.sim.is_some(), "the 🧪 tab opens the Simulator");
    let s = a.sim_rect();
    assert_eq!(
        (s.x, s.w, s.y),
        (v.x, v.w, v.y + haut),
        "in the ticket's place, under the volet's header, at its width"
    );
    assert!(s.y + s.h <= v.y + v.h, "inside the volet: {s:?} {v:?}");
    assert!(a.watch.panels.trading.sim_onglet, "its tab lit, the ticket not drawn under it");
    assert_eq!(a.dock.sous_sim, 0.0, "nothing more taken from the chart");
    assert_eq!(a.rw() + a.chart.viewport.width as f64, v.x, "the chart stops at the volet's edge");
    assert!(a.sim_vue(|v| v.en_volet), "drawn without a header of its own");
    // a click on the Simulator's content is the Simulator's (where its own ✕ was: nothing closes; its first chip,
    // « ↺ Remise à zéro », asks to confirm)
    clic(&mut a, s.x + s.w - 30.0, s.y + 29.0);
    assert!(a.ui.sim.is_some(), "no ✕ of its own in the volet");
    let premiere_puce = (0..60).any(|k| {
        clic(&mut a, s.x + 40.0, s.y + 4.0 * f64::from(k));
        a.ui.sim.as_ref().is_some_and(|s| s.confirme)
    });
    assert!(premiere_puce, "the Simulator's chips answer in the volet");
    // 🎫 Ticket: the ticket comes back, the volet stays
    clic(&mut a, v.x + 13.0 + 30.0, v.y + 93.0);
    a.placer(taille);
    assert!(a.ui.sim.is_none() && !a.watch.panels.trading.sim_onglet, "🎫 Ticket closes the Simulator");
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading, "the volet stays on its ticket");
    // the volet's ✕ closes both
    clic(&mut a, v.x + 93.5 + 50.0, v.y + 93.0);
    a.placer(taille);
    let d = v.w - 390.0;
    clic(&mut a, v.x + 345.0 + d + 15.0, v.y + 53.0);
    a.placer(taille);
    assert!(!a.watch.prefs.open, "the volet's ✕ closes it");
    assert!(a.ui.sim.is_none(), "the Simulator leaves with its volet");
    // the column's 🧪 (or the Home page, the search): the volet opens on its tab, then closes again
    a.toggle_simulateur();
    a.placer(taille);
    assert!(
        a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading && a.sim_en_volet(),
        "the volet opens on the 🧪 tab"
    );
    assert_eq!(a.sim_rect(), a.sim_place_volet().unwrap());
    a.toggle_simulateur();
    a.placer(taille);
    assert!(a.ui.sim.is_none() && !a.watch.prefs.open, "the 🧪 again closes both");
    // another volet takes its place
    a.toggle_simulateur();
    a.placer(taille);
    a.watch.toggle_tool(wl::Tool::Whales);
    a.placer(taille);
    assert!(a.ui.sim.is_none(), "another volet open: the Simulator leaves");
    // 📱 the phone keeps it full screen
    let mut c = cell_390();
    c.toggle_simulateur();
    let s = c.sim_rect();
    assert!(!c.sim_en_volet() && s.x == 0.0 && s.w == c.win_w(), "phone: full width {s:?}");
}

/// 🖥 The Simulator opened elsewhere than by its 🧪 (the DÉMO banner, the Zen mode left) opens the Trading volet on its
/// tab at the next image (it used to be closed at once: no Trading volet open); in the Zen mode it floats.
#[test]
fn pc_le_simulateur_ouvert_ailleurs_ouvre_son_volet() {
    dossier_a_soi("simulateur_ouvert_ailleurs");
    let mut a = pc_avec_volet(wl::Tool::Whales);
    a.watch.prefs.open = false;
    let _ = image_ecran(&mut a, 1440, 900);
    // 🟠 the DÉMO banner's ↺: the Simulator on its confirmation, in the Trading volet
    let r = a.bandeau_rect();
    assert!(a.bandeau_click(r.x + r.w - 10.0, r.cy()));
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(a.ui.sim.as_ref().is_some_and(|s| s.confirme), "kept, on its confirmation");
    assert!(a.sim_en_volet() && a.watch.panels.trading.sim_onglet && a.watch.prefs.panel == wl::Tool::Trading);
    // 🧘 opened in the Zen mode it floats; the Zen mode left, it goes into the volet
    a.toggle_simulateur();
    assert!(a.ui.sim.is_none() && !a.watch.prefs.open, "its 🧪 closes both");
    a.toggle_zen();
    let _ = image_ecran(&mut a, 1440, 900);
    a.toggle_simulateur();
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(a.ui.sim.is_some() && !a.sim_en_volet(), "Zen: it floats");
    a.toggle_zen();
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(a.sim_en_volet(), "the Zen mode left: in its volet, {:?}", a.sim_rect());
}

/// 🤖 Suivi 5 (Jee, 21 Nov): the Argos AI round lay over the Simulator panel's rows while they scrolled (🖥
/// « Personnalisé », « Coûts au SL »; 📱 Statistiques). It keeps its place (🖥 the window's bottom right, as in the
/// mock-up; 📱 the pill's end), over a band at the panel's bottom where nothing of the content goes; a click on it opens
/// Argos AI, not the panel under it.
#[test]
fn simulateur_rien_sous_le_rond_argos_ai() {
    for phone in [false, true] {
        let mut a = if phone { cell_pilule() } else { app() };
        if phone {
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        }
        a.top.prefs.bienvenue_vue = true;
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        let _ = image_ecran(&mut a, w, h);
        let avant = a.ai_bouton_rect();
        a.toggle_simulateur();
        let _ = image_ecran(&mut a, w, h);
        let (s, ai) = (a.sim_rect(), a.ai_bouton_rect());
        if phone {
            assert_eq!(ai, avant, "phone: the round keeps its place");
            assert!(s.contains(ai.cx(), ai.cy()), "phone: over the panel: {ai:?} {s:?}");
            let bande = a.sim_marge_bas();
            assert!(s.y + s.h - bande <= ai.y, "phone: the panel's band ({bande}) under the whole round");
        } else {
            // 🖥 the Simulator in the Trading volet: the round stays at the chart's bottom right, beside it
            assert!(ai.w > 0.0 && ai.x + ai.w <= s.x, "PC: the round beside the panel: {ai:?} {s:?}");
            assert_eq!(a.sim_marge_bas(), 0.0, "PC: no band, nothing of the panel under the round");
        }
        clic(&mut a, ai.cx(), ai.cy());
        assert!(a.ui.ai_open && a.ui.sim.is_some(), "phone {phone}: Argos AI opens, the panel stays");
    }
}

/// ⏪ Suivi 7 (Jee, 21 Nov): on the phone in 3 columns, the replay bar was cut off in the left column (no ✕ nor ▶);
/// with one chart, its ✕ was cut off too. The bar is now whole on the screen (narrow: the date under the buttons; in 3
/// columns, over the panes), and ▶ and ✕ answer where they are drawn.
#[test]
fn replay_barre_entiere_sur_le_telephone() {
    use argos_render::replay::ReplayClick;
    for layout in ["1", "3c"] {
        dossier_a_soi(&format!("replay_barre_telephone_{layout}"));
        let mut a = cell_pilule();
        a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        a.split.layout = layout.into();
        a.sync_panes();
        a.replay_start();
        let _ = image_ecran(&mut a, 402, 874);
        let (zone_w, ph) = a.replay_zone();
        let b = argos_render::replay::rect(zone_w, ph);
        let (x0, y0) = (a.rw(), a.top_h());
        assert!(x0 + b.x >= 4.0 && x0 + b.x + b.w <= 398.0, "{layout}: {b:?}");
        // over the other panes: above their navigation pills (− + ‹ › ↺ ↶ ↷), never on them
        let (aw, ah) = (a.area.0 as f64, a.area.1 as f64);
        for (k, c) in argos_render::splits::cells(&a.split.layout).iter().skip(1).enumerate().take(a.panes.len()) {
            let (vp, _) = a.pane_view(k, (c[2] * aw).round(), (c[3] * ah).round(), None);
            let p = argos_render::navbar::rect(&vp, vp.plot_h(), false);
            let (px, py) = (c[0] * aw + p.x, c[1] * ah + p.y);
            assert!(
                b.y + b.h <= py || b.x + b.w <= px || b.x >= px + p.w,
                "{layout}: pane {k}'s pill {p:?} under {b:?}"
            );
        }
        let y = b.y + 23.0;
        let x_de = |c: ReplayClick| {
            (0..b.w as i32)
                .map(|i| b.x + f64::from(i))
                .find(|&x| argos_render::replay::click(zone_w, ph, x, y) == Some(c))
                .unwrap_or_else(|| panic!("{layout}: {c:?}"))
        };
        clic(&mut a, x0 + x_de(ReplayClick::Play), y0 + y);
        assert!(a.replay.as_ref().is_some_and(|r| r.play), "{layout}: ▶ plays");
        clic(&mut a, x0 + x_de(ReplayClick::Exit), y0 + y);
        assert!(a.replay.is_none(), "{layout}: ✕ goes back to live");
    }
}

/// 🧪 Un seul moteur démo : le Simulateur et le volet Trading placent dans le même compte (`a.demo`), avec SL et TP
/// obligatoires et le même Gardien ; en replay, les ordres jouent sur chaque bougie révélée (TP en cents exacts),
/// les prix du direct n'y touchent pas, et la fin du replay ferme ce qui reste ; le bandeau ↺ ouvre la confirmation.
#[test]
fn simulateur_un_seul_moteur_demo() {
    let mut a = app();
    a.toggle_simulateur();
    // ⏯ replay : la chart s'arrête 100 bougies avant la fin
    a.sim_replay_start();
    assert!(a.ui.sim_replay && a.replay.is_some());
    let t0 = a.replay.as_ref().unwrap().t;
    let last = a.chart.bars.last().unwrap().close;
    // achat au marché (Simulateur) : SL 1 % sous l'entrée (les bougies du test font 4 points), TP à 2 R, 1 % du compte
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    assert_eq!((p.entry, p.stop, p.target), (last, Some(last * 0.99), Some(last * 1.02)));
    assert_eq!(a.watch.panels.journal.trades.last().map(|t| t.entry), Some(Some(last)), "inscrit au journal");
    // le direct ne touche pas le compte pendant le replay
    assert!(!a.demo_exits());
    // les bougies montent de 1 par bougie : le TP (+2 %, ~4 points) tombe à la 4e
    for _ in 0..5 {
        a.replay_step(1);
    }
    assert_eq!(a.ui.sim_last_t, t0 + 5 * 1800, "chaque bougie révélée a joué");
    assert!(a.demo.positions.is_empty(), "TP touché");
    let c = a.demo.history.last().unwrap();
    assert_eq!((c.reason.as_str(), c.r()), ("objectif", 2.0));
    assert_eq!(
        a.demo.balance,
        a.demo.start + argos_core::Money::from_cents(200_000),
        "+1 000 $ × 2 R, au centime"
    );
    assert!(a.sim_examen().is_some());
    // 🛡️ même Gardien : allumé, 1 % de 102 000 $ dépasse les 1 000 $ du plan
    a.watch.panels.trading.guardian = true;
    a.sim_place();
    assert!(a.demo.positions.is_empty() && a.banner.as_deref().unwrap_or("").contains("Gardien"));
    a.ui.sim.as_mut().unwrap().risque = 1; // 0,5 %
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1);
    // fin du replay : la position ferme au dernier prix du replay, le direct reprend
    a.sim_replay_end();
    a.replay = None;
    assert!(!a.ui.sim_replay && a.demo.positions.is_empty());
    assert_eq!(a.demo.history.last().unwrap().reason, "fin du replay");
    // 💼 volet Trading : même chemin (risque 100 $, stop 2 → TP à 2 R), le même compte
    a.rebuild();
    a.watch.panels.trading.guardian = false;
    (a.watch.panels.trading.risk, a.watch.panels.trading.sl_pips) = (100.0, 2.0);
    a.place_demo_order();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = &a.demo.positions[0];
    assert_eq!((p.qty, p.target.map(|t| t - p.entry)), (50.0, Some(4.0)));
    // 🟠 bandeau : ↺ ouvre le volet sur la confirmation
    a.ui.sim = None;
    let r = a.bandeau_rect();
    assert!(a.bandeau_click(r.x + r.w - 10.0, r.cy()));
    assert!(a.ui.sim.as_ref().is_some_and(|s| s.confirme));
}

/// ⚡ Lot 4 : un perpétuel en replay (Binance BTC) : le levier choisi part avec l'ordre, la marge isolée est tenue, la
/// ligne de liquidation est sur la chart, le funding du replay est reçu (taux négatif : l'acheteur reçoit) à chaque
/// échéance de 8 h ; au comptant (fichier), rien de tout ça.
#[test]
fn simulateur_perpetuel_en_replay() {
    use argos_render::goto::{Outcome, Target};
    dossier_a_soi("simulateur_perpetuel_en_replay");
    let mut a = app();
    assert!(a.perp_marche().is_none(), "un fichier : au comptant");
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    a.rebuild();
    a.toggle_simulateur();
    assert_eq!(a.perp_marche().map(|b| b.levier_max), Some(100.0));
    a.sim_replay_date();
    let t = a.chart.bars[150].time;
    a.goto_outcome(Outcome::Go(Target::At(t + 60)));
    assert!(a.ui.sim_replay, "{:?}", a.banner);
    {
        let s = a.ui.sim.as_mut().unwrap();
        s.levier = 20.0;
        s.funding_rp = 4;
        s.sl = 3;
    }
    assert!(a.sim_vue(|v| v.perp.is_some_and(|p| p.taux_8h == -0.0001 && !p.reel)));
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    // sans objectif : la position reste ouverte pendant que le prix monte
    a.demo.positions[0].target = None;
    let p = a.demo.positions[0].clone();
    assert_eq!(p.perp.map(|x| x.levier), Some(20.0));
    assert_eq!(a.demo.marge_utilisee(), p.marge());
    assert!(p.marge() > argos_core::Money::ZERO);
    assert_eq!(a.vue_ordres().ordres[0].liq.map(|l| l.1), Some(20.0), "la ligne de liquidation");
    let solde = a.demo.balance;
    a.replay_step(17);
    let q = &a.demo.positions[0];
    assert!(
        q.funding < argos_core::Money::ZERO,
        "8 h 30 de replay : au moins une échéance, reçue ({:?})",
        q.funding
    );
    assert_eq!(a.demo.balance, solde - q.funding);
}

/// ⏪ Replay multi-unités (lot 5 du Simulateur) : les autres cases de la mise en page avancent à l'heure du replay (la 4h
/// montre sa bougie en cours, faite des bougies de 30m déjà jouées, jamais la suite) ; changer d'unité pendant le
/// replay garde l'heure (une unité plus grande finit d'abord sa bougie en cours, et l'ordre ouvert la joue une seule
/// fois) ; une case d'un autre marché demande ses bougies autour de l'heure du replay ; au retour au direct, tout revient.
#[test]
fn replay_multi_unites() {
    dossier_a_soi("replay_multi_unites");
    let mut a = app();
    a.area = (1500, 900);
    a.split.layout = "3c".into();
    a.sync_panes();
    assert_eq!(a.panes.iter().map(|p| p.interval.secs()).collect::<Vec<_>>(), vec![3600, 14_400]);
    let h4_toutes = a.panes[1].bars.clone();
    a.toggle_simulateur();
    a.sim_replay_start();
    let t = a.replay.as_ref().unwrap().t;
    assert_eq!(t, START + 50 * 3600);
    let heure = a.replay_heure().unwrap();
    assert_eq!(heure, t + 1800);
    // 🪟 the 4h pane: its finished bars, then its bar in progress made of the 30m bars already shown
    let h4 = a.pane_bars(1).into_owned();
    let c = *h4.last().unwrap();
    assert_eq!(c.time, START + 48 * 3600);
    assert!(h4.iter().all(|b| b.time < heure));
    let deja: Vec<Bar> = a.chart.bars.iter().filter(|b| b.time >= c.time).copied().collect();
    assert_eq!(deja.len(), 5);
    assert_eq!((c.open, c.close), (deja[0].open, deja[4].close));
    assert_eq!(c.high, deja[4].high);
    let entiere = h4_toutes.iter().find(|b| b.time == c.time).unwrap();
    assert!(c.high < entiere.high, "never the rest of the 4h bar");
    assert_eq!(h4[..h4.len() - 1].to_vec(), h4_toutes[..h4.len() - 1].to_vec());
    // the 1h pane: its bar in progress too (one 30m bar)
    let h1 = a.pane_bars(0);
    assert_eq!(h1.last().map(|b| (b.time, b.close)), Some((t, deja[4].close)));
    drop(h1);
    // 🧪 a position stays open through the changes of unit (no target, prices rise)
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    a.demo.positions[0].target = None;
    a.replay_step(1);
    assert_eq!(a.pane_bars(1).last().map(|b| b.close), a.chart.bars.last().map(|b| b.close), "the 4h bar grows");
    // ⏪ 30m → 4h at 03:00 of the 4h bar: the 30m bars up to its end play first, then the chart stands on it
    let joue = a.ui.sim_last_t;
    a.chart.settings.scale.manual = Some((1.0, 2.0));
    a.set_interval(Interval(14_400));
    let (lo, hi) = a.chart.settings.scale.manual.unwrap();
    let bas = a.chart.bars.iter().map(|b| b.low).fold(f64::MAX, f64::min);
    assert!(
        lo > 2.0 && lo <= bas && hi >= a.chart.bars.last().unwrap().high,
        "the price scale fits the 4h bars, not the old bounds ({lo}, {hi})"
    );
    assert!(a.ui.replay_ut.is_none());
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("⏩")), "{:?}", a.banner);
    assert_eq!(a.replay.as_ref().unwrap().t, START + 48 * 3600);
    assert_eq!(a.chart.bars.last().map(|b| b.time), Some(START + 48 * 3600));
    assert_eq!(a.replay_heure(), Some(START + 52 * 3600));
    assert_eq!(a.ui.sim_last_t, START + 51 * 3600 + 1800, "played up to the end of the 4h bar");
    assert!(a.ui.sim_last_t > joue);
    assert_eq!(a.demo.positions.len(), 1);
    // ⏪ 4h → 30m: the same hour, nothing played twice
    a.set_interval(Interval(1800));
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("⏪")), "{:?}", a.banner);
    assert_eq!(a.chart.bars.last().map(|b| b.time), Some(START + 51 * 3600 + 1800));
    assert_eq!(a.replay_heure(), Some(START + 52 * 3600));
    assert_eq!(a.ui.sim_last_t, START + 51 * 3600 + 1800);
    a.replay_step(1);
    assert_eq!(a.ui.sim_last_t, START + 52 * 3600, "the next 30m bar");
    // 🪟 a pane of another market whose bars come after the replay's hour asks for the bars around that hour
    a.panes[0].coin = "binance:ETH".into();
    a.panes[0].asked = a.panes[0].key();
    a.panes[0].bars = bougies()
        .into_iter()
        .map(|b| Bar {
            time: b.time + 900 * 3600,
            ..b
        })
        .collect();
    a.sync_panes();
    assert!(a.panes[0].zone.is_some_and(|z| z > START + 52 * 3600), "{:?}", a.panes[0].zone);
    assert!(a.pane_bars(0).is_empty());
    // 📡 back to live: every bar again, and the latest bars for the other market
    a.replay_act(argos_render::replay::ReplayClick::Exit);
    assert!(a.replay_heure().is_none());
    assert_eq!(a.pane_bars(1).len(), a.panes[1].bars.len());
    a.sync_panes();
    assert_eq!(a.panes[0].zone, None);
}

/// 📅 💾 Séance de replay (lot 1 du Simulateur) : le replay part d'une date choisie (fenêtre « Aller à »), à ×10 au
/// plus ; « Sauver et quitter » garde le marché, la bougie, la vitesse et met les positions de côté sans les fermer ;
/// « Reprendre » les remet là où on était ; « Oublier » les ferme au dernier prix de la séance.
#[test]
fn simulateur_seance_de_replay() {
    use crate::actions::simulateur::SEANCE;
    use argos_core::simu::Seance;
    use argos_render::goto::{Outcome, Target};
    dossier_a_soi("simulateur_seance_de_replay");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    a.rebuild();
    a.toggle_simulateur();
    // 📅 « Choisir la date » : « Aller à » s'ouvre, la date choisie lance le replay sur sa bougie
    a.sim_replay_date();
    assert!(a.ui.goto.is_some() && a.ui.goto_replay);
    let t = a.chart.bars[150].time;
    a.goto_outcome(Outcome::Go(Target::At(t + 60)));
    assert!(a.ui.sim_replay && !a.ui.goto_replay, "{:?}", a.banner);
    assert_eq!(a.replay.as_ref().map(|r| r.t), Some(t), "la bougie de la date (pas de chargement : déjà là)");
    // ×10 : 1 → 2 → 5 → 10
    for _ in 0..3 {
        let r = a.replay.as_mut().unwrap();
        r.speed = argos_core::simu::Replay::next_speed(r.speed);
    }
    assert_eq!(a.replay.as_ref().unwrap().speed, 10);
    // une position, deux bougies, puis 💾 sauver et quitter
    a.ui.sim.as_mut().unwrap().sl = 3;
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    a.replay_step(1);
    a.replay_step(1);
    assert_eq!(a.demo.positions.len(), 1, "{:?} {:?}", a.demo.history.last(), a.banner);
    let (solde, fermes) = (a.demo.balance, a.demo.history.len());
    a.sim_seance_sauver();
    assert!(a.replay.is_none() && !a.ui.sim_replay, "retour au direct");
    assert!(a.demo.positions.is_empty(), "mise de côté : le direct n'y touche pas");
    assert_eq!((a.demo.balance, a.demo.history.len()), (solde, fermes), "rien fermé");
    let s = a.ui.seance.clone().expect("séance gardée");
    assert_eq!((s.base.as_str(), s.interval, s.t, s.speed, s.ouverts()), ("BTC", 1800, t + 2 * 1800, 10, 1));
    // gardée dans le fichier (les quantités f64 relues à 1e-12 près : le lot 2 les passe en décimales exactes)
    let lu = App::read_prefs::<Option<Seance>>(SEANCE).expect("gardée dans le fichier");
    assert_eq!((lu.t, lu.speed, lu.ouverts(), lu.positions[0].opened_at), (s.t, 10, 1, p.opened_at));
    // un autre replay attendrait : la séance a une position
    a.sim_replay_start();
    assert!(a.replay.is_none() && a.banner.as_deref().unwrap_or("").contains("séance gardée"));
    // ▶ reprendre : même bougie, même vitesse, la position revient (même heure d'ouverture, pour le journal)
    a.sim_seance_reprendre();
    assert_eq!(a.replay.as_ref().map(|r| (r.t, r.speed)), Some((t + 2 * 1800, 10)), "{:?}", a.banner);
    assert!(a.ui.sim_replay && a.ui.seance.is_none());
    assert_eq!(App::read_prefs::<Option<Seance>>(SEANCE), None);
    assert_eq!(a.demo.positions.len(), 1);
    assert_eq!((a.demo.positions[0].opened_at, a.demo.positions[0].entry), (p.opened_at, p.entry));
    // 🗑 oublier une séance : la position ferme au dernier prix de la séance, inscrite au journal
    let prix = a.sim_last().unwrap();
    a.sim_seance_sauver();
    a.sim_seance_oublier();
    assert!(a.ui.seance.is_none() && a.demo.positions.is_empty());
    let c = a.demo.history.last().unwrap();
    assert_eq!((c.reason.as_str(), c.exit), ("séance oubliée", prix));
    let j = a
        .watch
        .panels
        .journal
        .trades
        .iter()
        .find(|j| j.id == format!("demo-BTC-{}", p.opened_at))
        .unwrap();
    assert_eq!(j.exit, Some(prix));
    // 📜 une date avant l'historique : le replay part de la première bougie (ici, pas de réseau pour plus ancien)
    let premier = a.chart.bars[0].time;
    a.sim_replay_depuis(premier - 30 * 86_400, None);
    assert_eq!(a.replay.as_ref().map(|r| r.t), Some(premier));
    assert!(a.banner.as_deref().unwrap_or("").contains("commence"), "{:?}", a.banner);
}

/// 🎛 Gérer une position (lot 2 du Simulateur) : la moitié sort à 1 R, le stop passe à l'entrée, la suite sort au
/// break-even ; le journal garde un seul trade (sortie partielle notée, résultat total) ; ✂ la moitié et ⚖ le stop à
/// l'entrée à la main ; 🔀 un OCO dont l'un part et l'autre est annulé.
#[test]
fn simulateur_gerer_une_position() {
    use argos_render::simulateur::SimClick;
    dossier_a_soi("simulateur_gerer_une_position");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    a.rebuild();
    a.toggle_simulateur();
    // la gestion seule : les coûts de la bourse ont leur propre test (`simulateur_couts_reels`)
    a.demo.sans_frais = true;
    let t = a.chart.bars[150].time;
    a.sim_replay_depuis(t, None);
    assert!(a.ui.sim_replay);
    // achat au marché à 251, SL 1 % (1 R = 2,51), TP 2 R ; ½ à 1 R et break-even à 1 R
    {
        let s = a.ui.sim.as_mut().unwrap();
        (s.sl, s.sorties, s.be) = (3, 1, true);
    }
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    assert_eq!((p.gestion.paliers.len(), p.gestion.be_r), (1, Some(1.0)));
    let qty = p.qty;
    a.replay_step(1); // haut 253 : rien
    a.replay_step(1); // haut 254 : la moitié sort à 253,51, le stop passe à 251
    let x = &a.demo.positions[0];
    assert_eq!((x.qty, x.stop), (qty / 2.0, Some(251.0)), "{:?}", a.demo.history);
    a.replay_step(1); // bas 251 : la suite sort au break-even
    assert!(a.demo.positions.is_empty());
    let c = a.demo.history.last().unwrap();
    assert_eq!((c.reason.as_str(), c.pnl), ("break-even", argos_core::Money::ZERO));
    let j = a
        .watch
        .panels
        .journal
        .trades
        .iter()
        .find(|j| j.id == format!("demo-BTC-{}", p.opened_at))
        .unwrap();
    let partiel = argos_core::Money::resultat(253.51, 251.0, qty / 2.0, 1.0).to_f64();
    assert_eq!((j.exit, j.pnl), (Some(251.0), Some(partiel)), "un seul trade au journal, résultat total");
    assert!(j.notes.contains("✂") && j.notes.contains("objectif partiel"), "{}", j.notes);
    assert_eq!(a.demo.stats().trades, 1);
    // ✂ la moitié et ⚖ le stop à l'entrée, à la main
    {
        let s = a.ui.sim.as_mut().unwrap();
        (s.sorties, s.be) = (0, false);
    }
    a.sim_place();
    let id = a.demo.positions[0].id;
    a.sim_faire(SimClick::StopEntree(id));
    assert!(a.banner.as_deref().unwrap_or("").contains("Pas encore en gain"), "au prix d'entrée : refusé");
    a.replay_step(1);
    a.sim_faire(SimClick::StopEntree(id));
    assert_eq!(a.demo.positions[0].stop, Some(a.demo.positions[0].entry), "{:?}", a.banner);
    let q = a.demo.positions[0].qty;
    a.sim_faire(SimClick::Moitie(id));
    assert_eq!(a.demo.positions[0].qty, q / 2.0);
    assert!(a.demo.history.last().unwrap().partiel);
    a.sim_faire(SimClick::CloseTrade(id));
    assert!(a.demo.positions.is_empty());
    // 🔀 OCO : Buy Stop au-dessus et Sell Stop dessous ; la bougie suivante part vers le haut, la vente est annulée
    {
        let s = a.ui.sim.as_mut().unwrap();
        (s.kind, s.oco) = (argos_core::simu::Kind::Stop, true);
    }
    a.sim_place();
    assert_eq!(a.demo.pending.len(), 2, "{:?}", a.banner);
    assert!(a.banner.as_deref().unwrap_or("").contains("OCO"));
    a.replay_step(1);
    assert!(a.demo.pending.is_empty(), "l'autre ordre est annulé");
    // le Buy Stop est parti (les bougies du test font 4 points : le stop de 1 % est touché dans la même bougie, le pire
    // cas) ; la vente n'a jamais été ouverte
    let t1 = a.replay.as_ref().unwrap().t;
    let ouverts: Vec<argos_core::Side> = a
        .demo
        .positions
        .iter()
        .map(|p| (p.side, p.opened_at))
        .chain(a.demo.history.iter().map(|c| (c.side, c.opened_at)))
        .filter(|(_, t)| *t == t1)
        .map(|(s, _)| s)
        .collect();
    assert_eq!(ouverts, vec![argos_core::Side::Long]);
}

/// 💸 Lot 3 : un ordre démo paie les coûts de la bourse du marché ouvert (Binance : maker 0,02 %, taker 0,05 %, écart
/// et glissement) ; le trade fermé, le solde, le journal (P&L net et frais) le montrent. « Sans frais » les retire des
/// nouveaux ordres ; un symbole calculé n'a pas de barème.
#[test]
fn simulateur_couts_reels() {
    use argos_render::simulateur::SimClick;
    dossier_a_soi("simulateur_couts_reels");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    a.rebuild();
    a.toggle_simulateur();
    let bareme = argos_data::couts::bareme(argos_data::Exchange::Binance, "BTC");
    assert_eq!(a.couts_marche(), Some(("Binance", bareme)));
    // achat au marché : les coûts d'entrée (taker, demi-écart, glissement) sont payés tout de suite
    let avant = a.demo.balance;
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    assert_eq!((p.couts, p.maker_in), (Some(bareme), false));
    let entree = bareme.taux(true).total(p.qty, p.entry);
    assert!(entree > argos_core::Money::ZERO);
    assert_eq!(avant - a.demo.balance, entree);
    assert!(a.banner.as_deref().unwrap_or("").contains("coûts d'entrée"), "{:?}", a.banner);
    // fermé à la main au même prix : le trade perd exactement ses coûts
    a.sim_faire(SimClick::CloseTrade(p.id));
    let h = a.demo.history.last().unwrap().clone();
    assert_eq!(h.pnl, argos_core::Money::ZERO);
    assert_eq!(h.fees, entree + bareme.taux(true).total(p.qty, h.exit));
    assert!(h.glissement > argos_core::Money::ZERO && h.glissement < h.fees);
    assert_eq!(a.demo.balance, avant + h.net());
    let j = a
        .watch
        .panels
        .journal
        .trades
        .iter()
        .find(|j| j.id == format!("demo-BTC-{}", p.opened_at))
        .unwrap();
    assert_eq!((j.pnl, j.fees), (Some(h.net().to_f64()), Some(h.fees.to_f64())), "journal : P&L net et frais");
    // « Sans frais » : le prochain ordre ne paie rien
    a.sim_faire(SimClick::SansFrais(true));
    assert!(a.demo.sans_frais);
    let avant = a.demo.balance;
    a.sim_place();
    assert_eq!(a.demo.balance, avant);
    assert!(a.demo.positions[0].couts.is_none());
    a.sim_faire(SimClick::SansFrais(false));
    assert!(!a.demo.sans_frais && a.banner.as_deref().unwrap_or("").contains("Binance"));
    // un symbole calculé n'a pas de vrai carnet : pas de barème
    a.chart.source = Source::Synth { sym: "DXY".into() };
    assert!(a.couts_marche().is_none());
}

/// 💸 Lot 3 : le P&L du jour du Simulateur (volet, pilule DÉMO) est net, comme le solde : un ordre au marché ouvert au
/// prix inchangé a déjà coûté ses frais d'entrée ; fermé à la main au même prix, le jour reste égal à ce que le solde a
/// perdu.
#[test]
fn simulateur_jour_net_comme_le_solde() {
    use argos_render::simulateur::SimClick;
    dossier_a_soi("simulateur_jour_net_comme_le_solde");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    a.rebuild();
    a.toggle_simulateur();
    let depart = a.demo.balance;
    a.sim_place();
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    assert!(a.demo.balance < depart, "les coûts d'entrée sont payés");
    assert_eq!(a.sim_jour(), a.demo.balance - depart, "ouvert, prix inchangé : −coûts d'entrée");
    a.sim_faire(SimClick::CloseTrade(p.id));
    assert!(a.demo.positions.is_empty());
    assert_eq!(a.sim_jour(), a.demo.balance - depart, "fermé au même prix : le jour suit le solde");
}

/// 🌐 Web version: one rule for every key — the browser keeps its own shortcuts (reload, dev tools, tabs, history,
/// find, print…), the app keeps the ones it really uses, and Ctrl + V goes to the browser (whose "paste" event
/// brings the text) only while a text field is open; on the chart it stays « paste a drawing ».
#[test]
fn raccourcis_du_navigateur_passent() {
    use crate::platform::touche_du_navigateur as nav;
    // (key, ctrl, shift, alt, text field open, browser?)
    let table: &[(&str, bool, bool, bool, bool, bool)] = &[
        // browser: reload, dev tools, address bar, tabs, history, find, print, bookmarks, full screen
        ("F5", false, false, false, false, true),
        ("F6", false, false, false, true, true),
        ("F11", false, false, false, false, true),
        ("F12", false, false, false, true, true),
        ("r", true, false, false, false, true),
        ("R", true, true, false, false, true),
        ("I", true, true, false, false, true),
        ("J", true, true, false, true, true),
        ("C", true, true, false, false, true),
        ("l", true, false, false, true, true),
        ("t", true, false, false, false, true),
        ("T", true, true, false, false, true),
        ("w", true, false, false, true, true),
        ("n", true, false, false, false, true),
        ("Tab", true, false, false, true, true),
        ("Tab", true, true, false, false, true),
        ("PageDown", true, false, false, false, true),
        ("1", true, false, false, false, true),
        ("+", true, false, false, false, true),
        ("-", true, false, false, false, true),
        ("0", true, false, false, false, true),
        ("ArrowLeft", false, false, true, false, true),
        ("ArrowRight", false, false, true, true, true),
        ("Home", false, false, true, false, true),
        ("f", true, false, false, true, true),
        ("p", true, false, false, false, true),
        ("d", true, false, false, false, true),
        ("h", true, false, false, false, true),
        ("u", true, false, false, false, true),
        ("o", true, false, false, false, true),
        // 📋 paste: to the browser in a text field, to the app (paste a drawing) on the chart
        ("v", true, false, false, true, true),
        ("V", true, true, false, true, true),
        ("v", true, false, false, false, false),
        // the app's own shortcuts
        ("k", true, false, false, false, false),
        ("k", true, false, false, true, false),
        ("/", true, false, false, false, false),
        ("z", true, false, false, true, false),
        ("Z", true, true, false, false, false),
        ("y", true, false, false, false, false),
        ("c", true, false, false, false, false),
        ("s", true, false, false, true, false),
        ("a", true, false, false, true, false),
        ("x", true, false, false, true, false),
        ("Enter", true, false, false, true, false),
        ("ArrowUp", true, false, false, false, false),
        ("ArrowLeft", true, false, false, false, false),
        ("g", false, false, true, false, false),
        ("r", false, false, true, false, false),
        ("h", false, false, true, false, false),
        ("q", true, false, true, false, false),
        ("r", true, true, true, false, false),
        // typing and editing keys always go to the app (and so to the open field)
        ("Control", true, false, false, true, false),
        ("Shift", false, true, false, true, false),
        ("Alt", false, false, true, true, false),
        ("é", false, false, false, true, false),
        ("b", false, false, false, true, false),
        ("Backspace", false, false, false, true, false),
        ("Delete", false, false, false, true, false),
        ("ArrowLeft", false, false, false, true, false),
        ("Home", false, false, false, true, false),
        ("End", false, false, false, true, false),
        ("Enter", false, false, false, true, false),
        ("Escape", false, false, false, true, false),
        ("Tab", false, false, false, true, false),
        ("Dead", false, false, false, true, false),
    ];
    for &(k, ctrl, shift, alt, saisie, attendu) in table {
        assert_eq!(
            nav(k, ctrl, shift, alt, saisie),
            attendu,
            "{k} ctrl={ctrl} maj={shift} alt={alt} champ={saisie}"
        );
    }
}

/// 💼 Ordres sur la chart (comme TradingView) : préparer, SL du mauvais côté refusé (pastille + message), envoyer avec
/// « Buy », déplacer le TP de l'ordre, le SL d'une position (la fiche du Journal suit), fermer avec ✕.
#[test]
fn ordres_sur_la_chart() {
    use argos_render::ordres::Prise;
    let mut a = app();
    a.watch.panels.trading.guardian = false;
    a.watch.panels.trading.risk = 100.0;
    let last = a.chart.bars.last().unwrap().close;
    // préparer un achat 10 sous le prix : une Limite d'achat, SL et TP posés
    a.brouillon_nouveau(last - 10.0, true);
    let b = a.ui.brouillon.clone().unwrap();
    assert_eq!(b.genre, "Limite");
    assert!(b.sl < b.entree && b.tp > b.entree && b.qty > 0.0);
    // SL glissé au-dessus de l'entrée : pastille rouge et message, puis retour à sa place
    a.ordre_clic(Prise::BrSl);
    a.ordre_glisse(b.entree + 1.0);
    assert_eq!(a.vue_ordres().erreur, Some((Prise::BrSl, "Le Stop Loss doit être inférieur au prix d'entrée.")));
    a.ordre_lache();
    assert_eq!(a.ui.brouillon.as_ref().unwrap().sl, b.sl, "refusé : le SL revient");
    // « Buy » : l'ordre part au compte démo (ordre en attente), le brouillon disparaît
    a.ordre_clic(Prise::BrEnvoyer);
    assert!(a.ui.brouillon.is_none(), "{:?}", a.banner);
    let id = a.demo.pending[0].id;
    assert_eq!(a.vue_ordres().ordres[0].attente, Some("Limite"));
    // le TP de l'ordre se déplace (du bon côté)
    a.ordre_clic(Prise::Tp(id));
    a.ordre_glisse(last + 30.0);
    a.ordre_lache();
    assert_eq!(a.demo.pending[0].order.target, Some(last + 30.0));
    // une position au marché (volet Trading) : son SL se déplace, la fiche du Journal suit
    a.watch.panels.trading.tab = 0;
    a.watch.panels.trading.sl_pips = 4.0;
    a.place_demo_order();
    let p = a.demo.positions[0].clone();
    a.ordre_clic(Prise::Sl(p.id));
    a.ordre_glisse(p.entry - 3.0);
    a.ordre_lache();
    assert_eq!(a.demo.positions[0].stop, Some(p.entry - 3.0));
    let key = format!("demo-{}-{}", p.base, p.opened_at);
    assert_eq!(a.watch.panels.journal.trades.iter().find(|t| t.id == key).unwrap().sl, Some(p.entry - 3.0));
    // 🤖 lot 7 : une ligne glissée est une retouche du plan à la main, gardée pour la carte Argos AI du trade
    let r = a.demo.positions[0].retouches;
    assert!(r.stop_rapproche && r.stop_eloigne.is_none(), "{r:?}");
    a.ordre_clic(Prise::Sl(p.id));
    a.ordre_glisse(p.entry - 6.0);
    a.ordre_lache();
    assert_eq!(
        a.demo.positions[0].retouches.stop_eloigne,
        Some(p.entry - 6.0),
        "plus loin que le stop de départ"
    );
    // ✕ de la position : fermée et inscrite ; la carte Argos AI du trade le dit
    a.ordre_clic(Prise::Fermer(p.id));
    assert!(a.demo.positions.is_empty());
    assert_eq!(a.demo.history.last().unwrap().reason, "fermé à la main");
    let (avis, _) = a.avis_place(&argos_render::text::Text::default()).expect("la carte du trade");
    assert_eq!(avis.plan.pastille, "Pas suivi");
    assert!(avis.plan.texte.starts_with("Stop éloigné à la main"), "{avis:?}");
    // ✕ de l'ordre en attente : annulé
    a.ordre_clic(Prise::Fermer(id));
    assert!(a.demo.pending.is_empty());
}

/// ⌂ Accueil : le P&L du jour compte les trades démo fermés aujourd'hui (leur heure est en secondes ; avant, elle était
/// lue comme des millisecondes et rien n'était jamais « aujourd'hui »).
#[test]
fn accueil_pnl_du_jour_compte_les_trades_fermes() {
    let mut a = app();
    a.demo.history.push(argos_core::portfolio::ClosedTrade {
        base: "BTC".into(),
        side: argos_core::Side::Long,
        qty: 1.0,
        entry: 100.0,
        exit: 600.0,
        pnl: argos_core::Money::from_cents(50_000),
        fees: argos_core::Money::ZERO,
        opened_at: now_s() - 60,
        closed_at: now_s(),
        reason: "objectif".into(),
        risk: argos_core::Money::from_cents(25_000),
        partiel: false,
        glissement: argos_core::Money::ZERO,
        funding: argos_core::Money::ZERO,
        levier: None,
        plan: None,
        real_closed_at: None,
    });
    let d = a.accueil_donnees().demo.unwrap();
    assert_eq!(d.jour, argos_core::Money::from_cents(50_000));
}

/// ⌂ Rechargement (F5) : l'app note dans son onglet si l'Accueil est ouvert ou fermé ; fermé, elle rouvre la chart.
#[test]
fn accueil_ouvert_ou_ferme_est_note() {
    dossier_a_soi("accueil-f5-note");
    platform::onglet::ecrire(crate::actions::ACCUEIL_ONGLET, "0");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.open_accueil();
    a.draw();
    assert_eq!(platform::onglet::lire(crate::actions::ACCUEIL_ONGLET).as_deref(), Some("1"));
    // F5 on the Accueil: the Accueil again (same tab)
    let mut b = app();
    b.top.prefs.bienvenue_vue = true;
    b.accueil_au_depart();
    assert!(b.ui.accueil.is_some(), "F5 on the Accueil: the Accueil");
    b.ui.accueil = None;
    b.draw();
    assert_eq!(platform::onglet::lire(crate::actions::ACCUEIL_ONGLET).as_deref(), Some("0"));
    // F5 on the chart: the chart
    let mut c = app();
    c.top.prefs.bienvenue_vue = true;
    c.accueil_au_depart();
    assert!(c.ui.accueil.is_none(), "F5 on the chart: the chart");
    // topbar.json never holds the Accueil's state
    let ecrit = platform::read("topbar.json").unwrap_or_default();
    assert!(!ecrit.contains("accueil_ouvert"), "{ecrit}");
}

/// ⌂ Jee's bug (22 Nov): an Accueil left open in another tab or device (shared topbar.json, old versions) no longer
/// opens the Accueil at F5 here; the very first launch and « Accueil au lancement » still do.
#[test]
fn accueil_d_un_autre_onglet_ne_revient_pas_ici() {
    dossier_a_soi("accueil-f5-autre");
    platform::onglet::ecrire(crate::actions::ACCUEIL_ONGLET, "0");
    platform::write("topbar.json", r#"{"accueil_ouvert": true, "accueil_v2": true}"#).unwrap();
    let mut a = app();
    a.top.prefs = App::read_prefs("topbar.json");
    a.top.prefs.bienvenue_vue = true;
    a.accueil_au_depart();
    assert!(a.ui.accueil.is_none(), "the chart stays");
    // the very first launch: the Accueil and its 3 welcome screens
    let mut b = app();
    b.top.prefs.bienvenue_vue = false;
    b.accueil_au_depart();
    assert!(b.ui.accueil.is_some(), "first launch: the Accueil");
    // « Accueil au lancement » on: the Accueil
    let mut c = app();
    c.top.prefs.bienvenue_vue = true;
    c.top.prefs.accueil = true;
    c.accueil_au_depart();
    assert!(c.ui.accueil.is_some(), "the setting on: the Accueil");
}

/// 🛡 What covers the chart, as the app tells the page for the quality gate (bug B-041, 25 Nov 2026: the gate's gestures,
/// played on the first launch's welcome window left open, were measured as the chart's): the welcome window from the
/// very start, the Accueil once it is skipped, nothing once that is closed; a settings change come from elsewhere does
/// not bring them back over the chart; the search over it and the finger's crosshair are named too.
#[test]
fn par_dessus_dit_ce_qui_couvre_la_chart() {
    dossier_a_soi("par-dessus");
    platform::onglet::ecrire(crate::actions::ACCUEIL_ONGLET, "0");
    let mut a = app();
    assert!(a.par_dessus().is_empty(), "the chart alone: {:?}", a.par_dessus());
    // the very first launch: the welcome window, at once (before the first image)
    a.top.prefs.bienvenue_vue = false;
    a.accueil_au_depart();
    assert_eq!(a.par_dessus(), ["bienvenue"]);
    // « plus tard » (Échap, as « Passer »): the Accueil under it
    a.key_accueil(&Key::Named(NamedKey::Escape));
    assert_eq!(a.par_dessus(), ["accueil"]);
    a.key_accueil(&Key::Named(NamedKey::Escape));
    assert!(a.par_dessus().is_empty(), "{:?}", a.par_dessus());
    // the settings changed on another device (the top bar's, interface-2's, the open market's): still the chart alone
    a.recharger(&["topbar.json".into(), "interface-2.json".into(), "session.json".into()]);
    assert!(a.par_dessus().is_empty(), "{:?}", a.par_dessus());
    // the search over the chart
    a.open_palette();
    assert_eq!(a.par_dessus(), ["recherche"]);
    // the finger's crosshair (a long press): a drag would move it, not the chart
    a.ui.palette = None;
    a.ui.croix = Some((120.0, 80.0, 0.0));
    assert_eq!(a.par_dessus(), ["reticule"]);
}

/// 👋 The welcome screens, once seen, are not shown again by a top bar's settings come from another device (bug B-042,
/// 25 Nov 2026: `topbar.json` replaced the flag with its own — an older version, /app/, writes it there and never reads
/// interface-2's — and ⌂ then opened the 3 first-launch screens over the Accueil again). « Revoir l'accueil » still does.
#[test]
fn bienvenue_vue_reste_vue_apres_une_synchro() {
    dossier_a_soi("bienvenue-synchro");
    platform::onglet::ecrire(crate::actions::ACCUEIL_ONGLET, "0");
    let mut a = app();
    a.top.prefs.bienvenue_vue = false;
    a.accueil_au_depart();
    a.key_accueil(&Key::Named(NamedKey::Escape));
    a.key_accueil(&Key::Named(NamedKey::Escape));
    assert!(a.top.prefs.bienvenue_vue && App::bienvenue_lire(), "seen, kept in interface-2.json");
    // another device's top bar, written by an older version (its own flag: not seen)
    platform::write("topbar.json", r#"{"bienvenue_vue": false, "accueil_v2": true}"#).unwrap();
    a.recharger(&["topbar.json".into()]);
    assert!(a.top.prefs.bienvenue_vue, "still seen");
    a.open_accueil();
    assert_eq!(a.par_dessus(), ["accueil"], "⌂: the Accueil, without the first-launch screens");
    // « Revoir l'accueil »: the 3 screens again, on purpose
    a.revoir_accueil();
    assert_eq!(a.par_dessus(), ["bienvenue"]);
}

/// 🕗 Heure de New York par défaut (fuseau jamais choisi), comme la maquette ; un fuseau choisi est gardé.
#[test]
fn heure_de_new_york_par_defaut() {
    let mut a = app();
    a.chart.settings.timezone.clear();
    assert_eq!(a.fuseau(), "America/New_York");
    assert_eq!(a.tz_offset_s(), argos_render::settings::tz_offset("America/New_York", now_s()).unwrap());
    a.chart.settings.timezone = "Europe/Paris".into();
    assert_eq!(a.fuseau(), "Europe/Paris");
}

/// ⟳ Actualiser (Ctrl + R) : la vue revient sur la dernière bougie et un toast le dit (avant : rien de visible si la
/// vue était décalée) ; aussi dans Ctrl + K.
#[test]
fn actualiser_recentre_et_le_dit() {
    use argos_render::recherche::{filtrer, Cible, Palette};
    let mut a = app();
    a.chart.viewport.right_offset = 500.0;
    touche(&mut a, lettre("r"), ModifiersState::CONTROL);
    assert_eq!(a.chart.viewport.right_offset, argos_render::viewport::Viewport::default().right_offset);
    let p = Palette::new(&[]);
    assert!(filtrer(&p.items, "actualiser").iter().any(|&i| p.items[i].cible == Cible::Actualiser));
}

/// 🫧 Le verre ne se coupe plus pour quelques images lentes : 17 ms par image (60 i/s) le gardent ; seulement
/// plusieurs secondes au-dessus de 25 ms l'allègent puis le coupent, avec un message calme.
#[test]
fn verre_garde_a_17_ms() {
    let mut a = app();
    a.top.prefs.glass = true;
    argos_render::glass::set_level(0);
    for _ in 0..1000 {
        a.ui.glass_now = 17.0;
        a.frame_timing(17.0);
    }
    assert!(!a.ui.glass_off && argos_render::glass::level() == 0, "17 ms : le verre reste");
    for _ in 0..2000 {
        a.ui.glass_now = 60.0;
        a.frame_timing(60.0);
    }
    assert!(a.ui.glass_off, "longtemps très lent : coupé");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("Verre allégé")));
    argos_render::glass::set_level(0);
}

/// ✨ verre-2 écran 4 (pc-36) : Argos AI dit où l'on est (chart ou Accueil), dessiné à chaque image.
#[test]
fn argos_ai_dit_ou_on_est() {
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ui.ai_open = true;
    a.draw();
    assert!(a.ai.lieu.starts_with("Tu es sur la chart : "), "{}", a.ai.lieu);
    assert!(a.ai.lieu.contains(&a.symbol_label()));
    a.open_accueil();
    a.draw();
    assert_eq!(a.ai.lieu, "Tu es sur l'Accueil");
}

/// 🛡 verre-2 (maquette) : la pilule du Gardien de la barre du haut suit le vrai Gardien PropFirm (éteint : pas de
/// chiffres) ; 🔔 compte les alertes du symbole.
#[test]
fn barre_du_haut_gardien_et_alertes() {
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.watch.panels.trading.guardian = false;
    a.draw();
    assert_eq!(a.top.gardien, None);
    a.watch.panels.trading.guardian = true;
    a.draw();
    let (t, _) = a.top.gardien.clone().expect("Gardien allumé : la pilule a ses chiffres");
    assert!(t.ends_with(" %") && t.contains(" / "), "{t}");
    assert_eq!(a.top.alertes, a.alerts.lines(&a.symbol_label()).len());
}

/// ⌂ verre-2 (maquette) : ★ et ⋯ des tuiles de mises en page — favori gardé, dupliquer, renommer une autre que celle
/// ouverte, supprimer seulement après « Oui » (celle ouverte : une autre s'ouvre d'abord).
#[test]
fn tuiles_de_mises_en_page_etoile_et_menu() {
    use crate::actions::layouts::{Layout, Layouts, FICHIER};
    // (its own folder: the layouts file is written by other tests running at the same time)
    dossier_a_soi("tuiles-layouts");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.top.prefs.layout_name = "Argos".into();
    App::write_prefs(
        FICHIER,
        &Layouts {
            ouverte: "Argos".into(),
            liste: vec![
                Layout {
                    nom: "Argos".into(),
                    ..Default::default()
                },
                Layout {
                    nom: "Or".into(),
                    ..Default::default()
                },
            ],
        },
    );
    a.open_accueil();
    // ★ : favori, puis plus
    a.accueil_result(argos_render::accueil::AccClick::Favori(1));
    assert_eq!(a.favoris_layouts(), vec!["Or".to_string()]);
    assert_eq!(a.ui.accueil.as_ref().unwrap().favoris, vec!["Or".to_string()], "la tuile suit");
    a.accueil_result(argos_render::accueil::AccClick::Favori(1));
    assert!(a.favoris_layouts().is_empty());
    // ⋯ : le menu s'ouvre par-dessus l'Accueil
    let r = Rect::new(500.0, 300.0, 28.0, 28.0);
    a.accueil_result(argos_render::accueil::AccClick::MenuLayout(1, r));
    assert!(matches!(a.ui.list, Some((ListKind::TuileLayout(1), _))) && a.ui.accueil.is_some());
    a.ui.list = None;
    // 📄 dupliquer
    a.tuile_pick(ListKind::TuileLayout(1), 3, r);
    assert!(a.nom_layout(2).as_deref() == Some("Or (copie)"), "{:?}", a.liste_layouts());
    // ✏️ renommer « Or (copie) » (pas celle ouverte)
    a.tuile_pick(ListKind::TuileLayout(2), 2, r);
    assert_eq!(a.ui.layout_cible.as_deref(), Some("Or (copie)"));
    a.ui.name_box = None;
    a.renommer_layout_de("Or (copie)", "Nuit".into());
    assert_eq!(a.top.prefs.layout_name, "Argos", "celle ouverte ne change pas");
    assert_eq!(a.nom_layout(2).as_deref(), Some("Nuit"));
    // 🗑 « Supprimer… » demande d'abord ; Annuler ne supprime rien ; « Oui » supprime
    a.tuile_pick(ListKind::TuileLayout(2), 5, r);
    assert!(matches!(a.ui.list, Some((ListKind::SupprimerLayout(2), _))));
    a.ui.list = None;
    a.tuile_pick(ListKind::SupprimerLayout(2), 0, r);
    assert_eq!(a.liste_layouts().len(), 3);
    a.tuile_pick(ListKind::SupprimerLayout(2), 6, r);
    assert_eq!(a.liste_layouts().len(), 2);
    // celle ouverte : une autre s'ouvre d'abord, puis elle part
    a.tuile_pick(ListKind::SupprimerLayout(0), 6, r);
    assert_eq!(a.top.prefs.layout_name, "Or");
    assert_eq!(a.liste_layouts().into_iter().map(|x| x.0).collect::<Vec<_>>(), ["Or"]);
    assert!(a.banner.as_deref().unwrap_or("").contains("« Or » ouverte"));
}

/// 📱 Bug cell (Jee, 2026-11-09) : au démarrage, le volet Trading restauré (ouvert sur le PC) prenait tout l'écran du
/// cell. Sur cell on arrive sur la chart (Accueil seulement au premier lancement) ; sur PC et tablette, tout est
/// restauré. Une seule fois : ensuite, ouvrir un volet sur le cell marche.
#[test]
fn depart_sur_cell_toujours_la_chart() {
    for (phone, volet_garde) in [(true, false), (false, true)] {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.watch.prefs.open = true;
        a.ui.ai_open = true;
        a.open_accueil();
        a.premiere_image(phone);
        assert_eq!(a.watch.prefs.open, volet_garde, "cell {phone}");
        assert_eq!(a.ui.accueil.is_some(), volet_garde, "cell {phone}");
        a.watch.prefs.open = true;
        a.premiere_image(phone);
        assert!(a.watch.prefs.open, "une seule fois");
    }
    // tout premier lancement sur cell : les 3 écrans de l'Accueil restent
    let mut a = app();
    a.top.prefs.bienvenue_vue = false;
    a.open_accueil();
    a.premiere_image(true);
    assert!(a.ui.accueil.is_some());
}

/// 📱 verre-2 cell (maquette tel-04 et tel-25) : un volet ouvert et Argos AI sont des feuilles du bas, la chart reste
/// visible au-dessus (au moins 160 points) ; la feuille commence là où la chart s'arrête.
#[test]
fn cell_feuilles_du_bas() {
    for h in [500.0, 700.0, 900.0] {
        let haut = crate::render::feuille_haut(h);
        assert!(haut >= 160.0 && haut <= h - 120.0, "{h} : {haut}");
    }
    assert_eq!(crate::render::feuille_haut(100.0), 0.0, "écran minuscule : la feuille prend tout");
    let mut a = app();
    a.top.phone = true;
    let r = a.ai_rect();
    assert!(r.y > a.top_h() + 150.0 && (r.y + r.h - (a.top_h() + a.area.1 as f64)).abs() < 0.5, "{r:?}");
}

/// Jee (2026-11-09, maquette) : − + ‹ › ↺ sont tout en bas de la pile (chart + tous les panneaux d'indicateurs),
/// juste au-dessus de l'échelle de temps, sur PC comme sur cell ; avec 0, 1 ou 5 panneaux, « − » (en bas) dézoome.
#[test]
fn navigation_en_bas_de_la_pile() {
    use argos_render::indicators::Ind;
    let oscs: Vec<Ind> = Ind::ALL.iter().copied().filter(|i| i.oscillator()).take(5).collect();
    // (📱 cell et tablette : plus de pilule de navigation — Jee, 2026-11-09 ; voir cell_sans_pilule_de_navigation)
    for phone in [false] {
        for n in [0, 1, 5] {
            let mut a = app();
            a.top.phone = phone;
            a.tools.hidden = true;
            a.ind.on = oscs[..n.min(oscs.len())].to_vec();
            a.ui.frames = 5;
            a.refresh_indicators();
            let vp = a.chart.viewport;
            let full = !phone;
            let (cx, cy) = argos_render::navbar::centers(&vp, vp.plot_h(), full)[0];
            assert!(cy > vp.plot_h() - 60.0, "{phone} {n} : en bas");
            let avant = a.chart.viewport.bar_spacing;
            let (wx, wy) = (cx + a.rw(), cy + a.top_h());
            a.mouse_moved(PhysicalPosition::new(wx, wy));
            a.left_button(ElementState::Pressed);
            a.left_button(ElementState::Released);
            assert!(a.chart.viewport.bar_spacing < avant, "cell {phone}, {n} panneaux : « − » dézoome");
        }
    }
}

/// 📱 Jee (2026-11-09) : sur cell et tablette, pas de pilule − + ‹ › ↺ ↶ ↷ (pincer / glisser) ; le Replay est dans ⚙.
#[test]
fn cell_sans_pilule_de_navigation() {
    let mut a = cell_390();
    let vp = a.chart.viewport;
    assert!(
        !a.to_chart().navigation && a.to_chart().legende,
        "pas de pilule sur cell, la légende toujours là"
    );
    let (cx, cy) = argos_render::navbar::centers(&vp, vp.plot_h(), false)[0];
    let avant = a.chart.viewport.bar_spacing;
    let (wx, wy) = (cx + a.rw(), cy + a.top_h());
    toucher(&mut a, wx, wy);
    assert_eq!(a.chart.viewport.bar_spacing, avant, "le bas de la chart ne dézoome plus");
    let panneaux: Vec<argos_render::sheet::Act> = argos_render::sheet::PANELS.iter().flat_map(|(_, a)| a.iter().copied()).collect();
    assert!(panneaux.contains(&argos_render::sheet::Act::Replay), "Replay dans ⚙");
    // pas de ⚙ dans le coin des échelles (absent de la maquette, juste au-dessus d'Argos AI)
    let (gx, gy, gw, gh) = argos_render::context_menu::gear_rect(&a.chart.viewport);
    let (gx, gy) = (gx + gw / 2.0 + a.rw(), gy + gh / 2.0 + a.top_h());
    toucher(&mut a, gx, gy);
    assert!(a.ui.context.open.is_none(), "le ⚙ du coin n'existe plus sur cell");
}

/// 📱 Bug cell n° 3 (Jee, 2026-11-09) : toucher ⚙ (◎) dans la légende d'un indicateur doit ouvrir ses réglages.
#[test]
fn cell_toucher_reglages_d_indicateur() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::LegClick;
    for phone in [false, true] {
        let mut a = app();
        a.top.phone = phone;
        // un oscillateur dans sa bande (comme Cipher Pro sur la capture de Jee) et un indicateur sur le prix
        let osc = *Ind::ALL
            .iter()
            .find(|i| i.oscillator() && !i.params().1.is_empty())
            .expect("un oscillateur à inputs");
        a.ind.on = vec![Ind::Tvd, osc];
        a.ind.leg.0 = true;
        a.tools.hidden = true; // la barre des favoris n'est pas sur la légende
        a.ui.frames = 5; // 📱 après les premières images (la chart d'abord)
        a.refresh_indicators();
        let t = argos_render::text::Text::default();
        let ph = a.chart.viewport.plot_h();
        // 📱 sur cell, ⚙ est dans la barre d'actions de l'indicateur (son nom touché)
        if phone {
            a.ind.sel = Some(Ok(osc));
        }
        let pw = a.chart.viewport.plot_w();
        let cell = argos_render::indleg::Cell {
            actif: phone,
            sel: a.ind.sel,
            pw,
        };
        // la zone de ⚙ : on la cherche point par point
        let mut gear = None;
        'cherche: for y in (0..ph as i32).step_by(2) {
            for x in (0..400).step_by(2) {
                if argos_render::indleg::click_de(&t, &a.ind.on, &a.ind.out, a.ind.leg.0, ph, pw, cell, x as f64, y as f64) == Some(LegClick::Settings(osc)) {
                    gear = Some((x as f64 + 6.0, y as f64 + 6.0));
                    break 'cherche;
                }
            }
        }
        let (x, y) = gear.expect("⚙ dans l'en-tête de la bande");
        let (wx, wy) = (x + a.rw(), y + a.top_h());
        a.mouse_moved(PhysicalPosition::new(wx, wy));
        a.left_button(ElementState::Pressed);
        assert!(a.ui.ind_dlg.is_some(), "cell {phone} : appui, les réglages s'ouvrent");
        a.left_button(ElementState::Released);
        assert!(
            a.ui.ind_dlg.is_some(),
            "cell {phone} : souris, les réglages s'ouvrent ({:?} {:?})",
            a.pointer.mouse,
            (x, y)
        );
        a.ui.ind_dlg = None;
        a.touch(1, winit::event::TouchPhase::Started, (wx, wy));
        a.touch(1, winit::event::TouchPhase::Ended, (wx, wy));
        assert!(a.ui.ind_dlg.is_some(), "cell {phone} : les réglages s'ouvrent");
    }
}

/// 📱 Cell : au démarrage, la chart s'affiche d'abord ; les indicateurs se calculent à partir de la 3e image.
#[test]
fn cell_la_chart_d_abord() {
    use argos_render::indicators::Ind;
    let mut a = app();
    a.top.phone = true;
    a.ind.on = vec![Ind::Rsi];
    a.ind.key = Default::default();
    a.ui.frames = 0;
    a.refresh_indicators();
    assert_eq!(a.ind.key.0, 0, "1re image : pas encore d'indicateurs");
    a.ui.frames = 2;
    a.refresh_indicators();
    assert_eq!(a.ind.key.0, a.chart.bars.len(), "ensuite : calculés");
}

/// 📱 Jee (2026-11-09, 02:39) : sur cell, comme les barres latérales du PC — chaque barre (haut, bas) a sa petite
/// poignée au bord ; la toucher cache la barre, la toucher de nouveau la remontre ; toucher la chart ne fait rien de
/// spécial ; l'état est gardé ; jamais sur PC.
#[test]
fn cell_poignees_des_barres() {
    use winit::event::TouchPhase;
    let mut a = app();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    assert!(!a.haut_cache() && !a.bas_cache() && a.top_h() > 0.0, "visibles au départ");
    let tap = |a: &mut App, id: u64, r: Rect| {
        a.touch(id, TouchPhase::Started, (r.cx(), r.cy()));
        a.touch(id, TouchPhase::Ended, (r.cx(), r.cy()));
    };
    // un toucher sur la chart : rien ne se cache
    a.touch(1, TouchPhase::Started, (200.0, 300.0));
    a.touch(1, TouchPhase::Ended, (200.0, 300.0));
    assert!(!a.haut_cache() && !a.bas_cache(), "toucher la chart ne cache rien");
    // poignée du haut : cachée, puis revient
    let h = a.poignee_haut();
    tap(&mut a, 2, h);
    assert!(a.haut_cache() && a.top_h() == 0.0 && !a.bas_cache(), "le haut seulement");
    let h = a.poignee_haut();
    assert!(h.y < 4.0, "la poignée reste au bord du haut");
    tap(&mut a, 3, h);
    assert!(!a.haut_cache());
    // poignée du bas : indépendante, gardée dans les réglages
    let b = a.poignee_bas();
    tap(&mut a, 4, b);
    assert!(a.bas_cache() && a.top.prefs.cell_bas_cache && !a.haut_cache());
    let vieux: argos_render::topbar::Prefs = serde_json::from_str(r#"{"pct":true}"#).unwrap();
    assert!(!vieux.cell_haut_cache && !vieux.cell_bas_cache, "ancien fichier : barres visibles");
    // PC : pas de poignée
    let mut p = app();
    p.top.phone = false;
    p.pointer.win_mouse = (p.poignee_haut().cx(), p.poignee_haut().cy());
    assert!(!p.poignee_toucher());
}

/// 📱 ⌂ sur cell : l'Accueil s'ouvre (aussi depuis ☰ Appli), et « Accueil au lancement » le garde au démarrage.
#[test]
fn cell_accueil() {
    let mut a = app();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    a.top_act(argos_render::topmenu::TopAct::Accueil as i64, Rect::new(0.0, 0.0, 10.0, 10.0));
    assert!(a.ui.accueil.is_some(), "☰ Appli ▸ Accueil");
    a.ui.accueil = None;
    a.top.prefs.accueil = true;
    a.open_accueil();
    a.premiere_image(true);
    assert!(a.ui.accueil.is_some(), "Accueil au lancement : il reste sur cell");
}

/// 📱 cell-1 : chaque bouton de la nouvelle barre du bas (maquette) ouvre quelque chose ; les anciennes actions du
/// cell gardent un point d'accès (watchlist, unités, dessin, Panneaux par ⚙, Market par 👥, baleines, DÉMO, ƒx,
/// Argos AI par le bouton rond).
#[test]
fn cell_chaque_action_a_un_acces() {
    use argos_render::phonebar::{Icone, PhoneHit};
    let r = Rect::new(10.0, 700.0, 40.0, 40.0);
    let ouvert = |a: &App| {
        a.watch.prefs.open
            || a.ui.list.is_some()
            || a.ui.sheet.is_some()
            || a.ui.sim.is_some()
            || a.ui.pop.is_some()
            || a.ui.ac_page.is_some()
            || a.ui.editor.is_some()
            || a.ui.fx_page.is_some()
            || a.ui.notifs.is_some()
    };
    let mut hits = vec![PhoneHit::Symbol, PhoneHit::Interval, PhoneHit::Fx, PhoneHit::Demo];
    hits.extend(Icone::TOUTES.into_iter().map(PhoneHit::Outil));
    for h in hits {
        let mut a = app();
        a.top.phone = true;
        a.watch.prefs.open = false;
        a.phone_click(h, r);
        assert!(ouvert(&a), "{h:?} n'ouvre rien");
    }
    // Argos AI : le bouton rond, aussi sur cell (barres cachées ou non)
    let mut a = app();
    a.top.phone = true;
    let b = a.ai_bouton_rect();
    assert!(b.w > 0.0 && b.x + b.w <= a.win_w());
}

/// 🧰 Barre de droite de la maquette (Jee, 2026-11-09) : chaque bouton « action » ouvre quelque chose (dessin,
/// Gardien → Simulateur, Argos Script, Académie, Communauté, Portefeuille, Liquidations sur la chart, Marketplace).
#[test]
fn barre_droite_chaque_bouton_ouvre() {
    use argos_render::watchlist::Tool;
    let r = Rect::new(1200.0, 100.0, 38.0, 38.0);
    for t in Tool::ALL.into_iter().filter(|t| t.action()) {
        let mut a = app();
        let avant_ind = a.ind.on.len();
        a.outil_barre(t, r);
        let ouvert = a.ui.list.is_some()
            || a.ui.sheet.is_some()
            || a.ui.sim.is_some()
            || a.ui.editor.is_some()
            || a.ui.ac_page.is_some()
            || a.ui.pf_page.is_some()
            || a.ui.mk_page.is_some()
            || a.ind.on.len() != avant_ind
            || a.banner.is_some();
        assert!(ouvert, "{t:?} n'ouvre rien");
    }
}

/// 🫥 Jee (2026-11-09) : sur PC, la barre d'outils flottante disparaît quand la souris n'est pas près d'elle (elle
/// ne cache plus les bougies) et revient dès qu'on s'en approche ; au doigt, elle reste.
#[test]
fn barre_outils_se_cache_loin_de_la_souris() {
    let mut a = app();
    let (r, _) = toolbar::layout(&a.tools, &argos_render::text::Text::default());
    a.pointer.mouse = Some((r.x + r.w + 200.0, r.y + r.h + 200.0));
    assert!(!a.barre_outils_proche(false), "loin : cachée");
    assert!(a.barre_outils_proche(true), "une de ses fenêtres ouverte : visible");
    a.pointer.mouse = Some((r.x + 5.0, r.y - 10.0));
    assert!(a.barre_outils_proche(false), "tout près : visible");
    a.pointer.mouse = None;
    assert!(!a.barre_outils_proche(false), "souris hors de la chart : cachée");
    a.top.phone = true;
    assert!(a.barre_outils_proche(false), "au doigt : toujours là");
}

/// 🫥 Jee (2026-11-08, maquette) : sur PC, la rangée du haut et la barre de gauche s'estompent quand la souris
/// travaille sur la chart ; elles reviennent près d'elles, avec un menu ouvert, sur cell (barres cachées à part) ou
/// réglage éteint ; un ancien fichier de réglages garde l'estompage allumé.
#[test]
fn barres_estompees_sur_la_chart() {
    let mut a = app();
    a.pointer.mouse = Some((400.0, 300.0));
    assert!(a.barres_estompees(), "au milieu de la chart : estompées");
    a.pointer.mouse = Some((400.0, 10.0));
    assert!(!a.barres_estompees(), "près de la rangée du haut : pleines");
    a.pointer.mouse = Some((10.0, 300.0));
    assert!(!a.barres_estompees(), "près de la barre de gauche : pleine");
    a.pointer.mouse = Some((400.0, 300.0));
    a.open_palette();
    assert!(!a.barres_estompees(), "un menu ouvert : pleines");
    a.ui.palette = None;
    a.top.prefs.estomper = false;
    assert!(!a.barres_estompees(), "réglage éteint");
    a.top.prefs.estomper = true;
    a.top.phone = true;
    assert!(!a.barres_estompees(), "cell : pas d'estompage (barres cachées au toucher)");
    let vieux: argos_render::topbar::Prefs = serde_json::from_str(r#"{"pct":true}"#).unwrap();
    assert!(vieux.estomper);
}

/// 🎡 Cell (maquette tel-01) : la molette de la pilule « symbole · unité » — un toucher en haut passe à l'unité de
/// temps favorite d'avant, en bas à celle d'après ; les voisins sont montrés en petit.
#[test]
fn cell_molette_des_unites() {
    use argos_render::phonebar::PhoneHit;
    let mut a = app();
    a.top.phone = true;
    a.tools.favorites = vec![900, 1800, 3600];
    a.set_interval(Interval(1800));
    assert_eq!(a.voisins_intervalle(), (Some(900), Some(3600)));
    let m = a.cell_etat();
    assert_eq!(m.unite_voisines, (Some("15m".into()), Some("1h".into())));
    let r = Rect::new(0.0, 700.0, 44.0, 13.0);
    a.phone_click(PhoneHit::IntervalleSuiv, r);
    assert_eq!(a.chart.interval.secs(), 3600, "en bas : l'unité d'après");
    a.phone_click(PhoneHit::IntervallePrec, r);
    assert_eq!(a.chart.interval.secs(), 1800, "en haut : l'unité d'avant");
}

/// 📱 maquette-4 : la rangée du bas du cell suit la maquette (7 icônes + ⚙) ; ce qui en est sorti garde un accès —
/// le journal (ARGOS ▾), l'agenda (⚙ Panneaux), la communauté (Accueil), le dessin (barre d'outils).
#[test]
fn cell_rien_ne_se_perd() {
    use argos_render::phonebar::Icone;
    assert_eq!(Icone::TOUTES.len(), 8);
    assert_eq!(Icone::TOUTES.last(), Some(&Icone::Reglages));
    let mut a = app();
    a.top.phone = true;
    a.top_act(argos_render::topmenu::TopAct::Journal as i64, Rect::new(0.0, 0.0, 10.0, 10.0));
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Journal, "journal : ARGOS ▾");
    let panneaux: Vec<argos_render::sheet::Act> = argos_render::sheet::PANELS.iter().flat_map(|(_, a)| a.iter().copied()).collect();
    assert!(panneaux.contains(&argos_render::sheet::Act::Agenda), "agenda : ⚙ Panneaux");
}

/// 📱 Jee (2026-11-09) : sur cell, toucher la poignée de la barre des favoris l'ouvre (et la ferme) ; un petit
/// tremblement du doigt n'est pas un glisser.
#[test]
fn cell_poignee_des_favoris_s_ouvre_au_doigt() {
    use winit::event::TouchPhase;
    let mut a = app();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    a.tools.retracted = true;
    let text = argos_render::text::Text::default();
    let h = argos_render::toolbar::button_rect(&a.tools, &text, argos_render::toolbar::Btn::Handle).expect("poignée");
    // au doigt : à 20 points à gauche de la poignée (cible de 44 points), ça touche encore la poignée
    assert_eq!(
        argos_render::toolbar::hit(&a.tools, &text, h.cx() - 14.0, h.cy()),
        Some(argos_render::toolbar::Btn::Handle)
    );
    let (x, y) = (h.cx() + a.rw(), h.cy() + a.top_h());
    let avant = (a.tools.x, a.tools.y);
    a.touch(1, TouchPhase::Started, (x, y));
    a.touch(1, TouchPhase::Moved, (x + 4.0, y + 3.0));
    a.touch(1, TouchPhase::Ended, (x + 4.0, y + 3.0));
    assert!(!a.tools.retracted, "toucher la poignée ouvre la barre");
    // (un tremblement n'est pas un glisser : elle ne suit pas le doigt ; ouverte, elle glisse seulement vers la gauche
    // pour tenir dans l'écran, comme la maquette)
    let attendu = argos_render::toolbar::place_ouverte(&a.tools, &text);
    assert_eq!((a.tools.x, a.tools.y), (attendu, avant.1), "un tremblement ne la déplace pas");
    assert!(a.tools.x <= avant.0, "ouverte : vers la gauche, jamais vers le doigt");
    let h = argos_render::toolbar::button_rect(&a.tools, &text, argos_render::toolbar::Btn::Handle).unwrap();
    let (x, y) = (h.cx() + a.rw(), h.cy() + a.top_h());
    a.touch(2, TouchPhase::Started, (x, y));
    a.touch(2, TouchPhase::Ended, (x, y));
    assert!(a.tools.retracted, "encore : elle se referme");
}

/// 📱 Un cell de 390 × 844 : l'app, barres visibles.
fn cell_390() -> App {
    let mut a = app();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    a.area = (390, (844.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
    a
}

/// 📱 Ce que l'utilisateur voit changer après un toucher (menus, volets, pages, symbole, unité…).
fn empreinte(a: &App) -> String {
    let u = &a.ui;
    format!(
        "{:?}",
        (
            (
                a.watch.prefs.open,
                a.watch.prefs.panel,
                u.list.as_ref().map(|(k, _)| format!("{k:?}")),
                u.sheet.is_some(),
                u.sim.is_some()
            ),
            (
                u.pop.is_some(),
                u.ac_page.is_some(),
                u.editor.is_some(),
                u.argos.is_some(),
                u.accueil.is_some(),
                u.palette.is_some()
            ),
            (
                u.ai_open,
                a.banner.clone(),
                a.chart.interval,
                a.symbol_label(),
                a.tools.retracted,
                u.mk_page.is_some(),
                u.pf_page.is_some(),
                u.fx_page.is_some(),
                u.notifs.is_some(),
                u.plan.is_some()
            ),
            (a.top.prefs.cell_haut_cache, a.top.prefs.cell_bas_cache, a.ind.on.len(), u.marches.is_some()),
        )
    )
}

/// 👆 Un toucher (doigt posé puis levé au même endroit), coordonnées de la fenêtre.
fn toucher(a: &mut App, x: f64, y: f64) {
    use winit::event::TouchPhase;
    a.touch(9, TouchPhase::Started, (x, y));
    a.touch(9, TouchPhase::Ended, (x, y));
}

/// 📱 Jee (2026-11-09, « aucune fonction ne marche sur cell ») : un VRAI toucher sur chaque contrôle du cell, à sa
/// place à l'écran (390 × 844), fait quelque chose ; rien ne mange les touchers avant les contrôles.
#[test]
fn cell_fonctions() {
    use argos_render::ecrans::CellHit;
    // chaque zone de l'écran principal (mesurée dans la maquette) : un vrai toucher à sa place fait quelque chose
    let a0 = cell_390();
    let zones = argos_render::ecrans::zones(a0.win_w(), a0.cell_h(), &a0.cell_etat());
    assert!(zones.len() >= 25, "{} zones", zones.len());
    let mut muets = vec![];
    for (r, h) in &zones {
        let mut a = cell_390();
        let avant = empreinte(&a);
        toucher(&mut a, r.cx(), r.cy());
        let ouvert = a.ui.sim.is_some() || a.ui.ai_open || a.ui.split_menu.is_some() || a.banner.is_some();
        if empreinte(&a) == avant && !ouvert {
            muets.push(format!("{h:?}"));
        }
    }
    assert!(muets.is_empty(), "ces touchers ne font rien : {muets:?}");
    // la rangée du haut : ses 8 boutons, chacun une action différente
    let haut: Vec<_> = zones.iter().filter(|(r, _)| r.y < 50.0).collect();
    assert_eq!(haut.len(), 8, "{haut:?}");
    // Argos AI (bouton rond) ; un toucher sur la chart ne mange rien et ne cache rien
    let mut a = cell_390();
    let b = a.ai_bouton_rect();
    toucher(&mut a, b.cx(), b.cy());
    assert!(a.ui.ai_open, "Argos AI s'ouvre au doigt");
    let mut a = cell_390();
    let avant = (a.haut_cache(), a.bas_cache());
    toucher(&mut a, 195.0, 400.0);
    assert_eq!((a.haut_cache(), a.bas_cache()), avant, "toucher la chart ne cache pas les barres");
    // le symbole (🔎 le panneau de marchés, liste 21 nov.), puis une icône du bas : un 2e contrôle répond après le premier
    let mut a = cell_390();
    let at = |h: CellHit| zones.iter().find(|(_, k)| *k == h).unwrap().0;
    let (s1, t) = (at(CellHit::Symbole), at(CellHit::Icone(argos_render::phonebar::Icone::Trading)));
    toucher(&mut a, s1.cx(), s1.cy());
    assert!(a.ui.marches.is_some(), "la pastille du symbole ouvre le panneau de marchés");
    toucher(&mut a, t.cx(), t.cy());
    assert!(
        a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading,
        "un 2e contrôle répond après le premier"
    );
}

/// 📱 Choix B de Jee (2026-11-09, 04h48) : Argos AI partout au même coin, feuilles et menus compris (la maquette
/// « Symboles » le garde par-dessus la watchlist). Remplace « Argos AI passe sous les feuilles ».
#[test]
fn cell_argos_ai_par_dessus_les_feuilles() {
    let mut a = cell_390();
    let r = a.ai_bouton_rect();
    assert!(r.w > 0.0);
    a.watch.prefs.open = true;
    let o = a.ai_bouton_rect();
    assert!(
        o.w == r.w && o.y == r.y,
        "watchlist ouverte : Argos AI reste au coin (sans fenêtre, la largeur du test change)"
    );
    a.watch.prefs.open = false;
    a.ui.ai_open = true;
    assert!(a.ai_bouton_rect().w > 0.0);
}

/// 📱 Une feuille ouverte (📈) : la barre du bas répond encore (même icône → fermée).
#[test]
fn cell_barre_du_bas_sous_une_feuille() {
    use argos_render::ecrans::CellHit;
    use argos_render::phonebar::Icone;
    let mut a = cell_390();
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let b = z.iter().find(|(_, k)| *k == CellHit::Icone(Icone::Trading)).unwrap().0;
    toucher(&mut a, b.cx(), b.cy());
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading, "📈 ouvre son volet");
    // feuille ouverte : la barre d'icônes répond encore ; la pilule du symbole et ƒx passent sous la feuille
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    assert!(!z.iter().any(|(_, k)| matches!(k, CellHit::Symbole | CellHit::Fx | CellHit::Ai)));
    let t = z.iter().find(|(_, k)| *k == CellHit::Icone(Icone::Trading)).unwrap().0;
    toucher(&mut a, t.cx(), t.cy());
    assert!(!a.watch.prefs.open, "même icône : fermée");
}

/// ⌨️ Jee (2026-11-09) : sur cell, le clavier du téléphone écrit dans la feuille Symbole (« Symbole, nom… ») ;
/// Effacer efface, Échap ferme.
#[test]
fn cell_clavier_dans_la_feuille_symbole() {
    let mut a = cell_390();
    a.ui.add = Some(argos_render::wlmenu::AddPanel::new(Rect::new(0.0, 300.0, 390.0, 400.0), a.add_chips()));
    a.paste_text("ETH".into());
    assert_eq!(a.ui.add.as_ref().unwrap().query, "ETH");
    a.soft_key("Backspace");
    assert_eq!(a.ui.add.as_ref().unwrap().query, "ET");
    a.soft_key("Escape");
    assert!(a.ui.add.is_none());
}

/// 🎡 Jee (vidéo, 2026-11-09) : le doigt glisse sur la pilule « symbole · unité » : la roue s'ouvre, la liste suit le
/// doigt (100 points vers le haut = 4 lignes plus loin), la chart change en direct ; relâcher garde la ligne. Un
/// simple toucher ouvre toujours la liste des unités.
#[test]
fn cell_roue_de_la_pilule() {
    use argos_render::ecrans::CellHit;
    use winit::event::TouchPhase;
    let mut a = cell_390();
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let u = z.iter().find(|(_, k)| *k == CellHit::Unite).unwrap().0;
    let mut s: Vec<i64> = argos_core::interval::SECTIONS
        .iter()
        .flat_map(|(_, l)| l.iter().copied())
        .filter(|&x| a.interval_ok(Interval(x)))
        .collect();
    s.sort_unstable();
    s.dedup();
    let depart = s.iter().position(|&x| x == a.chart.interval.secs()).unwrap();
    let (x, y) = (u.cx(), u.cy());
    a.touch(1, TouchPhase::Started, (x, y));
    for k in 1..=20 {
        a.touch(1, TouchPhase::Moved, (x, y - k as f64 * 5.0));
    }
    assert!(a.ui.roue.as_ref().is_some_and(|r| r.ouverte), "la roue est ouverte");
    assert_eq!(a.chart.interval.secs(), s[(depart + 4).min(s.len() - 1)], "en direct : 4 lignes plus loin");
    a.touch(1, TouchPhase::Ended, (x, y - 100.0));
    assert!(a.ui.roue.is_none(), "relâcher referme la roue");
    assert_eq!(a.chart.interval.secs(), s[(depart + 4).min(s.len() - 1)], "la ligne est gardée");
    // un simple toucher : la liste des unités, comme avant
    toucher(&mut a, x, y);
    assert!(a.ui.list.is_some() && a.ui.roue.is_none(), "toucher sans glisser : la liste");
}

/// 👆 Le premier toucher après l'ouverture n'est jamais perdu : si la page n'a pas encore dessiné sa première image,
/// celle-ci ne referme pas le volet que le doigt vient d'ouvrir.
#[test]
fn cell_premier_toucher_apres_chargement() {
    use argos_render::ecrans::CellHit;
    let mut a = cell_390();
    a.ui.depart_vu = false;
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let s = z.iter().find(|(_, k)| *k == CellHit::Icone(argos_render::phonebar::Icone::Trading)).unwrap().0;
    toucher(&mut a, s.cx(), s.cy());
    assert!(a.watch.prefs.open, "le toucher ouvre le volet Trading");
    a.premiere_image(true);
    assert!(a.watch.prefs.open, "la première image ne la referme pas");
    // sans doigt avant la première image : le volet restauré du PC se ferme toujours
    let mut b = cell_390();
    b.ui.depart_vu = false;
    b.watch.prefs.open = true;
    b.premiere_image(true);
    assert!(!b.watch.prefs.open);
}

/// 🏷️ Jee (photos, 2026-11-09) : sur cell, ⌄ montre les noms des indicateurs ; toucher un nom ouvre sa barre
/// 👁 ⚙ 🗑 ⋯ ; ⋯ ouvre le menu complet (Cacher, Retirer… ; le reste « bientôt »).
#[test]
fn cell_legende_des_indicateurs() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::{click_de, Cell, LegClick};
    let mut a = cell_390();
    let osc = *Ind::ALL.iter().find(|i| !i.params().1.is_empty()).expect("un indicateur à réglages");
    a.ind.on = vec![osc];
    a.ui.frames = 5;
    a.refresh_indicators();
    a.ind.leg.0 = true;
    let t = argos_render::text::Text::default();
    let (ph, pw) = (a.chart.viewport.plot_h(), a.chart.viewport.plot_w());
    let z = argos_render::indleg::zones_de(&t, &a.ind.on, &a.ind.out, true, ph, pw, Cell { actif: true, sel: None, pw });
    let (nom, _) = *z.iter().find(|(_, c)| *c == LegClick::Nom(osc)).expect("le nom du RSI");
    a.legend_click(click_de(&t, &a.ind.on, &a.ind.out, true, ph, pw, Cell { actif: true, sel: None, pw }, nom.cx(), nom.cy()).unwrap());
    assert_eq!(a.ind.sel, Some(Ok(osc)), "toucher le nom ouvre sa barre");
    let cell = Cell {
        actif: true,
        sel: a.ind.sel,
        pw,
    };
    let z = argos_render::indleg::zones_de(&t, &a.ind.on, &a.ind.out, true, ph, pw, cell);
    for c in [LegClick::Eye(osc), LegClick::Settings(osc), LegClick::Remove(osc), LegClick::Plus(osc)] {
        assert!(z.iter().any(|(_, x)| *x == c), "{c:?} dans la barre");
    }
    a.legend_click(LegClick::Plus(osc));
    assert!(a.ui.list.is_some(), "⋯ : le menu");
    let r = a.ui.list.as_ref().unwrap().1.rect;
    assert!(
        r.x >= 0.0 && r.x + r.w <= a.win_w() && r.w < 460.0,
        "le menu tient dans l'écran du cell : {r:?}"
    );
    // un toucher sur « Cacher », à sa place dans le menu
    let r = a.ui.list.as_ref().unwrap().1.rect;
    let mut trouve = false;
    let mut y = r.y + 4.0;
    while y < r.y + r.h && !trouve {
        let mut m = a.ui.list.as_ref().unwrap().1.clone();
        if m.click(r.x + 60.0, y) == argos_render::listmenu::MenuClick::Pick(9) {
            toucher(&mut a, r.x + 60.0, y);
            trouve = true;
        }
        y += 4.0;
    }
    assert!(trouve, "la ligne Cacher");
    assert!(a.ind.out.hidden.contains(&osc), "Cacher au doigt");
    assert!(
        a.ind_plus_menu().iter().flat_map(|g| &g.items).any(|i| i.id == 9 && i.label == "Afficher"),
        "caché : la ligne devient Afficher"
    );
    a.legend_click(LegClick::Eye(osc));
    a.ind_plus_pick(9);
    assert!(a.ind.out.hidden.contains(&osc), "Cacher");
    a.ind_plus_pick(10);
    assert!(!a.ind.on.contains(&osc) && a.ind.sel.is_none(), "Retirer");
}

/// 📟 Tablette (820 × 1180) : chaque bouton du dock répond au doigt ; ⋯ ouvre la petite feuille des autres
/// (🐋 🧩 🎓 👥 🛒, avec leurs noms) et chacune y fait comme son bouton (liste 2, point 21).
#[test]
fn tablette_dock_et_ses_autres() {
    use argos_render::ecrans::{CellHit, DOCK, DOCK_AUTRES};
    let tablette = || {
        let mut a = cell_390();
        a.area = (820, (1180.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
        a
    };
    for (i, _) in DOCK {
        let mut a = tablette();
        let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
        let r = z.iter().find(|(_, c)| *c == CellHit::Icone(i)).unwrap_or_else(|| panic!("{i:?} absent")).0;
        let avant = empreinte(&a);
        toucher(&mut a, r.cx(), r.cy());
        assert_ne!(empreinte(&a), avant, "{i:?} ne fait rien ({r:?})");
    }
    for (k, (i, _, nom)) in DOCK_AUTRES.iter().enumerate() {
        let mut a = tablette();
        let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
        let r = z.iter().find(|(_, c)| *c == CellHit::Autres).expect("⋯ du dock").0;
        toucher(&mut a, r.cx(), r.cy());
        let (kind, m) = a.ui.list.as_ref().expect("⋯ ouvre la petite feuille");
        assert_eq!(*kind, ListKind::DockAutres);
        assert!(m.groups[0].items[k].label.ends_with(nom), "{nom}");
        let r = m.rect;
        let mut y = r.y + 4.0;
        while y < r.y + r.h && m.clone().click(r.x + 60.0, y) != argos_render::listmenu::MenuClick::Pick(k as i64) {
            y += 4.0;
        }
        assert!(y < r.y + r.h, "la ligne {nom}");
        let avant = empreinte(&a);
        toucher(&mut a, r.x + 60.0, y);
        assert_ne!(empreinte(&a), avant, "{i:?} ne fait rien depuis ⋯");
    }
}

/// 👆 Coordinateur / Jee (2026-11-09) : après un toucher, l'icône touchée ne reste pas surlignée (le survol se relâche
/// au doigt levé, cell et tablette) ; la vraie souris le rallume.
#[test]
fn doigt_leve_pas_de_survol() {
    use winit::event::TouchPhase;
    let mut a = cell_390();
    a.touch(3, TouchPhase::Started, (60.0, 820.0));
    assert!(a.survol(Some((60.0, 20.0))).is_some(), "doigt posé : appui visible");
    a.touch(3, TouchPhase::Ended, (60.0, 820.0));
    assert!(a.survol(Some((60.0, 20.0))).is_none(), "doigt levé : plus de surlignage");
    a.pointer.doigt_leve = false; // (une vraie souris qui bouge)
    assert!(a.survol(Some((60.0, 20.0))).is_some());
}

/// 📅 Source du calendrier absente (HTTP 404, serveur local sans `/api`) : « bientôt », pas une erreur.
#[test]
fn agenda_absent_bientot() {
    assert_eq!(crate::events::agenda_note("HTTP 404"), "📅 Calendrier économique : bientôt");
    assert!(crate::events::agenda_note("délai dépassé").starts_with("⚠️"));
}

/// 📱 Feuille Trading du cell (maquette) : 📈 l'ouvre, ✕ la ferme, au doigt, du premier coup.
#[test]
fn cell_feuille_trading_au_doigt() {
    use argos_render::ecrans::CellHit;
    use argos_render::phonebar::Icone;
    let mut a = cell_390();
    a.ui.frames = 5;
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let r = z.iter().find(|(_, c)| *c == CellHit::Icone(Icone::Trading)).unwrap().0;
    toucher(&mut a, r.cx(), r.cy());
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading, "📈 ouvre la feuille");
    let top = a.feuille_volet();
    (a.ui.rows, a.watch.phone, a.watch.panels.trading.cell) = (Some((top, top)), Some(390), true); // (posés par le dessin)
    let haut = a.top_h() + top;
    toucher(&mut a, 360.0, haut + 53.0);
    assert!(!a.watch.prefs.open, "✕ ferme la feuille");
}

/// 🧪 Le bouton Partager pendant les tests : l'image va dans un dossier de test (rust/target/tmp), jamais dans les Téléchargements.
#[test]
fn les_tests_n_ecrivent_pas_dans_les_telechargements() {
    let d = crate::platform::dossier_telechargements();
    assert!(d.starts_with(argos_core::dossiers::racine_des_tests()), "{d:?}");
}

/// 🅘 Cell (maquette) : ƒx ouvre la page Indicateurs plein écran ; Classiques se déplie ; ＋ met RSI sur la chart ;
/// ✕ ferme — au doigt, à leur place.
#[test]
fn cell_page_indicateurs_au_doigt() {
    use argos_render::ecrans::CellHit;
    let mut a = cell_390();
    a.ui.frames = 5;
    a.ind.on.retain(|i| *i != argos_render::indicators::Ind::Rsi);
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let fx = z.iter().find(|(_, c)| *c == CellHit::Fx).unwrap().0;
    toucher(&mut a, fx.cx(), fx.cy());
    assert!(a.ui.fx_page.is_some(), "ƒx ouvre la page");
    assert_eq!(
        a.ui.fx_page.as_ref().unwrap().ouverte,
        None,
        "le toucher qui ouvre la page ne déplie rien dedans"
    );
    // Classiques : 4e catégorie (PERSONNEL, 3 lignes, INTÉGRÉS)
    let y = 121.7 + 25.0 + 3.0 * 55.0 + 32.9;
    toucher(&mut a, 120.0, y + 27.0);
    assert_eq!(a.ui.fx_page.as_ref().unwrap().ouverte, Some(3), "Classiques se déplie");
    toucher(&mut a, 390.0 - 17.0 - 41.0 + 16.0, y + 55.0 + 4.0 + 20.0);
    assert!(a.ind.on.contains(&argos_render::indicators::Ind::Rsi), "＋ met RSI sur la chart");
    let w = a.win_w(); // (sans fenêtre, le test compte aussi la largeur de la watchlist)
    toucher(&mut a, w - 32.0, 52.7);
    assert!(a.ui.fx_page.is_none(), "✕ ferme");
}

/// 🅘 Jee (9 nov. 2026) : chacun des 43 indicateurs de la page ƒx s'ajoute par ＋ (et se retire), et ☆ le met en favori.
#[test]
fn page_indicateurs_les_43_au_plus_et_a_l_etoile() {
    use argos_render::indicators::Ind;
    for i in Ind::ALL {
        let mut a = cell_390();
        a.ind.on.retain(|x| *x != i);
        a.ind.favs.retain(|x| *x != i);
        a.toggle_indicator(i);
        a.ui.ind_dlg = None; // (« Confirmer les inputs » de certains : fermé)
        assert!(a.ind.on.contains(&i), "{i:?} : ＋ l'ajoute");
        a.legend_click(argos_render::indleg::LegClick::Fav(i));
        assert!(a.ind.favs.contains(&i), "{i:?} : ☆ le met en favori");
        a.toggle_indicator(i);
        assert!(!a.ind.on.contains(&i), "{i:?} : ✓ le retire");
    }
}

/// ⚙️ Cell (modèle de Jee) : les réglages d'Ichimoku s'ouvrent en feuille plein écran ; toucher « Tenkan-sen »,
/// taper 7 au clavier du cell, Entrée : la valeur s'applique tout de suite à la chart.
#[test]
fn cell_reglages_ichimoku_au_clavier() {
    use argos_render::indicators::{values, Ind};
    let mut a = cell_390();
    a.ui.frames = 5;
    a.ind.on = vec![Ind::Ichimoku];
    a.open_ind_dlg(Ind::Ichimoku);
    let d = a.ui.ind_dlg.as_ref().unwrap();
    assert!(d.cell && d.rect.w == a.win_w(), "feuille plein écran");
    let text = argos_render::text::Text::default();
    let champ = d.champs_saisie(&text)[0];
    toucher(&mut a, champ.cx(), champ.cy());
    assert!(a.ui.ind_dlg.as_ref().unwrap().typing(), "le champ prend le clavier");
    a.soft_key("Backspace");
    a.paste_text("7".into());
    a.soft_key("Enter");
    assert_eq!(values(&a.ind.set, Ind::Ichimoku)[0], 7.0, "Tenkan-sen = 7, appliqué en direct");
}

/// 🔔 Cell (maquette) : 🔔 ouvre la feuille des notifications ; le filtre Baleines trie le fil ; toucher une baleine
/// ouvre le volet Baleines ; à côté de la feuille, elle se ferme — au doigt, à leur place.
#[test]
fn cell_feuille_notifications_au_doigt() {
    use argos_render::ecrans::CellHit;
    use argos_render::notifs::{NotifClick, Notifs};
    let mut a = cell_390();
    a.ui.frames = 5;
    a.alerts.history.push(argos_core::alerts::Fired {
        id: 1,
        ts: now_s() - 120,
        text: "BTC a franchi 76 000".into(),
    });
    a.watch.whales.list.push(argos_data::whales::Whale {
        coin: "BTC".into(),
        time_ms: (now_s() - 60) * 1000,
        price: 76_000.0,
        size: 30.0,
        buy: true,
        src: "Binance",
        id: None,
        wallet: String::new(),
    });
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let cloche = z.iter().find(|(_, c)| *c == CellHit::Alertes).unwrap().0;
    toucher(&mut a, cloche.cx(), cloche.cy());
    assert!(a.ui.notifs.is_some(), "🔔 ouvre la feuille");
    // (the feed of before: ⚙ « Fil des notifications » of the alerts panel)
    a.ui.notifs.as_mut().unwrap().fil = true;
    let ev = a.notif_evenements();
    assert_eq!(
        ev.iter().filter(|e| e.sorte == 1 || e.sorte == 3).count(),
        2,
        "l'alerte et la baleine sont dans le fil"
    );
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let t = argos_render::text::Text::default();
    let r = Notifs::rect(w, h);
    let n = a.ui.notifs.clone().unwrap();
    let b = n.zones(&t, r, &ev).into_iter().find(|(_, c)| *c == NotifClick::Bientot("Baleines")).unwrap().0;
    toucher(&mut a, b.cx(), b.cy());
    assert_eq!(a.ui.notifs.as_ref().unwrap().filtre, 3, "filtre Baleines");
    let n = a.ui.notifs.clone().unwrap();
    let e = n.zones(&t, r, &ev).into_iter().find(|(_, c)| matches!(c, NotifClick::Evenement(_))).unwrap().0;
    toucher(&mut a, e.cx(), e.cy());
    assert!(a.ui.notifs.is_none() && a.watch.prefs.open, "la baleine ouvre son volet");
    assert_eq!(a.watch.prefs.panel, argos_render::watchlist::Tool::Whales);
    a.watch.prefs.open = false;
    toucher(&mut a, cloche.cx(), cloche.cy());
    assert!(a.ui.notifs.is_some());
    toucher(&mut a, 200.0, r.y - 60.0);
    assert!(a.ui.notifs.is_none(), "à côté, elle se ferme");
}

/// 🔔 PC (maquette `#alertes`) : 🔔 ouvre la fenêtre flottante des notifications sous le bouton (fil, filtres) ;
/// « + Nouvelle alerte » ouvre le menu des alertes d'avant ; un clic à côté la ferme.
#[test]
fn pc_cloche_ouvre_la_fenetre_des_notifications() {
    use argos_render::notifs::NotifClick;
    dossier_a_soi("cloche-volet");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    a.ui.frames = 5;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    let bouton = Rect::new(260.0, 12.0, 32.0, 36.0);
    a.notifs_pop(bouton);
    a.placer(taille);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Alertes, "🔔 ouvre le volet");
    // (the feed of before: ⚙ « Fil des notifications » of the alerts panel)
    a.ui.notifs.as_mut().unwrap().fil = true;
    let n = a.ui.notifs.clone().expect("🔔 ouvre le volet");
    assert!(n.volet);
    let ev = a.notif_evenements();
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let r = n.place(w, h, &ev);
    assert_eq!(Some(r), a.volet_rect(), "à la place et à la taille du volet flottant");
    assert!(a.rw() + a.chart.viewport.width as f64 <= r.x + 0.5, "la chart s'arrête à son bord");
    // un clic à côté (sur la chart) : le volet reste
    clic(&mut a, r.x - 300.0, r.y + 300.0);
    assert!(a.ui.notifs.is_some() && a.watch.prefs.open, "à côté : il reste ouvert");
    let t = argos_render::text::Text::default();
    let na = n.zones(&t, r, &ev).into_iter().find(|(_, c)| *c == NotifClick::NouvelleAlerte).unwrap().0;
    clic(&mut a, na.cx(), na.cy());
    assert!(a.ui.notifs.is_none() && a.ui.list.is_some(), "« + Nouvelle alerte » : le menu des alertes");
    a.draw();
    assert!(!a.watch.prefs.open, "le volet 🔔 vide se ferme");
    a.ui.list = None;
    // ✕ ferme le volet
    a.notifs_pop(bouton);
    a.placer(taille);
    let r = a.ui.notifs.as_ref().unwrap().pop.unwrap();
    clic(&mut a, r.x + r.w - 29.0, r.y + 25.0);
    assert!(a.ui.notifs.is_none() && !a.watch.prefs.open, "✕ : fermé");
    // le panneau 🔔 s'enregistre comme Watchlist (l'ancienne /app/ ne connaît pas « Alertes »)
    let mut p = a.watch.prefs.clone();
    p.panel = wl::Tool::Alertes;
    let j = serde_json::to_string(&p).unwrap();
    assert!(j.contains("\"panel\":\"Watchlist\""), "{j}");
}

/// ▦ Cell (maquette) : ▦ ouvre la feuille Splits en bas (verre, toute la largeur) ; toucher une disposition la
/// prend, toucher Symbole allume la synchro ; la feuille reste ouverte pour comparer ; au-dessus, elle se ferme ; Argos
/// AI reste par-dessus — au doigt, à leur place.
#[test]
fn cell_feuille_splits_au_doigt() {
    use argos_render::ecrans::CellHit;
    use argos_render::splits::Sync;
    let mut a = cell_390();
    a.ui.frames = 5;
    a.split = Default::default();
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let b = z.iter().find(|(_, c)| *c == CellHit::Disposition).unwrap().0;
    toucher(&mut a, b.cx(), b.cy());
    let m = a.ui.split_menu.clone().expect("▦ ouvre la feuille");
    let h = a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h();
    assert!(m.cell && m.rect.x == 0.0 && m.rect.w == a.win_w(), "feuille du bas, toute la largeur");
    assert!(
        (m.rect.y + m.rect.h - h).abs() < 0.5 && m.rect.h <= h * 0.8,
        "collée en bas, la hauteur de son contenu"
    );
    let q = m.layout_rect("4g").unwrap();
    toucher(&mut a, q.cx(), q.cy());
    assert_eq!(a.split.layout, "4g", "la disposition 2 × 2 est prise");
    assert!(a.ui.split_menu.is_some(), "la feuille reste ouverte");
    assert!(m.layout_rect("16g").is_none(), "au doigt : 1 à 4 cases seulement");
    // « Synchro › » ouvre le sous-menu, « ‹ Synchro » revient
    let q = m.synchro_rect(true).unwrap();
    toucher(&mut a, q.cx(), q.cy());
    assert!(a.ui.split_menu.as_ref().is_some_and(|m| m.synchro), "Synchro › ouvre le sous-menu");
    let s = a.ui.split_menu.as_ref().unwrap().sync_rect(Sync::Symbol).unwrap();
    toucher(&mut a, s.cx(), s.cy());
    assert!(a.split.sync_symbol, "Symbole : ON");
    toucher(&mut a, s.cx(), s.cy());
    assert!(!a.split.sync_symbol, "Symbole : OFF");
    let q = a.ui.split_menu.as_ref().unwrap().synchro_rect(false).unwrap();
    toucher(&mut a, q.cx(), q.cy());
    assert!(a.ui.split_menu.as_ref().is_some_and(|m| !m.synchro), "‹ Synchro revient aux cases");
    toucher(&mut a, 200.0, m.rect.y - 60.0);
    assert!(a.ui.split_menu.is_none(), "au-dessus, elle se ferme");
    // 🪟 Multi-chart de ⚙ Panneaux : la même feuille
    assert!(argos_render::sheet::Act::MultiChart.ready(), "Multi-chart n'est plus « bientôt »");
    a.sheet_act(argos_render::sheet::Act::MultiChart);
    assert!(a.ui.split_menu.as_ref().is_some_and(|m| m.cell), "⚙ Multi-chart ouvre la feuille Splits");
    a.split = Default::default();
    a.ui.split_menu = None;
}

/// 🪟 Cell, 4 cases : le titre de la case du haut à droite (symbole, ⇄, 🔗 Liée / Libre) passe sous la pilule Flux
/// de la maquette au lieu de la chevaucher ; il se touche à sa nouvelle place. Les cases du bas ne bougent pas.
#[test]
fn cell_cases_du_haut_sous_les_pilules() {
    use argos_render::ecrans::CellHit;
    let mut a = cell_390();
    a.split = Default::default();
    a.split.layout = "4g".into();
    a.sync_panes();
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let flux = z.iter().find(|(_, c)| *c == CellHit::Flux).unwrap().0;
    let dy = a.pane_dy(0);
    assert!(dy > 0.0 && a.pane_dy(1) == 0.0 && a.pane_dy(2) == 0.0, "seule la case du haut descend : {dy}");
    let (name, title) = a.pane_title(0);
    let (_, _, swap, lien) = App::pane_title_zones(&title, &name, false, dy);
    let x0 = a.area.0 as f64 / 2.0;
    for q in [swap, lien] {
        let fenetre = Rect::new(x0 + q.x, a.top_h() + q.y, q.w, q.h);
        let touche = fenetre.x < flux.x + flux.w && flux.x < fenetre.x + fenetre.w && fenetre.y < flux.y + flux.h && flux.y < fenetre.y + fenetre.h;
        assert!(!touche && fenetre.y >= flux.y + flux.h, "{fenetre:?} chevauche Flux {flux:?}");
    }
    assert!(a.pane_click(x0 + lien.cx(), lien.cy()), "« Libre » se touche à sa place");
    assert_eq!(a.split.liees.first(), Some(&true), "la case est liée");
    a.split = Default::default();
}

// — 🧮 orderflow réel —

fn trade_hl(coin: &str, time_ms: i64, price: f64, size: f64, buy: bool) -> argos_data::whales::Whale {
    argos_data::whales::Whale {
        coin: coin.into(),
        time_ms,
        price,
        size,
        buy,
        src: "Hyperliquid",
        id: None,
        wallet: String::new(),
    }
}

/// ⚡ The Trades module reads the exchange of the chart (its orderflow connection), never mixed with Hyperliquid;
/// when that exchange has no trades flow (not connected, or its flow is unavailable), Hyperliquid, announced.
#[test]
fn trades_suit_la_bourse_de_la_chart() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "SOL".into(),
    };
    a.orderflow_suivre();
    assert_eq!(a.tape_source(), (Exchange::Binance, None));
    let t0 = now_s() * 1000;
    let bn = |n: u64, coin: &str| argos_data::whales::Whale {
        id: Some(n),
        src: "Binance",
        ..trade_hl(coin, t0 + n as i64, 150.0, 1.0, true)
    };
    a.on_user_event(UserEvent::Orderflow(Exchange::Binance, LiveEvent::Whales(vec![bn(1, "SOL"), bn(2, "BTC")])));
    a.feeds.flow_coin = Some("SOL".into());
    a.on_user_event(UserEvent::Flow(LiveEvent::Tape(vec![trade_hl("SOL", t0, 151.0, 2.0, false)])));
    let vus = |a: &App| a.tape_vue().iter().map(|t| (t.src, t.coin.clone())).collect::<Vec<_>>();
    assert_eq!(vus(&a), vec![("Binance", "SOL".to_string())], "only Binance SOL");
    // its flow does not open: Hyperliquid, said in the module; the Binance trades are not shown under its name
    a.on_user_event(UserEvent::Orderflow(Exchange::Binance, LiveEvent::Indisponible("refusé".into())));
    assert_eq!(a.tape_source(), (Exchange::Hyperliquid, Some(Exchange::Binance)));
    assert!(a.tape_vue().is_empty());
    a.on_user_event(UserEvent::Flow(LiveEvent::Tape(vec![trade_hl("SOL", t0, 151.0, 2.0, false)])));
    assert_eq!(vus(&a), vec![("Hyperliquid", "SOL".to_string())]);
    // an exchange whose trades cannot be read for this coin (contract size unknown): the fallback too
    let sans = Exchange::ALL
        .iter()
        .copied()
        .find(|e| *e != Exchange::Hyperliquid && !argos_data::orderflow::reel_pour(*e, "ZZQ"))
        .unwrap();
    a.chart.source = Source::Live {
        exchange: sans,
        base: "ZZQ".into(),
    };
    assert_eq!(a.tape_source(), (Exchange::Hyperliquid, Some(sans)));
    effacer_series(&["binance-sol."]);
}

/// 🔁 ⚡ A stream that is cut and comes back gives its last trades again: the Trades module lists each of them once,
/// with the ones missed during the cut. By the trade's number when the exchange gives one, by its moment otherwise,
/// and for the first lot after the cut only: two real trades can be the same, they are never compared one to one.
#[test]
fn trades_redonnes_apres_une_coupure_listes_une_fois() {
    use argos_data::whales::Whale;
    let mut a = app();
    let t0 = now_s() * 1000;
    let cut = || LiveEvent::Disconnected("essai".into());
    // an exchange that numbers its trades (Binance)
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "ADA".into(),
    };
    assert_eq!(a.tape_source(), (Exchange::Binance, None));
    let bn = |n: u64| Whale {
        id: Some(n),
        src: "Binance",
        ..trade_hl("ADA", t0 + n as i64, 0.75, 100.0, true)
    };
    let binance = |a: &mut App, ev: LiveEvent| a.on_user_event(UserEvent::Orderflow(Exchange::Binance, ev));
    let numbers = |a: &App| a.tape_vue().iter().filter_map(|t| t.id).collect::<Vec<_>>();
    binance(&mut a, LiveEvent::Connected);
    binance(&mut a, LiveEvent::Whales(vec![bn(1), bn(2), bn(3)]));
    assert_eq!(numbers(&a), [3, 2, 1]);
    binance(&mut a, cut());
    binance(&mut a, LiveEvent::Connected);
    // (a lot of another coin comes first: it is not the first lot of this list)
    binance(&mut a, LiveEvent::Whales(vec![Whale { coin: "BTC".into(), ..bn(9) }]));
    // 2 and 3 given again, with 4 and 5 that were missed during the cut
    binance(&mut a, LiveEvent::Whales(vec![bn(5), bn(4), bn(3), bn(2)]));
    assert_eq!(numbers(&a), [5, 4, 3, 2, 1], "each trade once, the ones missed during the cut too");
    binance(&mut a, LiveEvent::Whales(vec![bn(6)]));
    assert_eq!(numbers(&a), [6, 5, 4, 3, 2, 1], "then the stream goes on");
    // a stream that cannot open, then opens: the same
    binance(&mut a, LiveEvent::Indisponible("essai".into()));
    assert!(a.tape_vue().is_empty(), "(its trades are not shown under Hyperliquid's name)");
    binance(&mut a, LiveEvent::Connected);
    binance(&mut a, LiveEvent::Whales(vec![bn(7), bn(6), bn(5)]));
    assert_eq!(numbers(&a), [7, 6, 5, 4, 3, 2, 1]);

    // an exchange without numbers (Hyperliquid): by the moment
    chart_hl(&mut a, "BTC");
    a.feeds.flow_coin = Some("BTC".into());
    let hl = |ms: i64, size: f64| trade_hl("BTC", t0 + ms, 90_000.0, size, true);
    let flow = |a: &mut App, ev: LiveEvent| a.on_user_event(UserEvent::Flow(ev));
    let sizes = |a: &App| a.tape_vue().iter().map(|t| t.size).collect::<Vec<_>>();
    flow(&mut a, LiveEvent::Connected);
    flow(&mut a, LiveEvent::Tape(vec![hl(10, 1.0), hl(20, 2.0), hl(20, 2.0)]));
    assert_eq!(
        sizes(&a),
        [2.0, 2.0, 1.0],
        "a new source starts its list; two real trades that are the same are both listed"
    );
    flow(&mut a, cut());
    flow(&mut a, LiveEvent::Connected);
    flow(&mut a, LiveEvent::Tape(vec![hl(10, 1.0), hl(20, 2.0), hl(20, 2.0), hl(30, 3.0), hl(30, 3.0)]));
    assert_eq!(
        sizes(&a),
        [3.0, 3.0, 2.0, 2.0, 1.0],
        "the three given again are not listed a second time, the two new ones are"
    );
    // the stream of another exchange is cut: this list is not concerned, its next lot is added whole
    binance(&mut a, cut());
    binance(&mut a, LiveEvent::Connected);
    flow(&mut a, LiveEvent::Tape(vec![hl(30, 3.0)]));
    assert_eq!(
        sizes(&a),
        [3.0, 3.0, 3.0, 2.0, 2.0, 1.0],
        "a lot that is not the first after a cut is added whole"
    );
    // cut again, and the first lot gives nothing new: nothing is added, and the lot after it is added whole
    flow(&mut a, cut());
    flow(&mut a, LiveEvent::Connected);
    flow(&mut a, LiveEvent::Tape(vec![hl(30, 3.0), hl(20, 2.0)]));
    assert_eq!(sizes(&a).len(), 6);
    flow(&mut a, LiveEvent::Tape(vec![hl(40, 4.0)]));
    assert_eq!(sizes(&a), [4.0, 3.0, 3.0, 3.0, 2.0, 2.0, 1.0]);
}

fn chart_hl(a: &mut App, base: &str) {
    a.chart.source = Source::Live {
        exchange: Exchange::Hyperliquid,
        base: base.into(),
    };
}

/// Retire du cache des tests les fichiers de ces séries (les autres tests tournent en même temps dans le même dossier).
fn effacer_series(prefixes: &[&str]) {
    let Some(d) = crate::actions::orderflow::store().map(|s| s.dir) else { return };
    for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
        if prefixes.iter().any(|p| e.file_name().to_string_lossy().starts_with(p)) {
            let _ = std::fs::remove_file(e.path());
        }
    }
    // le dossier des tests s'en va quand le dernier test a fini (il reste tant qu'un autre y écrit)
    let _ = std::fs::remove_dir(&d);
}

fn suivi<'a>(a: &'a App, s: &str) -> Option<&'a crate::actions::orderflow::FluxReel> {
    a.feeds.orderflow.iter().find(|f| f.cle.symbole == s)
}

#[test]
fn orderflow_reel_du_marche_hyperliquid_de_la_chart() {
    let mut a = app();
    a.orderflow_suivre();
    assert!(a.feeds.orderflow.is_empty(), "chart d'un fichier : rien à suivre");
    assert_eq!(a.orderflow_enregistre(), None, "pas de bouton ● Enregistrer");
    chart_hl(&mut a, "BTC");
    a.orderflow_suivre();
    assert!(suivi(&a, "BTC").is_some());
    // des trades de la minute en cours : seulement ceux du BTC sont rangés, le Tape garde tout comme avant
    let t0 = now_s() / 60 * 60_000;
    a.on_user_event(UserEvent::Flow(LiveEvent::Tape(vec![
        trade_hl("BTC", t0, 90_000.0, 2.0, true),
        trade_hl("BTC", t0 + 1, 90_005.0, 0.5, false),
        trade_hl("ETH", t0, 3_000.0, 9.0, true),
    ])));
    a.orderflow_suivre();
    assert!(suivi(&a, "BTC").unwrap().minutes.is_empty(), "minute en cours : pas encore fermée");
    a.orderflow_suivre_a(t0 + 3 * 60_000); // 3 min plus tard
    let f = suivi(&a, "BTC").unwrap();
    assert_eq!(f.minutes.len(), 1, "minute fermée, rangée et gardée");
    assert_eq!(
        (f.minutes[0].time, f.minutes[0].delta(), f.minutes[0].complete),
        (t0 / 1000, 1.5, false),
        "démarré pendant cette minute : incomplète"
    );
    // flux coupé : la minute en cours devient incomplète
    a.on_user_event(UserEvent::Flow(LiveEvent::Disconnected("essai".into())));
    assert!(suivi(&a, "BTC").unwrap().agg.en_cours().iter().all(|b| !b.complete));
    // autre chart puis retour : l'orderflow est relu du cache
    a.chart.source = Source::File("essai.json".into());
    a.orderflow_suivre();
    assert!(a.feeds.orderflow.is_empty());
    chart_hl(&mut a, "BTC");
    a.orderflow_suivre();
    assert_eq!(suivi(&a, "BTC").and_then(|f| f.minutes.first()).map(|b| b.delta()), Some(1.5), "relu du cache");
    effacer_series(&["hl-btc."]);
}

#[test]
fn orderflow_enregistre_en_continu_un_marche_non_affiche() {
    let mut a = app();
    chart_hl(&mut a, "SOL");
    a.orderflow_basculer();
    assert_eq!(a.orderflow_enregistre(), Some(true));
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("● SOL (Hyperliquid) enregistré")));
    // relu au redémarrage (orderflow.json)
    let relu: argos_core::orderflow::Enregistres = App::read_prefs("orderflow.json");
    assert_eq!(relu, a.feeds.enregistres);
    // la chart passe à l'ETH : le SOL reste suivi et ses trades sont rangés et enregistrés
    chart_hl(&mut a, "ETH");
    a.orderflow_suivre();
    assert!(suivi(&a, "SOL").is_some() && suivi(&a, "ETH").is_some());
    let t0 = now_s() / 60 * 60_000;
    a.on_user_event(UserEvent::Flow(LiveEvent::Tape(vec![trade_hl("SOL", t0, 150.0, 4.0, false)])));
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    assert_eq!(suivi(&a, "SOL").unwrap().minutes.iter().map(|b| b.delta()).collect::<Vec<_>>(), vec![-4.0]);
    let cle = suivi(&a, "SOL").unwrap().cle.clone();
    let store = crate::actions::orderflow::store().unwrap();
    use argos_core::orderflow::FlowStore;
    assert_eq!(store.lire(&cle, 0, i64::MAX).len(), 1, "enregistré dans le cache");
    // retiré de la liste (depuis sa chart) : plus suivi quand il n'est plus affiché ; ses données restent
    chart_hl(&mut a, "SOL");
    a.orderflow_basculer();
    assert_eq!(a.orderflow_enregistre(), Some(false));
    chart_hl(&mut a, "ETH");
    a.orderflow_suivre();
    assert!(suivi(&a, "SOL").is_none());
    assert_eq!(store.lire(&cle, 0, i64::MAX).len(), 1, "données gardées");
    // liste pleine : message clair
    for s in ["A", "B", "C", "D", "E"] {
        chart_hl(&mut a, s);
        a.orderflow_basculer();
    }
    chart_hl(&mut a, "F");
    a.orderflow_basculer();
    assert_eq!(a.banner.as_deref(), Some("● 5 symboles au plus : retire-en un"));
    assert_eq!(a.feeds.enregistres.liste.len(), 5);
    effacer_series(&["hl-sol.", "hl-eth."]);
}

#[test]
fn orderflow_reel_binance_sur_son_propre_flux() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "XRP".into(),
    };
    a.orderflow_suivre();
    assert_eq!(
        a.feeds
            .orderflow
            .iter()
            .map(|f| (f.cle.exchange.as_str(), f.cle.symbole.as_str()))
            .collect::<Vec<_>>(),
        vec![("binance", "XRP")]
    );
    assert_eq!(a.orderflow_enregistre(), Some(false), "bouton ● Enregistrer aussi pour Binance");
    let t0 = now_s() / 60 * 60_000;
    let bn = |id: u64, size: f64, buy: bool| argos_data::whales::Whale {
        id: Some(id),
        src: "Binance",
        ..trade_hl("XRP", t0, 2.5, size, buy)
    };
    // les trades d'Hyperliquid (flux du carnet) ne vont pas dans le XRP de Binance
    a.on_user_event(UserEvent::Flow(LiveEvent::Tape(vec![trade_hl("XRP", t0, 2.5, 1_000.0, true)])));
    a.on_user_event(UserEvent::Orderflow(
        Exchange::Binance,
        LiveEvent::Whales(vec![bn(7, 10.0, true), bn(8, 4.0, false)]),
    ));
    // reconnexion : Binance redonne le trade 8, il n'est pas recompté
    a.on_user_event(UserEvent::Orderflow(Exchange::Binance, LiveEvent::Connected));
    a.on_user_event(UserEvent::Orderflow(
        Exchange::Binance,
        LiveEvent::Whales(vec![bn(8, 4.0, false), bn(9, 1.0, true)]),
    ));
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    let f = &a.feeds.orderflow[0];
    assert_eq!(
        f.minutes.iter().map(|b| (b.delta(), b.complete)).collect::<Vec<_>>(),
        vec![(10.0 - 4.0 + 1.0, false)]
    );
    effacer_series(&["binance-xrp."]);
}

// — 🖌 pinceau de zone —

/// 🖌 Le module Pinceau de zone (entiers, dans la bulle) donne le même profil que le profil de volume intégré d'Argos
/// (`classic::volume_profile`) et le même delta estimé (`flow::delta_estime`) sur les mêmes bougies. Écarts permis :
/// l'arrondi de sortie (une demi-unité de la dernière décimale) ; le profil, lui, a les mêmes rangées (POC et zone de
/// valeur à la même place : les parts arrondies au millionième ne changent pas l'ordre des rangées de ces bougies).
#[test]
fn module_pinceau_egal_au_profil_integre() {
    use argos_module_host::{engine::Engine, input_of, sha384, Manifest, RawBar};
    use argos_script::indicators::{classic, flow};
    let raw: Vec<RawBar> = (0..120)
        .map(|i| {
            let t = i as f64;
            let c = ((100.0 + 6.0 * (t / 9.0).sin() + t * 0.05) * 100.0).round() / 100.0;
            let o = ((c - 0.4 * (t / 4.0).cos()) * 100.0).round() / 100.0;
            (
                1_700_000_000 + i as i64 * 3600,
                o,
                o.max(c) + 0.75,
                o.min(c) - 0.5,
                c,
                500.0 + (i * 37 % 400) as f64,
            )
        })
        .collect();
    let m = Manifest::parse(crate::actions::modules::PINCEAU_JSON).unwrap();
    m.check().unwrap();
    let wasm = crate::actions::modules::PINCEAU_WASM;
    argos_module_host::scan(wasm).unwrap();
    // zone = toutes les bougies, haut / bas = leurs extrêmes (réglages à 0) ; 30 rangées, 68 % comme le profil intégré
    let input = input_of("TEST", 3600, &raw, vec![0, 0, 0, 0, 30, 68]);
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/tmp")
        .join(format!("pinceau-{}", std::process::id()));
    let out = Engine::new()
        .unwrap()
        .run(&m, wasm, &sha384(wasm), &[], &dir, &[], &input)
        .expect("appel dans la bulle");
    let _ = std::fs::remove_dir_all(&dir); // le dossier du module (vide) ne reste pas
    let carte = |nom: &str| {
        out.cards
            .iter()
            .find(|c| c.name == nom)
            .map(|c| c.value as f64 / 10f64.powi(c.scale as i32))
            .unwrap()
    };
    // les mêmes bougies pour l'app (les prix et volumes entiers du module, remis en nombres)
    let (pk, vk) = (10f64.powi(input.price_scale as i32), 10f64.powi(input.volume_scale as i32));
    let bars: Vec<Bar> = input
        .bars
        .iter()
        .map(|b| Bar {
            time: b.t,
            open: b.o as f64 / pk,
            high: b.h as f64 / pk,
            low: b.l as f64 / pk,
            close: b.c as f64 / pk,
            volume: b.v as f64 / vk,
        })
        .collect();
    let mut vals = argos_script::inputs::defaults(&argos_script::v1_inputs::PROFILE_PARAMS);
    for (k, v) in [("bars", 120.0), ("rows", 30.0), ("va", 68.0)] {
        let i = argos_script::v1_inputs::PROFILE_PARAMS.iter().position(|p| p.key == k).unwrap();
        vals[i] = v;
    }
    let p = classic::volume_profile(&bars, &vals).unwrap();
    let tol = 0.5 * 10f64.powi(-(input.price_scale as i32 + 2)) + 1e-9;
    let pres = |module: f64, app: f64, nom: &str| assert!((module - app).abs() <= tol, "{nom} : module {module} / app {app}");
    pres(carte("POC"), p.bot + (p.poc as f64 + 0.5) * p.step, "POC");
    pres(carte("VAH"), p.bot + (p.va_hi as f64 + 1.0) * p.step, "VAH");
    pres(carte("VAL"), p.bot + p.va_lo as f64 * p.step, "VAL");
    let volume: f64 = bars.iter().map(|b| b.volume).sum();
    let delta: f64 = bars.iter().map(flow::delta_estime).sum();
    let tol_v = 120.0 * 30.0 * 0.5e-6 + 1e-9;
    assert!((carte("Volume de la zone") - volume).abs() <= tol_v, "volume");
    assert!((carte("Delta (estimé)") - delta).abs() <= 120.0 * 0.5e-6 + 1e-9, "delta estimé");
    let vwap = bars.iter().map(|b| (b.high + b.low + b.close) / 3.0 * b.volume).sum::<f64>() / volume;
    pres(carte("VWAP"), vwap, "VWAP");
}

/// 🖌 Clic droit sur un rectangle → « Pinceau de zone » : le module est installé, reçoit la zone et démarre ; ses
/// courbes ne sont dessinées que sur les bougies de la zone.
#[test]
fn pinceau_de_zone_sur_un_rectangle() {
    use argos_render::drawings::{Drawing, Point};
    let mut a = app();
    let (t0, t1) = (a.chart.bars[10].time, a.chart.bars[30].time);
    let (lo, hi) = a.chart.bars[10..=30]
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), b| (l.min(b.low), h.max(b.high)));
    a.chart
        .drawings
        .push(Drawing::new("rect", vec![Point { t: t0, p: hi }, Point { t: t1, p: lo }]));
    a.obj_action(0, ObjAction::Pinceau);
    let e = a.mods.list.iter().find(|e| e.id == "trading/pinceau-zone").expect("installé");
    assert!(e.on, "démarré");
    assert_eq!((e.values["debut"], e.values["fin"]), (t0 as f64, t1 as f64));
    assert_eq!((e.values["haut"], e.values["bas"]), ((hi * 1e6).round(), (lo * 1e6).round()));
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("🖌 Pinceau de zone")));
    let views = a.run_modules();
    let v = views.iter().find(|v| v.name.contains("pinceau-zone")).expect("dessin du module");
    let poc = &v.out.plots[0].values;
    assert_eq!(poc.len(), a.chart.bars.len());
    assert!(poc[..10].iter().chain(&poc[31..]).all(|x| x.is_nan()), "rien hors de la zone");
    assert!(poc[10..=30].iter().all(|x| x.is_finite() && *x == poc[10]), "POC plat dans la zone");
}

/// 📱 🖌 Au doigt sur le cell (390 × 844) : appui long sur un rectangle → son menu → toucher « 🖌 Pinceau de zone » →
/// le module tourne (courbes dans la zone) ; ses 6 chiffres sont dans la page 🧩 Modules.
#[test]
fn cell_pinceau_de_zone_au_doigt() {
    use argos_render::drawings::{Drawing, Point};
    use winit::event::TouchPhase;
    let mut a = cell_390();
    a.draw(); // l'écran du cell est dessiné une fois, comme dans l'app
              // sur cell, la chart prend la largeur de la fenêtre du téléphone : 390 (les tests n'ont pas de vraie fenêtre)
    a.chart.viewport.width = a.area.0;
    // une zone de bougies bien visibles au milieu de la chart du cell
    let w = a.chart.viewport.plot_w();
    let vues: Vec<usize> = {
        let f = a.frame_now().unwrap();
        let g = f.geo();
        (0..a.chart.bars.len())
            .filter(|&i| (w * 0.3..w * 0.7).contains(&g.x_of(a.chart.bars[i].time)))
            .collect()
    };
    let (i0, i1) = (vues[0], *vues.last().unwrap());
    assert!(i1 > i0 + 2, "plusieurs bougies dans la zone : {vues:?}");
    let (t0, t1) = (a.chart.bars[i0].time, a.chart.bars[i1].time);
    let (lo, hi) = a.chart.bars[i0..=i1]
        .iter()
        .fold((f64::MAX, f64::MIN), |(l, h), b| (l.min(b.low), h.max(b.high)));
    a.chart
        .drawings
        .push(Drawing::new("rect", vec![Point { t: t0, p: hi }, Point { t: t1, p: lo }]));
    // le doigt se pose sur un bord visible du rectangle : pas sous la barre d'outils flottante du cell
    let text = argos_render::text::Text::default();
    let bords: Vec<Point> = (0..=8)
        .flat_map(|k| {
            let (p, t) = (lo + (hi - lo) * k as f64 / 8.0, t0 + (t1 - t0) * k / 8);
            [Point { t: t0, p }, Point { t: t1, p }, Point { t, p: lo }, Point { t, p: hi }]
        })
        .collect();
    let (x, y) = bords
        .iter()
        .map(|q| a.frame_now().unwrap().geo().xy(q))
        .find(|&(x, y)| a.drawing_at(x, y) == Some(0) && !argos_render::toolbar::contains(&a.tools, &text, x, y))
        .expect("un bord du rectangle visible au doigt");
    let (wx, wy) = (x + a.rw(), y + a.top_h());
    // appui long : le doigt reste posé plus de LONG_MS, puis la fenêtre se réveille (comme l'app)
    a.touch(3, TouchPhase::Started, (wx, wy));
    std::thread::sleep(std::time::Duration::from_millis(crate::input::touch::LONG_MS as u64 + 100));
    a.touch_tick();
    a.touch(3, TouchPhase::Ended, (wx, wy));
    // (Jee, 17 Nov: the long press on a drawing shows its bar; its ⋯ has the rest of the drawing's menu)
    assert_eq!(a.selected, Some(0), "le rectangle choisi : sa barre");
    assert!(a.ui.list.is_none() && a.ui.croix.is_none(), "ni menu ni croix");
    a.open_list(ListKind::DrawBar(0, argos_render::drawbar::DbBtn::More), Rect::new(20.0, 200.0, 40.0, 40.0));
    let r = a.ui.list.as_ref().unwrap().1.rect;
    assert!(r.x >= 0.0 && r.x + r.w <= a.win_w(), "le menu tient dans l'écran du cell : {r:?}");
    // la ligne « 🖌 Pinceau de zone », touchée à sa place dans le ⋯ de la barre
    let id = 100 + argos_render::objmenu::ObjAction::Pinceau as i64;
    let mut y = r.y + 4.0;
    let mut trouve = false;
    while y < r.y + r.h && !trouve {
        let mut m = a.ui.list.as_ref().unwrap().1.clone();
        if m.click(r.x + 60.0, y) == argos_render::listmenu::MenuClick::Pick(id) {
            toucher(&mut a, r.x + 60.0, y);
            trouve = true;
        }
        y += 4.0;
    }
    assert!(trouve, "la ligne 🖌 Pinceau de zone");
    assert!(a.mods.list.iter().any(|e| e.id == "trading/pinceau-zone" && e.on), "démarré au doigt");
    let views = a.run_modules();
    let poc = &views.iter().find(|v| v.name.contains("pinceau-zone")).unwrap().out.plots[0].values;
    assert!(poc[i0..=i1].iter().all(|v| v.is_finite()), "les courbes dans la zone");
    // les 6 chiffres : dans la page 🧩 Modules (fiche du module)
    a.mods.open = Some("trading/pinceau-zone".into());
    let lignes: Vec<String> = a.modules_menu().iter().flat_map(|g| g.items.iter().map(|i| i.label.clone())).collect();
    for nom in ["Volume de la zone", "Delta (estimé)", "POC", "VAH", "VAL", "VWAP"] {
        assert!(
            lignes.iter().any(|l| l.starts_with(&format!("📈 {nom} · "))),
            "{nom} dans 🧩 Modules : {lignes:?}"
        );
    }
}

/// 📱 Cell (390 × 844) : un dessin caché sous la barre d'outils flottante. Un appui long sur sa partie cachée ouvre le
/// menu du dessin, pas celui de la barre ; sans dessin dessous, l'appui long sur la barre garde le menu de la barre.
#[test]
fn cell_appui_long_dessin_sous_la_barre() {
    use argos_render::drawings::Drawing;
    use winit::event::TouchPhase;
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    let text = argos_render::text::Text::default();
    let (bar, _) = argos_render::toolbar::layout(&a.tools, &text);
    assert!(bar.w > 0.0, "la barre flottante est là");
    // le bord gauche de la pilule ⋮⋮ (son centre est déjà au-dessus de l'échelle de prix, où un dessin ne se touche pas)
    let (cx, cy) = (bar.x + 3.0, bar.cy());
    assert!(cx < a.chart.viewport.plot_w(), "dans la chart");
    let appui_long = |a: &mut App, x: f64, y: f64| {
        let (wx, wy) = (x + a.rw(), y + a.top_h());
        a.touch(4, TouchPhase::Started, (wx, wy));
        std::thread::sleep(std::time::Duration::from_millis(crate::input::touch::LONG_MS as u64 + 100));
        a.touch_tick();
        a.touch(4, TouchPhase::Ended, (wx, wy));
        a.ui.list.as_ref().map(|l| l.0)
    };
    // sans dessin : le menu de la barre
    assert!(matches!(appui_long(&mut a, cx, cy), Some(ListKind::ToolMenu(_))), "menu de la barre");
    a.ui.list = None;
    // une ligne horizontale qui passe juste sous le centre de la barre
    let p = a.frame_now().unwrap().geo().point_at(cx, cy);
    a.chart.drawings.push(Drawing::new("hline", vec![p]));
    assert_eq!(
        a.drawing_at(cx, cy),
        Some(0),
        "la ligne est sous la barre : {:?} {} {:?}",
        a.ui.context.is_open(),
        a.rail.hide_draw,
        a.frame_now().map(|f| f.geo().xy(&p))
    );
    assert!(argos_render::toolbar::contains(&a.tools, &text, cx, cy));
    assert!(
        appui_long(&mut a, cx, cy).is_none() && a.selected == Some(0),
        "le dessin caché est choisi (sa barre) : {:?}",
        a.ui.list.as_ref().map(|l| &l.0)
    );
}

/// 📱 Choix de Jee (15 nov.) : sur le cell, la barre flottante ouverte se replie toute seule en ⋮⋮, à sa place de la
/// maquette, dès qu'un bouton a été choisi ; l'action a bien eu lieu.
#[test]
fn cell_barre_se_replie_apres_un_choix() {
    use argos_render::toolbar::{self, Btn};
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    // comme l'app en passant sur cell : barre en ⋮⋮, à sa place de la maquette
    (a.tools.pad, a.tools.vertical, a.tools.retracted) = (true, false, true);
    let pl = argos_render::ecrans::cell().place("favbar", a.win_w(), a.cell_h()).unwrap();
    (a.tools.x, a.tools.y) = (pl.x - a.rw(), pl.y - a.top_h());
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    toolbar::clamp(&mut a.tools, &argos_render::text::Text::default(), pw, ph);
    let text = argos_render::text::Text::default();
    let pilule = toolbar::layout(&a.tools, &text).0;
    assert!(a.tools.retracted, "repliée au départ (⋮⋮)");
    // ouvrir : toucher la poignée ⋮⋮
    let ouvrir = |a: &mut App| {
        let h = toolbar::button_rect(&a.tools, &text, Btn::Handle).unwrap();
        let (x, y) = (h.cx() + a.rw(), h.cy() + a.top_h());
        toucher(a, x, y);
        assert!(!a.tools.retracted, "ouverte");
    };
    ouvrir(&mut a);
    // un intervalle favori qui n'est pas l'actuel, touché à sa place
    let (r, secs) = toolbar::layout(&a.tools, &text)
        .1
        .into_iter()
        .filter_map(|(r, b)| match b {
            Btn::Favorite(s) if s != a.chart.interval.secs() && r.x > 40.0 => Some((r, s)),
            _ => None,
        })
        .next_back()
        .expect("un intervalle dans la barre ouverte");
    let (x, y) = (r.cx() + a.rw(), r.cy() + a.top_h());
    toucher(&mut a, x, y);
    assert_eq!(a.chart.interval.secs(), secs, "l'intervalle choisi");
    assert!(a.tools.retracted, "repliée après le choix");
    let p = toolbar::layout(&a.tools, &text).0;
    assert_eq!((p.x, p.y), (pilule.x, pilule.y), "revenue à sa place de la maquette");
    // une fenêtre de la barre (types de graphique) : elle s'ouvre, la barre reste ouverte ; le choix la replie
    ouvrir(&mut a);
    a.tool_click(Btn::Style, 0.0, 0.0);
    assert!(a.ui.pop.is_some() && !a.tools.retracted, "la fenêtre s'ouvre, la barre reste");
    a.replier_barre_apres_choix();
    assert!(a.tools.retracted);
    // PC : rien ne change
    let mut b = app();
    b.tools.retracted = false;
    b.replier_barre_apres_choix();
    assert!(!b.tools.retracted, "PC : la barre reste ouverte");
}

/// 📱 🗺 Plan de l'app sur le cell (choix de Jee) : le bouton 🗺 ouvre la page ; chaque bloc, tuile et « Voir », touché
/// à sa place (après défilement au doigt s'il le faut), ferme le plan et ouvre vraiment sa partie de l'app ;
/// ‹ Retour et ✕ ferment.
#[test]
fn cell_plan_chaque_toucher_agit() {
    use argos_render::ecrans::CellHit;
    use argos_render::plan::{Lieu, Plan};
    let ouvrir = || {
        let mut a = cell_390();
        a.draw();
        // comme l'app sur cell : la barre des outils repliée en ⋮⋮
        (a.tools.pad, a.tools.retracted) = (true, true);
        a.cell_click(CellHit::Plan, Rect::new(340.0, 16.0, 30.0, 28.0));
        assert!(a.ui.plan.is_some(), "🗺 ouvre le plan");
        assert!(a.page_ouverte(), "une page : le bouton rond Argos AI reste par-dessus (choix B de Jee)");
        a
    };
    let h = |a: &App| a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h();
    let vue = |a: &App| format!("{} {} {}", empreinte(a), a.ui.raccourcis.is_some(), a.ui.split_menu.is_some());
    // ‹ Retour et ✕
    for k in 0..2 {
        let mut a = ouvrir();
        let c = argos_render::plan::croix(a.win_w());
        let (x, y) = if k == 0 { (40.0, 43.0) } else { (c.cx(), c.cy()) };
        toucher(&mut a, x, y);
        assert!(
            a.ui.plan.is_none(),
            "‹ Retour / ✕ ferment : {:?} w {} {}",
            a.pointer.win_mouse,
            a.win_w(),
            vue(&a)
        );
    }
    let n = Plan::default().zones(390.0).len();
    assert!(n >= 20, "{n} zones");
    for k in 0..n {
        let mut a = ouvrir();
        let avant = vue(&a);
        let (w, hh) = (a.win_w(), h(&a));
        // le doigt fait défiler jusqu'à la zone
        let (r, l) = loop {
            let p = a.ui.plan.as_ref().unwrap();
            let (r, l) = p.zones(w)[k];
            // (au-dessus du bouton rond Argos AI, posé par-dessus tout en bas à droite)
            if r.y >= 90.0 && r.y + r.h <= hh - 110.0 {
                break (r, l);
            }
            let s = p.scroll;
            a.ui.plan.as_mut().unwrap().wheel(40.0, w, hh);
            assert!(a.ui.plan.as_ref().unwrap().scroll > s, "{l:?} jamais visible");
        };
        toucher(&mut a, r.x + 30.0, r.cy());
        assert!(a.ui.plan.is_none(), "{l:?} : le plan se ferme");
        if l != Lieu::Chart {
            assert_ne!(vue(&a), avant, "{l:?} : quelque chose s'ouvre");
        }
    }
}

/// 🧮 A Coinbase BTC chart followed, then three real messages of its public feed (frozen text, no network: a
/// « last_match » and two « match ») read by the page's parser (`whales::parse_all`) and sent as the page does; the
/// start of the minute of the trades (ms).
fn coinbase_trois_trades(a: &mut App) -> i64 {
    a.chart.source = Source::Live {
        exchange: Exchange::Coinbase,
        base: "BTC".into(),
    };
    a.orderflow_suivre();
    assert_eq!(a.orderflow_cle().map(|c| c.symbole), Some("BTC".into()));
    let t0 = now_s() / 60 * 60_000 + 60_000;
    let iso = |ms: i64| {
        chrono::DateTime::from_timestamp_millis(ms)
            .unwrap()
            .format("%Y-%m-%dT%H:%M:%S%.6fZ")
            .to_string()
    };
    let msg = |kind: &str, id: u64, ms: i64, side: &str, size: &str| {
        format!(
            r#"{{"type":"{kind}","trade_id":{id},"maker_order_id":"a","taker_order_id":"b","side":"{side}","size":"{size}","price":"62071.79","product_id":"BTC-USD","sequence":1,"time":"{}"}}"#,
            iso(ms)
        )
    };
    a.on_user_event(UserEvent::Orderflow(Exchange::Coinbase, LiveEvent::Connected));
    for m in [
        msg("last_match", 900, t0 + 1_000, "buy", "0.5"),
        msg("match", 901, t0 + 2_000, "sell", "0.25"),
        msg("match", 902, t0 + 3_000, "buy", "0.1"),
    ] {
        let list = argos_data::whales::parse_all(&m);
        assert_eq!(list.len(), 1, "{m}");
        a.on_user_event(UserEvent::Orderflow(Exchange::Coinbase, LiveEvent::Whales(list)));
    }
    t0
}

/// 🧮 Coinbase from end to end: its real messages go to the chart's orderflow and give a complete minute with its real
/// delta (the taker is the opposite of Coinbase's « side »).
#[test]
fn orderflow_coinbase_de_bout_en_bout() {
    let mut a = app();
    let t0 = coinbase_trois_trades(&mut a);
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    let f = a.feeds.orderflow.iter().find(|f| f.cle.exchange == "coinbase").unwrap();
    let m = f.minutes.iter().find(|b| b.time == t0 / 1000).expect("the minute of the trades");
    assert!(m.complete, "a complete minute of real trades");
    // numbered trades: last_match counts too (never seen before); Coinbase's « side » is the maker's, the taker does
    // the opposite: buy → taker sells −0.5, sell → taker buys +0.25, buy → −0.1
    assert!((m.delta() - (-0.5 + 0.25 - 0.1)).abs() < 1e-9, "{}", m.delta());
    effacer_series(&["coinbase-btc."]);
}

/// 🩺 The page's diagnostic (`window.argosOrderflow`): for the followed Coinbase market, its exchange and coin, the
/// trades received and put, the closed minutes and the last minute put; counters and times only.
#[test]
fn diagnostic_orderflow_de_la_page() {
    let mut a = app();
    let t0 = coinbase_trois_trades(&mut a);
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    let d: serde_json::Value = serde_json::from_str(&a.orderflow_diag()).unwrap();
    let m = d["marches"].as_array().unwrap().iter().find(|m| m["bourse"] == "coinbase").unwrap().clone();
    assert_eq!(
        (m["piece"].as_str(), m["recus"].as_u64(), m["ranges"].as_u64()),
        (Some("BTC"), Some(3), Some(3)),
        "{m}"
    );
    assert!(m["fermees"].as_u64().unwrap() >= 1, "{m}");
    assert_eq!(m["derniere_minute"].as_i64(), Some(t0 / 1000), "{m}");
    let texte = a.orderflow_diag();
    assert!(!texte.contains("cle") && !texte.contains("key") && !texte.contains("journal"), "{texte}");
    effacer_series(&["coinbase-btc."]);
}

/// 🧮 Suspect (a): following the same Coinbase market again (each refresh calls `orderflow_suivre`) neither recreates
/// it (no new cut, the minute in progress stays complete) nor loses the minutes already put.
#[test]
fn orderflow_suivre_deux_fois_garde_le_marche() {
    let mut a = app();
    let t0 = coinbase_trois_trades(&mut a);
    a.orderflow_suivre_a(t0 + 30_000);
    a.orderflow_suivre_a(t0 + 40_000);
    let f = a.feeds.orderflow.iter().find(|f| f.cle.exchange == "coinbase").unwrap();
    assert_eq!(f.recus, (3, 3), "the same market, its counters kept");
    let en_cours = f.agg.en_cours();
    assert!(
        en_cours.iter().any(|b| b.time == t0 / 1000 && b.complete),
        "the minute in progress is still complete"
    );
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    a.orderflow_suivre_a(t0 + 3 * 60_000 + 5_000);
    let f = a.feeds.orderflow.iter().find(|f| f.cle.exchange == "coinbase").unwrap();
    assert!(f.minutes.iter().any(|b| b.time == t0 / 1000 && b.complete), "the closed minute is kept");
    assert_eq!(a.feeds.orderflow.iter().filter(|f| f.cle.exchange == "coinbase").count(), 1);
    effacer_series(&["coinbase-btc."]);
}

/// 🧮 Suspect (b): the real minutes of the chart's market (`orderflow_chart`, what the footprint reads) start on the
/// minute, like the bars of a re-read Coinbase chart, so each falls on its bar (the footprint matches them by start
/// time).
#[test]
fn orderflow_coinbase_tombe_sur_les_bougies() {
    let mut a = app();
    let t0 = coinbase_trois_trades(&mut a);
    a.chart.interval = Interval(60);
    a.orderflow_suivre_a(t0 + 3 * 60_000);
    let reel = a.orderflow_chart();
    let m = reel.iter().find(|b| b.time == t0 / 1000).expect("the minute of the trades, on the minute");
    assert!(m.complete);
    assert!(
        reel.iter().all(|b| b.time % 60 == 0),
        "every real bar starts on a minute, like the chart's bars"
    );
    effacer_series(&["coinbase-btc."]);
}

/// 🧮 The real cause seen in the page (20 Nov.): at launch, the Home page (Accueil) opens before the event loop
/// exists and follows the chart's market (`refresh_eco` → `orderflow_suivre`) while there is no proxy yet. The trades
/// connection must not be noted as open then, or it is never opened at all (no WebSocket, 0 trades received).
#[test]
fn orderflow_flux_sans_proxy_pas_note_ouvert() {
    let mut a = app();
    assert!(a.proxy.is_none(), "like the app before its event loop");
    a.chart.source = Source::Live {
        exchange: Exchange::Coinbase,
        base: "BTC".into(),
    };
    a.orderflow_suivre();
    assert!(a.feeds.orderflow.iter().any(|f| f.cle.exchange == "coinbase"), "the market is followed");
    assert!(
        a.feeds.orderflow_flux.is_empty(),
        "nothing opened without a proxy, so the next call opens it: {:?}",
        a.feeds.orderflow_flux
    );
    effacer_series(&["coinbase-btc."]);
}

/// 🏦 Orderflow réel des bourses branchées après Binance : chacune sur son propre flux, le sens du preneur compté, un
/// trade redonné après une reconnexion jamais recompté (par son numéro ; sans numéro, par son heure), et la minute
/// coupée par la reconnexion marquée incomplète.
#[test]
fn orderflow_reel_des_bourses_sur_leur_propre_flux() {
    // (bourse, source, identifiant, la bourse numérote-t-elle ses trades ?)
    for (ex, src, id, numero) in [
        (Exchange::Okx, "OKX", "okx", true),
        (Exchange::Bitstamp, "Bitstamp", "bitstamp", true),
        (Exchange::Bitunix, "Bitunix", "bitunix", false),
        (Exchange::Bybit, "Bybit", "bybit", false),
        (Exchange::Bitget, "Bitget", "bitget", true),
        (Exchange::Blofin, "BloFin", "blofin", true),
        (Exchange::Weex, "WEEX", "weex", false),
        (Exchange::Gate, "Gate.io", "gate", true),
        (Exchange::Mexc, "MEXC", "mexc", true),
        (Exchange::Kucoin, "KuCoin", "kucoin", true),
        (Exchange::Coinbase, "Coinbase", "coinbase", true),
        (Exchange::Kraken, "Kraken", "kraken", true),
        (Exchange::Aster, "Aster", "aster", true),
        (Exchange::BinanceSpot, "Binance", "binance-spot", true),
        (Exchange::OkxSpot, "OKX", "okx-spot", true),
        (Exchange::BybitSpot, "Bybit", "bybit-spot", true),
        (Exchange::BitgetSpot, "Bitget", "bitget-spot", true),
    ] {
        let mut a = app();
        a.chart.source = Source::Live {
            exchange: ex,
            base: "SOL".into(),
        };
        a.orderflow_suivre();
        assert_eq!(
            a.feeds
                .orderflow
                .iter()
                .map(|f| (f.cle.exchange.as_str(), f.cle.symbole.as_str()))
                .collect::<Vec<_>>(),
            vec![(id, "SOL")]
        );
        // ● la liste des marchés enregistrés accepte ce marché
        // (sans écrire orderflow.json : un autre test le relit en même temps)
        let cle = a.orderflow_cle().unwrap();
        assert_eq!(a.feeds.enregistres.basculer(cle), Ok(true), "{src}");
        assert_eq!(a.orderflow_enregistre(), Some(true), "{src}");
        let t0 = now_s() / 60 * 60_000;
        // le trade n° n arrive n − 40 secondes après le début de la minute
        let trade = |n: u64, size: f64, buy: bool| argos_data::whales::Whale {
            id: numero.then_some(n),
            src,
            ..trade_hl("SOL", t0 + (n as i64 - 40) * 1000, 150.0, size, buy)
        };
        a.on_user_event(UserEvent::Orderflow(ex, LiveEvent::Whales(vec![trade(41, 30.0, true), trade(42, 12.0, false)])));
        // reconnexion : la bourse redonne le trade 42, il n'est pas recompté (son numéro, ou son heure)
        a.on_user_event(UserEvent::Orderflow(ex, LiveEvent::Connected));
        a.on_user_event(UserEvent::Orderflow(ex, LiveEvent::Whales(vec![trade(42, 12.0, false), trade(43, 5.0, true)])));
        a.orderflow_suivre_a(t0 + 3 * 60_000);
        let f = a.feeds.orderflow.iter().find(|f| f.cle.exchange == id).unwrap();
        assert_eq!(
            f.minutes.iter().map(|b| (b.delta(), b.complete)).collect::<Vec<_>>(),
            vec![(30.0 - 12.0 + 5.0, false)],
            "{src}"
        );
        effacer_series(&[&format!("{id}-sol.")]);
    }
}

/// 👆 Défaut cell (15 nov.) : deux touchers rapprochés à des endroits différents (le nom d'un indicateur, puis son 👁)
/// étaient pris pour un double-toucher et le second se perdait. Un double ne compte que rapproché dans le temps (350 ms)
/// ET au même endroit (24 points) ; deux touchers éloignés font chacun leur action.
#[test]
fn double_toucher_seulement_au_meme_endroit() {
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    a.run_action(ChartAction::ResetPrice);
    assert!(a.chart.settings.scale.auto());
    let (x, y) = (120.0 + a.rw(), 400.0 + a.top_h());
    // deux touchers rapprochés, à 150 points l'un de l'autre : pas de double (le prix reste automatique)
    a.pointer.last_chart_click = None;
    toucher(&mut a, x, y);
    toucher(&mut a, x + 150.0, y);
    assert!(a.chart.settings.scale.auto(), "deux endroits différents : pas un double-toucher");
    // au même endroit (à quelques points près) : double-toucher (main libre)
    a.pointer.last_chart_click = None;
    toucher(&mut a, x, y);
    toucher(&mut a, x + 6.0, y + 4.0);
    // (⛶ Jee, 17 Nov: on the phone a double tap on the chart puts it full screen, the price stays automatic)
    assert!(a.ui.plein_ecran.is_some() && a.chart.settings.scale.auto(), "même endroit : double-toucher");
    a.quitter_plein_ecran();
    // au même endroit mais trop tard : pas de double
    a.run_action(ChartAction::ResetPrice);
    a.pointer.last_chart_click = None;
    toucher(&mut a, x, y);
    std::thread::sleep(std::time::Duration::from_millis(400));
    toucher(&mut a, x, y);
    assert!(
        a.chart.settings.scale.auto() && a.ui.plein_ecran.is_none(),
        "trop espacés dans le temps : pas un double-toucher"
    );
}

/// 👆 Défaut cell : deux vrais touchers (doigt) à 500 ms d'écart, à deux endroits (le nom d'un indicateur dans la
/// légende, puis son 👁) agissent tous les deux : la barre s'ouvre, puis l'indicateur se cache.
#[test]
fn deux_touchers_rapproches_agissent_tous_les_deux() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::{zones_de, Cell, LegClick};
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    let ind = Ind::ALL[0];
    a.ind.on = vec![ind];
    a.ui.frames = 5;
    a.refresh_indicators();
    a.ind.leg.0 = true;
    let t = argos_render::text::Text::default();
    let (ph, pw) = (a.chart.viewport.plot_h(), a.chart.viewport.plot_w());
    let ou = |a: &App, c: LegClick| {
        let cell = Cell {
            actif: true,
            sel: a.ind.sel,
            pw,
        };
        let r = zones_de(&t, &a.ind.on, &a.ind.out, true, ph, pw, cell)
            .into_iter()
            .find(|(_, x)| *x == c)
            .unwrap()
            .0;
        (r.cx() + a.rw(), r.cy() + a.top_h())
    };
    let (x, y) = ou(&a, LegClick::Nom(ind));
    toucher(&mut a, x, y);
    assert_eq!(a.ind.sel, Some(Ok(ind)), "1er toucher : la barre de l'indicateur");
    std::thread::sleep(std::time::Duration::from_millis(500));
    let (x2, y2) = ou(&a, LegClick::Eye(ind));
    assert!((x2 - x).hypot(y2 - y) > 24.0, "deux endroits différents");
    toucher(&mut a, x2, y2);
    assert!(a.ind.leg.1.contains(&ind), "2e toucher (500 ms après, ailleurs) : 👁 cache l'indicateur");
}

/// 🖐 Glisser la chart : le geste est en cours (le verre garde son flou) ; au lâcher, il ne l'est plus (le verre est
/// recalculé à l'image suivante).
#[test]
fn geste_en_cours_pendant_le_glisser() {
    let mut a = app();
    assert!(!a.geste_en_cours());
    let (wx, wy) = (300.0 + a.rw(), 200.0 + a.top_h());
    a.pointer.last_chart_click = None;
    a.mouse_moved(PhysicalPosition::new(wx, wy));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(wx + 40.0, wy));
    assert!(a.geste_en_cours(), "glisser : geste en cours");
    a.left_button(ElementState::Released);
    assert!(!a.geste_en_cours(), "lâché : fini");
}

/// 📱 Menu ARGOS du cell : la feuille du bas de la maquette s'ouvre ; chaque toucher (carte du compte, lignes, ⏳ et son
/// reste) ouvre vraiment sa partie de l'app, ou dit « bientôt ».
#[test]
fn cell_menu_argos_chaque_toucher_agit() {
    use argos_render::ecrans::CellHit;
    let ouvrir = || {
        let mut a = cell_390();
        a.draw();
        a.cell_click(CellHit::Argos, Rect::new(60.0, 16.0, 70.0, 28.0));
        assert!(a.ui.argos.as_ref().is_some_and(|m| m.feuille), "ARGOS : la feuille du cell");
        a
    };
    let vue = |a: &App| {
        let m = a.ui.argos.as_ref().map(|m| m.bientot);
        format!("{} {} {} {m:?}", empreinte(a), a.ui.raccourcis.is_some(), a.ui.accueil.is_some())
    };
    let n = ouvrir().ui.argos.as_ref().unwrap().zones().len();
    for bientot in [false, true] {
        for k in 0..n {
            let mut a = ouvrir();
            if bientot {
                let z = a.ui.argos.as_ref().unwrap().zones();
                let (b, _) = *z.iter().find(|(_, x)| *x == argos_render::topmenu::TopAct::Bientot).unwrap();
                toucher(&mut a, b.x + 40.0, b.cy());
                assert!(a.ui.argos.as_ref().is_some_and(|m| m.bientot), "⏳ ouvre le reste");
            }
            let z = a.ui.argos.as_ref().unwrap().zones();
            let Some(&(r, act)) = z.get(k) else { continue };
            let avant = vue(&a);
            // à gauche de la zone (loin du bouton rond Argos AI, en bas à droite)
            toucher(&mut a, r.x + 40.0_f64.min(r.w / 2.0), r.cy());
            let apres = vue(&a);
            assert!(apres != avant || a.banner.is_some(), "{act:?} (⏳ {bientot}) : rien ne se passe");
        }
    }
}

/// 🗂 Un seul menu flottant à la fois (Jee : « les menus s'empilent ») : pour chaque paire d'ouvertures de la barre du
/// haut et de la barre de droite, ouvrir A puis B laisse seulement ce que B ouvre seul.
#[test]
fn un_seul_menu_a_la_fois() {
    use argos_render::topbar::{TopClick, TopMenu};
    let r = Rect::new(200.0, 0.0, 40.0, 36.0);
    type Ouverture = (&'static str, Box<dyn Fn(&mut App)>);
    let ouvertures: Vec<Ouverture> = vec![
        ("Argos", Box::new(move |a| a.top_click_do(TopClick::Menu(TopMenu::Argos, r)))),
        ("Splits", Box::new(move |a| a.top_click_do(TopClick::Menu(TopMenu::Split, r)))),
        ("Mises en page", Box::new(move |a| a.top_click_do(TopClick::Menu(TopMenu::Layouts, r)))),
        ("Réglages", Box::new(move |a| a.top_click_do(TopClick::Menu(TopMenu::Settings, r)))),
        ("Appli", Box::new(move |a| a.top_click_do(TopClick::Menu(TopMenu::App, r)))),
        ("Recherche", Box::new(|a| a.top_click_do(TopClick::Recherche))),
        ("Alertes", Box::new(move |a| a.top_click_do(TopClick::Alertes(r)))),
        ("Indicateurs", Box::new(move |a| a.top_click_do(TopClick::Indicateurs(r)))),
        ("Communauté", Box::new(move |a| a.outil_barre(wl::Tool::Communaute, r))),
        // les volets de droite : comme leur bouton de la barre d'icônes (volet ouvert sur ce panneau)
        (
            "Volet Trading",
            Box::new(|a| (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Trading)),
        ),
        (
            "Volet Modules",
            Box::new(|a| (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Modules)),
        ),
    ];
    let neuf = || {
        let mut a = app();
        a.watch.prefs.open = false;
        a.draw();
        a
    };
    let seul: Vec<[bool; crate::state::OUVERTS]> = ouvertures
        .iter()
        .map(|(_, f)| {
            let mut a = neuf();
            f(&mut a);
            a.draw();
            a.ui.ouverts
        })
        .collect();
    for (k, (na, fa)) in ouvertures.iter().enumerate() {
        assert!(seul[k].iter().any(|x| *x), "{na} ouvre quelque chose");
        for (j, (nb, fb)) in ouvertures.iter().enumerate() {
            if j == k {
                continue;
            }
            let mut a = neuf();
            fa(&mut a);
            a.draw();
            fb(&mut a);
            a.draw();
            assert_eq!(a.ui.ouverts, seul[j], "{na} puis {nb} : seulement {nb}");
        }
    }
    // un sous-menu ouvert depuis l'intérieur du volet garde le volet
    let mut a = neuf();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Trading);
    a.draw();
    a.pointer.side_mouse = Some((10.0, 100.0));
    a.open_list(ListKind::Alerts, Rect::new(10.0, 100.0, 50.0, 20.0));
    a.draw();
    assert!(a.watch.prefs.open && a.ui.list.is_some(), "sous-menu du volet : le volet reste");
}

/// Le texte de l'avertissement d'une boîte de clé, lignes recollées.
fn texte_avis(b: &KeyBox) -> String {
    b.avis.iter().map(|(_, l)| l.as_str()).collect::<Vec<_>>().join(" ")
}

/// ⚠️ L'avertissement de la boîte de la clé Claude dit tout ce qui part avec une question, et rien n'est envoyé sans clé
/// prête : sans clé, la boîte s'ouvre avec l'avertissement ; clé gardée, la boîte du code ; jamais d'envoi.
#[test]
fn assistant_sans_cle_aucun_envoi_et_avertissement() {
    use argos_render::assistant::KeyState;
    let mut a = app();
    for (etat, boite) in [(KeyState::None, "claude"), (KeyState::Locked, "claude-unlock"), (KeyState::Ready, "")] {
        a.ai.key = etat;
        a.session.key_box = None;
        a.ai_send("Analyse BTC".into());
        assert!(!a.ai.busy && a.ai.msgs.is_empty(), "{etat:?} : rien n'est envoyé");
        assert_eq!(a.session.key_box.as_ref().map_or("", |b| b.exchange.as_str()), boite, "{etat:?}");
    }
    a.ai.key = KeyState::None;
    a.ai_send("Analyse BTC".into());
    let avis = texte_avis(&a.session.key_box.take().unwrap());
    assert!(avis.contains("quittent l'appareil"), "{avis}");
    // le clic « clé » du volet ouvre la même boîte
    assert_eq!(texte_avis(&a.boite_cle_claude()), avis);
}

/// ✉️ Tout ce que `ai_requete` met dans l'envoi est dans l'avertissement (même nombre de bougies et de messages).
#[test]
fn avertissement_dit_tout_ce_qui_part() {
    let mut a = app();
    a.ind.on = vec![argos_render::indicators::Ind::Rsi];
    a.ui.ai_mem.notes = vec!["NOTE-PERSO".into()];
    a.ui.ai_mem.method = "METHODE-PERSO".into();
    a.ai.msgs = (0..20).map(|i| (i % 2 == 0, format!("message {i}"))).collect();
    let (system, history) = a.ai_requete("");
    let avis = texte_avis(&a.boite_cle_claude());
    let bougies = system.lines().filter(|l| l.contains(" o") && l.contains(" v10")).count();
    let derniere = a.chart.bars.last().unwrap().close.to_string();
    let quoi = [
        (system.contains(&a.symbol_label()), "le symbole"),
        (system.contains(&a.chart.interval.short()), "l'unité de temps"),
        (system.contains(&derniere), "le dernier prix"),
        (system.contains(argos_render::indicators::Ind::Rsi.name()), "les indicateurs affichés"),
        (
            system.contains("NOTE-PERSO") && system.contains("METHODE-PERSO"),
            "tes notes 🧠 Mémoire et ta méthode",
        ),
        (true, "(sauf en 🧪 Sandbox)"),
    ];
    for (envoye, dit) in quoi {
        assert!(envoye && avis.contains(dit), "{dit} : envoyé {envoye}, avertissement « {avis} »");
    }
    assert!(avis.contains(&format!("les {bougies} dernières bougies")), "{bougies} bougies : {avis}");
    assert!(
        avis.contains(&format!("les {} derniers messages", history.len())),
        "{} messages : {avis}",
        history.len()
    );
    a.ui.ai_mem.sandbox = true;
    assert!(!a.ai_requete("").0.contains("NOTE-PERSO"), "🧪 Sandbox : pas de notes");
}

/// 🖥 Colonne d'icônes de la maquette PC : elle monte du haut au bas de la fenêtre (la barre du haut s'arrête avant
/// elle) ; chaque bouton agit, même à côté de la barre du haut ; ⚙ en bas ouvre « Personnaliser la barre » (le menu du
/// clic droit) ; › replie la barre et la colonne disparaît (la barre du haut reprend toute la largeur).
#[test]
fn colonne_d_icones_pc_du_haut_au_bas() {
    use argos_render::watchlist::ICON_BAR;
    let mut a = app();
    a.area = (1330, 676);
    let pose = |a: &mut App| (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
    pose(&mut a);
    let haut = a.watch.colonne.expect("colonne PC");
    assert_eq!(haut, a.top_h() - 10.0);
    assert_eq!(a.top_w(), a.win_w() - ICON_BAR as f64, "la barre du haut s'arrête avant la colonne");
    let col = a.watch.col_rect(a.watch.col_h).unwrap();
    assert_eq!(
        (a.side_x() + col.x - (a.win_w() - 50.0), a.top_h() + col.y),
        (0.0, 10.0),
        "44 de large à 6 du bord, 10 du haut"
    );
    assert_eq!(col.w, 44.0);
    // 1er bouton (watchlist) à côté de la barre du haut : il ouvre son volet, un 2e clic le ferme
    let t0 = a.watch.bar_tools()[0];
    let x = a.win_w() - 28.0;
    clic(&mut a, x, 37.0);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == t0, "{t0:?} ouvert");
    pose(&mut a);
    let x = a.win_w() - 28.0;
    clic(&mut a, x, 37.0);
    assert!(!a.watch.prefs.open, "fermé");
    // chaque bouton de la colonne répond (volet ouvert, ou fenêtre / action à lui)
    pose(&mut a);
    for (i, t) in a.watch.bar_tools().into_iter().enumerate() {
        let ir = a.watch.tool_rect(i);
        assert!(ir.y + ir.h <= a.watch.col_h - 10.0 - 83.0, "{t:?} au-dessus de ›");
        assert_eq!(a.watch.tool_at(ir.cx(), ir.cy()).map(|x| x.1), Some(t), "{t:?} se touche");
    }
    // ✨ Argos AI dans la case du coin : sous l'échelle de prix, à droite de l'axe du temps (toute la case se clique),
    // le rond centré à 2 points au moins des bords ; jamais sur les étiquettes de valeur des panneaux ni sur › ⚙
    let (ai, rond) = (a.ai_bouton_rect(), a.ai_rond());
    let vp = &a.chart.viewport;
    let (echelle, axe) = (a.rw() + vp.plot_w(), a.top_h() + vp.plot_h());
    assert_eq!(
        (ai.x, ai.y, ai.w, ai.h),
        (echelle, axe, vp.price_axis as f64, vp.time_axis as f64),
        "la case du coin"
    );
    assert!(rond.x >= ai.x + 2.0 && rond.y >= ai.y + 2.0 && rond.x + rond.w <= ai.x + ai.w - 2.0 && rond.y + rond.h <= ai.y + ai.h - 2.0);
    assert!((rond.cx() - ai.cx()).abs() < 0.01 && (rond.cy() - ai.cy()).abs() < 0.01, "rond centré");
    // ⚙ : Personnaliser la barre
    let bas = a.top_h() + a.watch.col_h - 10.0;
    let x = a.win_w() - 28.0;
    clic(&mut a, x, bas - 43.0 + 18.0);
    assert!(
        matches!(a.ui.list.as_ref().map(|l| &l.0), Some(ListKind::IconBar(None))),
        "⚙ ouvre le menu de la barre"
    );
    a.ui.list = None;
    // › : la barre se replie, la colonne s'en va
    let x = a.win_w() - 28.0;
    clic(&mut a, x, bas - 83.0 + 18.0);
    assert!(a.watch.prefs.folded && a.watch.width() == 0);
    assert!(a.colonne_pc().is_none());
    assert_eq!(a.top_w(), a.win_w());
}

/// 🖥 Cadre PC d'interface-2 : le rail replié en bande de 12 (un clic la déplie), la barre de favoris visible à sa place
/// de la maquette ; rien n'est écrit dans rail.json ni toolbar.json (l'ancienne /app/, même stockage, ne voit aucun
/// changement) ; une sauvegarde de la barre ou du rail garde les champs de l'ancienne /app/ tels quels.
#[test]
fn cadre_pc_rail_replie_et_fichiers_de_l_ancienne_app_intacts() {
    let mut a = app();
    a.area = (1330, 676);
    // (les réglages d'interface-2 d'un premier lancement ; le fichier est partagé avec les autres tests)
    a.appliquer_cadre(&crate::actions::Interface2::default());
    // rail.json et toolbar.json de l'ancienne /app/ (même stockage) : rien n'y est écrit (dossier à ce test seul)
    let d = argos_core::dossiers::de_test("cadre-pc");
    std::fs::create_dir_all(&d).unwrap();
    crate::persist::saved::DOSSIER_TEST.with(|x| *x.borrow_mut() = Some(d));
    App::write_prefs("rail.json", &a.rail);
    App::write_prefs("toolbar.json", &a.tools);
    let avant = (platform::read("rail.json"), platform::read("toolbar.json"));
    a.appliquer_cadre(&crate::actions::Interface2::default());
    assert_eq!((platform::read("rail.json"), platform::read("toolbar.json")), avant, "rien d'écrit");
    assert!(a.rail.folded && a.rw() == crate::app::BANDE_W, "rail replié");
    assert_eq!((a.tools.x, a.tools.y, a.tools.hidden, a.tools.retracted), (64.0, 110.0, false, false));
    assert!(a.barre_outils_proche(false), "barre de favoris toujours là, rail replié");
    assert!(a.top.prefs.pct, "tickers en % (maquette) au premier passage dans /app2/");
    assert!(a.to_chart().legende, "légende O H L C · Vol toujours là sur PC");
    // la bande : 12 de large à 10 du bord, sans languette ; au survol le rail s'ouvre par-dessus la chart (la chart ne
    // bouge pas) et ses boutons agissent ; la souris partie, il se referme
    let b = a.bande_rail().expect("bande du rail");
    assert_eq!((b.x, b.w), (10.0, 12.0));
    assert!(a.fold_tabs().iter().all(|(_, left)| !left));
    let rw0 = a.rw();
    a.mouse_moved(PhysicalPosition::new(b.cx(), b.cy()));
    assert!(a.ui.rail_survol, "rail ouvert au survol");
    assert_eq!(a.rw(), rw0, "par-dessus : la chart ne bouge pas");
    let (r, btn) = rail::layout(a.area.1 as f64)
        .into_iter()
        .find(|(_, x)| matches!(x, rail::Btn::Group(_)))
        .unwrap();
    let (wx, wy) = (crate::actions::RAIL_X + r.cx(), a.top_h() + r.cy());
    a.mouse_moved(PhysicalPosition::new(wx, wy));
    assert!(a.ui.rail_survol);
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert!(a.ui.list.is_some() || a.tool != "cursor", "{btn:?} agit");
    a.ui.list = None;
    a.mouse_moved(PhysicalPosition::new(600.0, 400.0));
    assert!(!a.ui.rail_survol, "refermé");
    // écrire la barre ou le rail : les champs de l'ancienne /app/ restent ceux du fichier
    let nom = "essai-cadre-barre.json";
    platform::write(nom, r#"{"x": 7.0, "y": 8.0, "hidden": true, "favorites": [60]}"#).unwrap();
    a.tools.x = 300.0;
    App::write_prefs_garde(nom, &a.tools, &["x", "y", "hidden", "retracted", "vertical"]);
    let v: serde_json::Value = serde_json::from_str(&platform::read(nom).unwrap()).unwrap();
    assert_eq!((v["x"].as_f64(), v["y"].as_f64(), v["hidden"].as_bool()), (Some(7.0), Some(8.0), Some(true)));
    assert!(v.get("retracted").is_none(), "absent avant : absent après");
}

/// 🖥 Pied flottant du cadre PC : caché loin du bas de la chart, une pilule de 1050 × 32 centrée à 8 du bas au survol ;
/// ses boutons agissent comme ceux de l'ancienne barre du bas (📅 Aller à, fuseau horaire) ; plus de ⚙ dans le coin.
#[test]
fn pied_flottant_pc_au_survol() {
    let mut a = app();
    a.area = (1330, 676);
    (a.chart.viewport.width, a.chart.viewport.height, a.chart.viewport.footer) = (1330, 676, 0);
    let vp = a.chart.viewport;
    let (dx, dy) = (a.rw(), a.top_h());
    a.mouse_moved(PhysicalPosition::new(dx + 300.0, dy + 200.0));
    assert!(a.pied_visible().is_none(), "loin du bas : caché");
    a.mouse_moved(PhysicalPosition::new(dx + 400.0, dy + vp.height as f64 - 20.0));
    let r = a.pied_visible().expect("au survol du bas");
    assert_eq!(r.w, 1050.0);
    assert_eq!(r.y + r.h, vp.height as f64 - 8.0);
    assert!((r.x + r.w / 2.0 - vp.plot_w() / 2.0).abs() <= 1.0, "centré");
    // les boutons du pied : 📅 Aller à
    let pv = argos_render::bottombar::pied_vp(r.w);
    let yb = argos_render::bottombar::PIED_BULLE + argos_render::bottombar::PIED_H / 2.0;
    let x_de = |h: argos_render::bottombar::Hit| {
        (0..r.w as i32)
            .map(f64::from)
            .find(|x| argos_render::bottombar::hit(&pv, Some((*x, yb)), None) == Some(h))
            .unwrap()
    };
    let xg = x_de(argos_render::bottombar::Hit::GoTo);
    clic(&mut a, dx + r.x + xg, dy + r.y + yb);
    assert!(a.ui.goto.is_some(), "📅 ouvre « Aller à »");
    a.ui.goto = None;
    // fuseau horaire
    let xs = x_de(argos_render::bottombar::Hit::Sessions);
    clic(&mut a, dx + r.x + xs, dy + r.y + yb);
    assert!(
        matches!(a.ui.list.as_ref().map(|l| &l.0), Some(ListKind::Timezone)),
        "sessions : menu Fuseau horaire"
    );
    // plus de ⚙ dans le coin de l'axe (le menu d'échelle reste au clic droit)
    assert!(a.to_chart().navigation);
    assert_eq!(a.chart.viewport.footer, 0);
}

/// 🖥 ▾ au bout de la fiche du symbole : les deux rangées du haut se replient (la chart remonte) en une fine bande à
/// poignée (demande de Jee : comme le rail) ; au survol elles s'ouvrent par-dessus la chart (la chart ne bouge pas),
/// souris partie elles se referment ; ▴ « garder ouverte » les remet fixes. Impossible de les perdre.
#[test]
fn rangees_du_haut_repliees_puis_rouvertes() {
    use argos_render::formfactor::{RANGEE1_PC, RANGEE2_PC, RANGEE3_PC};
    let mut a = app();
    a.area = (1330, 676);
    (a.chart.viewport.width, a.chart.viewport.height) = (1330, 676);
    let h0 = a.top_h();
    let w = a.top_w();
    let t = argos_render::text::Text::default();
    assert_eq!(
        // (the middle of the 3rd row: 8 + row 1 + 6 + row 2 + 6, half of row 3)
        a.top.click(
            w - 8.0 - 17.0,
            8.0 + RANGEE1_PC + 6.0 + RANGEE2_PC + 6.0 + RANGEE3_PC / 2.0,
            w,
            &t,
            &|_: &str| false
        ),
        argos_render::topbar::TopClick::ReplierBandeaux
    );
    assert!(a.bande_haut().is_none());
    a.top_click_do(argos_render::topbar::TopClick::ReplierBandeaux);
    let h1 = argos_render::topbar::SANS_BANDEAUX_H as f64;
    assert!(a.top.sans_bandeaux && a.top_h() == h1 && a.top_h() < h0);
    // la bande (couchée, sous la 1re rangée) ; survol : ouvertes par-dessus la chart
    let b = a.bande_haut().expect("bande des rangées repliées");
    assert!(b.w > b.h && b.h == 12.0);
    a.mouse_moved(PhysicalPosition::new(b.cx(), b.cy()));
    assert!(a.top.survol_bandeaux && a.top_h() == h1, "ouvertes au survol, la chart ne bouge pas");
    a.mouse_moved(PhysicalPosition::new(400.0, 500.0));
    assert!(!a.top.survol_bandeaux && a.bande_haut().is_some(), "refermées, la bande toujours là");
    // ▴ dans les rangées ouvertes : gardées ouvertes
    a.mouse_moved(PhysicalPosition::new(b.cx(), b.cy()));
    let (x, y) = (w - 8.0 - 17.0, 8.0 + RANGEE1_PC + 6.0 + RANGEE2_PC + 6.0 + RANGEE3_PC / 2.0);
    a.mouse_moved(PhysicalPosition::new(x, y));
    assert!(a.top.survol_bandeaux);
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert!(!a.top.sans_bandeaux && a.top_h() == h0, "gardées ouvertes");
}

/// 🖥 Colonne PC d'interface-2 : les outils de la maquette, dans son ordre (sans ⚙ Réglages, déjà dans la barre du
/// haut) ; « Personnaliser la barre » a une étoile par outil : ⭐ l'ajoute à la colonne, ☆ l'en retire ; chaque icône
/// de la colonne agit ; watchlist.json (partagé avec l'ancienne /app/) n'est pas touché.
#[test]
fn colonne_pc_icones_de_la_maquette_et_etoiles() {
    use argos_render::watchlist::Tool;
    let mut a = app();
    a.area = (1330, 676);
    a.appliquer_cadre(&crate::actions::Interface2::default());
    (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
    assert_eq!(a.watch.bar_tools(), Tool::MAQUETTE.to_vec(), "liste et ordre de la maquette");
    assert!(!a.watch.bar_tools().contains(&Tool::Reglages), "plus de ⚙ dans la colonne");
    // (ce que watchlist.json garderait : l'ordre et les masqués de l'ancienne /app/)
    let avant = (a.watch.prefs.order.clone(), a.watch.prefs.hidden_tools.clone());
    // chaque icône de la colonne se touche et agit (volet ouvert ou action à elle)
    for (i, t) in a.watch.bar_tools().into_iter().enumerate() {
        let r = a.watch.tool_rect(i);
        assert_eq!(a.watch.tool_at(r.cx(), r.cy()).map(|x| x.1), Some(t), "{t:?}");
    }
    // ⭐ dans « Personnaliser la barre » : le Journal entre dans la colonne, puis en sort
    let menu = a.icon_bar_menu(None);
    let journal = menu[0].items.iter().find(|m| m.label.ends_with("Journal")).expect("ligne Journal");
    assert_eq!(journal.star, Some(false));
    let id = journal.id;
    a.icon_bar_pick(id);
    assert_eq!(a.watch.bar_tools().last(), Some(&Tool::Journal), "⭐ ajoute l'icône");
    assert_eq!(a.icon_bar_menu(None)[0].items.iter().find(|m| m.id == id).unwrap().star, Some(true));
    a.icon_bar_pick(id);
    assert!(!a.watch.bar_tools().contains(&Tool::Journal), "☆ la retire");
    assert_eq!(
        (a.watch.prefs.order.clone(), a.watch.prefs.hidden_tools.clone()),
        avant,
        "réglages de watchlist.json inchangés"
    );
    // le clic droit sur la colonne ouvre toujours le menu
    let r = a.watch.tool_rect(0);
    a.mouse_moved(PhysicalPosition::new(a.side_x() + r.cx(), a.top_h() + r.cy()));
    a.right_click();
    assert!(matches!(a.ui.list.as_ref().map(|l| &l.0), Some(ListKind::IconBar(_))));
}

/// 🖥 Colonne repliée comme la maquette (`colrep`) : › la replie en une bande de 12 avec poignée (22 gardés à droite de
/// la chart) ; au survol elle s'élargit par-dessus la chart et ses icônes agissent ; ‹ « Garder la barre ouverte » la
/// rouvre ; rien dans watchlist.json (interface-2.json).
#[test]
fn colonne_repliee_en_bande_et_survol() {
    let mut a = app();
    a.area = (1330, 676);
    a.appliquer_cadre(&crate::actions::Interface2::default());
    let pose = |a: &mut App| (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
    pose(&mut a);
    let plie = a.watch.prefs.folded;
    // › en bas de la colonne
    let [(r1, _), _] = a.watch.col_boutons(0.0).unwrap();
    let (x, y) = (a.side_x() + r1.cx(), a.top_h() + r1.cy());
    clic(&mut a, x, y);
    assert!(a.watch.col_repliee && a.watch.width() == 22, "repliée en bande");
    assert_eq!(a.watch.prefs.folded, plie, "watchlist.json : rien de changé");
    pose(&mut a);
    let b = a.watch.col_bande().expect("bande");
    assert_eq!(b.w, 12.0);
    // survol : elle s'élargit, ses icônes agissent
    a.mouse_moved(PhysicalPosition::new(a.side_x() + b.cx(), a.top_h() + b.cy()));
    assert!(a.watch.col_survol && a.watch.col_bande().is_none());
    let r = a.watch.tool_rect(1);
    let t1 = a.watch.bar_tools()[1];
    let (x, y) = (a.side_x() + r.cx(), a.top_h() + r.cy());
    a.mouse_moved(PhysicalPosition::new(x, y));
    assert_eq!(a.watch.tool_at(r.cx(), r.cy()).map(|z| z.1), Some(t1));
    // ‹ : gardée ouverte
    let [(r1, _), _] = a.watch.col_boutons(0.0).unwrap();
    let (x, y) = (a.side_x() + r1.cx(), a.top_h() + r1.cy());
    a.mouse_moved(PhysicalPosition::new(x, y));
    clic(&mut a, x, y);
    assert!(!a.watch.col_repliee && a.watch.width() == argos_render::watchlist::ICON_BAR, "rouverte");
}

/// 🖥 Légende d'un panneau d'indicateur (maquette : 👁 ⚙ ↑ ↓ ▾ ✕, plus ✨ et ⭐ gardés) : chaque bouton agit —
/// masquer, réglages, monter / descendre, replier (22 de haut, la chart reprend la place) puis rouvrir, Argos AI ouvert
/// sur l'indicateur avec ses réglages actuels, favori, retirer.
#[test]
fn legende_des_panneaux_chaque_bouton_agit() {
    use argos_render::indicators::{self as ind, Ind};
    use argos_render::indleg::LegClick;
    let mut a = app();
    a.area = (1330, 676);
    a.appliquer_cadre(&crate::actions::Interface2::default());
    for i in [Ind::Rsi, Ind::Macd] {
        a.toggle_indicator(i);
    }
    a.refresh_indicators();
    let ordre = |a: &App| a.ind.out.bands(0.0).into_iter().map(|(b, _)| b).collect::<Vec<_>>();
    let v0 = ordre(&a);
    assert_eq!(v0.len(), 2, "deux panneaux");
    let (bas, haut) = (v0[0], v0[1]);
    // les boutons d'un panneau, au survol : 👁 ⚙ ↑ ↓ ▾ ✨ ⭐ ✕ (plus de ✥ ni de </>)
    ind::set_survol(Some(bas));
    let t = argos_render::text::Text::default();
    let z = argos_render::indleg::zones(&t, &a.ind.on, &a.ind.out, false, 500.0);
    assert!(z.iter().any(|(_, c)| *c == LegClick::Replier(bas)) && z.iter().any(|(_, c)| *c == LegClick::Monter(bas)));
    assert!(!z.iter().any(|(_, c)| matches!(c, LegClick::Move(_))));
    // { } seulement si l'indicateur a un code source (le Pine de Jee), juste après ⚙ ; sinon pas de bouton
    let code = z.iter().position(|(_, c)| *c == LegClick::Code(bas));
    assert_eq!(
        code.is_some(),
        argos_render::sources::a_un_code(bas),
        "{bas:?} : {{ }} si et seulement si une source"
    );
    if let Some(k) = code {
        assert!(k > 0 && matches!(z[k - 1].1, LegClick::Settings(_) | LegClick::Eye(_)), "{{ }} après 👁 ⚙");
    }
    ind::set_survol(None);
    // 👁
    a.legend_click(LegClick::Eye(bas));
    assert!(a.ind.out.hidden.contains(&bas));
    a.legend_click(LegClick::Eye(bas));
    // ↑ puis ↓
    a.legend_click(LegClick::Monter(bas));
    assert_eq!(ordre(&a), vec![haut, bas], "monté");
    a.legend_click(LegClick::Descendre(bas));
    assert_eq!(ordre(&a), vec![bas, haut], "redescendu");
    // ▾ : replié à 22, la chart reprend la place ; encore : rouvert
    let h0 = a.ind.out.osc_h();
    a.legend_click(LegClick::Replier(haut));
    assert_eq!(haut.band_h(), ind::BAND_REPLIE);
    assert!(a.ind.out.osc_h() < h0);
    a.legend_click(LegClick::Replier(haut));
    assert!(!haut.replie() && a.ind.out.osc_h() == h0);
    // ⭐
    let fav = a.ind.favs.contains(&bas);
    a.legend_click(LegClick::Fav(bas));
    assert_ne!(a.ind.favs.contains(&bas), fav);
    a.legend_click(LegClick::Fav(bas));
    // ✨ : la question nomme l'indicateur et ses réglages
    let q = App::question_indicateur(bas, &ind::values(&a.ind.out.inputs, bas));
    assert!(q.contains(bas.name()));
    for p in bas.params().1 {
        assert!(q.contains(p.label), "réglage {}", p.label);
    }
    a.legend_click(LegClick::Ai(bas));
    assert!(a.ui.ai_open, "Argos AI ouvert");
    // ⚙
    a.legend_click(LegClick::Settings(bas));
    assert!(a.ui.ind_dlg.is_some(), "réglages ouverts");
    a.ui.ind_dlg = None;
    // ✕
    a.legend_click(LegClick::Remove(haut));
    assert!(!a.ind.on.contains(&haut));
    ind::set_ordre(&[]);
    ind::set_replies(&[]);
}

/// 🖥 Rail de gauche comme les deux autres barres : bande → survol ouvert → sortie refermé ; › à côté du rail ouvert le
/// garde ouvert (la souris peut partir), ‹ le replie de nouveau (il revient au survol).
#[test]
fn rail_garde_ouvert_puis_replie() {
    let mut a = app();
    a.area = (1330, 676);
    a.appliquer_cadre(&crate::actions::Interface2::default());
    let b = a.bande_rail().expect("bande");
    a.mouse_moved(PhysicalPosition::new(b.cx(), b.cy()));
    assert!(a.ui.rail_survol, "ouvert au survol");
    let r = a.rail_garde_rect().expect("bouton ‹ ›");
    clic(&mut a, r.cx(), r.cy());
    assert!(a.ui.rail_garde, "› : gardé ouvert");
    a.mouse_moved(PhysicalPosition::new(700.0, 400.0));
    // (défaut de Jee : le rail gardé ouvert passait sur la légende, la carte Lens, les panneaux) : il prend sa place,
    // la chart commence après lui ; la légende ⌄ N et son bouton ‹ ne se croisent pas
    assert!(!a.rail.folded && a.rw() >= rail::RAIL_W as f64, "la chart se décale");
    let leg = Rect::new(a.rw() + 8.0, a.top_h() + 8.0, 52.0, 22.0);
    assert!(leg.x >= rail::RAIL_W as f64, "légende à droite du rail");
    let r = a.rail_garde_rect().unwrap();
    assert!(r.y > leg.y + leg.h + 100.0, "‹ loin de la légende");
    let r = a.rail_garde_rect().unwrap();
    clic(&mut a, r.cx(), r.cy());
    assert!(!a.ui.rail_garde, "‹ : replié");
    a.mouse_moved(PhysicalPosition::new(700.0, 400.0));
    assert!(!a.ui.rail_survol && a.bande_rail().is_some(), "refermé, la bande toujours là");
}

/// 🖥 Re-cliquer le bouton de ce qui est ouvert le ferme (demande de Jee) : 🛡 Gardien (volet Trading), le volet
/// Symboles (Watchlist ; since 22 Nov reached by « Ma watchlist › » of the pinned market panel), ARGOS ▾ ; un autre volet ouvert : 🛡 ouvre Trading, le clic suivant le ferme.
#[test]
fn deuxieme_clic_ferme() {
    use argos_render::topbar::{TopClick, TopMenu};
    let mut a = app();
    a.area = (1330, 676);
    a.top_click_do(TopClick::Gardien);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading, "🛡 ouvre Trading");
    a.top_click_do(TopClick::Gardien);
    assert!(!a.watch.prefs.open, "🛡 encore : fermé");
    a.symboles_volet();
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Watchlist);
    a.top_click_do(TopClick::Gardien);
    assert!(
        a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading,
        "un autre volet : 🛡 ouvre Trading"
    );
    a.top_click_do(TopClick::Gardien);
    assert!(!a.watch.prefs.open);
    a.symboles_volet();
    a.symboles_volet();
    assert!(!a.watch.prefs.open, "symbole encore : fermé");
    let r = Rect::new(60.0, 8.0, 100.0, 36.0);
    a.top_click_do(TopClick::Menu(TopMenu::Argos, r));
    assert!(a.ui.argos.is_some(), "ARGOS ▾ ouvert");
    a.top_click_do(TopClick::Menu(TopMenu::Argos, r));
    assert!(a.ui.argos.is_none(), "ARGOS ▾ encore : fermé");
    // rangées repliées ouvertes au survol, volet ouvert : elles s'arrêtent au bord du volet (il passe devant)
    a.top_click_do(TopClick::Gardien);
    (a.top.sans_bandeaux, a.top.survol_bandeaux) = (true, true);
    let z = a.haut_survol_zone().unwrap();
    assert!(z.x + z.w <= a.side_x() + 0.5, "le haut du volet n'est pas caché");
}

/// 🖥 Rangées du haut ouvertes au survol (demande de Jee) : elles tiennent entre le rail (bande ou gardé ouvert) et le
/// volet ouvert (sinon la colonne d'icônes) ; aucune zone des rangées ne croise le rail, le volet ni la colonne.
#[test]
fn rangees_ouvertes_entre_rail_et_volet() {
    let mut a = app();
    a.area = (1330, 676);
    a.appliquer_cadre(&crate::actions::Interface2::default());
    (a.top.sans_bandeaux, a.top.survol_bandeaux) = (true, true);
    let t = argos_render::text::Text::default();
    for garde in [false, true] {
        if garde != a.ui.rail_garde {
            (a.ui.rail_garde, a.rail.folded) = (garde, !garde);
        }
        for volet in [None, Some(wl::Tool::Watchlist), Some(wl::Tool::Trading), Some(wl::Tool::Modules)] {
            (a.watch.prefs.open, a.watch.prefs.panel) = (volet.is_some(), volet.unwrap_or(wl::Tool::Watchlist));
            (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
            a.top.rangs = a.rangs_haut();
            let (g, d) = a.top.rangs.unwrap();
            let rail_d = if garde { rail::RAIL_W as f64 } else { 22.0 };
            let droite = if volet.is_some() { a.side_x() } else { a.top_w() };
            assert!(g >= rail_d && d <= droite, "{garde} {volet:?} : {g} … {d}");
            // chaque zone cliquable des rangées 2 et 3 est entre ces bords
            let w = a.top_w();
            for y in [74.0, 115.0] {
                for x in (0..w as i32).step_by(4).map(f64::from) {
                    let c = a.top.zone_en(x, y, w, &t);
                    if c {
                        assert!(x >= g - 1.0 && x <= d + 1.0, "{garde} {volet:?} : zone à {x}");
                    }
                }
            }
        }
    }
}

/// 🧮 Bourse à contrats (BloFin), pièce dont la taille du contrat est inconnue : pas d'orderflow réel (l'estimé reste),
/// la liste des contrats est demandée une fois, et si la pièce reste inconnue un message clair le dit ; une fois la
/// taille apprise : l'orderflow réel.
#[test]
fn orderflow_bourse_a_contrats_taille_inconnue_puis_apprise() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Blofin,
        base: "ZZTEST".into(),
    };
    a.orderflow_suivre();
    assert_eq!(a.orderflow_cle(), None, "taille inconnue : pas de réel");
    assert!(
        a.orderflow_chart().is_empty() && a.orderflow_enregistre().is_none(),
        "l'estimé, pas de ● Enregistrer"
    );
    assert!(a.feeds.contrats_demandes.contains(&Exchange::Blofin), "liste des contrats demandée");
    a.orderflow_contrats_lus(Exchange::Blofin, Err("réseau coupé".into()));
    assert_eq!(
        a.banner.as_deref(),
        Some("🧮 BloFin ZZTEST : taille du contrat inconnue : footprint réel indisponible, estimé affiché (réseau coupé)")
    );
    argos_data::contracts::set_contracts("BloFin", vec![("ZZTEST".into(), 10.0)]);
    a.banner = None;
    a.orderflow_contrats_lus(Exchange::Blofin, Ok(()));
    assert_eq!(a.orderflow_cle().map(|c| c.symbole), Some("ZZTEST".to_string()), "taille apprise : le réel");
    assert!(a.feeds.orderflow.iter().any(|f| f.cle.exchange == "blofin" && f.cle.symbole == "ZZTEST"));
    assert_eq!(a.banner, None);
    effacer_series(&["blofin-zztest."]);
}

/// 🔑 Flux KuCoin indisponible (jeton refusé) : l'estimé à la place du réel et un message clair, une seule fois même si
/// les essais se répètent ; dès qu'il se connecte : le réel revient.
#[test]
fn orderflow_flux_indisponible_estime_et_message_puis_retour() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Kucoin,
        base: "SOL".into(),
    };
    a.orderflow_suivre();
    let t0 = now_s() / 60 * 60_000;
    a.on_user_event(UserEvent::Orderflow(Exchange::Kucoin, LiveEvent::Connected));
    a.on_user_event(UserEvent::Orderflow(
        Exchange::Kucoin,
        LiveEvent::Whales(vec![argos_data::whales::Whale {
            id: Some(7),
            src: "KuCoin",
            ..trade_hl("SOL", t0 + 1_000, 150.0, 3.0, true)
        }]),
    ));
    assert!(!a.orderflow_chart().is_empty(), "connecté : le réel");
    a.on_user_event(UserEvent::Orderflow(Exchange::Kucoin, LiveEvent::Indisponible("jeton refusé".into())));
    assert!(a.orderflow_chart().is_empty(), "indisponible : l'estimé");
    assert_eq!(a.banner.as_deref(), Some("🧮 KuCoin : flux indisponible, estimé affiché (jeton refusé)"));
    a.banner = None;
    a.on_user_event(UserEvent::Orderflow(Exchange::Kucoin, LiveEvent::Indisponible("jeton refusé".into())));
    assert_eq!(a.banner, None, "un seul message, même si les essais se répètent");
    a.on_user_event(UserEvent::Orderflow(Exchange::Kucoin, LiveEvent::Connected));
    assert!(!a.orderflow_chart().is_empty(), "reconnecté : le réel revient");
    effacer_series(&["kucoin-sol."]);
}

/// 🟠 PC (demande de Jee) : la pilule DÉMO est dans la 1re rangée de la barre du haut, centrée dans l'espace libre entre
/// la mise en page et la recherche ; elle ne croise aucun bouton de la barre ni les rangées dessous ; plus étroit, son
/// texte se raccourcit ; son ↺ ouvre toujours la confirmation.
#[test]
fn pilule_demo_dans_la_barre_du_haut() {
    let t = argos_render::text::Text::default();
    for largeur in [1330u32, 990] {
        let mut a = app();
        a.area = (largeur, 676);
        // (one frame: 🛟 Support is placed after the pill)
        a.draw();
        let (r, niveau) = a.bandeau_place();
        let w = a.top_w();
        assert!(r.w > 0.0, "{largeur} : la pilule a sa place");
        assert!(r.y >= 8.0 && r.y + r.h <= 52.0, "{largeur} : dans la 1re rangée {r:?}");
        let (x0, x1, _, _) = a.top.espace_libre(w, &t).unwrap();
        assert!(r.x >= x0 - 0.5 && r.x + r.w <= x1 + 0.5, "{largeur} : dans l'espace libre");
        assert!(((r.x + r.w / 2.0) - (x0 + x1) / 2.0).abs() < 1.0, "{largeur} : centrée");
        for x in (r.x as i32..(r.x + r.w) as i32).step_by(3).map(f64::from) {
            assert!(!a.top.zone_en(x, r.cy(), w, &t), "{largeur} : sur un bouton de la barre à {x}");
        }
        assert!(niveau <= 2);
        a.ui.sim = None;
        assert!(a.bandeau_click(r.x + r.w - 10.0, r.cy()), "↺");
        assert!(a.ui.sim.as_ref().is_some_and(|s| s.confirme));
    }
}

/// 🐢 Au repos (aucun geste depuis 2 s), les données en direct ne redessinent plus qu'une fois par seconde au plus : le
/// redessin retenu est noté et fait à la seconde suivante ; un geste rend les redessins immédiats.
#[test]
fn redessins_de_donnees_au_repos() {
    use std::time::Duration as D;
    assert!(
        !crate::app::redessin_donnees_retenu(D::from_millis(500), D::from_millis(10)),
        "geste récent : tout de suite"
    );
    assert!(crate::app::redessin_donnees_retenu(D::from_secs(3), D::from_millis(300)), "au repos : attend");
    assert!(
        !crate::app::redessin_donnees_retenu(D::from_secs(3), D::from_millis(1000)),
        "au repos, 1 s passée : redessine"
    );
    let mut a = app();
    let il_y_a = |s: u64| Instant::now().checked_sub(D::from_secs(s)).unwrap();
    a.ui.derniere_entree = il_y_a(5);
    a.ui.dernier_redessin_donnees = il_y_a(5);
    a.redraw_donnees();
    assert!(!a.ui.redessin_en_attente, "le premier passe");
    a.redraw_donnees();
    a.redraw_donnees();
    assert!(a.ui.redessin_en_attente, "les suivants attendent la seconde");
    a.ui.derniere_entree = Instant::now();
    a.redraw_donnees();
    assert!(!a.ui.redessin_en_attente, "après un geste : tout de suite");
}

/// 🖥 Colonne de modules à droite, `n` modules cochés, `h` de haut (sans fenêtre : sa place posée comme le dessin le fait).
fn colonne_modules(a: &mut App, n: usize, h: f64) -> Rect {
    use argos_render::dock::DockPos;
    a.watch.dock.on = true;
    a.watch.dock.pos = DockPos::Right;
    for (k, i) in a.watch.dock.items.iter_mut().enumerate() {
        i.on = k < n;
    }
    a.area = (1100, h as u32);
    (a.chart.viewport.width, a.chart.viewport.height) = (1100, h as u32);
    a.dock.right = 320.0;
    let r = Rect::new(a.rw() + a.area.0 as f64, a.top_h(), 320.0, h);
    a.dock.rect = Some(r);
    a.dock_mise_en_page();
    r
}

/// 🖥 5 modules à 600 de haut puis 10 à 900 (sans données en direct, les listes sont courtes) : chacun garde au moins
/// titre + une ligne, le total dépasse la colonne, la molette (sur un titre) la fait défiler jusqu'au dernier module, et
/// aucun module ne croise le bouton Argos AI.
#[test]
fn colonne_de_modules_defile() {
    dossier_a_soi("modules-defile");
    use argos_render::dock as dk;
    use winit::event::MouseScrollDelta;
    for (n, h) in [(5usize, 600.0), (10, 900.0)] {
        let mut a = app();
        let r = colonne_modules(&mut a, n, h);
        assert_eq!(a.dock.places.len(), n);
        assert!(a.dock.places.iter().all(|(_, p)| p.h >= dk::hauteur_min()), "{n} : aucun module écrasé");
        assert!(a.dock.total > r.h, "{n} : plus haut que la colonne ({})", a.dock.total);
        // la molette sur le titre du 1er module : la colonne défile jusqu'en bas
        a.mouse_moved(PhysicalPosition::new(r.x + 60.0, r.y + 10.0));
        assert!(a.pointer.dock_mouse.is_some());
        for _ in 0..200 {
            a.mouse_wheel(MouseScrollDelta::LineDelta(0.0, -1.0));
            a.dock_mise_en_page();
        }
        let max = dk::defil_max(r.h, a.dock.total);
        assert!((a.dock.defil - max).abs() < 0.01, "{n} : défilée jusqu'au bout");
        let (m, der) = *a.dock.places.last().unwrap();
        assert!((der.y + der.h - a.dock.defil - r.h).abs() < 0.01, "{n} : {m:?} arrive en bas de la colonne");
        assert!(a.dock.bodies.iter().any(|(x, _)| *x == m), "{n} : le dernier module est visible");
        // et elle remonte
        for _ in 0..200 {
            a.mouse_wheel(MouseScrollDelta::LineDelta(0.0, 1.0));
            a.dock_mise_en_page();
        }
        assert_eq!(a.dock.defil, 0.0);
        // ✨ Argos AI ne croise aucun module visible (ni la colonne)
        let ai = a.ai_bouton_rect();
        assert!(ai.x + ai.w <= r.x, "{n} : Argos AI à gauche de la colonne");
        for (_, b) in &a.dock.bodies {
            let b = Rect::new(b.x + r.x, b.y + r.y, b.w, b.h);
            assert!(ai.x + ai.w <= b.x || b.x + b.w <= ai.x || ai.y + ai.h <= b.y || b.y + b.h <= ai.y);
        }
    }
}

/// 🖥 Poignée sous un module : glisser change la hauteur de ce module seulement et la garde (interface-2.json) ; jamais
/// sous titre + une ligne ; double-clic : hauteur automatique ; Échap pendant le glisser : rien ne change.
#[test]
fn poignee_regle_la_hauteur_d_un_module() {
    dossier_a_soi("poignee-modules");
    use argos_render::dock as dk;
    let mut a = app();
    let r = colonne_modules(&mut a, 3, 900.0);
    let avant: Vec<f64> = a.dock.places.iter().map(|(_, p)| p.h).collect();
    let (m0, p0) = a.dock.places[0];
    let py = r.y + p0.y + p0.h + 3.0;
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py));
    a.left_button(ElementState::Pressed);
    assert!(matches!(a.dock.geste, Some(crate::state::GesteDock::Poignee { .. })), "la poignée est prise");
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py + 60.0));
    a.left_button(ElementState::Released);
    a.dock_mise_en_page();
    assert_eq!(a.dock.places[0].1.h, avant[0] + 60.0, "{m0:?} : 60 de plus");
    assert_eq!(a.dock.places[1].1.h, avant[1], "les autres ne bougent pas");
    assert_eq!(a.dock.places[2].1.h, avant[2]);
    assert_eq!(App::interface2().modules_hauteurs, vec![(m0, avant[0] + 60.0)], "gardée dans interface-2.json");
    // vers le haut, très loin : le minimum
    let py = r.y + a.dock.places[0].1.y + a.dock.places[0].1.h + 3.0;
    a.dock.poignee_avant = None;
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py - 2000.0));
    a.left_button(ElementState::Released);
    a.dock_mise_en_page();
    assert_eq!(a.dock.places[0].1.h, dk::hauteur_min());
    // Échap pendant le glisser : la hauteur d'avant revient
    let py = r.y + a.dock.places[0].1.y + a.dock.places[0].1.h + 3.0;
    a.dock.poignee_avant = None;
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py + 200.0));
    assert!(a.dock_annuler());
    a.left_button(ElementState::Released);
    a.dock_mise_en_page();
    assert_eq!(a.dock.places[0].1.h, dk::hauteur_min(), "Échap : rien n'a changé");
    // double-clic sur la poignée : automatique
    let py = r.y + a.dock.places[0].1.y + a.dock.places[0].1.h + 3.0;
    a.mouse_moved(PhysicalPosition::new(r.x + 100.0, py));
    for _ in 0..2 {
        a.left_button(ElementState::Pressed);
        a.left_button(ElementState::Released);
    }
    a.dock_mise_en_page();
    assert_eq!(a.dock.places[0].1.h, avant[0], "hauteur automatique");
    assert!(App::interface2().modules_hauteurs.is_empty());
}

/// 🖥 Glisser un titre de la place 1 à la place 3 : l'ordre change (comme ↑ ↓) et est gardé dans dock.json ; un clic
/// simple sur le titre ne change rien.
#[test]
fn glisser_un_titre_de_module_change_l_ordre() {
    dossier_a_soi("titre-modules");
    use argos_render::dock::{DockPrefs, Module};
    let mut a = app();
    let r = colonne_modules(&mut a, 3, 900.0);
    let ordre = |a: &App| a.watch.dock.shown().iter().map(|i| i.module).collect::<Vec<_>>();
    assert_eq!(ordre(&a), vec![Module::Book, Module::Trades, Module::Funding]);
    let t0 = r.y + a.dock.places[0].1.y + 12.0;
    // clic simple : rien
    a.mouse_moved(PhysicalPosition::new(r.x + 80.0, t0));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(r.x + 80.0, t0 + 2.0));
    a.left_button(ElementState::Released);
    assert_eq!(ordre(&a), vec![Module::Book, Module::Trades, Module::Funding], "clic simple : rien ne bouge");
    // glissé jusqu'au bas du 3e module
    let (_, p2) = a.dock.places[2];
    a.mouse_moved(PhysicalPosition::new(r.x + 80.0, t0));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(r.x + 80.0, r.y + p2.y + p2.h - 4.0));
    assert!(matches!(a.dock.geste, Some(crate::state::GesteDock::Titre { bouge: true, .. })));
    a.left_button(ElementState::Released);
    assert_eq!(ordre(&a), vec![Module::Trades, Module::Funding, Module::Book]);
    let lu: DockPrefs = App::read_prefs("dock.json");
    assert_eq!(lu.shown().iter().map(|i| i.module).collect::<Vec<_>>(), ordre(&a), "gardé dans dock.json");
}

/// 🖥 ✕ dans l'en-tête d'un module : ce module seulement est décoché (gardé dans dock.json) ; le ✕ du dernier masque la
/// colonne ; le titre, à côté du ✕, se glisse toujours.
#[test]
fn croix_d_un_module_le_decoche() {
    dossier_a_soi("croix-modules");
    use argos_render::dock::{self as dk, DockPrefs, Module};
    let mut a = app();
    let r = colonne_modules(&mut a, 3, 900.0);
    let ordre = |a: &App| a.watch.dock.shown().iter().map(|i| i.module).collect::<Vec<_>>();
    // zone de clic ≥ 24 px, dans l'en-tête, à gauche de ⋮⋮
    let (_, p1) = a.dock.places[1];
    let c = dk::croix_rect(p1);
    assert!(c.w >= 24.0 && c.h >= 24.0 && c.y >= p1.y && c.y + c.h <= p1.y + dk::HEAD_H);
    assert!(c.x + c.w <= p1.x + p1.w - 20.0, "le ✕ laisse la place à ⋮⋮");
    // le titre se glisse toujours (appui à côté du ✕, loin du ✕)
    let t0 = r.y + a.dock.places[0].1.y + 12.0;
    a.mouse_moved(PhysicalPosition::new(r.x + 80.0, t0));
    a.left_button(ElementState::Pressed);
    assert!(matches!(a.dock.geste, Some(crate::state::GesteDock::Titre { .. })), "le titre est pris");
    a.left_button(ElementState::Released);
    // ✕ du 2e module (Trades) : lui seul est décoché, sans glisser
    a.mouse_moved(PhysicalPosition::new(r.x + c.cx(), r.y + c.cy()));
    a.left_button(ElementState::Pressed);
    assert!(a.dock.geste.is_none(), "le ✕ ne commence pas un glisser");
    a.left_button(ElementState::Released);
    assert_eq!(ordre(&a), vec![Module::Book, Module::Funding]);
    assert!(a.watch.dock.on);
    let lu: DockPrefs = App::read_prefs("dock.json");
    assert_eq!(lu.shown().iter().map(|i| i.module).collect::<Vec<_>>(), ordre(&a), "gardé dans dock.json");
    // ✕ des deux derniers : la colonne est masquée
    for _ in 0..2 {
        a.dock_mise_en_page();
        let c = dk::croix_rect(a.dock.places[0].1);
        a.mouse_moved(PhysicalPosition::new(r.x + c.cx(), r.y + c.cy()));
        a.left_button(ElementState::Pressed);
        a.left_button(ElementState::Released);
    }
    assert!(ordre(&a).is_empty());
    assert!(!a.watch.dock.on, "dernier ✕ : colonne masquée");
    assert!(!App::read_prefs::<DockPrefs>("dock.json").on);
}

/// 🖥 La colonne de modules commence sous la barre du haut : la bande des rangées repliées et les rangées ouvertes au
/// survol s'arrêtent à son bord (comme pour un volet).
#[test]
fn bande_du_haut_s_arrete_a_la_colonne_de_modules() {
    dossier_a_soi("bande-modules");
    let mut a = app();
    let r = colonne_modules(&mut a, 3, 900.0);
    a.top.sans_bandeaux = true;
    let b = a.bande_haut().expect("bande");
    assert!(b.x + b.w <= r.x, "bande {b:?} / colonne {r:?}");
    let (_, d) = a.rangs_haut().unwrap();
    assert!(d <= r.x, "rangées ouvertes jusqu'à {d}");
    a.top.survol_bandeaux = true;
    let z = a.haut_survol_zone().unwrap();
    assert!(z.x + z.w <= r.x);
}

/// 📐 L'écran PC dans un état : largeur `w`, hauteur `h`, rangées du haut (0 ouvertes, 1 repliées, 2 repliées et
/// survolées), rail gardé ouvert, volet ouvert, colonne de modules, colonne d'icônes repliée, carte Lens (coin, taille) ;
/// avec Cipher, RSI et Argos Lens. Renvoie les chevauchements interdits.
#[allow(clippy::too_many_arguments)]
fn chevauchements_pc(
    w: u32,
    h: u32,
    haut: usize,
    rail_ouvert: bool,
    volet: usize,
    dock: Option<argos_render::dock::DockPos>,
    col_repliee: bool,
    lens: (usize, usize),
    liste: bool,
) -> Vec<String> {
    chevauchements_pc_zen(w, h, haut, rail_ouvert, volet, dock, col_repliee, lens, liste, 0, false)
}

/// 📐 The same, with 🧘 Zen (`zen`: 0 off, 1 on, 2 on with the top bar brought back at the top edge) and the
/// favourites bar shown or hidden (`sans_favoris`).
#[allow(clippy::too_many_arguments)]
fn chevauchements_pc_zen(
    w: u32,
    h: u32,
    haut: usize,
    rail_ouvert: bool,
    volet: usize,
    dock: Option<argos_render::dock::DockPos>,
    col_repliee: bool,
    lens: (usize, usize),
    liste: bool,
    zen: usize,
    sans_favoris: bool,
) -> Vec<String> {
    use argos_render::indicators::Ind;
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2 {
        rail_deplie: rail_ouvert,
        sans_bandeaux: haut > 0,
        colonne_repliee: col_repliee,
        ..Default::default()
    });
    a.top.survol_bandeaux = haut == 2;
    a.top.prefs.bienvenue_vue = true;
    // (volet : 0 fermé, 1 ouvert, 2 ⤢ large)
    a.watch.prefs.open = volet > 0;
    a.ui.volet_large = volet == 2;
    a.watch.prefs.panel = wl::Tool::Modules;
    a.watch.dock.on = dock.is_some();
    a.watch.dock.pos = dock.unwrap_or(argos_render::dock::DockPos::Right);
    for (k, i) in a.watch.dock.items.iter_mut().enumerate() {
        i.on = k < 5;
    }
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::Lens);
    (v[6], v[7]) = (lens.0 as f64, lens.1 as f64);
    a.ind.set.insert(Ind::Lens, v);
    a.ind.on = vec![Ind::Cipher, Ind::Rsi, Ind::Lens];
    // (⌄ liste des indicateurs sur le prix ouverte)
    a.ind.leg.0 = liste;
    (a.ui.zen, a.ui.zen_haut) = (zen > 0, zen == 2);
    if sans_favoris {
        a.tools.hidden = true;
    }
    // (comme l'app : chaque image place l'écran avec les indicateurs de l'image d'avant)
    for _ in 0..2 {
        a.placer(winit::dpi::PhysicalSize::new(w, h));
        a.refresh_indicators();
    }
    a.dock_mise_en_page();
    let _ = argos_render::render(&a.to_chart());
    let zones = a.zones_pc();
    let mut d = crate::actions::zones::chevauchements(&zones, w as f64, h as f64);
    if liste && !zones.iter().any(|(n, _, _)| n == "liste des indicateurs") {
        d.push("liste des indicateurs ouverte : absente des zones".into());
    }
    d
}

/// 📐 Rien ne se chevauche sur PC : à 1440, 1640 et 1966 de large, 900 et 760 de haut, rangées du haut ouvertes /
/// repliées / repliées et survolées, rail replié ou gardé ouvert, volet fermé, ouvert ou large (⤢), colonne de modules masquée / à
/// gauche / à droite / en bas, colonne d'icônes dépliée ou repliée ; puis la carte Lens dans ses 5 places × 3 tailles :
/// aucune paire de zones ne se croise, sauf les superpositions voulues (`actions::zones::chevauchements`), et la carte
/// Lens est toujours entière dans la zone des prix.
#[test]
fn rien_ne_se_chevauche_sur_pc() {
    use argos_render::dock::DockPos;
    let mut defauts: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    let mut n = 0;
    let mut note = |d: Vec<String>, etat: String| {
        for d in d {
            let e = defauts.entry(d).or_insert((0, etat.clone()));
            e.0 += 1;
        }
    };
    let docks = [None, Some(DockPos::Left), Some(DockPos::Right), Some(DockPos::Bottom)];
    for (w, h) in [(1440u32, 900u32), (1640, 900), (1966, 900), (1440, 760), (1966, 760)] {
        for haut in 0..3 {
            for rail in [false, true] {
                for volet in 0..3 {
                    for dock in docks {
                        for col in [false, true] {
                            let etat = format!(
                                "{w}×{h}, haut {}, rail {}, volet {}, modules {dock:?}, icônes {}",
                                ["ouvert", "replié", "survolé"][haut],
                                if rail { "ouvert" } else { "replié" },
                                ["fermé", "ouvert", "large"][volet],
                                if col { "repliées" } else { "dépliées" }
                            );
                            note(chevauchements_pc(w, h, haut, rail, volet, dock, col, (0, 0), false), etat);
                            n += 1;
                        }
                    }
                }
            }
        }
        // 🔎 la carte Lens : 5 places × 3 tailles
        for coin in 0..5 {
            for taille in 0..3 {
                for dock in [None, Some(DockPos::Right)] {
                    for col in [false, true] {
                        let etat = format!("{w}×{h}, Lens place {coin} taille {taille}, modules {dock:?}, icônes repliées {col}");
                        note(chevauchements_pc(w, h, 1, false, 0, dock, col, (coin, taille), false), etat);
                        n += 1;
                    }
                }
            }
        }
    }
    // ⌄ la liste des indicateurs sur le prix ouverte : la barre de favoris (à sa place par défaut) ne la cache pas
    for (w, h) in [(1440u32, 900u32), (1966, 900), (1440, 760)] {
        for haut in 0..3 {
            for dock in [None, Some(DockPos::Right)] {
                let etat = format!("{w}×{h}, haut {haut}, modules {dock:?}, liste des indicateurs ouverte");
                note(chevauchements_pc(w, h, haut, false, 0, dock, false, (0, 0), true), etat);
                n += 1;
            }
        }
    }
    let liste: Vec<String> = defauts.iter().map(|(d, (k, e))| format!("{d} — {k}/{n} états, ex. {e}")).collect();
    assert!(liste.is_empty(), "{} chevauchements :\n{}", liste.len(), liste.join("\n"));
}

/// 🔑 📋 CoinMarketCap key box (web): Ctrl + V is left to the browser, whose "paste" event brings the text to the
/// key field; the box is then complete (Save lit) and a second paste adds to the same field.
#[test]
fn boite_cle_cmc_recoit_le_texte_colle() {
    dossier_a_soi("boite-cle-cmc-coller");
    use crate::platform::touche_du_navigateur as nav;
    assert!(nav("v", true, false, false, true), "Ctrl + V reaches the browser, which fires the paste event");
    let mut a = app();
    a.cmc_key_box();
    let cle = |a: &App| {
        a.session
            .key_box
            .as_ref()
            .and_then(|b| b.fields.iter().find(|f| f.id == "key"))
            .map(|f| f.value.clone())
    };
    assert_eq!(cle(&a).as_deref(), Some(""), "box opens with an empty key field");
    assert!(!a.session.key_box.as_ref().unwrap().complete(), "Save stays grey while the key is empty");
    a.paste_text("0123456789abcdef".into());
    a.paste_text("fedcba9876543210".into());
    assert_eq!(
        cle(&a).as_deref(),
        Some("0123456789abcdeffedcba9876543210"),
        "pasted text lands in the key field"
    );
    assert!(a.session.key_box.as_ref().unwrap().complete(), "Save lights up once the key is pasted");
}

/// 🔑 Boîte « + Ajouter une API » ouverte : un clic à côté (chart, barre du haut, colonne d'icônes, colonne de modules) la
/// ferme et oublie ce qui a été tapé, sans rien faire d'autre (pas de volet ouvert, pas de menu, pas de dessin, pas de
/// module touché) ; un clic dans la boîte la garde ouverte.
#[test]
fn boite_cle_api_se_ferme_au_clic_a_cote() {
    dossier_a_soi("boite-cle");
    let mut a = app();
    let r = colonne_modules(&mut a, 3, 676.0);
    let w = a.win_w();
    let ouvrir = |a: &mut App| {
        let mut b = KeyBox::new("binance", "Binance", w, a.top_h() + a.area.1 as f64);
        b.type_text("cle-pas-gardee");
        a.session.key_box = Some(b);
    };
    let etat = |a: &App| {
        (
            a.watch.prefs.open,
            a.ui.list.is_some(),
            a.ui.context.is_open(),
            a.chart.drawings.len(),
            a.watch.dock.shown().iter().map(|i| i.module).collect::<Vec<_>>(),
            a.dock.geste.is_some(),
        )
    };
    // dans la boîte : elle reste
    ouvrir(&mut a);
    let br = a.session.key_box.as_ref().unwrap().rect;
    clic(&mut a, br.x + 20.0, br.y + 20.0);
    assert!(a.session.key_box.is_some(), "un clic dans la boîte la garde");
    let avant = etat(&a);
    let dehors = [
        (a.rw() + 200.0, a.top_h() + 200.0, "chart"),
        (300.0, 20.0, "barre du haut"),
        (w - 28.0, 37.0 + 60.0, "colonne d'icônes"),
        (r.x + 60.0, r.y + 10.0, "titre d'un module"),
    ];
    for (x, y, ou) in dehors {
        if br.contains(x, y) {
            continue;
        }
        ouvrir(&mut a);
        clic(&mut a, x, y);
        assert!(a.session.key_box.is_none(), "{ou} : la boîte se ferme");
        assert_eq!(etat(&a), avant, "{ou} : rien d'autre ne se passe");
    }
}

/// 🖥 L'app sur PC (1440 × 900, cadre d'interface-2) avec le volet `t` ouvert.
fn pc_avec_volet(t: wl::Tool) -> App {
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, t);
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    (a.watch.colonne, a.watch.col_h) = (a.colonne_pc(), a.area.1 as f64);
    a
}

/// 🖥 Chaque volet de droite a son ✕ (en haut à droite, comme la maquette) qui le ferme, et Échap le ferme quand rien
/// d'autre n'est ouvert par-dessus ; le rond Argos AI de l'en-tête est à gauche du ✕, pas dessous.
#[test]
fn chaque_volet_se_ferme_par_x_et_echap() {
    dossier_a_soi("volets-x");
    use wl::Tool::*;
    for t in [Watchlist, Indicators, Trading, Journal, Exchanges, Market, Agenda, News, Whales, Modules] {
        let mut a = pc_avec_volet(t);
        // (📈 Trading : la feuille de la maquette a son propre ✕, dans son en-tête)
        let x = if t == Trading {
            Rect::new(345.0 + a.watch.pw() - 390.0, 38.0, 30.0, 30.0)
        } else {
            a.watch.fermer_rect().expect("✕")
        };
        let ai = argos_render::panels::Panels::ai_rect(a.watch.pw());
        assert!(ai.x + ai.w <= x.x, "{t:?} : le rond AI est à gauche du ✕");
        // (🖥 volet flottant : sous la 1re rangée, à sa place à lui)
        let v = a.volet_rect().expect("volet flottant");
        assert!((v.y - crate::render::VOLET_HAUT).abs() < 0.01 && v.x + v.w <= a.win_w(), "{t:?} : {v:?}");
        let (px, py) = (v.x + x.cx(), v.y + x.cy());
        clic(&mut a, px, py);
        assert!(!a.watch.prefs.open, "{t:?} : ✕ ferme");
        let mut a = pc_avec_volet(t);
        a.run_action(ChartAction::CloseMenus);
        assert!(!a.watch.prefs.open, "{t:?} : Échap ferme");
    }
    // Échap ferme d'abord le menu ouvert par-dessus, le volet ensuite
    let mut a = pc_avec_volet(Trading);
    a.open_list(ListKind::Intervals, Rect::new(300.0, 100.0, 10.0, 10.0));
    assert!(a.ui.list.is_some());
    a.run_action(ChartAction::CloseMenus);
    assert!(a.ui.list.is_none() && a.watch.prefs.open, "le menu d'abord");
    a.run_action(ChartAction::CloseMenus);
    assert!(!a.watch.prefs.open);
}

/// 🔌 Volet API ouvert : un clic sur la chart le ferme sans rien faire d'autre ; 📈 volet Trading : il reste ouvert (on
/// clique la chart en travaillant, comme la maquette).
#[test]
fn volet_api_se_ferme_au_clic_sur_la_chart() {
    dossier_a_soi("volet-api");
    let mut a = pc_avec_volet(wl::Tool::Exchanges);
    a.set_tool("trend_line");
    let avant = (a.chart.drawings.len(), a.selected, a.symbol_label(), a.demo.positions.len());
    let (cx, cy) = (a.rw() + 300.0, a.top_h() + 250.0);
    clic(&mut a, cx, cy);
    assert!(!a.watch.prefs.open, "API fermé");
    assert_eq!(
        (a.chart.drawings.len(), a.selected, a.symbol_label(), a.demo.positions.len()),
        avant,
        "rien d'autre"
    );
    // la boîte de clé par-dessus : le premier clic ferme la boîte, pas le volet
    let mut a = pc_avec_volet(wl::Tool::Exchanges);
    let w = a.win_w();
    a.session.key_box = Some(KeyBox::new("binance", "Binance", w, a.top_h() + a.area.1 as f64));
    clic(&mut a, cx, cy);
    assert!(a.session.key_box.is_none() && a.watch.prefs.open, "la boîte d'abord");
    // Trading : reste ouvert
    let mut a = pc_avec_volet(wl::Tool::Trading);
    clic(&mut a, cx, cy);
    assert!(a.watch.prefs.open, "Trading reste ouvert");
}

/// 🏷️ Flèche ⌄ fermée : les noms des panneaux du bas se cachent aussi, sauf celui sous la souris (avec ses boutons) et
/// les panneaux repliés ; l'état ouvert / fermé est gardé dans interface-2.json (pas dans le fichier partagé).
#[test]
fn fleche_fermee_cache_les_noms_des_panneaux() {
    dossier_a_soi("fleche-legende");
    use argos_render::indicators::{set_replies, set_survol, Ind};
    use argos_render::indleg::{nom_visible, LegClick};
    set_survol(None);
    set_replies(&[]);
    assert!(nom_visible(Ind::Cipher, true), "liste ouverte : les noms");
    assert!(!nom_visible(Ind::Cipher, false), "fermée : plus de nom");
    set_survol(Some(Ind::Cipher));
    assert!(
        nom_visible(Ind::Cipher, false) && !nom_visible(Ind::Rsi, false),
        "au survol : seulement ce panneau"
    );
    set_survol(None);
    set_replies(&[Ind::Rsi]);
    assert!(nom_visible(Ind::Rsi, false), "replié : toujours son nom");
    set_replies(&[]);
    let mut a = app();
    a.ind.leg.0 = true;
    a.legend_click(LegClick::Toggle);
    assert!(!a.ind.leg.0);
    assert_eq!(App::interface2().legende_ouverte, Some(false), "gardé dans interface-2.json");
    let mut b = app();
    b.ind.leg.0 = true;
    b.appliquer_cadre(&App::interface2());
    assert!(!b.ind.leg.0, "relu au lancement");
}

/// 🔑 CoinMarketCap : avec la clé, le dernier relevé s'ajoute au panneau Marché (les valeurs gratuites restent) ; la clé
/// elle-même n'y est jamais ; sans clé, rien.
#[test]
fn cle_cmc_jamais_dans_le_panneau() {
    let mut a = app();
    a.session.cmc = Some("CLE-SECRETE-123".into());
    a.feeds.cmc_last = Some(argos_data::cmcap::CapPoint {
        t: now_s(),
        total: 3e12,
        btc_d: 60.0,
        eth_d: 10.0,
        usdc_d: Some(2.0),
        ..Default::default()
    });
    let gratuit = Some(argos_data::market::Global {
        total: 2.9e12,
        ..Default::default()
    });
    let mut m = argos_data::market::MarketInfo {
        global: gratuit.clone(),
        ..Default::default()
    };
    a.cmc_over(&mut m);
    assert!(m.cmc_cle && m.cmc.is_some(), "le relevé s'ajoute");
    assert_eq!(m.global, gratuit, "le marché gratuit reste tel quel");
    assert!(!format!("{m:?}").contains("CLE-SECRETE"), "la clé n'est jamais dans le panneau");
    a.session.cmc = None;
    let mut m = argos_data::market::MarketInfo::default();
    a.cmc_over(&mut m);
    assert!(!m.cmc_cle && m.cmc.is_none());
}

/// 🖥 ⤢ : le volet passe large (min(920, 62 % de la fenêtre)), gardé dans interface-2.json, la chart s'arrête toujours à
/// son bord (prix visibles) ; ⬓ : le module équivalent est coché et allumé dans la colonne de modules, le volet se
/// ferme ; pas de ⬓ sans module équivalent (API).
#[test]
fn volet_large_et_ancre_en_module() {
    use argos_render::dock::Module;
    dossier_a_soi("volet-large");
    let mut a = pc_avec_volet(wl::Tool::Watchlist);
    let normal = a.watch.pw();
    let v = a.volet_rect().unwrap();
    let r = a.watch.agrandir_rect().unwrap();
    clic(&mut a, v.x + r.cx(), v.y + r.cy());
    assert!(a.ui.volet_large && App::interface2().volet_large, "⤢ : large, gardé");
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    assert_eq!(a.watch.pw(), (1440.0f64 * 0.62).min(920.0).round());
    assert!(a.watch.pw() > normal);
    let v = a.volet_rect().unwrap();
    assert!(a.rw() + a.chart.viewport.width as f64 <= v.x + 0.5, "la chart s'arrête au bord du volet");
    // ⬓ : le module Watchlist dans la colonne, le volet fermé
    let r = a.watch.ancrer_rect().expect("⬓");
    clic(&mut a, v.x + r.cx(), v.y + r.cy());
    assert!(!a.watch.prefs.open, "volet fermé");
    assert!(a.watch.dock.on && a.watch.dock.items.iter().any(|i| i.module == Module::Watchlist && i.on));
    let lu: argos_render::dock::DockPrefs = App::read_prefs("dock.json");
    assert!(lu.items.iter().any(|i| i.module == Module::Watchlist && i.on), "gardé dans dock.json");
    // pas d'équivalent : pas de ⬓
    assert!(pc_avec_volet(wl::Tool::Exchanges).watch.ancrer_rect().is_none());
}

/// 📈 Volet Trading sur PC : la feuille de la maquette (Ticket / Simulateur, Gardien, VENDRE / ACHETER…) avec ses
/// propres ⬓ ⤢ ✕ (pas de doublon dans l'en-tête) ; « ⋯ Toutes les options » ouvre l'ancien ticket complet (rien n'est
/// retiré) ; ⤢ élargit le volet ; ⬓ met les positions démo dans la colonne de modules.
#[test]
fn volet_trading_pc_feuille_de_la_maquette() {
    use argos_render::dock::Module;
    dossier_a_soi("trading-pc");
    let mut a = pc_avec_volet(wl::Tool::Trading);
    assert!(a.watch.panels.feuille_trading(), "la feuille de la maquette");
    assert!(a.watch.fermer_rect().is_none() && a.watch.agrandir_rect().is_none(), "pas de doublon de ⬓ ⤢ ✕");
    let v = a.volet_rect().unwrap();
    let d = a.watch.pw() - 390.0;
    // ⤢ de la feuille
    clic(&mut a, v.x + 307.0 + d + 15.0, v.y + 38.0 + 15.0);
    assert!(a.ui.volet_large, "⤢ : large");
    a.ui.volet_large = false;
    // ⋯ Toutes les options : l'ancien ticket complet
    a.watch.panels.trading.options = true;
    assert!(
        !a.watch.panels.feuille_trading() && a.watch.fermer_rect().is_some(),
        "l'ancien ticket, avec le ✕ du volet"
    );
    a.watch.panels.trading.options = false;
    // ⬓ de la feuille : les positions démo dans la colonne de modules
    clic(&mut a, v.x + 269.0 + d + 15.0, v.y + 38.0 + 15.0);
    assert!(!a.watch.prefs.open, "volet fermé");
    assert!(a.watch.dock.items.iter().any(|i| i.module == Module::Positions && i.on));
}

/// 🎫 Module 31 (« ok maquette module 31 ») : le ticket de la feuille Trading passe de vrais ordres démo — au marché
/// avec TP et SL (notés au journal), sans SL (noté sans), bloqué par le Gardien sans SL, une Limite qui attend, un
/// Reduce-only qui réduit la position ; ses champs se tapent dans leur boîte ; ⌖ prend le prix sur la chart ; le
/// curseur % suit la souris jusqu'au lâcher ; « Carnet › » allume le module Carnet ; ses lignes se voient sur la chart.
#[test]
fn ticket_module31_de_bout_en_bout() {
    use argos_render::panels::ticket::Champ;
    use argos_render::wlmenu::NameFor;
    dossier_a_soi("ticket-module31");
    let mut a = pc_avec_volet(wl::Tool::Trading);
    let last = a.chart.bars.last().unwrap().close;
    // (each frame: the last price, then what the ticket shows)
    let image = |a: &mut App| {
        a.watch.panels.trading.prix = a.chart.bars.last().unwrap().close;
        a.ticket_sync();
    };
    image(&mut a);
    a.watch.panels.trading.guardian = false;
    assert!(a.watch.panels.trading.pouvoir > 0.0 && a.watch.panels.trading.estimate.max_qty > 0.0);
    // nothing leaves without a size
    a.passer_ticket(true);
    assert!(a.demo.positions.is_empty() && a.banner.as_deref().is_some_and(|b| b.contains("Entre une taille")));
    // 1. at market, 5 % of the buying power, TP/SL ticked (by default): a long with its SL and TP, in the journal
    a.watch.panels.trading.set_pct(5.0);
    a.passer_ticket(true);
    assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
    let p = a.demo.positions[0].clone();
    assert!(p.stop.is_some_and(|s| s < p.entry) && p.target.is_some_and(|t| t > p.entry), "{p:?}");
    let j = a.watch.panels.journal.trades.last().unwrap();
    assert_eq!((j.sl, j.tp), (p.stop, p.target));
    // 2. TP/SL unticked: a long without SL (Argos AI will say « Entré sans stop »), noted without SL
    a.watch.panels.trading.tpsl = false;
    a.passer_ticket(true);
    assert_eq!(a.demo.positions.len(), 2, "{:?}", a.banner);
    assert!(a.demo.positions[1].stop.is_none() && a.banner.as_deref().is_some_and(|b| b.contains("sans SL")));
    assert_eq!(a.watch.panels.journal.trades.last().unwrap().sl, None);
    // 3. 🛡 the Gardien on: no order without SL
    a.watch.panels.trading.guardian = true;
    a.passer_ticket(true);
    assert_eq!(a.demo.positions.len(), 2);
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("SL obligatoire")), "{:?}", a.banner);
    a.watch.panels.trading.guardian = false;
    // 4. a Limit under the price: it waits (GTC)
    (a.watch.panels.trading.tab, a.watch.panels.trading.px, a.watch.panels.trading.tpsl) = (1, last - 10.0, true);
    image(&mut a);
    let o = a.watch.panels.trading.ordre(true);
    assert!(o.sl.is_some() && o.tp.is_some());
    a.passer_ticket(true);
    assert_eq!(a.demo.pending.len(), 1, "{:?}", a.banner);
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("GTC")), "{:?}", a.banner);
    // its lines on the chart: Limite, TP, SL
    let v = a.vue_ordres().ticket.expect("the ticket's lines");
    assert_eq!((v.entree, v.buy), (Some(("Limite", last - 10.0)), true));
    // 5. reduce-only at market: a sell takes at most the longs off, it closes them (noted in the journal)
    let longs: f64 = a.demo.positions.iter().map(|p| p.qty).sum();
    (a.watch.panels.trading.tab, a.watch.panels.trading.reduce_only) = (0, true);
    image(&mut a);
    a.watch.panels.trading.set_qty(longs * 10.0);
    assert!((a.watch.panels.trading.qty - longs).abs() < 1e-6, "at most the position");
    a.passer_ticket(false);
    assert!(a.demo.positions.is_empty(), "{:?} {:?}", a.banner, a.demo.positions);
    assert!(a
        .watch
        .panels
        .journal
        .trades
        .iter()
        .filter(|t| t.id.starts_with("demo-"))
        .all(|t| t.exit.is_some()));
    a.watch.panels.trading.reduce_only = false;
    // 6. a field typed in its box; a wrong value says why
    a.saisir_ticket(Champ::Quantite);
    assert_eq!(a.ui.name_box.as_ref().map(|b| b.kind), Some(NameFor::Ticket(Champ::Quantite)));
    a.ui.name_box.as_mut().unwrap().value = "0,5".into();
    a.name_ok();
    assert_eq!(a.watch.panels.trading.qty, 0.5);
    a.saisir_ticket(Champ::Prix);
    a.ui.name_box.as_mut().unwrap().value = "abc".into();
    a.name_ok();
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("écris un nombre")), "{:?}", a.banner);
    // 7. ⌖ then a touch on the chart: the order's price takes its level
    a.watch.panels.trading.tab = 1;
    a.watch.panels.trading.cible = true;
    // (a free place of the chart: no bar of favourites, no card over it)
    let (x, y) = (500.0, 400.0);
    assert!(!a.couche_haute(x + a.rw(), y + a.top_h()));
    let attendu = a.scale_now().unwrap().price_of(y);
    clic_chart(&mut a, x, y);
    let t = &a.watch.panels.trading;
    assert!(
        !t.cible && (t.px - attendu).abs() <= argos_render::position::tick_of(&a.chart.bars),
        "{} ≠ {attendu}",
        t.px
    );
    // 8. the % slider: pressed, it follows the mouse, until let go
    let v = a.volet_rect().unwrap();
    let pw = a.watch.pw();
    let tr = a.watch.panels.ticket_curseur(pw);
    assert!(a.watch.sur_curseur_ticket(tr.x + 5.0, tr.cy(), v.h), "a finger put there drags it");
    a.mouse_moved(PhysicalPosition::new(v.x + tr.x + tr.w * 0.1, v.y + tr.cy()));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(v.x + tr.x + tr.w * 0.6, v.y + tr.cy() + 40.0));
    assert_eq!(a.watch.panels.trading.pct, 60.0, "it follows, even beside the track");
    a.left_button(ElementState::Released);
    a.mouse_moved(PhysicalPosition::new(v.x + tr.x + tr.w * 0.9, v.y + tr.cy()));
    assert_eq!(a.watch.panels.trading.pct, 60.0, "let go: it stays");
    // 9. « Carnet › »: the Book module in the column
    a.ticket_carnet();
    assert!(a.watch.dock.on && a.watch.dock.items.iter().any(|i| i.module == argos_render::dock::Module::Book && i.on));
}

/// 🔑 Volet Marché crypto : le bouton « 🔑 Clé API » de l'en-tête ouvre directement la boîte de la clé CoinMarketCap
/// (plus la liste du volet API) ; la ligne « 🔑 Ajoute ta clé… » aussi.
#[test]
fn cle_du_marche_ouvre_la_boite_cmc() {
    dossier_a_soi("cle-marche");
    let mut a = pc_avec_volet(wl::Tool::Market);
    let v = a.volet_rect().unwrap();
    let k = argos_render::cmc::key_rect(a.watch.pw(), argos_render::watchlist::FERME_PLACE);
    clic(&mut a, v.x + k.cx(), v.y + k.cy());
    assert_eq!(a.session.key_box.as_ref().map(|b| b.exchange.as_str()), Some("cmc"), "la boîte CoinMarketCap");
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Market, "pas de détour par le volet API");
    a.session.key_box = None;
    let l = argos_render::cmc::cle_rect(a.watch.pw(), argos_render::watchlist::FERME_PLACE);
    clic(&mut a, v.x + l.cx(), v.y + l.cy());
    assert_eq!(a.session.key_box.as_ref().map(|b| b.exchange.as_str()), Some("cmc"));
}

/// 🖥 Jee (20 Nov): in the module column, « 🔑 Ajoute ta clé CoinMarketCap pour plus » did nothing; it (and « 🔑 Clé
/// API ») opens the CoinMarketCap key box, as in the panel.
#[test]
fn cle_du_marche_dans_la_colonne_ouvre_la_boite_cmc() {
    use argos_render::dock::Module;
    dossier_a_soi("cle-marche-colonne");
    let mut a = app();
    colonne_modules(&mut a, 0, 900.0);
    for i in a.watch.dock.items.iter_mut() {
        i.on = i.module == Module::Market;
    }
    a.dock_mise_en_page();
    let (_, b) = *a.dock.bodies.iter().find(|(m, _)| *m == Module::Market).expect("module Marché crypto");
    assert!(!a.watch.market.cmc_cle);
    let l = argos_render::cmc::cle_rect(b.w, 0.0);
    a.dock_click(b.x + l.cx(), b.y + l.cy());
    assert_eq!(a.session.key_box.as_ref().map(|k| k.exchange.as_str()), Some("cmc"), "the key line");
    a.session.key_box = None;
    let k = argos_render::cmc::key_rect(b.w, 0.0);
    a.dock_click(b.x + k.cx(), b.y + k.cy());
    assert_eq!(a.session.key_box.as_ref().map(|k| k.exchange.as_str()), Some("cmc"), "🔑 Clé API");
    a.session.key_box = None;
    // elsewhere in the module: nothing
    a.dock_click(b.x + 20.0, b.y + 300.0);
    assert!(a.session.key_box.is_none());
}

/// 🛡 Pastille Gardien de la barre du haut : le volet Trading s'ouvre sur la feuille de la maquette (même si l'ancien
/// ticket était ouvert), en haut, avec la carte du Gardien et « 🛡 Activer » quand il est éteint. Ancien ticket (⋯ Toutes
/// les options) : Marchés ouvre le volet Marché crypto, Trade revient à la feuille, Compte ouvre les comptes démo.
#[test]
fn gardien_et_onglets_de_l_ancien_ticket() {
    use argos_render::panels::{Kind, PanelClick};
    dossier_a_soi("gardien-onglets");
    let mut a = pc_avec_volet(wl::Tool::Watchlist);
    a.watch.prefs.open = false;
    a.watch.panels.trading.options = true;
    a.watch.panels.trading.guardian = false;
    a.top_click_do(argos_render::topbar::TopClick::Gardien);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading);
    assert!(!a.watch.panels.trading.options, "la feuille, pas l'ancien ticket");
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    assert!(a.watch.panels.feuille_trading(), "carte du Gardien en tête de la feuille");
    // ancien ticket : les onglets du bas (zones sous la hauteur `h`)
    let (pw, h) = (a.watch.pw(), 800.0);
    a.watch.panels.trading.options = true;
    let y = h - 58.0 + 18.0;
    // (3 onglets sur la largeur utile : bords de 16, écart de 8)
    let bw = (pw - 32.0 - 16.0) / 3.0;
    let x = |k: f64| 16.0 + k * (bw + 8.0) + bw / 2.0;
    assert_eq!(a.watch.panels.click(Kind::Trading, x(2.0), y, pw, h), PanelClick::Accounts, "Compte");
    assert_eq!(a.watch.panels.click(Kind::Trading, x(0.0), y, pw, h), PanelClick::Marches, "Marchés");
    a.watch.panels.click(Kind::Trading, x(1.0), y, pw, h);
    assert!(!a.watch.panels.trading.options, "Trade : retour à la feuille");
}

/// 🅘 PC : ƒx de la barre du haut ouvre la page Indicateurs de la maquette dans le volet flottant (la chart s'arrête à
/// son bord) ; un 2e ƒx la ferme ; ouvrir un autre volet la ferme aussi.
#[test]
fn fx_ouvre_le_volet_indicateurs() {
    use argos_render::topbar::TopClick;
    dossier_a_soi("fx-volet");
    let mut a = pc_avec_volet(wl::Tool::Watchlist);
    a.watch.prefs.open = false;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    let fx = || TopClick::Indicateurs(Rect::new(230.0, 20.0, 20.0, 20.0));
    a.top_click_do(fx());
    assert!(a.ui.fx_page.is_some() && a.ui.fx_dans_volet, "la page Indicateurs");
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Indicators, "dans le volet");
    a.placer(taille);
    let v = a.fx_rect().expect("à la place du volet flottant");
    assert!((v.y - crate::render::VOLET_HAUT).abs() < 0.01);
    assert!(a.rw() + a.chart.viewport.width as f64 <= v.x + 0.5, "la chart s'arrête à son bord");
    a.top_click_do(fx());
    assert!(a.ui.fx_page.is_none() && !a.watch.prefs.open, "2e ƒx : fermé");
    a.top_click_do(fx());
    a.watch.prefs.panel = wl::Tool::Trading;
    a.placer(taille);
    assert!(a.ui.fx_page.is_none(), "un autre volet : la page part");
}

/// 🔎 PC : le volet « Symboles » de la maquette (« Ma watchlist › » du panneau des marchés) dans le volet flottant ; un symbole de la liste
/// s'ouvre et va dans les récents ; ⋯ Toutes les options laisse la watchlist d'avant ; un 2e clic ferme.
#[test]
fn symbole_ouvre_le_volet_symboles() {
    dossier_a_soi("symboles-volet");
    let mut a = pc_avec_volet(wl::Tool::Watchlist);
    a.watch.prefs.open = false;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    a.symboles_volet();
    assert!(a.ui.symboles.is_some() && a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Watchlist);
    a.placer(taille);
    let v = a.sym_rect().expect("à la place du volet flottant");
    assert!((v.y - crate::render::VOLET_HAUT).abs() < 0.01);
    let d = a.sym_donnees();
    // ⋯ Toutes les options : la watchlist d'avant reste ouverte dans le volet
    let mut s = a.ui.symboles.clone().unwrap();
    let mut y = 120.0;
    while y < v.h && !matches!(s.click(40.0, y, v.w, v.h, &d), argos_render::symboles::SymClick::Options) {
        y += 4.0;
    }
    assert!(y < v.h, "⋯ Toutes les options trouvé");
    clic(&mut a, v.x + 40.0, v.y + y);
    assert!(a.ui.symboles.is_none() && a.watch.prefs.open, "la watchlist d'avant");
    a.symboles_volet();
    a.placer(taille);
    a.symboles_volet();
    assert!(a.ui.symboles.is_none() && !a.watch.prefs.open, "2e clic : fermé");
}

/// ⚙ PC : ⚙ ouvre la fenêtre « Réglages » de la maquette contre la colonne d'icônes ; « ⋯ Toutes les options » ouvre
/// les Panneaux d'avant (toutes les tuiles) ; un clic à côté la ferme.
#[test]
fn reglages_ouvre_la_fenetre_de_la_maquette() {
    use argos_render::reglages::RegClick;
    dossier_a_soi("reglages-fenetre");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    a.reglages_ouvrir();
    a.placer(taille);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Reglages, "⚙ ouvre le volet Réglages");
    let r = a.ui.reglages.clone().expect("volet Réglages").rect();
    assert_eq!(Some(r), a.volet_rect(), "à la place et à la taille du volet flottant");
    assert!(a.rw() + a.chart.viewport.width as f64 <= r.x + 0.5, "la chart s'arrête à son bord");
    // un clic à côté (sur la chart) : le volet reste
    clic(&mut a, r.x - 300.0, r.y + 300.0);
    assert!(a.ui.reglages.is_some() && a.watch.prefs.open, "à côté : il reste ouvert");
    // ⤢ : volet large
    clic(&mut a, r.x + r.w - 63.0, r.y + 29.0);
    a.placer(taille);
    assert!(a.ui.volet_large && a.ui.reglages.as_ref().unwrap().rect().w > r.w, "⤢ : plus large");
    let large = a.ui.reglages.as_ref().unwrap().rect();
    clic(&mut a, large.x + large.w - 63.0, r.y + 29.0);
    assert!(!a.ui.volet_large, "⤢ encore : normal");
    a.placer(taille);
    // « ⋯ Toutes les options » : tout en bas
    let mut g = a.ui.reglages.clone().unwrap();
    g.wheel(10_000.0);
    a.ui.reglages = Some(g.clone());
    let mut y = r.y + r.h - 4.0;
    while y > r.y && g.click(r.x + 40.0, y) != RegClick::Options {
        y -= 2.0;
    }
    clic(&mut a, r.x + 40.0, y);
    assert!(a.ui.reglages.is_none() && a.ui.sheet.is_some(), "les Panneaux d'avant");
    a.ui.sheet = None;
    // ✕ ferme le volet
    a.reglages_ouvrir();
    a.placer(taille);
    let r = a.ui.reglages.as_ref().unwrap().rect();
    clic(&mut a, r.x + r.w - 29.0, r.y + 29.0);
    assert!(a.ui.reglages.is_none() && !a.watch.prefs.open, "✕ : fermé");
}

/// 🔑 Boîte « Clé API — CoinMarketCap » ouverte devant une recherche restée ouverte (volet Symboles) : coller (Ctrl + V
/// de la page, menu Coller du cell) et Effacer vont à la boîte, pas à la recherche ; Enregistrer est grisé tant que la
/// clé est vide (le lien et la fréquence sont déjà remplis), actif dès qu'elle est collée. Fausse clé de test.
#[test]
fn coller_va_a_la_boite_de_cle_d_abord() {
    dossier_a_soi("coller-cle");
    let mut a = app();
    a.ui.symboles = Some(argos_render::symboles::Symboles {
        recherche: Some(String::new()),
        ..Default::default()
    });
    a.cmc_key_box();
    assert!(!a.session.key_box.as_ref().unwrap().complete(), "clé vide : Enregistrer grisé");
    a.paste_text("cle-de-test-0000-aaaa-bbbb".into());
    let b = a.session.key_box.as_ref().unwrap();
    assert_eq!(b.fields[b.focus].value, "cle-de-test-0000-aaaa-bbbb", "le texte collé va à la clé");
    assert_eq!(a.ui.symboles.as_ref().unwrap().recherche.as_deref(), Some(""), "pas à la recherche");
    assert!(b.complete(), "clé collée : Enregistrer actif");
    a.soft_key("Backspace");
    let b = a.session.key_box.as_ref().unwrap();
    assert_eq!(b.fields[b.focus].value, "cle-de-test-0000-aaaa-bbb", "Effacer va à la clé");
    a.soft_key("Escape");
    assert!(a.session.key_box.is_none());
}

/// ⚙ La colonne de modules ouverte (à droite, à gauche), rangées du haut ouvertes ou repliées (survolées), avec ou
/// sans volet ouvert, à 1440 et 2368 de large : un clic sur le ⚙ de la barre du haut ouvre Réglages, et la fenêtre
/// Réglages ne passe pas sous la colonne.
#[test]
fn reglages_s_ouvre_avec_la_colonne_de_modules() {
    use argos_render::dock::DockPos;
    use argos_render::topbar::{TopClick, TopMenu};
    dossier_a_soi("reglages-colonne");
    let mut rates = vec![];
    for (w0, h0) in [(1440u32, 900u32), (2368, 1155)] {
        for pos in [DockPos::Right, DockPos::Left] {
            for replies in [false, true] {
                for volet in [None, Some(wl::Tool::Modules), Some(wl::Tool::Watchlist)] {
                    let cas = format!("{w0} {pos:?} replié {replies} volet {volet:?}");
                    let mut a = app();
                    a.appliquer_cadre(&crate::actions::Interface2::default());
                    a.top.prefs.bienvenue_vue = true;
                    a.top.sans_bandeaux = replies;
                    (a.watch.prefs.open, a.watch.prefs.panel) = (volet.is_some(), volet.unwrap_or(wl::Tool::Watchlist));
                    a.watch.dock.on = true;
                    a.watch.dock.pos = pos;
                    for (k, i) in a.watch.dock.items.iter_mut().enumerate() {
                        i.on = k < 3;
                    }
                    let taille = winit::dpi::PhysicalSize::new(w0, h0);
                    a.placer(taille);
                    let Some(d) = a.dock.rect else {
                        rates.push(format!("{cas} : pas de colonne"));
                        continue;
                    };
                    // rangées repliées : la souris passe d'abord sur leur bande (elles s'ouvrent)
                    if let Some(b) = a.haut_survol_zone() {
                        a.mouse_moved(PhysicalPosition::new(b.cx(), b.cy()));
                        a.placer(taille);
                    }
                    // le ⚙ de la barre du haut (en partant de la droite)
                    let text = argos_render::text::Text::default();
                    let w = a.top_w();
                    let mut gear = None;
                    'y: for y in (4..48).step_by(4).map(f64::from) {
                        for x in (0..w as i32).rev().step_by(3).map(f64::from) {
                            if matches!(a.top.clone().click(x, y, w, &text, &|_| false), TopClick::Menu(TopMenu::Settings, _)) {
                                gear = Some((x, y));
                                break 'y;
                            }
                        }
                    }
                    let Some((gx, gy)) = gear else {
                        rates.push(format!("{cas} : pas de ⚙"));
                        continue;
                    };
                    a.mouse_moved(PhysicalPosition::new(gx, gy));
                    a.placer(taille);
                    clic(&mut a, gx, gy);
                    a.placer(taille);
                    if !(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Reglages && a.ui.reglages.is_some()) {
                        rates.push(format!("{cas} : ⚙ ({gx}, {gy}) n'ouvre pas Réglages (colonne {d:?})"));
                        continue;
                    }
                    let r = a.ui.reglages.as_ref().unwrap().rect();
                    if let Some(d2) = a.dock.rect {
                        if r.x < d2.x + d2.w && d2.x < r.x + r.w && r.y < d2.y + d2.h && d2.y < r.y + r.h {
                            rates.push(format!("{cas} : Réglages {r:?} sous la colonne {d2:?}"));
                        }
                    }
                }
            }
        }
    }
    assert!(rates.is_empty(), "{}", rates.join("\n"));
}

/// 📐 Chevauchements (Jee) : dans les fenêtres qui descendent jusqu'en bas à droite (⚙ Réglages, 🔔), la dernière
/// ligne, défilée jusqu'au bout, ne passe jamais sous le rond Argos AI ; à 3 hauteurs de fenêtre.
#[test]
fn derniere_ligne_jamais_sous_le_rond_argos_ai() {
    dossier_a_soi("fin-de-liste-ai");
    let croise = |a: Rect, b: Rect| a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h;
    for haut in [600, 760, 900] {
        let mut a = app();
        let taille = winit::dpi::PhysicalSize::new(1440, haut);
        a.placer(taille);
        a.draw();
        let ai = a.ai_bouton_rect();
        // ⚙ Réglages (dans le volet)
        a.reglages_ouvrir();
        a.placer(taille);
        let mut g = a.ui.reglages.clone().unwrap();
        g.wheel(100_000.0);
        let l = g.derniere_ligne();
        assert!(!croise(l, ai), "Réglages ({haut}) : « ⋯ Toutes les options » {l:?} sous le rond {ai:?}");
        // la fenêtre sur toute la hauteur dispo : la ligne ne sort pas en bas
        assert!(l.y + l.h <= g.rect().y + g.rect().h, "Réglages ({haut}) : dans la fenêtre");
        a.ui.reglages = None;
        a.watch.prefs.open = false;
        a.placer(taille);
        // 🔔 (beaucoup d'événements : le fil défile)
        for k in 0..40 {
            a.alerts.history.push(argos_core::alerts::Fired {
                id: k,
                ts: now_s() - 60 * k as i64,
                text: format!("Alerte {k}"),
            });
        }
        a.notifs_pop(Rect::new(260.0, 12.0, 32.0, 36.0));
        a.placer(taille);
        let ev = a.notif_evenements();
        let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
        let mut n = a.ui.notifs.clone().unwrap();
        n.wheel(100_000.0, &ev);
        let r = n.place(w, h, &ev);
        let t = argos_render::text::Text::default();
        let opt = n
            .zones(&t, r, &ev)
            .into_iter()
            .find(|(_, c)| *c == argos_render::notifs::NotifClick::Options)
            .unwrap()
            .0;
        assert!(!croise(opt, ai), "🔔 ({haut}) : « ⋯ Toutes les options » {opt:?} sous le rond {ai:?}");
        assert!(opt.y + opt.h <= r.y + r.h, "🔔 ({haut}) : dans la fenêtre");
    }
}

/// 🖼 Une image de l'écran entier (sans fenêtre), à `w` × `h` : l'image (0xRRGGBB) et les zones nommées de l'écran.
fn image_ecran(a: &mut App, w: u32, h: u32) -> (Vec<u32>, Vec<(String, Rect)>) {
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(w, h));
    a.ui.frames = 5;
    // (deux images : la 1re place l'écran avec les indicateurs, la 2e est la bonne)
    a.draw_frame();
    a.draw_frame();
    let zones = a.zones_pc().into_iter().map(|(n, _, r)| (n, r)).collect();
    (std::mem::take(&mut a.ui.image_test), zones)
}

/// 🖼 Compare une image de l'écran à sa référence, zone par zone ; `masque` : pixels qui bougent seuls (horloge, compte
/// à rebours), ignorés. Renvoie les zones changées.
fn comparer_ecran(nom: &str, img: &[u32], masque: &[bool], w: u32, h: u32, zones: &[(String, Rect)]) -> Vec<String> {
    let dossier = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("references");
    std::fs::create_dir_all(&dossier).unwrap();
    let chemin = dossier.join(format!("{nom}.png"));
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    for (p, v) in pm.pixels_mut().iter_mut().zip(img) {
        *p = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
    }
    if std::env::var("ARGOS_REFERENCES").is_ok_and(|v| v == "maj") || !chemin.exists() {
        pm.save_png(&chemin).unwrap();
        eprintln!("🖼 référence écrite : {}", chemin.display());
        return vec![];
    }
    let r = tiny_skia::Pixmap::load_png(&chemin).unwrap();
    if (r.width(), r.height()) != (w, h) {
        return vec![format!("{nom} : taille changée")];
    }
    let diff = |i: usize| {
        let (a, b) = (r.pixels()[i], pm.pixels()[i]);
        !masque[i] && (a.red().abs_diff(b.red()) > 12 || a.green().abs_diff(b.green()) > 12 || a.blue().abs_diff(b.blue()) > 12)
    };
    let mut changees: Vec<String> = vec![];
    let mut dedans = vec![false; img.len()];
    for (z, q) in zones {
        let n = (q.y.max(0.0) as u32..((q.y + q.h) as u32).min(h))
            .flat_map(|y| (q.x.max(0.0) as u32..((q.x + q.w) as u32).min(w)).map(move |x| (y * w + x) as usize))
            .filter(|&i| {
                dedans[i] = true;
                diff(i)
            })
            .count();
        if n > 0 {
            changees.push(format!("{nom} · {z} : {n} pixels"));
        }
    }
    let ailleurs = (0..img.len()).filter(|&i| !dedans[i] && diff(i)).count();
    if ailleurs > 0 {
        changees.push(format!("{nom} · hors des zones nommées : {ailleurs} pixels"));
    }
    if !changees.is_empty() {
        let _ = pm.save_png(dossier.join(format!("{nom}.nouveau.png")));
    }
    changees
}

/// 🖼 Images de référence de l'ÉCRAN ENTIER (Jee, 16 nov. : « à chaque fois que tu fais de quoi tu changes des affaires
/// ailleurs ») à 1440 × 900 et 2368 × 1155 : (a) repos, rangées du haut repliées, 3 panneaux (Cipher, TBO, AI Trend),
/// légende, échelles, annonces de l'Agenda, carte Lens ; (b) rangées ouvertes ; (c) volet Trading ; (d) colonne de
/// modules. Comparées zone par zone (barre du haut, rail, chart, panneaux, légende, échelle, colonne, volet…) : le
/// message dit quelle zone a bougé. Changement voulu : ARGOS_REFERENCES=maj, regarder, commiter les images.
#[test]
fn images_de_reference_de_l_ecran() {
    use argos_render::indicators::Ind;
    let scenes: [(&str, fn(&mut App)); 4] = [
        ("repos", |a| a.top.sans_bandeaux = true),
        ("rangees-ouvertes", |a| a.top.sans_bandeaux = false),
        ("volet-trading", |a| (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Trading)),
        ("colonne-modules", |a| {
            a.watch.dock.on = true;
            a.watch.dock.pos = argos_render::dock::DockPos::Right;
            for (k, i) in a.watch.dock.items.iter_mut().enumerate() {
                i.on = k < 3;
            }
        }),
    ];
    // le temps figé (horloge, compte à rebours) : juste après la dernière bougie
    crate::types::NOW_FIGE.with(|n| n.set(Some(START + 200 * 1800 + 600)));
    let mut changees = vec![];
    for (w, h) in [(1440u32, 900u32), (2368, 1155)] {
        for (nom, f) in scenes {
            dossier_a_soi(&format!("ref-ecran-{nom}-{w}"));
            let neuve = || {
                let mut a = app();
                a.appliquer_cadre(&crate::actions::Interface2::default());
                a.top.prefs.bienvenue_vue = true;
                a.watch.prefs.open = false;
                a.ind.on = vec![Ind::Cipher, Ind::Tbo, Ind::AiTrend, Ind::Lens];
                let t = |k: i64| START + k * 1800;
                let ev = |c: &str, ts: i64| argos_data::eco::Event {
                    title: "Annonce".into(),
                    country: c.into(),
                    impact: "High".into(),
                    ts,
                    forecast: String::new(),
                    previous: String::new(),
                    actual: String::new(),
                };
                a.watch.eco.agenda = vec![ev("USD", t(150)), ev("CAD", t(170)), ev("EUR", t(185))];
                a
            };
            let mut a = neuve();
            f(&mut a);
            let (img, zones) = image_ecran(&mut a, w, h);
            // (le temps est figé : rien ne bouge seul ; le masque reste pour une zone animée à venir)
            let masque = vec![false; img.len()];
            let nom = format!("ecran-{nom}-{w}");
            let mut c = comparer_ecran(&nom, &img, &masque, w, h, &zones);
            if !c.is_empty() {
                // (un réglage global changé au même moment par un autre test : on redessine une fois avant de conclure)
                let mut b = neuve();
                f(&mut b);
                let (img3, _) = image_ecran(&mut b, w, h);
                c = comparer_ecran(&nom, &img3, &masque, w, h, &zones);
            }
            changees.extend(c);
        }
    }
    assert!(
        changees.is_empty(),
        "zones de l'écran changées (si c'est demandé : ARGOS_REFERENCES=maj, regarder, commiter et citer les zones) :\n{}",
        changees.join("\n")
    );
}

/// 🖱 Curseurs comme TradingView (Jee, 16 nov.) : la croix au repos (même en main libre), la main fermée seulement en
/// glissant la chart clic tenu, la croix de nouveau au relâcher (sur la chart ou ailleurs) ; la main qui pointe sur un
/// dessin ; la double flèche verticale sur une ligne d'alerte.
#[test]
fn curseurs_comme_tradingview() {
    use crate::types::Curseur::{Icone, Point};
    use winit::window::CursorIcon;
    dossier_a_soi("curseurs");
    let mut a = app();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let bouge = |a: &mut App, x: f64, y: f64| a.mouse_moved(PhysicalPosition::new(x + a.rw(), y + a.top_h()));
    let (cx, cy) = (a.chart.viewport.plot_w() * 0.3, a.chart.viewport.plot_h() * 0.5);
    // au repos : la croix (le curseur choisi par défaut)
    bouge(&mut a, cx, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Crosshair), "la croix au repos");
    // clic tenu : pas encore la main ; en glissant : la main fermée ; relâché : la croix
    a.pointer.last_chart_click = None;
    a.left_button(ElementState::Pressed);
    assert_ne!(a.curseur_pose(), Icone(CursorIcon::Grabbing), "pas de main fermée avant de glisser");
    bouge(&mut a, cx + 30.0, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Grabbing), "main fermée en glissant");
    a.left_button(ElementState::Released);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Crosshair), "relâché : la croix revient");
    bouge(&mut a, cx + 31.0, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Crosshair), "et elle reste");
    // main libre (prix non automatique) : toujours la croix au repos
    a.chart.settings.scale.manual = Some((100.0, 300.0));
    assert!(!a.chart.settings.scale.auto());
    bouge(&mut a, cx, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Crosshair), "main libre : la croix au repos");
    a.chart.settings.scale.manual = None;
    // relâché hors de la chart pendant un glisser : le geste s'arrête, plus de main fermée au retour
    a.pointer.last_chart_click = None;
    bouge(&mut a, cx, cy);
    a.left_button(ElementState::Pressed);
    bouge(&mut a, cx + 20.0, cy);
    a.pointer.mouse = None; // (la souris est sortie de la chart)
    a.left_button(ElementState::Released);
    assert!(a.pointer.drag.is_none(), "le glisser s'arrête au relâcher hors de la chart");
    bouge(&mut a, cx, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Crosshair), "retour sur la chart : la croix");
    // sur un dessin : la main qui pointe
    let sc = a.scale_now().expect("échelle");
    let p = sc.price_of(cy);
    a.chart.drawings.push(Drawing::hline(p));
    bouge(&mut a, cx + 5.0, cy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Pointer), "main qui pointe sur un dessin");
    a.chart.drawings.clear();
    // ✨ la case du coin d'Argos AI, jusqu'au bord : la main qui pointe
    let c = a.ai_bouton_rect();
    for (x, y) in [(c.cx(), c.cy()), (c.x + 1.0, c.y + 1.0), (c.x + c.w - 1.0, c.y + c.h - 1.0)] {
        let (dx, dy) = (a.rw(), a.top_h());
        bouge(&mut a, x - dx, y - dy);
        assert_eq!(a.curseur_pose(), Icone(CursorIcon::Pointer), "case du coin ({x}, {y})");
    }
    // sur une ligne d'alerte : la double flèche verticale
    let y2 = cy - 80.0;
    a.add_price_alert(sc.price_of(y2));
    bouge(&mut a, cx, y2);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::NsResize), "double flèche sur une alerte");
    // choix du curseur (menu du 1er bouton) : Point, Flèche ; gardés dans interface-2.json ; Gomme
    bouge(&mut a, cx, cy);
    a.curseurs_choix(1);
    assert_eq!(a.ui.curseur, crate::types::CurseurChoisi::Point);
    assert_eq!(a.curseur_pose(), Point, "le point tout de suite");
    a.curseurs_choix(2);
    bouge(&mut a, cx, cy + 1.0);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Default), "la flèche");
    let i2 = App::interface2();
    assert_eq!(i2.curseur, crate::types::CurseurChoisi::Fleche, "gardé dans interface-2.json");
    a.curseurs_choix(10);
    assert_eq!(a.tool, "erase", "la gomme depuis le groupe du curseur");
    let m = a.menu_curseurs();
    assert_eq!(m.iter().map(|g| g.items.len()).sum::<usize>(), 5, "Croix, Point, Flèche, Démonstration, Gomme");
}

/// 🖍 Curseur Démonstration (TradingView) : Alt tenu + glisser = un trait qui suit la souris (la chart ne bouge pas),
/// Alt + clic = un point ; ils s'effacent 3 s après le relâcher ; sans Alt, la chart se glisse comme d'habitude.
#[test]
fn curseur_demonstration() {
    use winit::keyboard::ModifiersState;
    dossier_a_soi("curseur-demo");
    let mut a = app();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let bouge = |a: &mut App, x: f64, y: f64| a.mouse_moved(PhysicalPosition::new(x + a.rw(), y + a.top_h()));
    a.curseurs_choix(3);
    assert!(a.demo_active(), "Démonstration choisie");
    let (cx, cy) = (300.0, 250.0);
    // Alt + glisser : un trait
    a.pointer.mods = ModifiersState::ALT;
    bouge(&mut a, cx, cy);
    a.left_button(ElementState::Pressed);
    for k in 1..6 {
        bouge(&mut a, cx + 10.0 * k as f64, cy + 4.0 * k as f64);
    }
    a.left_button(ElementState::Released);
    assert_eq!(a.ui.demo_traits.len(), 1);
    assert!(a.ui.demo_traits[0].pts.len() >= 6 && a.ui.demo_traits[0].fini.is_some(), "un trait fini");
    assert!(a.pointer.drag.is_none(), "la chart n'a pas été glissée");
    // Alt + clic : un point
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert_eq!(a.ui.demo_traits.len(), 2);
    assert_eq!(a.ui.demo_traits[1].pts.len(), 1, "un point");
    // dessinés en couleur d'accent, puis effacés après 3 s
    let mut pm = argos_render::px::new(1100, 700).unwrap();
    a.dessiner_demo(&mut pm);
    assert!(pm.pixels().iter().any(|p| p.alpha() > 0), "les traits sont dessinés");
    for t in &mut a.ui.demo_traits {
        t.fini = Instant::now().checked_sub(std::time::Duration::from_secs(4));
    }
    a.dessiner_demo(&mut pm);
    assert!(a.ui.demo_traits.is_empty(), "effacés après 3 s");
    // sans Alt : la chart se glisse
    a.pointer.mods = ModifiersState::empty();
    a.pointer.last_chart_click = None;
    a.left_button(ElementState::Pressed);
    assert!(a.pointer.drag.is_some() && a.ui.demo_traits.is_empty(), "sans Alt : glisser la chart");
    a.left_button(ElementState::Released);
    assert_eq!(
        a.menu_curseurs().iter().map(|g| g.items.len()).sum::<usize>(),
        5,
        "Croix, Point, Flèche, Démonstration, Gomme"
    );
}

/// { } Le code d'un indicateur intégré (bouton de la légende, comme TradingView) : TBO Pro ouvre son Pine dans
/// l'éditeur ✎, en lecture seule (taper ne change rien, ▶ et 💾 refusés) ; un indicateur sans source n'ouvre rien.
#[test]
fn code_d_un_indicateur_en_lecture_seule() {
    use argos_render::editor::{EdBtn, EdClick, EdKey};
    use argos_render::indicators::Ind;
    let mut a = app();
    a.legend_click(argos_render::indleg::LegClick::Code(Ind::Tbo));
    let ed = a.ui.editor.as_ref().expect("l'éditeur s'ouvre");
    assert!(ed.lecture_seule && ed.text().contains("indicator("), "le Pine de TBO Pro, en lecture seule");
    let avant = ed.text();
    if let Some(e) = a.ui.editor.as_mut() {
        e.insert("xyz");
        e.key(EdKey::Backspace);
        e.key(EdKey::Enter);
    }
    a.editor_click(EdClick::Btn(EdBtn::Run));
    assert_eq!(a.ui.editor.as_ref().unwrap().text(), avant, "le texte ne change pas");
    assert!(a.scripts.is_empty(), "rien n'est ajouté à la chart ni sauvegardé");
    a.ui.editor = None;
    // { } B1: an indicator kept in Rust opens its real Rust source, read only, without « Créer une copie »
    a.legend_click(argos_render::indleg::LegClick::Code(Ind::ArgosMurs));
    let ed = a.ui.editor.as_ref().expect("Argos Murs: its Rust code");
    assert!(ed.lecture_seule && ed.rust, "read only, Rust");
    let t = ed.text();
    assert!(t.lines().next().unwrap().contains("Rust, lecture seule") && t.contains("fn "), "the Rust file");
    a.editor_click(EdClick::Btn(EdBtn::Run));
    a.editor_click(EdClick::Item(argos_render::editor::EdItem::Copy));
    assert_eq!(a.ui.editor.as_ref().unwrap().text(), t, "nothing changes");
    assert!(a.scripts.is_empty(), "no copy, nothing added");
    assert!(
        matches!(a.ui.editor.as_ref().unwrap().extra, argos_render::editor::Extra::None),
        "no copy prompt"
    );
    a.ui.editor = None;
    a.legend_click(argos_render::indleg::LegClick::Code(Ind::Lens));
    assert!(a.ui.editor.as_ref().is_some_and(|e| e.rust), "Argos Lens: its Rust code too");
}

/// 🧱 Couches (Jee, 16 nov. : « le curseur pour l'alerte passe au travers ») : une fenêtre ⚙ d'indicateur ouverte
/// par-dessus une ligne d'alerte et un dessin → flèche normale, aucun clic ne passe (pas d'alerte attrapée), la molette
/// ne bouge pas la chart, même hors de la fenêtre ; un menu ouvert au même endroit : pareil sous le menu.
#[test]
fn rien_ne_passe_sous_une_fenetre_ouverte() {
    use crate::types::Curseur::Icone;
    use argos_render::indicators::Ind;
    use winit::window::CursorIcon;
    dossier_a_soi("couches");
    let mut a = app();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let bouge = |a: &mut App, wx: f64, wy: f64| a.mouse_moved(PhysicalPosition::new(wx, wy));
    let (cx, cy) = (a.chart.viewport.plot_w() * 0.4, a.chart.viewport.plot_h() * 0.45);
    let (wx, wy) = (cx + a.rw(), cy + a.top_h());
    let sc = a.scale_now().expect("échelle");
    // une ligne d'alerte et un dessin exactement à cet endroit : sans fenêtre, ↕ et la main
    a.add_price_alert(sc.price_of(cy));
    bouge(&mut a, wx, wy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::NsResize), "sans fenêtre : ↕ sur l'alerte");
    // le cas de Jee : ⚙ d'Argos TVD Pro ouvert, posé par-dessus la ligne d'alerte
    a.ind.on.push(Ind::Tvd);
    a.open_ind_dlg(Ind::Tvd);
    let r = a.ui.ind_dlg.as_ref().expect("⚙ ouvert").rect;
    if let Some(d) = a.ui.ind_dlg.as_mut() {
        d.rect = Rect::new(wx - r.w / 2.0, wy - 40.0, r.w, r.h);
    }
    bouge(&mut a, wx, wy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Default), "⚙ par-dessus : la flèche, pas ↕");
    // hors de la fenêtre modale aussi : la chart ne reçoit rien
    let (hx, hy) = (a.rw() + 30.0, a.top_h() + cy);
    bouge(&mut a, hx, hy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Default), "fenêtre modale : rien sur la chart, même à côté");
    let off = (a.chart.viewport.right_offset, a.chart.viewport.bar_spacing);
    a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, 3.0));
    assert_eq!(
        (a.chart.viewport.right_offset, a.chart.viewport.bar_spacing),
        off,
        "la molette ne bouge pas la chart"
    );
    bouge(&mut a, wx, wy);
    a.left_button(ElementState::Pressed);
    assert!(a.pointer.alert_drag.is_none(), "le clic n'attrape pas l'alerte dessous");
    a.left_button(ElementState::Released);
    a.ui.ind_dlg = None;
    // un menu ouvert au même endroit : la flèche sous le menu, pas ↕
    a.open_list(ListKind::Intervals, Rect::new(wx - 40.0, wy - 60.0, 40.0, 24.0));
    let m = a.ui.list.as_ref().expect("menu").1.rect;
    bouge(&mut a, m.x + 10.0, m.y + m.h / 2.0);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::Default), "sous un menu : la flèche");
    a.ui.list = None;
    bouge(&mut a, wx, wy);
    assert_eq!(a.curseur_pose(), Icone(CursorIcon::NsResize), "fenêtres fermées : ↕ revient");
}

/// 📊 PC (maquette) : 📊 du haut ouvre la fenêtre Type de graphique sous son bouton, par-dessus les rangées 2 et 3 (son
/// haut au-dessus de la chart) ; une case cliquée sur les rangées du haut change le type (le clic va à la fenêtre, pas
/// à la barre du haut).
#[test]
fn type_de_graphique_sous_le_bouton_du_haut() {
    use argos_render::topbar::TopClick;
    dossier_a_soi("type-haut");
    let mut a = app();
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    a.chart.style = 2;
    a.top_click_do(TopClick::Style(Rect::new(180.0, 12.0, 34.0, 32.0)));
    let p = a.ui.pop.as_ref().expect("la fenêtre Type").rect;
    let haut = a.top_h() + p.y;
    assert!((haut - crate::render::VOLET_HAUT).abs() < 0.5, "sous la 1re rangée : {haut}");
    assert!(p.y < 0.0, "par-dessus les rangées du haut");
    // la 1re case (en haut à gauche de la grille) : au-dessus de la chart
    let (wx, wy) = (a.rw() + p.x + 40.0, a.top_h() + p.y + 44.0 + 10.0);
    assert!(wy < a.top_h(), "la case est sur les rangées du haut");
    a.mouse_moved(PhysicalPosition::new(wx, wy));
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert_eq!(a.chart.style, 0, "le clic choisit la 1re case");
}

/// 📚 Step 2 of the Argos Script engine: indicator settings saved by name. The old by-position file is converted on
/// the first launch (copied as it is to indicator_inputs.avant-noms.json first), every value is found again by name,
/// the old file keeps being written by position for /app/; a setting changed on one device arrives by name on the
/// other (sync of indicator_settings.json); a change /app/ makes to the old file afterwards is not imported back.
#[test]
fn reglages_des_indicateurs_par_nom() {
    use crate::actions::indicators::{REGLAGES_AVANT, REGLAGES_NOMS, REGLAGES_POSITIONS};
    use argos_render::indicators::{values, Ind, Settings};
    dossier_a_soi("reglages-par-nom");
    // an old by-position file, as /app/ and the previous versions wrote it
    let mut rsi = values(&Settings::new(), Ind::Rsi);
    rsi[0] = 21.0;
    let ancien: Settings = [(Ind::Rsi, rsi.clone())].into();
    let brut = serde_json::to_string_pretty(&ancien).unwrap();
    crate::platform::write(REGLAGES_POSITIONS, &brut).unwrap();
    assert!(crate::platform::read(REGLAGES_NOMS).is_none());
    // first launch: converted, the old file saved as it is
    let (set, _) = App::lire_reglages();
    assert_eq!(set, ancien, "nothing lost");
    assert_eq!(crate::platform::read(REGLAGES_AVANT).as_deref(), Some(brut.as_str()), "old file kept as it is");
    let noms: serde_json::Value = serde_json::from_str(&crate::platform::read(REGLAGES_NOMS).unwrap()).unwrap();
    let cle = argos_render::catalogue::reglages(Ind::Rsi).1[0].key;
    let id = argos_render::catalogue::fiche(Ind::Rsi).id;
    assert_eq!(noms["format"], 1);
    assert_eq!(noms["indicateurs"][id][cle], 21.0, "by name");
    // device A changes a setting; device B (same synced folder) reloads it by name
    let mut a = app();
    a.ind.set = App::lire_reglages().0;
    a.ind.set.get_mut(&Ind::Rsi).unwrap()[0] = 34.0;
    a.garder_reglages();
    let mut b = app();
    b.ind.set = Settings::new();
    b.recharger(&[REGLAGES_NOMS.to_string()]);
    assert_eq!(b.ind.set[&Ind::Rsi][0], 34.0, "arrived by name on the other device");
    let positions: Settings = serde_json::from_str(&crate::platform::read(REGLAGES_POSITIONS).unwrap()).unwrap();
    assert_eq!(positions[&Ind::Rsi][0], 34.0, "the old file is still written for /app/");
    // one way: /app/ changes the old file afterwards, it is not imported back
    let mut change = positions.clone();
    change.get_mut(&Ind::Rsi).unwrap()[0] = 99.0;
    crate::platform::write(REGLAGES_POSITIONS, &serde_json::to_string(&change).unwrap()).unwrap();
    b.recharger(&[REGLAGES_POSITIONS.to_string()]);
    assert_eq!(b.ind.set[&Ind::Rsi][0], 34.0, "not imported back");
    assert_eq!(App::lire_reglages().0[&Ind::Rsi][0], 34.0, "the source is the file by name");
    assert_eq!(
        crate::platform::read(REGLAGES_AVANT).as_deref(),
        Some(brut.as_str()),
        "backup never overwritten"
    );
}

/// 📜 Bollinger, a native replaced by TradingView's script: Jee-like saved settings (Length 20, ×2, a custom basis
/// colour, width 2 on the upper band), by name (indicator_settings.json) or by position only (indicator_inputs.json),
/// are converted once at load to the script's keys, after a dated copy of the files (never overwritten); the drawing
/// has the same colour, width and bands; the old /app/ still gets the native's positions; a second load changes
/// nothing.
#[test]
fn bollinger_reglages_convertis() {
    use crate::actions::indicators::{nom_avant_conversion, REGLAGES_NOMS, REGLAGES_POSITIONS};
    use argos_render::indicators::{compute, values, Ind, Settings};
    use argos_script::catalogue::par_nom;
    const COL: f64 = 0x12_34_56 as f64;
    // the native's layout: len, mult, col, ep1, ty1, ep2, ty2, ep3, ty3
    let natif = vec![20.0, 2.0, COL, 2.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    let date = chrono::Local::now().format("%Y-%m-%d").to_string();
    let bars: Vec<argos_core::Bar> = (0..200)
        .map(|i| {
            let c = 100.0 + (i as f64 * 0.21).sin() * 4.0 + i as f64 * 0.03;
            argos_core::Bar {
                time: i * 3600,
                open: c - 0.2,
                high: c + 1.0,
                low: c - 1.0,
                close: c,
                volume: 10.0,
            }
        })
        .collect();
    let verifier = |set: &Settings| {
        let p = Ind::Bollinger.params().1;
        let n = par_nom(p, &values(set, Ind::Bollinger));
        assert_eq!((n["Length"], n["StdDev"], n["Basis · couleur"], n["Upper · épaisseur"]), (20.0, 2.0, COL, 2.0));
        assert_eq!((n["Basis MA Type"], n["Source"], n["Offset"]), (0.0, 0.0, 0.0), "SMA, close, 0");
        let out = compute(&bars, 3600, &[Ind::Bollinger], set, None, 0, &Default::default(), &[]);
        let (_, v) = out.integres.first().expect("drawn by its script");
        assert_eq!(v.style.plot(0).couleur, Some(0x12_34_56), "the basis colour");
        assert_eq!(v.style.plot(1).epaisseur, Some(2.0), "the upper band's width");
        assert_eq!((v.style.plot(0).epaisseur, v.style.plot(2).epaisseur), (None, None), "width 1 elsewhere");
        let bb = argos_script::indicators::classic::bollinger_n(&bars, 20, 2.0);
        let last = |t: &str| v.out.plots.iter().find(|x| x.title == t).unwrap().values[199];
        assert!((last("Basis") - bb.basis[199]).abs() < 1e-9 && (last("Upper") - bb.upper[199]).abs() < 1e-9);
    };
    // 1) by name, as the previous version wrote it
    dossier_a_soi("bollinger-convertis");
    let noms = format!(
        "{{\"format\":1,\"indicateurs\":{{\"Bollinger\":{}}},\"scripts\":{{}}}}",
        serde_json::to_string(
            &[
                ("len", 20.0),
                ("mult", 2.0),
                ("col", COL),
                ("ep1", 2.0),
                ("ty1", 0.0),
                ("ep2", 1.0),
                ("ty2", 0.0),
                ("ep3", 1.0),
                ("ty3", 0.0)
            ]
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>()
        )
        .unwrap()
    );
    let positions = serde_json::to_string(&Settings::from([(Ind::Bollinger, natif.clone())])).unwrap();
    crate::platform::write(REGLAGES_NOMS, &noms).unwrap();
    crate::platform::write(REGLAGES_POSITIONS, &positions).unwrap();
    let (set, _) = App::lire_reglages();
    verifier(&set);
    assert_eq!(
        crate::platform::read(&nom_avant_conversion(REGLAGES_NOMS, &date)).as_deref(),
        Some(noms.as_str()),
        "copy"
    );
    assert_eq!(
        crate::platform::read(&nom_avant_conversion(REGLAGES_POSITIONS, &date)).as_deref(),
        Some(positions.as_str()),
        "copy of the by-position file"
    );
    assert_eq!(
        nom_avant_conversion(REGLAGES_POSITIONS, "2026-11-19"),
        "indicator_inputs.avant-conversion-2026-11-19.json"
    );
    let converti = crate::platform::read(REGLAGES_NOMS).unwrap();
    assert!(!converti.contains("\"len\"") && converti.contains("Upper · épaisseur"), "{converti}");
    // once: a second load changes nothing, the copies stay
    assert_eq!(App::lire_reglages().0, set);
    assert_eq!(crate::platform::read(REGLAGES_NOMS).unwrap(), converti);
    // the old /app/ still reads the native's positions
    let mut a = app();
    a.ind.set = set;
    a.garder_reglages();
    let ancien: Settings = serde_json::from_str(&crate::platform::read(REGLAGES_POSITIONS).unwrap()).unwrap();
    assert_eq!(ancien[&Ind::Bollinger], natif);
    assert_eq!(
        crate::platform::read(&nom_avant_conversion(REGLAGES_NOMS, &date)).as_deref(),
        Some(noms.as_str()),
        "never overwritten"
    );
    // 2) by position only (first launch of the by-name settings)
    dossier_a_soi("bollinger-convertis-positions");
    crate::platform::write(REGLAGES_POSITIONS, &positions).unwrap();
    let (set, _) = App::lire_reglages();
    verifier(&set);
    assert_eq!(
        crate::platform::read(&nom_avant_conversion(REGLAGES_POSITIONS, &date)).as_deref(),
        Some(positions.as_str())
    );
    assert_eq!(App::lire_reglages().0, set, "the by-name file made from it is read the second time");
}

/// ⚙ Défauts ▸ « Enregistrer comme défaut » (comme TradingView) : les réglages gardés dans indicator_defaults.json
/// (par nom) servent chaque fois que l'indicateur est ajouté ; « Réinitialiser les réglages » y revient.
#[test]
fn enregistrer_comme_defaut() {
    use argos_render::indicators::{values, Ind};
    dossier_a_soi("mes-defauts");
    let mut a = app();
    a.ind.on.clear();
    a.toggle_indicator(Ind::Tvd);
    a.ui.ind_dlg = None;
    a.open_ind_dlg(Ind::Tvd);
    let mut dl = a.ui.ind_dlg.take().expect("la fenêtre de réglages");
    assert!(dl.mes_defauts.is_none() && dl.enregistrable);
    dl.vals[0] = 21.0;
    a.ind_dlg_result(dl.clone(), argos_render::inddlg::IndClick::Changed);
    dl.mes_defauts = Some((dl.vals.clone(), dl.texts.clone()));
    a.ind_dlg_result(dl, argos_render::inddlg::IndClick::SaveDefaults);
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("défaut")));
    let f: crate::actions::indicators::MesDefauts = App::read_prefs(crate::actions::indicators::MES_DEFAUTS);
    assert_eq!(
        f.indicateurs[argos_render::catalogue::fiche(Ind::Tvd).id]
            .values()
            .filter(|v| **v == 21.0)
            .count(),
        1
    );
    // un autre réglage ensuite, on retire l'indicateur, on le remet : il repart des défauts enregistrés
    let mut v = values(&a.ind.set, Ind::Tvd);
    v[0] = 30.0;
    a.ind.set.insert(Ind::Tvd, v);
    a.ui.ind_dlg = None;
    a.toggle_indicator(Ind::Tvd);
    a.toggle_indicator(Ind::Tvd);
    assert_eq!(values(&a.ind.set, Ind::Tvd)[0], 21.0);
    // la fenêtre rouverte les connaît (Réinitialiser y revient)
    a.ui.ind_dlg = None;
    a.open_ind_dlg(Ind::Tvd);
    assert_eq!(a.ui.ind_dlg.as_ref().and_then(|d| d.mes_defauts.as_ref()).map(|m| m.0[0]), Some(21.0));
}

/// ⚙ Step 3 of the Argos Script engine: a user script's settings window. Its input() calls make the window (the same
/// one as the indicators, groups = their `group`); a value changed there changes the result, is saved by name under
/// the script's stable id (indicator_settings.json, « scripts »), comes back after a reload and on another device;
/// the script's code changing keeps the settings whose title still exists.
#[test]
fn reglages_d_un_script_utilisateur() {
    use argos_render::catalogue::ValeurScript;
    use argos_render::indleg::LegClick;
    dossier_a_soi("reglages-script");
    let src = "//@version=6\nindicator(\"Mon script\", overlay = true)\nlen = input.int(10, \"Length\", minval = 1, group = \"Calcul\")\nshow = input.bool(true, \"Show\", group = \"Style\")\nplot(show ? ta.sma(close, len) : na)\n";
    let mut a = app();
    a.scripts = vec![SavedScript {
        name: "Mon script".into(),
        src: src.into(),
        on: true,
        ..Default::default()
    }];
    a.ids_des_scripts();
    let id = a.scripts[0].id.clone();
    assert_eq!(id.len(), 16, "a stable id");
    a.ind.key = Default::default();
    a.refresh_indicators();
    let avant = a.ind.out.scripts[0].out.plots[0].values.last().copied();
    // ⚙ in the script's legend opens its window, made from its inputs
    a.legend_click(LegClick::ScriptSettings(0));
    let mut dl = a.ui.ind_dlg.take().expect("the script's settings window");
    assert_eq!(dl.script.as_ref().map(|s| s.0.clone()), Some(id.clone()));
    assert_eq!(dl.vals, vec![10.0, 1.0], "its defaults");
    // Length 10 → 3: saved by name, the result changes
    dl.vals[0] = 3.0;
    a.ind_dlg_result(dl, argos_render::inddlg::IndClick::Changed);
    assert_eq!(a.ind.scripts_set[&id]["Length"], ValeurScript::Num(3.0));
    a.refresh_indicators();
    assert_ne!(a.ind.out.scripts[0].out.plots[0].values.last().copied(), avant, "the chosen value is used");
    // reload and another device: it comes back by name
    let mut b = app();
    b.recharger(&["scripts.json".to_string(), "indicator_settings.json".to_string()]);
    assert_eq!(b.scripts[0].id, id);
    assert_eq!(b.valeurs_du_script(&id)["Length"], argos_script::lang::InVal::Num(3.0));
    // the code changes (Length renamed, Show kept): Show's value stays, the old Length stays in the file
    a.ui.ind_dlg = None;
    a.legend_click(LegClick::ScriptSettings(0));
    let mut dl = a.ui.ind_dlg.take().unwrap();
    dl.vals[1] = 0.0;
    a.ind_dlg_result(dl, argos_render::inddlg::IndClick::Changed);
    a.scripts[0].src = src.replace("\"Length\"", "\"Period\"");
    a.ind.key = Default::default();
    a.refresh_indicators();
    assert_eq!(a.valeurs_du_script(&id)["Show"], argos_script::lang::InVal::Bool(false), "kept by name");
    assert!(
        a.ind.out.scripts[0].out.plots[0].values.iter().all(|v| v.is_nan()),
        "Show off is used by the new code"
    );
}

/// 🧪📜 Lot 6a: a strategy written in Argos Script on the chart is offered in the 🧪 card; chosen, its direction /
/// stop / target / costs lines give way to its ⚙; ▶️ gives its own trades (filled like TradingView) in the card, without
/// the backtest's arrows (the script draws its own); gone from the chart, the card says so; a built-in chosen again
/// forgets it.
#[test]
fn backtest_d_une_strategie_argos_script() {
    dossier_a_soi("backtest-strategie");
    // the bars go up by 1 each: bought at the open after bar 20k, closed at the open after bar 20k + 10: +10 each time
    let src = "//@version=6\nstrategy(\"Ma stratégie\", overlay = true)\nif bar_index % 20 == 0\n    strategy.entry(\"L\", strategy.long)\nif bar_index % 20 == 10\n    strategy.close(\"L\")\n";
    let mut a = app();
    a.scripts = vec![SavedScript {
        name: "Ma stratégie".into(),
        src: src.into(),
        on: true,
        ..Default::default()
    }];
    a.ids_des_scripts();
    let id = a.scripts[0].id.clone();
    a.ind.key = Default::default();
    a.refresh_indicators();
    let ligne = |a: &App, debut: &str| {
        a.backtest_menu()
            .into_iter()
            .flat_map(|g| g.items)
            .find(|i| i.label.starts_with(debut))
            .unwrap_or_else(|| panic!("no line « {debut} »"))
    };
    let titres = |a: &App| a.backtest_menu().into_iter().map(|g| g.title).collect::<Vec<_>>();
    let s = ligne(&a, "📜 Ma stratégie");
    assert!(!s.checked && titres(&a).contains(&"SENS".to_string()));
    assert!(!ligne(&a, "📊 Rapport complet").enabled, "no report before a run");
    assert!(a.backtest_pick(s.id), "the menu stays open");
    assert_eq!(a.ui.bt_prefs.script, id);
    assert!(ligne(&a, "📜 Ma stratégie").checked && !ligne(&a, "Croisement EMA 9 / 21").checked);
    assert_eq!(titres(&a), ["🧪 STRATÉGIE", "RÉGLAGES", ""], "its costs are in its ⚙");
    assert!(!a.backtest_pick(ligne(&a, "▶️ Lancer").id));
    let v = a.ui.bt.as_ref().expect("the card");
    assert!(v.script && v.name == "📜 Ma stratégie");
    assert_eq!(v.report.trades.len(), 10);
    assert!(v.report.trades.iter().all(|t| t.exit - t.entry == 10.0));
    assert_eq!(v.report.net_pnl, argos_core::Money::from_f64(100.0));
    assert_eq!(v.report.win_rate, 100.0);
    let capital = argos_core::nombres::format_number(1_000_000.0, 0);
    assert_eq!((v.settings.as_str(), v.frais.as_str()), (format!("{capital} $ · 1 contrat").as_str(), "0 %"));
    assert!(v.ouverte.is_none(), "every trade closed");
    let rows = argos_render::dock::bt_rows(v, 0);
    assert!(rows.iter().all(|r| r.0 != "R moyen"), "no R: its stop is not known");
    assert!(a.bt_marks().is_empty(), "the script draws its own arrows");
    assert!(a.ind.out.scripts[0].out.strategie.as_ref().is_some_and(|b| b.marques.len() == 20));
    // ⚙ its properties: 2 contracts an order, then ▶️ again: twice the result
    a.legend_click(argos_render::indleg::LegClick::ScriptSettings(0));
    let mut dl = a.ui.ind_dlg.take().expect("the strategy's settings window");
    assert_eq!(dl.vals.len(), 7, "capital, size, size type, pyramiding, commission, slippage, on the close");
    assert_eq!(dl.vals[1], 1.0);
    dl.vals[1] = 2.0;
    a.ind_dlg_result(dl, argos_render::inddlg::IndClick::Changed);
    a.refresh_indicators();
    a.run_backtest();
    let v = a.ui.bt.as_ref().unwrap();
    assert_eq!(v.report.net_pnl, argos_core::Money::from_f64(200.0));
    assert_eq!(v.settings, format!("{capital} $ · 2 contrats"));
    // 📊 the full report page (lot 6b): from the 🧪 menu, after a run; ⬇️ Export CSV in TradingView's columns
    assert!(ligne(&a, "📊 Rapport complet").enabled);
    assert!(!a.backtest_pick(ligne(&a, "📊 Rapport complet").id), "the menu closes");
    assert!(a.ui.bt_page.is_some() && a.page_ouverte());
    assert_eq!(a.ui.bt.as_ref().unwrap().detail.lignes.len(), 10);
    a.rapport_csv();
    let fichier = platform::dossier_telechargements().join("argos-backtest-ma-stratégie-essai.csv");
    let csv = std::fs::read_to_string(&fichier).unwrap_or_else(|e| panic!("{}: {e} ({:?})", fichier.display(), a.banner));
    let rangs: Vec<&str> = csv.lines().collect();
    assert_eq!(rangs.len(), 21, "a header, then 2 rows a trade");
    assert!(rangs[0].starts_with("Trade #,Type,Signal,Date/Time,Price USD,Contracts,Profit USD"));
    assert!(rangs[1].starts_with("10,Exit long,Close entry(s) order L,") && rangs[2].starts_with("10,Entry long,L,"));
    let _ = std::fs::remove_file(&fichier);
    // the wheel scrolls its table; closed, no page is open
    a.rapport_molette(120.0);
    a.ui.bt_page = None;
    assert!(!a.page_ouverte());
    // taken off the chart: the card says so
    a.scripts[0].on = false;
    a.ind.key = Default::default();
    a.refresh_indicators();
    a.ui.bt = None;
    a.run_backtest();
    assert!(a.ui.bt.is_none() && a.banner.as_deref().is_some_and(|b| b.contains("n'est plus sur la chart")));
    // a built-in chosen again: the script is forgotten
    a.backtest_pick(0);
    assert!(a.ui.bt_prefs.script.is_empty());
}

/// 🎯 The Stratégie volet (Jee, 23 Nov): its 🎯 in the right bar only while a strategy is on the chart; a strategy added
/// opens the volet on it (🖥, once), its result made by itself (no « ▶️ Lancer ») and again when its ⚙ changes; ⤢ /
/// « 📊 Rapport complet » open the full report of it, « ⚙ Tous les réglages » its settings; 📱 its tile in the ⋯ Argos
/// sheet opens it as a sheet; taken off the chart, the 🎯 goes and the volet closes.
#[test]
fn volet_strategie_suit_la_chart() {
    use argos_render::strategie::StratClick;
    dossier_a_soi("volet-strategie");
    let mut a = pc_avec_volet(wl::Tool::Watchlist);
    a.watch.prefs.open = false;
    a.scripts = vec![];
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(!a.watch.bar_tools().contains(&wl::Tool::Strategie), "no strategy: no 🎯");
    let src = "//@version=6\nstrategy(\"Ma stratégie\", overlay = true)\nif bar_index % 20 == 0\n    strategy.entry(\"L\", strategy.long)\nif bar_index % 20 == 10\n    strategy.close(\"L\")\n";
    a.scripts = vec![SavedScript {
        name: "Ma stratégie".into(),
        src: src.into(),
        on: true,
        ..Default::default()
    }];
    a.ids_des_scripts();
    a.ind.key = Default::default();
    // (the frame that runs it, then the next one opens the volet before its layout)
    let _ = image_ecran(&mut a, 1440, 900);
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Strategie, "added: the volet opens on it");
    assert_eq!(a.watch.bar_tools().last(), Some(&wl::Tool::Strategie), "its 🎯 at the end of the bar");
    let v = a.watch.strategie.vue.as_ref().expect("its result, without ▶️ Lancer");
    assert!(v.script && v.name == "📜 Ma stratégie" && v.detail.lignes.len() == 10);
    assert_eq!(v.report.net_pnl, argos_core::Money::from_f64(100.0));
    assert_eq!(a.watch.strategie.noms, ["Ma stratégie"]);
    assert!(a.ui.bt.is_none(), "the 🧪 card is not made for it");
    // closed, the next runs leave it closed (opened by itself once); its 🎯 opens it again
    a.watch.prefs.open = false;
    a.ind.key = Default::default();
    let _ = image_ecran(&mut a, 1440, 900);
    assert!(!a.watch.prefs.open);
    let r = a.watch.tool_rect(a.watch.bar_tools().len() - 1);
    a.watch.click(r.cx(), r.cy(), 800.0);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Strategie);
    // ⚙ its properties: 2 contracts an order, its result twice as big, by itself
    a.legend_click(argos_render::indleg::LegClick::ScriptSettings(0));
    let mut dl = a.ui.ind_dlg.take().expect("the strategy's settings window");
    dl.vals[1] = 2.0;
    a.ind_dlg_result(dl, argos_render::inddlg::IndClick::Changed);
    a.refresh_indicators();
    assert_eq!(a.watch.strategie.vue.as_ref().unwrap().report.net_pnl, argos_core::Money::from_f64(200.0));
    // « ⚙ Tous les réglages »: the same window
    a.strategie_click(StratClick::Reglages);
    assert!(a.ui.ind_dlg.take().is_some(), "its settings");
    // ⤢ / 📊 the full report of it
    a.strategie_click(StratClick::Rapport);
    assert!(a.ui.bt_page.is_some() && a.ui.bt.as_ref().is_some_and(|v| v.name == "📜 Ma stratégie"));
    a.ui.bt_page = None;
    // the volet and the report draw (🖥 and 📱)
    let _ = image_ecran(&mut a, 1440, 900);
    // 📱 its tile, first of the tools of the ⋯ Argos sheet, opens it as a sheet
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.watch.prefs.open = false;
    let _ = image_ecran(&mut a, 402, 874);
    a.pfeuille_ouvrir(crate::actions::pilule::PKind::Argos);
    let f = a.pfeuille_vue().expect("the Argos sheet");
    let outils = f.sections.iter().find(|s| s.titre == "OUTILS").expect("its tools");
    assert_eq!(outils.tuiles[0].id, "strategie");
    a.ui.pfeuille = None;
    a.tuile_ouvrir("strategie", Rect::new(0.0, 0.0, 1.0, 1.0));
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Strategie);
    let _ = image_ecran(&mut a, 402, 874);
    // taken off the chart: no 🎯, the volet closes, no tile
    a.scripts[0].on = false;
    a.ind.key = Default::default();
    a.refresh_indicators();
    assert!(!a.watch.bar_tools().contains(&wl::Tool::Strategie) && a.watch.strategie.vue.is_none());
    assert!(!a.watch.prefs.open, "its volet closes");
    a.pfeuille_ouvrir(crate::actions::pilule::PKind::Argos);
    let f = a.pfeuille_vue().unwrap();
    assert!(f.sections.iter().flat_map(|s| &s.tuiles).all(|t| t.id != "strategie"));
}

/// 🏷️ Curseur sur les légendes (Jee : « il reste en croix ») : légende du prix, liste ouverte et nom des panneaux du
/// bas = la flèche ; chaque bouton (⌄ N, 👁 ⚙ { } ↑ ↓ ▾ ✕…) = la main ; les tableaux des indicateurs = la flèche ;
/// jamais la croix ; légende repliée et dépliée, à 1440 et 2368 ; hors des légendes, la croix reste.
#[test]
fn curseur_sur_les_legendes() {
    use crate::types::Curseur::Icone;
    use argos_render::indicators::Ind;
    use winit::window::CursorIcon;
    dossier_a_soi("curseur-legendes");
    let oscs: Vec<Ind> = Ind::ALL.iter().copied().filter(|i| i.oscillator()).take(2).collect();
    assert_eq!(oscs.len(), 2);
    for (w, h) in [(1440u32, 900u32), (2368, 1155)] {
        for ouverte in [false, true] {
            let mut a = app();
            a.appliquer_cadre(&crate::actions::Interface2::default());
            a.top.prefs.bienvenue_vue = true;
            a.tools.hidden = true;
            a.ind.on = vec![Ind::Tvd, Ind::AiTrend, oscs[0], oscs[1]];
            a.ind.leg.0 = ouverte;
            a.ui.frames = 5;
            let taille = winit::dpi::PhysicalSize::new(w, h);
            // (deux images, comme l'app : la 2e place l'écran avec les indicateurs)
            a.ui.taille_test = Some(taille);
            a.draw_frame();
            a.draw_frame();
            let (rw, th) = (a.rw(), a.top_h());
            let quoi = format!("{w}×{h} {}", if ouverte { "dépliée" } else { "repliée" });
            let curseur = |a: &mut App, wx: f64, wy: f64| {
                a.mouse_moved(PhysicalPosition::new(wx, wy));
                a.curseur_pose()
            };
            // chaque bouton de légende : la main
            let t = argos_render::text::Text::default();
            let vp = a.chart.viewport;
            let cell = argos_render::indleg::Cell {
                actif: false,
                sel: None,
                pw: vp.plot_w(),
            };
            let boutons = argos_render::indleg::zones_de(&t, &a.ind.on, &a.ind.out, ouverte, vp.plot_h(), vp.plot_w(), cell);
            assert!(!boutons.is_empty(), "{quoi} : des boutons");
            for (r, c) in &boutons {
                let (x, y) = (rw + r.cx(), th + r.cy());
                if a.couche_au_dessus(x, y) && !a.sur_legende(x, y) {
                    continue; // (sous une autre couche)
                }
                assert_eq!(curseur(&mut a, x, y), Icone(CursorIcon::Pointer), "{quoi} : main sur {c:?} {r:?}");
            }
            // le texte des légendes (hors boutons) : la flèche
            for (n, r) in a.legendes() {
                let (x, y) = (r.x + 6.0, r.y + r.h / 2.0);
                if a.legende_bouton(x - rw, y - th).is_some() {
                    continue;
                }
                assert_eq!(curseur(&mut a, x, y), Icone(CursorIcon::Default), "{quoi} : flèche sur {n} {r:?}");
            }
            // the tables (AI Trend…): the move cursor on their body (📋 tableaux-1: they move with the mouse)
            let tableaux = argos_render::indicators::tableaux();
            assert!(!tableaux.is_empty(), "{quoi} : le tableau d'AI Trend");
            for r in tableaux {
                assert_eq!(
                    curseur(&mut a, rw + r.cx(), th + r.cy()),
                    Icone(CursorIcon::Move),
                    "{quoi} : déplacer sur le tableau {r:?}"
                );
            }
            // ailleurs dans la zone de prix : la croix reste
            let (cx, cy) = (rw + vp.plot_w() * 0.45, th + vp.plot_h() * 0.4);
            if !a.sur_legende(cx, cy) && !a.couche_au_dessus(cx, cy) {
                assert_eq!(curseur(&mut a, cx, cy), Icone(CursorIcon::Crosshair), "{quoi} : la croix sur la chart");
            }
        }
    }
}

/// 🛟 Support (PC): 🛟 in the top bar opens the panel in place of the floating panel; modal layer (a click on
/// the chart places nothing there); a typed ticket goes into the tickets folder (billet.json, note.txt) and comes back
/// in « Mes billets »; state, reply, red dot for Argos replies; Escape leaves the field then closes.
#[test]
fn support_billet_de_bout_en_bout() {
    use argos_core::retours::{Etat, Genre};
    use argos_render::support::{Champ, SupClick};
    dossier_a_soi("support");
    let d = argos_core::dossiers::de_test("retours");
    let _ = std::fs::remove_dir_all(&d);
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    // 🛟 in the 1st row: find it, click
    let t = argos_render::text::Text::default();
    let w = a.win_w();
    let x = (0..w as usize)
        .map(|x| x as f64)
        .find(|&x| a.top.clone().click(x, 28.0, w, &t, &|_: &str| false) == argos_render::topbar::TopClick::Support)
        .expect("🛟 Support dans la barre du haut");
    clic(&mut a, x + 4.0, 28.0);
    a.placer(taille);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Support, "🛟 ouvre le volet Support");
    let r = a.ui.support.as_ref().expect("volet Support").rect();
    assert_eq!(Some(r), a.volet_rect(), "à la place du volet flottant");
    assert!(a.modale_ouverte() && a.couche_au_dessus(a.rw() + 200.0, a.top_h() + 200.0), "couche modale");
    // the chart receives nothing: a selected horizontal line is not placed
    a.pick_tool("hline");
    let avant = a.chart.drawings.len();
    let (cx, cy) = (a.rw() + 300.0, a.top_h() + 250.0);
    clic(&mut a, cx, cy);
    assert_eq!(a.chart.drawings.len(), avant, "rien posé sur la chart sous la couche modale");
    assert!(a.ui.support.is_some(), "et le volet reste ouvert");
    // a ticket: type, subject, description, chart screenshot
    let g = a.ui.support.as_mut().unwrap();
    g.genre = Genre::Idee;
    g.focus = Some(Champ::Objet);
    a.key_support_nom("", Some("Un thème clair".into()));
    a.key_support_nom("Enter", None);
    a.key_support_nom("", Some("Pour lire en plein soleil.".into()));
    a.support_faire(SupClick::Capturer);
    assert!(a.ui.support.as_ref().unwrap().image.is_some(), "📷 la chart capturée");
    a.support_faire(SupClick::Envoyer);
    let g = a.ui.support.as_ref().unwrap();
    assert!(g.message.as_deref().is_some_and(|m| m.contains("#1")), "{:?}", g.message);
    assert!(g.objet.is_empty() && g.image.is_none(), "le formulaire se vide");
    assert_eq!(g.billets.len(), 1);
    let b = g.billets[0].clone();
    let dos = d.join(b.dossier());
    assert!(
        dos.join("billet.json").exists() && dos.join("note.txt").exists() && dos.join("capture.png").exists(),
        "{dos:?}"
    );
    assert_eq!((b.genre, b.objet.as_str(), b.contexte.ut.is_empty()), (Genre::Idee, "Un thème clair", false));
    // state, reply (you), Argos reply: red dot, then read
    a.support_faire(SupClick::Etat(b.id.clone(), Etat::EnCours));
    assert_eq!(a.ui.support.as_ref().unwrap().billets[0].etat, Etat::EnCours);
    a.ui.support.as_mut().unwrap().reponse = "Merci".into();
    a.support_faire(SupClick::Repondre(b.id.clone()));
    assert_eq!(a.ui.support.as_ref().unwrap().billets[0].reponses[0].auteur, "admin");
    argos_store::retours::changer(&d, &b.id, |x| x.repondre("argos", "C'est noté.", "2026-11-17T10:00:00Z")).unwrap();
    a.support_faire(SupClick::Relire);
    assert_eq!(a.ui.support_non_lues, 1, "réponse d'Argos pas lue");
    a.draw();
    assert_eq!(a.top.support, 1, "point rouge du bouton");
    a.support_faire(SupClick::Lu(b.id.clone()));
    assert_eq!(a.ui.support_non_lues, 0);
    // 💡 the « Qu'est-ce qu'on construit ensuite ? » card: Enter sends a 💡 Idée ticket by the same path, no image
    a.support_faire(SupClick::Relire);
    a.ui.support.as_mut().unwrap().focus = Some(Champ::Idee);
    a.key_support_nom("", Some("Un panneau de marchés au survol du symbole".into()));
    a.key_support_nom("Enter", None);
    let g = a.ui.support.as_ref().unwrap();
    assert_eq!(
        g.idee_message.as_deref(),
        Some("💡 Idée envoyée (billet n° 2). Merci !"),
        "{:?}",
        g.idee_message
    );
    assert!(g.idee.is_empty(), "le champ se vide");
    let i = g.billets.iter().find(|x| x.numero == 2).expect("billet n° 2").clone();
    assert_eq!((i.genre, i.objet.as_str()), (Genre::Idee, "Un panneau de marchés au survol du symbole"));
    assert!(!d.join(i.dossier()).join("capture.png").exists(), "sans image");
    // ✕: hidden 30 days, the date kept with the app's settings
    a.ui.support.as_mut().unwrap().idee_cachee = Some(crate::types::now_s() + argos_render::support::IDEE_CACHEE_S);
    a.support_faire(SupClick::CacherIdee);
    assert!(crate::App::interface2().idee_cachee.is_some_and(|t| t > crate::types::now_s() + 29 * 86_400));
    crate::App::interface2_changer(|i| i.idee_cachee = None);
    // Escape: leaves the field, then closes the panel
    a.ui.support.as_mut().unwrap().focus = Some(Champ::Reponse);
    a.key_support_nom("Escape", None);
    assert!(a.ui.support.is_some());
    a.key_support_nom("Escape", None);
    assert!(a.ui.support.is_none() && !a.watch.prefs.open, "Échap ferme");
    let _ = std::fs::remove_dir_all(&d);
}

/// 🔎 PC (Jee, 21 Nov): the mouse resting 0.3 s on the symbol pill opens the market panel just under it; away 0.3 s
/// from the pill and the panel, it closes; while its field has the keyboard it stays; a row opens its market and
/// closes it.
#[test]
fn panneau_marches_au_survol_pc() {
    dossier_a_soi("marches");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let p = a.pastille_symbole().expect("pastille du symbole");
    let recule = |a: &mut App| a.ui.marches_horloge -= std::time::Duration::from_millis(350);
    a.pointer.win_mouse = (p.cx(), p.cy());
    assert!(a.marches_tick(), "il attend ses 0,3 s");
    assert!(a.ui.marches.is_none());
    recule(&mut a);
    a.marches_tick();
    let m = a.ui.marches.clone().expect("0,3 s sur la pastille : ouvert");
    assert!((m.rect.x - p.x).abs() <= 6.0 && m.rect.y >= p.y + p.h && m.rect.y + m.rect.h <= a.taille_points().1);
    // on the panel: it stays; away from both: 0.3 s later it closes
    a.pointer.win_mouse = (m.rect.cx(), m.rect.cy());
    a.marches_tick();
    recule(&mut a);
    a.marches_tick();
    assert!(a.ui.marches.is_some(), "souris sur le panneau : il reste");
    a.pointer.win_mouse = (900.0, 700.0);
    a.marches_tick();
    recule(&mut a);
    a.marches_tick();
    assert!(a.ui.marches.is_none(), "0,3 s dehors : fermé");
    // the field has the keyboard: it stays open without the mouse; Escape closes
    a.marches_ouvrir(p, false);
    a.ui.marches.as_mut().unwrap().focus = true;
    a.marches_tick();
    recule(&mut a);
    a.marches_tick();
    assert!(a.ui.marches.is_some(), "champ actif : il reste ouvert");
    // a row (a recent market) opens its market and closes the panel
    a.ui.sym_recents = vec!["hl:ETH".into()];
    a.marches_remplir();
    let r = a.ui.marches.as_ref().unwrap().ligne_rect(0).expect("la ligne récente");
    clic(&mut a, r.cx(), r.cy());
    assert!(a.ui.marches.is_none(), "un marché choisi : fermé");
    assert_eq!(a.ui.sym_recents.first().map(String::as_str), Some("hl:ETH"));
}

/// 🖱 PC (Jee, 22 Nov: « the same panel as on hover, closed by a click outside »): a click on the symbol pill opens the
/// market panel, pinned: the mouse gone 1 s, it stays; a click on the chart closes it; a 2nd click on the pill too (and
/// the hover doesn't reopen it while the mouse stays there). Its « Ma watchlist › » row opens the Symboles panel.
#[test]
fn panneau_marches_au_clic_pc() {
    dossier_a_soi("marches-clic");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    a.placer(winit::dpi::PhysicalSize::new(1440, 900));
    let p = a.pastille_symbole().expect("pastille du symbole");
    let recule = |a: &mut App, ms| a.ui.marches_horloge -= std::time::Duration::from_millis(ms);
    a.pointer.win_mouse = (p.cx(), p.cy());
    clic(&mut a, p.cx(), p.cy());
    let m = a.ui.marches.clone().expect("clic sur la pastille : ouvert");
    assert!(m.epingle && (m.rect.x - p.x).abs() <= 6.0 && m.rect.y >= p.y + p.h);
    assert!(a.ui.symboles.is_none(), "plus le volet Symboles");
    // the mouse gone 1 s: still open
    a.pointer.win_mouse = (900.0, 700.0);
    for _ in 0..4 {
        a.marches_tick();
        recule(&mut a, 350);
    }
    a.marches_tick();
    assert!(a.ui.marches.is_some(), "épinglé : il reste sans la souris");
    // a click on the chart closes it
    clic(&mut a, 900.0, 700.0);
    assert!(a.ui.marches.is_none(), "clic dehors : fermé");
    // open again, 2nd click on the pill: closed, and the hover doesn't reopen it
    a.pointer.win_mouse = (p.cx(), p.cy());
    clic(&mut a, p.cx(), p.cy());
    assert!(a.ui.marches.is_some());
    clic(&mut a, p.cx(), p.cy());
    assert!(a.ui.marches.is_none(), "2e clic sur la pastille : fermé");
    for _ in 0..3 {
        a.marches_tick();
        recule(&mut a, 350);
    }
    assert!(a.ui.marches.is_none(), "la souris restée sur la pastille ne le rouvre pas");
    // the mouse leaves then comes back: the hover works again
    a.pointer.win_mouse = (900.0, 700.0);
    a.marches_tick();
    a.pointer.win_mouse = (p.cx(), p.cy());
    a.marches_tick();
    recule(&mut a, 350);
    a.marches_tick();
    assert!(a.ui.marches.as_ref().is_some_and(|m| !m.epingle), "survol : ouvert, pas épinglé");
    // a click on the pill pins the hover panel
    clic(&mut a, p.cx(), p.cy());
    assert!(a.ui.marches.as_ref().is_some_and(|m| m.epingle));
    // « Ma watchlist › »: the Symboles panel
    let wr = a.ui.marches.as_ref().unwrap().watchlist_rect().expect("Ma watchlist (épinglé)");
    clic(&mut a, wr.cx(), wr.cy());
    assert!(a.ui.marches.is_none() && a.ui.symboles.is_some(), "le volet Symboles");
}

/// 👆 « Ma watchlist › » (Jee, 21 Nov, « Marchés + lien Watchlist »): by the finger (phone sheet, tablet panel) a row at
/// the top of the market panel; touching it closes the panel and opens the whole watchlist. The PC has no such row.
#[test]
fn ma_watchlist_au_doigt() {
    dossier_a_soi("marches-ma-watchlist");
    let mut a = app();
    let p = Rect::new(20.0, 60.0, 200.0, 34.0);
    a.marches_ouvrir(p, false);
    assert!(a.ui.marches.as_ref().unwrap().watchlist_rect().is_none(), "PC : pas de ligne");
    for feuille in [true, false] {
        argos_render::actions::set_tactile(true);
        a.watch.prefs.open = false;
        a.marches_ouvrir(p, feuille);
        let r = a.ui.marches.as_ref().unwrap().watchlist_rect().expect("la ligne au doigt");
        assert!(r.h >= argos_render::formfactor::TOUCH_MIN_PT);
        assert!(a.marches_click(r.cx(), r.cy()));
        argos_render::actions::set_tactile(false);
        assert!(a.ui.marches.is_none(), "panneau fermé");
        assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Watchlist, "la watchlist complète");
    }
}

/// 🔎 The market panel is never empty (Jee, 21 Nov): FAVORIS = the flagged rows, then Jee's watchlist rows (no duplicate
/// with RÉCENTS, 10 at most); nothing at all → POPULAIRES (biggest 24 h volumes already in memory), else the hint.
#[test]
fn panneau_marches_jamais_vide() {
    dossier_a_soi("marches-jamais-vide");
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    a.watch.prefs.flags.clear();
    a.ui.sym_recents = vec!["BTC".into()];
    a.marches_ouvrir(Rect::new(20.0, 60.0, 200.0, 34.0), false);
    let m = a.ui.marches.clone().unwrap();
    let titres: Vec<&str> = m.sections().iter().map(|(t, _)| *t).collect();
    assert_eq!(titres, ["RÉCENTS", "FAVORIS"]);
    let ids: Vec<&str> = m.favoris.iter().map(|l| l.id.as_str()).collect();
    assert!(ids.contains(&"hl:ETH") && ids.len() <= 10, "{ids:?}");
    assert!(!ids.contains(&"hl:BTC"), "BTC is already in RÉCENTS: {ids:?}");
    // a new profile without a watchlist: POPULAIRES from the prices the app has, by 24 h volume
    (a.store, a.ui.sym_recents, a.feeds.tous) = (None, vec![], None);
    a.lists.clear();
    a.watch.prefs.list = None;
    a.watch.selected = "BTC".into();
    a.feeds.quotes.clear();
    a.marches_remplir();
    assert!(
        a.ui.marches.as_ref().unwrap().sections().is_empty(),
        "nothing in memory: « Tape un nom de marché. »"
    );
    for k in 1..=10 {
        let q = argos_data::watchlist::Quote {
            coin: format!("C{k}"),
            mark: "1".into(),
            prev: 1.0,
            volume: k as f64 * 1000.0,
            growth: false,
            oracle: 1.0,
            oi: 0.0,
            funding: 0.0,
        };
        a.feeds.quotes.insert(q.coin.clone(), q);
    }
    a.marches_remplir();
    let m = a.ui.marches.as_ref().unwrap();
    let s = m.sections();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].0, "POPULAIRES");
    let ids: Vec<&str> = s[0].1.iter().map(|l| l.id.as_str()).collect();
    assert_eq!(ids, ["C10", "C9", "C8", "C7", "C6", "C5", "C4", "C3"]);
}

/// ⭐ FAVORIS follow the chart's exchange (Jee, 22 Nov): on a Binance perp chart, the Hyperliquid favourites BTC and
/// ETH show as their Binance perps; one Binance doesn't have stays on Hyperliquid; on a Hyperliquid chart nothing changes.
#[test]
fn favoris_sur_la_bourse_du_graphique() {
    dossier_a_soi("marches-favoris-bourse");
    let ins = |ex: &str, sym: &str, base: &str, quote: &str, kind| argos_core::Instrument {
        exchange: ex.into(),
        symbol: sym.into(),
        base: base.into(),
        quote: quote.into(),
        kind,
        group: "Crypto".into(),
        price_decimals: 2,
        size_decimals: 3,
        growth: false,
        active: true,
    };
    use argos_core::MarketKind::{Perp, Spot};
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    let st = a.store.as_mut().unwrap();
    let binance = [
        ins("binance", "BTCUSDT", "BTC", "USDT", Perp),
        ins("binance", "BTCUSDC", "BTC", "USDC", Perp),
        ins("binance", "ETHUSDC", "ETH", "USDC", Perp),
        ins("binance", "ETHUSDT", "ETH", "USDT", Perp),
    ];
    st.save_catalog("binance", &binance, now_s()).unwrap();
    st.save_catalog("binance-spot", &[ins("binance-spot", "HYPEUSDT", "HYPE", "USDT", Spot)], now_s())
        .unwrap();
    a.load_lists();
    a.watch.prefs.flags.clear();
    a.watch.prefs.flags.push(("hl:BTC".into(), 0));
    a.watch.prefs.flags.push(("hl:ETH".into(), 0));
    a.watch.prefs.flags.push(("hl:HYPE".into(), 0));
    a.ui.sym_recents.clear();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    a.watch.selected = "binance:BTCUSDC".into();
    a.marches_ouvrir(Rect::new(20.0, 60.0, 200.0, 34.0), false);
    let m = a.ui.marches.clone().unwrap();
    let f = |id: &str| m.favoris.iter().find(|l| l.id == id).cloned();
    let btc = f("binance:BTCUSDC").expect("BTC on Binance, the chart's quote first");
    assert_eq!((btc.bourse.as_str(), btc.perp), ("Binance", true));
    assert!(f("binance:ETHUSDC").is_some(), "ETH: the chart's quote (USDC) first: {:?}", m.favoris);
    assert!(f("hl:HYPE").is_some(), "no HYPE perp on Binance (only spot): stays on Hyperliquid");
    assert!(f("hl:BTC").is_none() && f("hl:ETH").is_none());
    // the chart opened on Binance without a Binance row selected (opened by its address): USDT first
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    a.watch.selected = "BTC".into();
    a.marches_remplir();
    let ids: Vec<String> = a.ui.marches.as_ref().unwrap().favoris.iter().map(|l| l.id.clone()).collect();
    assert!(
        ids.contains(&"binance:BTCUSDT".to_string()) && ids.contains(&"binance:ETHUSDT".to_string()),
        "{ids:?}"
    );
    // a Hyperliquid chart: the favourites as they are
    a.chart.source = Source::Live {
        exchange: Exchange::Hyperliquid,
        base: "BTC".into(),
    };
    a.marches_remplir();
    let ids: Vec<String> = a.ui.marches.as_ref().unwrap().favoris.iter().map(|l| l.id.clone()).collect();
    for id in ["hl:BTC", "hl:ETH", "hl:HYPE"] {
        assert!(ids.iter().any(|i| i == id), "{ids:?}");
    }
    assert!(!ids.iter().any(|i| i.starts_with("binance")), "{ids:?}");
}

/// 📈 FAVORIS brought to Binance get their 24 h change (Jee's capture 202611221633: nothing on the right) from the
/// Binance tickers the app already read: no new request; a spot market or a non-USDT quote gets none from them.
#[test]
fn favoris_binance_avec_variation_24h() {
    dossier_a_soi("marches-favoris-variation");
    let ins = |sym: &str, base: &str, quote: &str| argos_core::Instrument {
        exchange: "binance".into(),
        symbol: sym.into(),
        base: base.into(),
        quote: quote.into(),
        kind: argos_core::MarketKind::Perp,
        group: "Crypto".into(),
        price_decimals: 2,
        size_decimals: 3,
        growth: false,
        active: true,
    };
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    let st = a.store.as_mut().unwrap();
    st.save_catalog("binance", &[ins("BTCUSDT", "BTC", "USDT"), ins("ETHUSDC", "ETH", "USDC")], now_s())
        .unwrap();
    a.load_lists();
    a.watch.prefs.flags.clear();
    a.watch.prefs.flags.push(("binance:BTCUSDT".into(), 0));
    a.watch.prefs.flags.push(("binance:ETHUSDC".into(), 0));
    a.ui.sym_recents.clear();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    a.watch.selected = "binance:BTCUSDT".into();
    let t = |base: &str, price: f64, prev: f64| argos_data::tickers::Ticker {
        ex: Exchange::Binance,
        base: base.into(),
        price,
        prev,
        vol_usd: 1e9,
    };
    a.feeds.tickers.insert(
        Exchange::Binance,
        (Instant::now(), vec![t("BTC", 80_600.0, 79_000.0), t("ETH", 2_430.0, 2_500.0)]),
    );
    a.marches_ouvrir(Rect::new(20.0, 60.0, 200.0, 34.0), false);
    let m = a.ui.marches.clone().unwrap();
    let f = |id: &str| m.favoris.iter().find(|l| l.id == id).cloned().unwrap();
    let v = f("binance:BTCUSDT").variation.expect("24 h change from the tickers");
    assert!((v - 2.025).abs() < 0.01, "{v}");
    assert_eq!(f("binance:ETHUSDC").variation, None, "the tickers are the USDT perps only");
}

/// 🔎 The Symboles panel's header says the exchange of the list shown (Jee's capture, 22 Nov: « BINANCE PERPÉTUEL »
/// above a Hyperliquid -USDC list on a Binance chart); nothing when the list mixes exchanges.
#[test]
fn volet_symboles_en_tete_bourse_de_la_liste() {
    dossier_a_soi("symboles-en-tete");
    let mut a = app();
    let row = |coin: &str| wl::Row::new(coin, coin, "1", 1.0);
    a.top.source = "Binance Perpétuel".into();
    a.watch.sections = vec![wl::Section {
        name: "Crypto".into(),
        rows: vec![row("BTC"), row("hl:ETH"), row("xyz:NVDA")],
    }];
    assert_eq!(a.sym_donnees().bourse, "Hyperliquid");
    a.watch.sections[0].rows.push(row("binance:BTCUSDT"));
    assert_eq!(a.sym_donnees().bourse, "", "mixed: no exchange");
    a.watch.sections[0].rows = vec![row("binance:BTCUSDT")];
    assert_eq!(a.sym_donnees().bourse, "Binance");
    a.watch.sections[0].rows.push(row("oanda:XAU_USD"));
    assert_eq!(a.sym_donnees().bourse, "", "an OANDA row: not only Binance");
}

/// 🔍 PC (Jee, 17 Nov): one navigation pill, centred at the bottom of the chart, always visible (no hover needed),
/// above the economic-event row: the hand on its buttons, + zooms in, − zooms out, ↺ resets; with or without the Lens
/// card (`rien_ne_se_chevauche_sur_pc` checks it crosses no other zone).
#[test]
fn pilule_de_navigation_fixe_pc() {
    use crate::types::Curseur::Icone;
    use argos_render::indicators::Ind;
    use winit::window::CursorIcon;
    dossier_a_soi("pilule-navigation");
    for lens in [false, true] {
        let mut a = app();
        a.appliquer_cadre(&crate::actions::Interface2::default());
        a.top.prefs.bienvenue_vue = true;
        a.ind.on = if lens { vec![Ind::Lens, Ind::Rsi] } else { vec![Ind::Rsi] };
        a.ui.frames = 5;
        a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(1440, 900));
        a.draw_frame();
        a.draw_frame();
        let vp = a.chart.viewport;
        let p = argos_render::navbar::rect(&vp, vp.plot_h(), true);
        assert!((p.cx() - vp.plot_w() / 2.0).abs() < 0.5, "centred {p:?}");
        assert!(p.y + p.h <= vp.plot_h() - 34.0 + 0.01, "above the event row {p:?}");
        let c = argos_render::navbar::centers(&vp, vp.plot_h(), true);
        let (rw, th) = (a.rw(), a.top_h());
        let bouton = |k: usize| (rw + c[k].0, th + c[k].1);
        let (x, y) = bouton(1);
        a.mouse_moved(PhysicalPosition::new(x, y));
        assert_eq!(a.curseur_pose(), Icone(CursorIcon::Pointer), "the hand on +");
        let avant = a.chart.viewport.bar_spacing;
        clic(&mut a, x, y);
        assert!(a.chart.viewport.bar_spacing > avant, "+ zooms in");
        let (x, y) = bouton(0);
        clic(&mut a, x, y);
        clic(&mut a, x, y);
        assert!(a.chart.viewport.bar_spacing < avant, "− zooms out");
        let (x, y) = bouton(4);
        clic(&mut a, x, y);
        assert_eq!(a.chart.viewport.bar_spacing, avant, "↺ resets the zoom");
    }
}

/// 🖍 The bar of the selected drawing (Jee's mockup): it shows on the selected drawing, in the chart; its padlock
/// locks (a locked drawing stays selectable, its bar too) and unlocks; the width menu changes the width; ⠿ moves it;
/// 🗑 removes the drawing and ↶ brings it back.
#[test]
fn barre_du_dessin_choisi() {
    use argos_render::drawbar::DbBtn;
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.track_drawings();
    assert!(a.drawbar().is_none(), "nothing selected: no bar");
    a.selected = Some(0);
    let (_, bar) = a.drawbar().expect("the bar of the selected drawing");
    let vp = a.chart.viewport;
    assert!(
        bar.rect.x >= 0.0 && bar.rect.x + bar.rect.w <= vp.plot_w() && bar.rect.y >= 0.0,
        "{:?}",
        bar.rect
    );
    let at = |a: &App, b: DbBtn| {
        let (_, bar) = a.drawbar().expect("bar");
        let r = bar.at(b).expect("button");
        (r.cx() + a.rw(), r.cy() + a.top_h())
    };
    // padlock: locked, still selected with its bar; again: unlocked
    let (x, y) = at(&a, DbBtn::Lock);
    clic(&mut a, x, y);
    assert!(a.chart.drawings[0].locked() && a.drawbar().is_some(), "locked, the bar stays");
    clic(&mut a, x, y);
    assert!(!a.chart.drawings[0].locked(), "unlocked");
    // width: 3px from its menu
    let (x, y) = at(&a, DbBtn::Width);
    clic(&mut a, x, y);
    assert!(
        matches!(a.ui.list.as_ref().map(|l| l.0), Some(ListKind::DrawBar(0, DbBtn::Width))),
        "the width menu"
    );
    a.drawbar_pick(0, DbBtn::Width, 3);
    assert_eq!(a.chart.drawings[0].style.width, 3.0);
    a.ui.list = None;
    // ⠿ drags the bar
    let before = a.drawbar().unwrap().1.rect;
    let (x, y) = at(&a, DbBtn::Handle);
    a.mouse_moved(PhysicalPosition::new(x, y));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(x + 40.0, y + 30.0));
    a.left_button(ElementState::Released);
    let after = a.drawbar().unwrap().1.rect;
    assert!(
        (after.x - before.x - 40.0).abs() < 0.5 && (after.y - before.y - 30.0).abs() < 0.5,
        "{before:?} → {after:?}"
    );
    // 🗑 at once, ↶ brings it back
    let (x, y) = at(&a, DbBtn::Trash);
    clic(&mut a, x, y);
    assert!(a.chart.drawings.is_empty(), "removed");
    a.track_drawings();
    touche(&mut a, lettre("z"), ModifiersState::CONTROL);
    assert_eq!(a.chart.drawings.len(), 1, "undo brings it back");
}

/// 🎨 The palette of the drawing bar: the pencil opens it under its button; a swatch colours the line; the slider
/// sets the opacity; « + » keeps the colour among the custom ones; T colours the text; a 2nd click closes it.
#[test]
fn palette_de_la_barre_du_dessin() {
    use argos_render::colorpick::split;
    use argos_render::drawbar::DbBtn;
    dossier_a_soi("palette-dessin");
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.selected = Some(0);
    let at = |a: &App, b: DbBtn| {
        let r = a.drawbar().expect("bar").1.at(b).expect("button");
        (r.cx() + a.rw(), r.cy() + a.top_h())
    };
    let (x, y) = at(&a, DbBtn::LineColor);
    clic(&mut a, x, y);
    let cp = a.ui.colorpick.as_ref().expect("the palette").2.clone();
    assert!(cp.rect.y > y - 20.0 || cp.rect.y + cp.rect.h < y, "under (or over) its button");
    // the first vivid colour (2nd row, 1st column)
    let (sx, sy) = (cp.rect.x + 14.0 + 11.0, cp.rect.y + 14.0 + 26.0 + 11.0);
    clic(&mut a, sx, sy);
    assert_eq!(a.chart.drawings[0].style.color, "#f23645");
    // the slider: press at its start, drag to its middle
    let cp = a.ui.colorpick.as_ref().unwrap().2.clone();
    let (_, h) = argos_render::colorpick::ColorPick::size();
    let ty = cp.rect.y + h - 14.0 - 16.0;
    let (x0, x1) = (cp.rect.x + 14.0 + 1.0, cp.rect.x + 14.0 + (cp.rect.w - 28.0 - 70.0) / 2.0);
    a.mouse_moved(PhysicalPosition::new(x0, ty));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(x1, ty));
    a.left_button(ElementState::Released);
    let st = &a.chart.drawings[0].style;
    assert!((st.opacity - 0.5).abs() < 0.05, "half opacity: {}", st.opacity);
    assert_eq!(st.color, "#f23645", "the colour stays « #rrggbb » (older versions read it)");
    let _ = split;
    // a 2nd click on the pencil closes it; T opens it for the text
    let (x, y) = at(&a, DbBtn::LineColor);
    clic(&mut a, x, y);
    assert!(a.ui.colorpick.is_none(), "closed");
    let (x, y) = at(&a, DbBtn::TextColor);
    clic(&mut a, x, y);
    assert!(
        a.ui.colorpick.as_ref().is_some_and(|c| c.1 == argos_render::objdlg::Which::Text),
        "the text colour palette"
    );
    let cp = a.ui.colorpick.as_ref().unwrap().2.clone();
    clic(&mut a, cp.rect.x + 14.0 + 11.0, cp.rect.y + 14.0 + 11.0);
    assert_eq!(a.chart.drawings[0].style.tcolor, "#ffffff");
}

/// 🖍 Drawing templates (bar ▸ Modèle): save the style under a name, apply it to another drawing of the same tool
/// (its text kept), apply the default, remove it with ✕.
#[test]
fn modeles_de_dessin() {
    use argos_render::drawbar::DbBtn;
    dossier_a_soi("modeles-dessin");
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    let mut d = argos_render::drawings::Drawing::hline(p);
    d.style.color = "#ff9800".into();
    d.style.width = 3.0;
    a.chart.drawings.push(d);
    let mut other = argos_render::drawings::Drawing::hline(p * 0.99);
    other.style.text = "garde".into();
    a.chart.drawings.push(other);
    a.selected = Some(0);
    // Template ▸ « Sauvegarder… » opens the name box; its OK keeps the template
    a.drawbar_pick(0, DbBtn::Template, 300);
    assert!(a.ui.name_box.as_ref().is_some_and(|b| b.kind == argos_render::wlmenu::NameFor::DrawingTemplate));
    a.ui.name_box.as_mut().unwrap().value = "Mon rayon or".into();
    a.name_ok();
    assert_eq!(a.template_names("hline"), vec!["Mon rayon or".to_string()]);
    // applied to the other line: colour and width, its text kept
    a.selected = Some(1);
    a.drawbar_pick(1, DbBtn::Template, 400);
    let st = &a.chart.drawings[1].style;
    assert_eq!((st.color.as_str(), st.width, st.text.as_str()), ("#ff9800", 3.0, "garde"));
    // the default template
    a.drawbar_pick(1, DbBtn::Template, 301);
    assert_eq!(a.chart.drawings[1].style.width, argos_render::drawings::Style::default().width);
    // ✕ removes it
    a.drawbar_remove(1, DbBtn::Template, 400, argos_render::shapes::Rect::new(100.0, 100.0, 10.0, 10.0));
    assert!(a.template_names("hline").is_empty());
}

/// ⚙ The settings window of a drawing (Jee's mockup): « Droite » opens the shared palette with « Épaisseur » and
/// « Style de ligne »; its width box sets the width; ✎ renames; « Modèle ⌄ » ✕ removes a saved template.
#[test]
fn reglages_du_dessin_palette_et_modeles() {
    dossier_a_soi("reglages-dessin");
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.templates
        .insert("hline".into(), vec![("Or".into(), argos_render::drawings::Style::default())]);
    a.open_obj_dlg(0, None);
    let dl = a.ui.obj_dlg.take().unwrap();
    let mut d = a.chart.drawings[0].clone();
    let mut dl2 = dl.clone();
    // « Droite »: the palette with the line part, over the window
    let sw = (dl.rect.x + 18.0 + 76.0 + 10.0, dl.rect.y + 106.0 + 17.0);
    let c = dl2.click(sw.0, sw.1, &mut d);
    a.dlg_result(dl2, c);
    let cp = a.ui.colorpick.as_ref().expect("the palette").2.clone();
    assert!(cp.line.is_some(), "with width and line style");
    assert!(a.ui.obj_dlg.is_some(), "the window stays open under it");
    // ✕ of « Modèle ⌄ » removes the saved template
    let c = argos_render::objdlg::DlgClick::RemoveTemplate(0);
    let dl = a.ui.obj_dlg.take().unwrap();
    a.dlg_result(dl, c);
    assert!(a.templates.get("hline").is_some_and(|v| v.is_empty()), "removed");
}

/// 🔔 The alarm of the drawing bar opens « Créer une alerte sur »; « Créer » adds an alert that follows the line
/// (price crossing it); the right-click « Ajouter une alerte » opens the same window.
#[test]
fn alerte_sur_le_dessin() {
    use argos_render::drawbar::DbBtn;
    dossier_a_soi("alerte-dessin");
    let mut a = app();
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    a.selected = Some(0);
    let r = a.drawbar().unwrap().1.at(DbBtn::Alert).expect("the alarm");
    let (x, y) = (r.cx() + a.rw(), r.cy() + a.top_h());
    clic(&mut a, x, y);
    let w = a.ui.draw_alert.as_ref().expect("the alert window").1.clone();
    let before = a.alerts.list.len();
    // « Créer » (bottom right)
    clic(&mut a, w.rect.x + w.rect.w - 24.0 - 47.0, w.rect.y + w.rect.h - 42.0);
    assert!(a.ui.draw_alert.is_none(), "closed");
    assert_eq!(a.alerts.list.len(), before + 1);
    let al = a.alerts.list.last().unwrap();
    let argos_core::alerts::Kind::Multi { conds, .. } = &al.kind else {
        panic!("multi")
    };
    assert_eq!(conds[0].op, argos_core::alerts::Op::Cross);
    assert!(matches!(conds[0].r, argos_core::alerts::Src::Trend { p0, .. } if p0 == p));
    // the right-click row opens the same window
    a.obj_action(0, argos_render::objmenu::ObjAction::Alert);
    assert!(a.ui.draw_alert.is_some());
}

/// 🖍 Bar ⋯ sync: « Pas de sync » keeps a drawing on its chart; « Synchroniser dans la mise en page » shows it on the
/// split panes of the same symbol; « Synchronisation globale » copies it into every saved layout of the symbol; a
/// removed drawing, or one set back to « Pas de sync », loses its copies; unsynced drawings are never touched.
#[test]
fn synchro_des_dessins() {
    use crate::actions::layouts::{sync_dans, Layout, Layouts};
    use argos_render::drawings::Drawing;
    let mut a = app();
    let coin = a.main_coin();
    let mut d = Drawing::hline(100.0);
    a.chart.drawings.push(d.clone());
    assert!(a.pane_drawings(&coin).is_empty(), "Pas de sync: only on its chart");
    a.chart.drawings[0].style.sync = 1;
    assert_eq!(a.pane_drawings(&coin).len(), 1, "in the layout: the panes of the same symbol");
    assert!(a.pane_drawings("autre:XYZ").is_empty(), "not on another symbol");
    // global: two layouts of this symbol (one with its own unsynced drawing), one of another symbol
    let saved = |v: Vec<Drawing>| saved::Saved {
        drawings: v,
        ..Default::default()
    };
    let mine = Drawing::hline(50.0);
    let mut all = Layouts {
        ouverte: "A".into(),
        liste: vec![
            Layout {
                nom: "A".into(),
                dessins: Some(("btc.json".into(), saved(vec![mine.clone()]))),
                ..Default::default()
            },
            Layout {
                nom: "B".into(),
                dessins: Some(("eth.json".into(), saved(vec![]))),
                ..Default::default()
            },
        ],
    };
    d.style.sync = 2;
    assert!(sync_dans(&mut all, "btc.json", std::slice::from_ref(&d)));
    let a_dessins = &all.liste[0].dessins.as_ref().unwrap().1.drawings;
    assert_eq!(a_dessins, &vec![mine.clone(), d.clone()], "copied, the layout's own drawing untouched");
    assert!(all.liste[1].dessins.as_ref().unwrap().1.drawings.is_empty(), "another symbol untouched");
    // changed: the copy follows; removed or back to « Pas de sync »: the copies go, the original stays
    d.style.width = 4.0;
    assert!(sync_dans(&mut all, "btc.json", std::slice::from_ref(&d)));
    assert_eq!(all.liste[0].dessins.as_ref().unwrap().1.drawings[1].style.width, 4.0);
    assert!(sync_dans(&mut all, "btc.json", &[]));
    assert_eq!(all.liste[0].dessins.as_ref().unwrap().1.drawings, vec![mine], "copies removed, the rest kept");
    assert!(!sync_dans(&mut all, "btc.json", &[]), "nothing more to change: no write");
}

/// 🖍 On a COPY of Jee's layouts.json (path in ARGOS_LAYOUTS_COPIE; skipped without it): reading and writing back
/// changes nothing byte for byte, and a global drawing added then removed leaves every byte as it was.
#[test]
fn synchro_sur_une_copie_de_layouts() {
    use crate::actions::layouts::{sync_dans, Layouts};
    let Ok(path) = std::env::var("ARGOS_LAYOUTS_COPIE") else { return };
    let raw = std::fs::read_to_string(&path).expect("the copy");
    let mut all: Layouts = serde_json::from_str(&raw).expect("layouts");
    // read and written back by this version: nothing lost, nothing changed — only new style fields (with their
    // defaults) may appear in the drawings
    fn inside(old: &serde_json::Value, new: &serde_json::Value) -> bool {
        match (old, new) {
            (serde_json::Value::Object(o), serde_json::Value::Object(n)) => o.iter().all(|(k, v)| n.get(k).is_some_and(|w| inside(v, w))),
            (serde_json::Value::Array(o), serde_json::Value::Array(n)) => o.len() == n.len() && o.iter().zip(n).all(|(v, w)| inside(v, w)),
            _ => old == new,
        }
    }
    let old: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let first = serde_json::to_string_pretty(&all).unwrap();
    assert!(inside(&old, &serde_json::from_str(&first).unwrap()), "nothing lost or changed");
    let file = all
        .liste
        .iter()
        .find_map(|l| l.dessins.as_ref().map(|d| d.0.clone()))
        .expect("a layout with drawings");
    let mut g = argos_render::drawings::Drawing::hline(1.0);
    g.style.sync = 2;
    let before: Vec<_> = all.liste.iter().map(|l| l.dessins.as_ref().map(|d| d.1.drawings.len())).collect();
    sync_dans(&mut all, &file, &[g]);
    let after: Vec<_> = all.liste.iter().map(|l| l.dessins.as_ref().map(|d| d.1.drawings.len())).collect();
    assert_ne!(before, after, "copied into the layouts of that symbol");
    sync_dans(&mut all, &file, &[]);
    assert_eq!(serde_json::to_string_pretty(&all).unwrap(), first, "removed: every byte as it was");
}

/// 📱 The drawing bar on the phone (390 points), by touch: the line touched shows its bar, whole inside the screen,
/// touch zones of at least 40 points; the padlock locks by touch (the line no longer moves under the finger); 🗑
/// removes; the template and the text colour are in ⋯.
#[test]
fn barre_du_dessin_au_doigt_sur_cell() {
    use argos_render::drawbar::DbBtn;
    use winit::event::TouchPhase;
    let mut a = cell_390();
    a.draw();
    a.chart.viewport.width = a.area.0;
    let p = a.chart.bars[a.chart.bars.len() - 5].close;
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(p));
    let y = a.frame_now().unwrap().geo().y_of(p);
    let x = a.chart.viewport.plot_w() / 2.0;
    toucher_chart(&mut a, x, y);
    assert_eq!(a.selected, Some(0), "the line touched is selected");
    let (_, bar) = a.drawbar().expect("its bar");
    assert!(
        bar.rect.x >= 0.0 && bar.rect.x + bar.rect.w <= a.chart.viewport.width as f64,
        "inside: {:?}",
        bar.rect
    );
    assert!(bar.btns.iter().all(|(r, _)| r.w >= 40.0 && r.h >= 38.0), "touch zones");
    assert_eq!(bar.folded, vec![DbBtn::Template, DbBtn::TextColor], "in ⋯");
    // padlock by touch: locked; a drag on the line no longer moves it
    let r = bar.at(DbBtn::Lock).unwrap();
    toucher_chart(&mut a, r.cx(), r.cy());
    assert!(a.chart.drawings[0].locked(), "locked by touch");
    let (wx, wy) = (x + a.rw(), y + a.top_h());
    a.touch(9, TouchPhase::Started, (wx, wy));
    a.touch(9, TouchPhase::Moved, (wx, wy + 60.0));
    a.touch(9, TouchPhase::Ended, (wx, wy + 60.0));
    assert_eq!(a.chart.drawings[0].pts[0].p, p, "a locked line stays");
    // unlock, then 🗑
    let r = a.drawbar().unwrap().1.at(DbBtn::Lock).unwrap();
    toucher_chart(&mut a, r.cx(), r.cy());
    let r = a.drawbar().unwrap().1.at(DbBtn::Trash).unwrap();
    toucher_chart(&mut a, r.cx(), r.cy());
    assert!(a.chart.drawings.is_empty(), "removed by touch");
}

/// 📱 A touch at (x, y) of the chart (chart coordinates).
fn toucher_chart(a: &mut App, x: f64, y: f64) {
    let (wx, wy) = (x + a.rw(), y + a.top_h());
    toucher(a, wx, wy);
}

/// 📡 The live prices are asked for the chart's Hyperliquid market, the demo positions (with the Hyperliquid
/// equivalent of an OANDA symbol, as `demo_price` reads it) and nothing from the Hyperliquid list itself (its Crypto
/// section is in the feed already).
#[test]
fn prix_en_direct_demandes() {
    let mut a = app();
    chart_hl(&mut a, "BTC");
    for base in ["ETH", "XAUUSD"] {
        let o = argos_core::portfolio::Order {
            base: base.into(),
            side: argos_core::Side::Long,
            qty: 1.0,
            price: 100.0,
            stop: None,
            target: None,
            couts: None,
            perp: None,
        };
        a.demo.open(o, now_s()).unwrap();
    }
    a.want_prices();
    let w = a.feeds.hl_wanted.lock().unwrap().clone();
    for c in ["BTC", "ETH", "xyz:GOLD"] {
        assert!(w.contains(&c.to_string()), "{c} in {w:?}");
    }
    let mut d = w.clone();
    d.dedup();
    assert_eq!(d.len(), w.len(), "no double");
}

/// 🎯 vitesse-1: while only the mouse moves the chart image is kept (only the crosshair layer and what goes over it
/// are drawn again), and the image is exactly the one `render` draws; a change of the bars draws it again; a long /
/// short position (it lights up under the mouse) draws the chart whole each time.
#[test]
fn image_gardee_quand_seule_la_souris_bouge() {
    use argos_render::indicators::Ind;
    let mut a = app();
    // « jee crypto »: their numbers hold NaN (warm-up), never equal to themselves: the image is kept all the same
    a.ind.on = vec![Ind::AnchoredVwap, Ind::Cipher, Ind::Tvd, Ind::Tbo, Ind::AiTrend];
    a.refresh_indicators();
    let sig = a.ind_sig();
    let mut kept = None;
    let mut c = a.to_chart();
    assert!(c.indicators.is_some_and(|o| o.cipher.is_some()));
    c.navigation = true;
    // (other tests running at the same time may change the theme, the glass or the language, shared by the whole
    // program: the image is then rightly drawn again; the series is tried again, a few times at most)
    let mut kept_all = false;
    for _ in 0..5 {
        let mut place = None;
        kept_all = true;
        for m in [Some((100.0, 80.0)), Some((240.0, 120.0)), Some((300.0, c.viewport.plot_h() - 15.0)), None] {
            c.crosshair = m;
            let before = argos_render::render(&c);
            let img = crate::fond::image(&mut kept, &c, &sig);
            let after = argos_render::render(&c);
            if before.data() != after.data() {
                kept_all = false; // shared settings changed meanwhile: try the series again
                break;
            }
            assert_eq!(img.data(), after.data(), "the same image as render, mouse {m:?}");
            let p = kept.as_ref().expect("kept").place();
            kept_all &= *place.get_or_insert(p) == p;
        }
        if kept_all {
            break;
        }
    }
    assert!(kept_all, "kept, not drawn again");
    // other bars: drawn again
    let mut bars = c.bars.to_vec();
    if let Some(b) = bars.last_mut() {
        b.close += 1.0;
        b.high = b.high.max(b.close);
    }
    let mut c2 = c.clone();
    c2.bars = &bars;
    let before = argos_render::render(&c2);
    let img = crate::fond::image(&mut kept, &c2, &sig);
    if before.data() == argos_render::render(&c2).data() {
        assert_eq!(img.data(), before.data(), "drawn again for other bars");
    }
    // a position drawn: no image kept
    let mut c3 = c.clone();
    let (t0, p0) = (bars[bars.len() - 20].time, bars[bars.len() - 20].close);
    let pt = |t: i64, p: f64| argos_render::drawings::Point { t, p };
    c3.drawings.push(argos_render::drawings::Drawing::new(
        "long",
        vec![pt(t0, p0), pt(t0, p0 * 1.02), pt(t0, p0 * 0.99)],
    ));
    let before = argos_render::render(&c3);
    let img = crate::fond::image(&mut kept, &c3, &sig);
    assert!(kept.is_none());
    if before.data() == argos_render::render(&c3).data() {
        assert_eq!(img.data(), before.data(), "a position drawn: drawn whole");
    }
}

/// 📡 The live prices are asked only for the markets on screen: the ticker band, and the list rows in view when the
/// list is open (not the 120 markets of the list), plus the chart, demo positions and alerts.
#[test]
fn prix_en_direct_seulement_a_l_ecran() {
    use argos_data::watchlist::{Quote, Section};
    let mut a = app();
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(1440, 900));
    let quotes: Vec<Quote> = (0..120)
        .map(|i| Quote {
            coin: format!("C{i:03}"),
            mark: "1.5".into(),
            prev: 1.0,
            volume: 1e9 - i as f64,
            growth: false,
            oracle: 1.5,
            oi: 1.0,
            funding: 0.0,
        })
        .collect();
    let crypto = Section {
        name: "Crypto".into(),
        quotes: quotes.clone(),
    };
    a.on_user_event(UserEvent::Quotes(vec![crypto], quotes));
    let wanted = |a: &App| a.feeds.hl_wanted.lock().unwrap().clone();
    a.watch.prefs.open = false;
    a.draw();
    let band = wanted(&a);
    assert!(!band.is_empty() && band.len() < 40, "the band only: {band:?}");
    a.watch.prefs.open = true;
    a.watch.prefs.panel = wl::Tool::Watchlist;
    a.draw();
    let open = wanted(&a);
    assert!(open.len() > band.len() && open.len() < 120, "+ the rows in view: {}", open.len());
    assert!(!open.iter().any(|c| c.contains(':')), "Hyperliquid names, no « exchange: » prefix: {open:?}");
}

/// 📱 A phone of 402 × 874 (iPhone 17 Pro) with the single pill.
fn cell_pilule() -> App {
    let mut a = app();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    a.watch.phone = Some(402);
    a.area = (402, (874.0 - a.top_h() - a.bas_reserve()) as u32);
    // (no window in the tests: the chart takes the phone's size, as the layout does)
    (a.chart.viewport.width, a.chart.viewport.height) = a.area;
    a
}

/// 📱 Liste 3 (points 5, 17, 19): on the iPhone with the single pill, an open panel (Journal, Marché crypto, API…) is a
/// bottom sheet that goes down to the bottom of the screen: the pill hides under it, so no room is kept for it (the
/// black band of ~70 px); closed, the pill's room comes back.
#[test]
fn volet_jusqu_en_bas_sans_bande_noire() {
    let mut a = cell_pilule();
    let avec_pilule = a.bas_reserve();
    assert!(avec_pilule > 60.0, "the pill's room: {avec_pilule}");
    for outil in [wl::Tool::Journal, wl::Tool::Market, wl::Tool::Exchanges] {
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, outil);
        assert!(a.cell_etat().feuille, "{outil:?}: a sheet, the pill hides");
        assert_eq!(a.bas_reserve(), 0.0, "{outil:?}: no black band under the sheet");
    }
    a.watch.prefs.open = false;
    assert_eq!(a.bas_reserve(), avec_pilule, "closed: the pill's room comes back");
}

/// 📱 What a touch changes, a little more than `empreinte` (splits, shortcuts, objects tree, replay, Zen, the pill).
fn empreinte_pilule(a: &App) -> String {
    let u = &a.ui;
    format!(
        "{} {:?}",
        empreinte(a),
        (
            u.split_menu.is_some(),
            u.raccourcis.is_some(),
            u.obj_tree.is_some(),
            a.replay.is_some(),
            u.zen,
            u.pfeuille.as_ref().map(|p| format!("{:?}", p.kind)),
            u.pilule_repliee,
            a.tool
        )
    )
}

/// The zone of `k` on the phone's screen (window).
fn zone_pilule(a: &App, k: argos_render::ecrans::CellHit) -> Rect {
    argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat())
        .into_iter()
        .find(|(_, h)| *h == k)
        .map(|(r, _)| r)
        .unwrap_or_else(|| panic!("{k:?} on the screen"))
}

/// Touches what `t` is in the open sheet of the pill (scrolled into view first).
fn toucher_feuille(a: &mut App, t: argos_render::ecrans::feuille::Toucher) {
    let text = argos_render::text::Text::default();
    for _ in 0..40 {
        let f = a.pfeuille_vue().expect("a sheet open");
        let (w, h) = a.pfeuille_fenetre();
        if let Some((r, _)) = f.zones(&text, w, h).into_iter().find(|(r, z)| *z == t && r.h >= 30.0) {
            return toucher(a, r.cx(), r.cy());
        }
        a.pfeuille_defiler(60.0);
    }
    panic!("{t:?} in the sheet");
}

/// 📱 Jee's single pill (17 Nov): every action of the old phone screen (top pill, DÉMO and flux pills, symbol ·
/// interval pill, ƒx, Argos AI, icon bar, tablet icons) still has a way, by real touches, and it does exactly what
/// it did.
#[test]
fn pilule_chaque_action_a_un_acces() {
    use argos_render::ecrans::feuille::Toucher as T;
    use argos_render::ecrans::CellHit as C;
    use argos_render::phonebar::Icone as I;
    dossier_a_soi("pilule_acces");
    enum Pas {
        Pilule(C),
        Tuile(&'static str),
        Carte,
        Partager,
    }
    use Pas::*;
    let chemins: Vec<(C, Vec<Pas>)> = vec![
        (C::Accueil, vec![Pilule(C::Plus), Tuile("accueil")]),
        (C::Argos, vec![Pilule(C::Plus), Carte]),
        (C::MiseEnPage, vec![Pilule(C::Plus), Tuile("layouts")]),
        (C::Alertes, vec![Pilule(C::Plus), Tuile("notifs")]),
        // (▦ Splits: none on the phone, Jee 21 Nov, list 2)
        (C::Partager, vec![Pilule(C::Plus), Partager]),
        (C::Recherche, vec![Pilule(C::Plus), Tuile("recherche")]),
        (C::Plan, vec![Pilule(C::Plus), Tuile("plan")]),
        (C::Demo, vec![Pilule(C::Demo)]),
        (C::Flux, vec![Pilule(C::Flux)]),
        (C::Symbole, vec![Pilule(C::Symbole)]),
        (C::Unite, vec![Pilule(C::Unite)]),
        (C::Fx, vec![Pilule(C::Fx)]),
        (C::Ai, vec![Pilule(C::Ai)]),
        (C::Icone(I::Trading), vec![Pilule(C::Trading), Tuile("trading")]),
        (C::Icone(I::Demo), vec![Pilule(C::Trading), Tuile("simulateur")]),
        (C::Icone(I::PropFirm), vec![Pilule(C::Plus), Tuile("propfirm")]),
        (C::Icone(I::Baleines), vec![Pilule(C::Plus), Tuile("baleines"), Tuile("baleines1")]),
        (C::Icone(I::Modules), vec![Pilule(C::Plus), Tuile("modules"), Tuile("modules1")]),
        (C::Icone(I::Script), vec![Pilule(C::Plus), Tuile("script")]),
        (C::Icone(I::Academie), vec![Pilule(C::Plus), Tuile("communaute"), Tuile("academie")]),
        (C::Icone(I::Reglages), vec![Pilule(C::Plus), Tuile("reglages")]),
        (C::Icone(I::Communaute), vec![Pilule(C::Plus), Tuile("communaute"), Tuile("communaute1")]),
        (C::Icone(I::Macro), vec![Pilule(C::Plus), Tuile("marches"), Tuile("macro")]),
        (C::Icone(I::Marketplace), vec![Pilule(C::Plus), Tuile("modules"), Tuile("marketplace")]),
    ];
    for (avant, chemin) in chemins {
        let mut a_old = cell_pilule();
        a_old.cell_click(avant, Rect::new(10.0, 10.0, 40.0, 40.0));
        let mut a = cell_pilule();
        for p in chemin {
            match p {
                Pilule(k) => {
                    let r = zone_pilule(&a, k);
                    toucher(&mut a, r.cx(), r.cy());
                }
                Tuile(id) => toucher_feuille(&mut a, T::Tuile(id.into())),
                Carte => toucher_feuille(&mut a, T::Carte),
                Partager => toucher_feuille(&mut a, T::Partager),
            }
        }
        assert_eq!(empreinte_pilule(&a), empreinte_pilule(&a_old), "{avant:?}: the same as before");
    }
}

/// 📱 Every tile of the pill's sheets (« Argos », « Trading · DÉMO », each « X | Y ») does something when touched,
/// and the sheet closes (a tile « X et Y » opens its small sheet).
#[test]
fn pilule_chaque_tuile_agit() {
    use crate::actions::pilule::PKind;
    use argos_render::ecrans::feuille::Toucher as T;
    dossier_a_soi("pilule_tuiles");
    let mut kinds = vec![PKind::Argos, PKind::Trading];
    kinds.extend(crate::actions::pilule::DUOS.iter().map(|d| PKind::Duo(d.0)));
    kinds.push(PKind::Duo("portefeuille"));
    let mut n = 0;
    for k in kinds {
        let mut a0 = cell_pilule();
        a0.pfeuille_ouvrir(k.clone());
        let ids: Vec<String> = a0
            .pfeuille_vue()
            .unwrap()
            .sections
            .iter()
            .flat_map(|s| s.tuiles.iter().map(|t| t.id.clone()))
            .collect();
        assert!(!ids.is_empty(), "{k:?}");
        for id in ids {
            let mut a = cell_pilule();
            let rien = empreinte_pilule(&a);
            a.pfeuille_ouvrir(k.clone());
            toucher_feuille(&mut a, T::Tuile(id.clone()));
            // (a tile « X et Y » opens its small sheet: that is something too)
            assert_ne!(empreinte_pilule(&a), rien, "{k:?} {id}: does something");
            n += 1;
        }
    }
    assert!(n >= 30, "{n} tiles");
}

/// 📰 The Actualités' field « Chercher dans les nouvelles » takes a paste from the page (Ctrl + V in the browser, like
/// the Explorer's address); closed, the paste does not reach it.
#[test]
fn actualites_recherche_collee() {
    let mut a = pc_avec_volet(wl::Tool::News);
    a.on_user_event(UserEvent::Paste("sec".into()));
    assert_eq!(a.watch.eco.recherche, None, "field closed: nothing typed");
    a.watch.eco.recherche = Some("bit".into());
    a.on_user_event(UserEvent::Paste("coin ETF".into()));
    assert_eq!(a.watch.eco.recherche.as_deref(), Some("bitcoin ETF"));
}

/// 📱 The Explorer and the Stats Hyperliquid in the ••• sheet, beside « Marchés et Macro » (captures
/// marches-suivi-1: absent from the phone): « Explorer et Stats Hyperliquid » → each opens its panel.
#[test]
fn pilule_explorer_et_stats() {
    use crate::actions::pilule::PKind;
    use argos_render::ecrans::feuille::Toucher as T;
    dossier_a_soi("pilule_hl");
    let a = {
        let mut a = cell_pilule();
        a.pfeuille_ouvrir(PKind::Argos);
        a
    };
    let ids: Vec<String> = a
        .pfeuille_vue()
        .unwrap()
        .sections
        .iter()
        .flat_map(|s| s.tuiles.iter().map(|t| t.id.clone()))
        .collect();
    let m = ids.iter().position(|i| i == "marches").unwrap();
    assert_eq!(ids[m + 1], "hyperliquid", "{ids:?}");
    for (id, outil) in [("explorer", wl::Tool::Explorer), ("stats", wl::Tool::Stats)] {
        let mut a = cell_pilule();
        a.pfeuille_ouvrir(PKind::Argos);
        toucher_feuille(&mut a, T::Tuile("hyperliquid".into()));
        toucher_feuille(&mut a, T::Tuile(id.into()));
        assert!(a.watch.prefs.open && a.watch.prefs.panel == outil, "{id}");
    }
}

/// 📱 The pill folds by its bar and comes back by the ⌃ tab; the chart takes the room; kept in interface-2.json; a
/// sheet drags down to close; outside closes it.
#[test]
fn pilule_repli_et_feuilles() {
    use argos_render::ecrans::CellHit as C;
    dossier_a_soi("pilule_repli");
    let mut a = cell_pilule();
    let haut_chart = |a: &App| 874.0 - a.top_h() - a.bas_reserve();
    let avant = haut_chart(&a);
    let r = zone_pilule(&a, C::Replier);
    toucher(&mut a, r.cx(), r.cy());
    assert!(a.ui.pilule_repliee && App::interface2().pilule_repliee, "folded, kept");
    assert!(haut_chart(&a) > avant + 60.0, "the chart takes the room");
    assert!(a.ai_bouton_rect().w == 0.0, "folded: nothing but the ⌃ tab");
    let r = zone_pilule(&a, C::Rouvrir);
    toucher(&mut a, r.cx(), r.cy());
    assert!(!a.ui.pilule_repliee && !App::interface2().pilule_repliee);
    // ⋯ opens « Argos »; outside closes it; pulled down far, it closes too
    let r = zone_pilule(&a, C::Plus);
    toucher(&mut a, r.cx(), r.cy());
    assert!(a.pfeuille_vue().is_some_and(|f| f.titre == "Argos"));
    toucher(&mut a, 200.0, 20.0);
    assert!(a.ui.pfeuille.is_none(), "outside: closed");
    toucher(&mut a, r.cx(), r.cy());
    a.pfeuille_defiler(-40.0);
    a.pfeuille_lacher();
    assert!(a.ui.pfeuille.as_ref().is_some_and(|p| p.tire == 0.0), "a little: comes back");
    a.pfeuille_defiler(-600.0);
    assert!(a.ui.pfeuille.is_none(), "far: closed");
}

/// ✎ The « Dessins » sheet: the 109 tools by their categories (and « Outils »), favourites first, the search finds
/// by name; ☆ adds a favourite; touching a tool closes the sheet and turns it on with a banner to touch the chart.
#[test]
fn pilule_feuille_dessins() {
    use argos_render::ecrans::feuille::Toucher as T;
    use argos_render::ecrans::CellHit as C;
    dossier_a_soi("pilule_dessins");
    let mut a = cell_pilule();
    let r = zone_pilule(&a, C::Dessins);
    toucher(&mut a, r.cx(), r.cy());
    let f = a.pfeuille_vue().expect("Dessins");
    assert_eq!(f.titre, "Dessins");
    assert_eq!(f.onglets.len(), 1 + crate::actions::pilule::categories().len());
    let tous: std::collections::HashSet<&str> = crate::actions::pilule::categories().iter().flat_map(|c| c.2.clone()).collect();
    let manquants: Vec<&str> = argos_render::tools::TOOLS.iter().map(|d| d.id).filter(|id| !tous.contains(id)).collect();
    assert!(manquants.len() <= 2, "every tool in a tab (found by the search at worst): {manquants:?}");
    // a category tab, ☆ on its first tool, then the favourites tab has it
    toucher_feuille(&mut a, T::Onglet(2));
    let premier = a.pfeuille_vue().unwrap().sections[0].tuiles[0].id.clone();
    toucher_feuille(&mut a, T::Etoile(premier.clone()));
    assert!(a.rail.favs.contains(&premier));
    toucher_feuille(&mut a, T::Onglet(0));
    assert!(a.pfeuille_vue().unwrap().sections[0].tuiles.iter().any(|t| t.id == premier));
    // the search
    toucher_feuille(&mut a, T::Champ);
    a.on_user_event(UserEvent::Paste("fib".into()));
    let f = a.pfeuille_vue().unwrap();
    assert!(f.onglets.is_empty() && !f.sections[0].tuiles.is_empty());
    assert!(f.sections[0]
        .tuiles
        .iter()
        .all(|t| t.id.contains("fib") || t.nom.to_lowercase().contains("fib")));
    let outil = f.sections[0].tuiles[0].id.clone();
    toucher_feuille(&mut a, T::Tuile(outil.clone()));
    assert!(a.ui.pfeuille.is_none(), "closed");
    assert_eq!(a.tool, outil.as_str(), "the tool is on");
    assert!(a.banner.as_deref().is_some_and(|b| b.ends_with("touche le graphique")));
}

/// 👆 A long press on the phone's chart (the finger held more than LONG_MS): where the finger is.
fn appui_long_cell(a: &mut App, x: f64, y: f64) {
    use winit::event::TouchPhase;
    let (wx, wy) = (x + a.rw(), y + a.top_h());
    a.touch(5, TouchPhase::Started, (wx, wy));
    std::thread::sleep(std::time::Duration::from_millis(crate::input::touch::LONG_MS as u64 + 100));
    a.touch_tick();
}

/// What a line of the chart's menu changed (drawings, alerts, settings, view, orders, menus and pages).
fn effet_menu(a: &App) -> String {
    format!(
        "{} {} {} {:?} {:?} {:?} {}",
        empreinte_pilule(a),
        a.chart.drawings.len(),
        a.alerts.list.len(),
        a.chart.settings,
        a.vue_ordres(),
        (a.chart.viewport.bar_spacing, a.chart.viewport.right_offset),
        a.ui.obj_tree.is_some()
    )
}

/// 👆 Jee (17 Nov, option A): on the phone and the tablet, a long press on the chart lays the crosshair (no menu); the
/// finger still down moves it; it stays after the finger lifts; « ⋯ » (40 points) beside its price opens « À … » with
/// every line of today's chart menu at that price, in its order (the « bientôt » ones grey); each line does what the
/// menu does; the next touch on the chart only takes the crosshair away; the scales keep their menus.
#[test]
fn croix_au_doigt() {
    use argos_render::context_menu::Entry;
    use argos_render::ecrans::feuille::Toucher as T;
    use winit::event::TouchPhase;
    dossier_a_soi("croix_au_doigt");
    let mut a = cell_pilule();
    a.chart.drawings.push(argos_render::drawings::Drawing::hline(1.0)); // (« Effacer tous les dessins » shows)
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let (x, y) = (pw * 0.45, ph * 0.4);
    appui_long_cell(&mut a, x, y);
    a.touch(5, TouchPhase::Moved, (x + a.rw(), y + 30.0 + a.top_h()));
    a.touch(5, TouchPhase::Ended, (x + a.rw(), y + 30.0 + a.top_h()));
    let (cx, cy, prix) = a.ui.croix.expect("the crosshair");
    assert!((cy - (y + 30.0)).abs() < 0.5 && (cx - x).abs() < 0.5, "it followed the finger");
    assert!(!a.ui.context.is_open() && a.ui.list.is_none(), "no menu");
    assert_eq!(a.to_chart().crosshair, Some((cx, cy)), "drawn, the finger lifted");
    let b = a.croix_bouton().expect("« ⋯ »");
    assert!(
        b.w >= 40.0 && b.h >= 40.0 && b.x + b.w <= a.rw() + pw && b.x > a.rw() + pw - 50.0,
        "beside the price label: {b:?}"
    );
    let z = a.croix_zone().expect("its touch zone");
    let t = argos_render::ecrans::feuille::TOUCHER;
    assert!(z.w >= t && z.h >= t && z.contains(b.x + 1.0, b.y + 1.0), "40 screen points at 0.8: {z:?}");
    // « ⋯ » → « À … »: every line of the menu
    toucher(&mut a, b.cx(), b.cy());
    let f = a.pfeuille_vue().expect("the sheet");
    assert!(f.titre.starts_with("À "), "{}", f.titre);
    let tuiles: Vec<_> = f.sections.iter().flat_map(|s| s.tuiles.clone()).collect();
    let mut lignes = vec![];
    for e in argos_render::context_menu::entries(argos_render::context_menu::Source::Chart, &a.ctx(), &{
        let mut om = argos_render::context_menu::OpenMenu {
            source: argos_render::context_menu::Source::Chart,
            anchor: argos_render::context_menu::Anchor::Cursor(0.0, 0.0),
            price: prix,
            decimals: argos_render::viewport::data_decimals(&a.chart.bars),
            hover: None,
            sub: None,
            sub_scroll: 0,
        };
        om.hover = None;
        om
    }) {
        match e {
            Entry::Item { label, action, .. } => lignes.push((label, action)),
            Entry::Sub { entries, .. } => entries.into_iter().for_each(|s| {
                if let Entry::Item { label, action, .. } = s {
                    lignes.push((label, action))
                }
            }),
            Entry::Sep => {}
        }
    }
    assert_eq!(
        tuiles.iter().map(|t| t.nom.clone()).collect::<Vec<_>>(),
        lignes.iter().map(|l| l.0.clone()).collect::<Vec<_>>()
    );
    assert!(tuiles.iter().zip(&lignes).all(|(t, l)| t.grise == l.1.is_none()), "the « bientôt » lines grey");
    // each line that works does what the menu does (and the sheet and crosshair go)
    for (t, (_, action)) in tuiles.iter().zip(&lignes) {
        let Some(action) = *action else { continue };
        let mut b2 = cell_pilule();
        b2.chart.drawings.push(argos_render::drawings::Drawing::hline(1.0));
        b2.run_action(action);
        let mut a2 = cell_pilule();
        a2.chart.drawings.push(argos_render::drawings::Drawing::hline(1.0));
        a2.ui.croix = Some((cx, cy, prix));
        a2.croix_menu();
        toucher_feuille(&mut a2, T::Tuile(t.id.clone()));
        assert!(a2.ui.pfeuille.is_none() && a2.ui.croix.is_none(), "{}: the sheet and the crosshair go", t.nom);
        assert_eq!(effet_menu(&a2), effet_menu(&b2), "{}: as the menu", t.nom);
    }
    // the next touch on the chart: the crosshair goes, nothing else
    let mut c = cell_pilule();
    c.ui.croix = Some((cx, cy, prix));
    let avant = effet_menu(&c);
    let (tx, ty) = (x + c.rw(), y + c.top_h() + 60.0);
    toucher(&mut c, tx, ty);
    assert!(c.ui.croix.is_none());
    assert_eq!(c.to_chart().crosshair, None, "no crosshair left where the finger was");
    assert_eq!(effet_menu(&c), avant, "the touch only took the crosshair away");
    // a long press on the price scale: its menu, as before
    let mut d = cell_pilule();
    appui_long_cell(&mut d, pw + 10.0, ph * 0.5);
    assert!(d.ui.context.is_open() && d.ui.croix.is_none(), "the price scale's menu");
}

/// 🔎 Compact interface in the desktop app: drawn at 0.8 pixels per point (layout in points = window / 0.8), the
/// screen image has the window's size, nothing overlaps (the same checks as `rien_ne_se_chevauche_sur_pc`) and the
/// text stays sharp (some pixels at full contrast in the legend, not only blurred grey).
#[test]
fn interface_compacte_dessinee_a_0_8() {
    use argos_render::indicators::Ind;
    dossier_a_soi("compacte-0-8");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = vec![Ind::Cipher, Ind::Rsi];
    a.ui.frames = 5;
    argos_render::px::set_scale(0.8);
    // 1440 × 900 window pixels = 1800 × 1125 points
    a.ui.taille_test = Some(winit::dpi::PhysicalSize::new(1800, 1125));
    a.draw_frame();
    a.draw_frame();
    let img = std::mem::take(&mut a.ui.image_test);
    let zones = a.zones_pc();
    argos_render::px::set_scale(1.0);
    assert_eq!(img.len(), 1440 * 900, "the image has the window's size");
    let d = crate::actions::zones::chevauchements(&zones, 1800.0, 1125.0);
    assert!(d.is_empty(), "{d:?}");
    // sharp text: in the price legend, some pixels are bright (the text core), not only mid greys
    let l = zones.iter().find(|(n, _, _)| n == "légende du prix").expect("price legend").2;
    let (x0, y0) = ((l.x * 0.8) as usize, (l.y * 0.8) as usize);
    let clair = (y0..y0 + 30)
        .flat_map(|y| (x0..x0 + 200).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let v = img[y * 1440 + x];
            ((v >> 16) & 255) > 200 && ((v >> 8) & 255) > 200 && (v & 255) > 200
        })
        .count();
    assert!(clair > 20, "sharp legend text ({clair} bright pixels)");
}

/// 📱 Jee (17 Nov, « ok légende »): on the phone, the price legend (title, O H L C · Vol) starts just under the DÉMO
/// pill instead of running under it; the PC does not change; top row hidden: back to the top.
#[test]
fn cell_legende_sous_la_pilule_demo() {
    let mut a = cell_390();
    let _ = a.to_chart();
    let demo = argos_render::ecrans::cell().place("demoband", a.win_w(), a.cell_h()).expect("the DÉMO pill");
    let d = argos_render::haut_legende();
    // the title is drawn 24 under the chart's top, its letters ≈ 13 high
    let haut_titre = a.top_h() + 24.0 + d - 13.0;
    assert!(
        haut_titre >= demo.y + demo.h,
        "the title starts under the pill: {haut_titre} vs {}",
        demo.y + demo.h
    );
    assert!(haut_titre <= demo.y + demo.h + 8.0, "just under it");
    a.top.prefs.cell_haut_cache = true;
    let _ = a.to_chart();
    assert_eq!(argos_render::haut_legende(), 0.0, "top row hidden: at the top");
    let p = app();
    let _ = p.to_chart();
    assert_eq!(argos_render::haut_legende(), 0.0, "the PC: unchanged");
}

/// ⛶ PC (Jee, 17 Nov, « ok plein écran »): the chart menu's last line « ⛶ Plein écran : <name> » (« 3 clics » on the
/// right) puts the chart full screen — the bars go as in Zen, the panels go — and « Quitter le plein écran » or Échap
/// bring everything back; three clicks on the chart do it too and the third undoes the double-click's switch; three
/// clicks on a panel: that panel alone, the whole height; nothing with a split.
#[test]
fn plein_ecran_pc() {
    use crate::actions::plein_ecran::PleinEcran;
    use argos_render::context_menu::{entries, Anchor, Entry, OpenMenu, Source};
    use argos_render::indicators::Ind;
    dossier_a_soi("plein-ecran-pc");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = vec![Ind::Cipher];
    a.ui.frames = 5;
    a.refresh_indicators();
    let ph = a.chart.viewport.plot_h();
    assert!(!a.ind.out.bands(ph).is_empty(), "the Cipher panel");
    let derniere = |a: &App| {
        let _ = a.to_chart();
        let om = OpenMenu {
            source: Source::Chart,
            anchor: Anchor::Cursor(0.0, 0.0),
            price: 1.0,
            decimals: 2,
            hover: None,
            sub: None,
            sub_scroll: 0,
        };
        entries(Source::Chart, &a.ctx(), &om).pop()
    };
    // the right-click on the chart: its last line
    a.mouse_moved(PhysicalPosition::new(a.rw() + 300.0, a.top_h() + 100.0));
    a.right_click();
    let Some(Entry::Item { label, action, .. }) = derniere(&a) else {
        panic!("a line")
    };
    assert_eq!(label, format!("Plein écran : {}", a.symbol_label()));
    assert_eq!(argos_render::actions::shortcut(action.unwrap()).as_deref(), Some("3 clics"));
    a.run_action(action.unwrap());
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Prix));
    assert!(a.ui.zen && a.top_h() == 0.0, "the bars go, as in Zen");
    a.refresh_indicators();
    assert!(
        a.ind.out.bands(ph).is_empty() && a.ind.on.contains(&Ind::Cipher),
        "its panels go, the indicator stays on"
    );
    let Some(Entry::Item { label, .. }) = derniere(&a) else { panic!("a line") };
    assert_eq!(label, "Quitter le plein écran");
    // Échap: back
    touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
    assert!(a.ui.plein_ecran.is_none() && !a.ui.zen && a.top_h() > 0.0, "everything as before");
    a.refresh_indicators();
    assert!(!a.ind.out.bands(ph).is_empty(), "the panel is back");
    // three clicks on the chart: full screen, the scale as before the double-click
    let auto = a.chart.settings.scale.auto();
    let (x, y) = (a.rw() + 300.0, a.top_h() + 260.0);
    a.pointer.last_chart_click = None;
    for _ in 0..3 {
        clic(&mut a, x, y);
    }
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Prix), "three clicks");
    assert_eq!(a.chart.settings.scale.auto(), auto, "the third click undid the double-click's switch");
    // a double-click on it comes back
    a.pointer.last_chart_click = None;
    clic(&mut a, x, y);
    clic(&mut a, x, y);
    assert!(a.ui.plein_ecran.is_none(), "double-click: back");
    // three clicks on the Cipher panel: it alone, the whole height, its height as before
    a.pointer.triple = None;
    a.refresh_indicators();
    let (_, haut) = a.ind.out.bands(ph)[0];
    let avant = a.chart.settings.bands.get(&Ind::Cipher).copied();
    let (x, y) = (a.rw() + 200.0, a.top_h() + haut + (ph - haut) * 0.7);
    a.pointer.last_chart_click = None;
    for _ in 0..3 {
        clic(&mut a, x, y);
    }
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Panneau(Ind::Cipher)));
    assert_eq!(a.chart.settings.bands.get(&Ind::Cipher).copied(), avant, "the double-click's enlarging undone");
    let c = a.to_chart();
    assert!(c.title.is_none() && !c.legende, "the panel alone, without the price legend");
    assert!(
        !c.settings.scale.last_price_label && c.events.is_empty() && c.alerts.is_empty(),
        "nothing of the prices"
    );
    drop(c);
    a.refresh_indicators();
    argos_render::indicators::fit_bands(&a.ind.out, ph); // (as each drawing does)
    assert_eq!(a.ind.out.bands(ph), vec![(Ind::Cipher, 0.0)], "Cipher alone from the top");
    assert_eq!(a.ind.out.osc_h(), ph, "the whole height");
    // the indicator taken off the chart: back from full screen, no ghost state
    a.toggle_indicator(Ind::Cipher);
    a.refresh_indicators();
    assert!(a.ui.plein_ecran.is_none() && !a.ui.zen && a.ui.bulle_plein.is_none(), "Cipher removed: back");
    a.toggle_indicator(Ind::Cipher);
    a.refresh_indicators();
    a.entrer_plein_ecran(PleinEcran::Panneau(Ind::Cipher));
    a.toggle_zen();
    assert!(a.ui.plein_ecran.is_none() && !a.ui.zen, "the Zen button: back, Zen as before");
    let _ = a.to_chart();
    assert_eq!(argos_render::indicators::plein_ecran(), None);
    // a split: no line
    a.panes.push(crate::Pane::new("ETH".into(), a.chart.interval));
    assert!(
        !a.plein_possible()
            && derniere(&a).is_some_and(|e| !matches!(
                e,
                Entry::Item {
                    action: Some(argos_render::actions::ChartAction::PleinEcran),
                    ..
                }
            )),
        "split: no line"
    );
}

/// ⛶ Phone and tablet: a double tap on the chart puts it full screen (the top row and the pill fold away, the
/// panels go), a double tap again comes back; on a panel: that panel; the bubble « Double tap pour revenir » the
/// first 3 times only (interface-2.json).
#[test]
fn plein_ecran_au_doigt() {
    use crate::actions::plein_ecran::PleinEcran;
    use argos_render::context_menu::{entries, Anchor, Entry, OpenMenu, Source};
    use argos_render::indicators::Ind;
    dossier_a_soi("plein-ecran-doigt");
    let mut a = cell_pilule();
    a.ind.on = vec![Ind::Cipher];
    a.ui.frames = 5;
    a.refresh_indicators();
    let ph = a.chart.viewport.plot_h();
    let (bx, by) = (100.0, 60.0);
    toucher_chart(&mut a, bx, by);
    toucher_chart(&mut a, bx, by);
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Prix), "double tap on the chart");
    assert!(a.haut_cache() && a.bas_cache() && a.pilule_pliee(), "the top row and the pill fold away");
    assert_eq!(a.ui.bulle_plein.map(|b| b.0), Some("Double tap pour revenir"));
    let (bx2, by2) = (120.0, 80.0);
    toucher_chart(&mut a, bx2, by2);
    toucher_chart(&mut a, bx2, by2);
    assert!(a.ui.plein_ecran.is_none() && !a.pilule_pliee(), "double tap again: back");
    // on the panel
    a.refresh_indicators();
    let (_, haut) = a.ind.out.bands(a.chart.viewport.plot_h())[0];
    toucher_chart(&mut a, 100.0, haut + 20.0);
    toucher_chart(&mut a, 100.0, haut + 20.0);
    assert_eq!(a.ui.plein_ecran, Some(PleinEcran::Panneau(Ind::Cipher)), "double tap on the panel");
    a.quitter_plein_ecran();
    // the bubble: 3 times only
    a.entrer_plein_ecran(PleinEcran::Prix);
    a.quitter_plein_ecran();
    a.entrer_plein_ecran(PleinEcran::Prix);
    assert_eq!(App::interface2().plein_ecran_bulles, 3, "counted in interface-2.json");
    a.quitter_plein_ecran();
    a.entrer_plein_ecran(PleinEcran::Prix);
    assert!(a.ui.bulle_plein.is_none(), "the 4th time: no bubble");
    a.quitter_plein_ecran();
    let _ = ph;
    // 📱 the phone: no « Plein écran » line in the long press menu (one chart on the screen; Jee, 21 Nov, list 2)
    let _ = a.to_chart();
    let om = OpenMenu {
        source: Source::Chart,
        anchor: Anchor::Cursor(0.0, 0.0),
        price: 1.0,
        decimals: 2,
        hover: None,
        sub: None,
        sub_scroll: 0,
    };
    let lignes = entries(Source::Chart, &a.ctx(), &om);
    assert!(
        !lignes.iter().any(|e| matches!(e, Entry::Item { label, .. } if label.contains("Plein écran"))),
        "{lignes:?}"
    );
}

/// 🖥 Jee (17 Nov): the navigation pill shows only when the mouse comes into its zone (like the floating footer),
/// and its buttons still act; in Zen the top bar is hidden, comes back at the top edge with the legend going down
/// under it, and everything is as before out of Zen.
#[test]
fn pilule_de_navigation_et_barre_du_haut_au_survol() {
    use argos_render::navbar;
    dossier_a_soi("nav-survol");
    let mut a = app();
    let _ = a.to_chart();
    let (rw, vp) = (a.rw(), a.chart.viewport);
    let ph = vp.plot_h();
    let p = navbar::rect(&vp, ph, true);
    // the mouse in the middle of the chart: no pill
    a.mouse_moved(PhysicalPosition::new(rw + vp.plot_w() / 2.0, a.top_h() + ph / 3.0));
    let c = a.to_chart();
    assert!(!navbar::visible(&vp, ph, c.crosshair, true), "away from it: hidden");
    assert!(a.pad_bouton(p.cx(), p.cy()).is_none() || c.crosshair.is_none_or(|m| !navbar::visible(&vp, ph, Some(m), true)));
    // into its zone (24 around it): shown
    a.mouse_moved(PhysicalPosition::new(rw + p.cx(), a.top_h() + p.y - 10.0));
    let c = a.to_chart();
    assert!(navbar::visible(&vp, ph, c.crosshair, true), "in its zone: shown");
    // each button acts: Zen
    let zen = navbar::centers(&vp, ph, true)[7];
    assert_eq!(a.pad_bouton(zen.0, zen.1), Some(navbar::NavBtn::Zen));
    a.pointer.last_chart_click = None;
    let th = a.top_h();
    clic(&mut a, rw + zen.0, th + zen.1);
    assert!(a.ui.zen, "the Zen button turned Zen on");

    // 🧘 Zen: the chart from the top of the window, the top bar hidden, the legend at its place
    assert_eq!(a.top_h(), 0.0);
    a.mouse_moved(PhysicalPosition::new(600.0, 200.0));
    assert!(a.zen_haut_rect().is_none() && a.decalage_legende() == 0.0, "the top bar hidden");
    // the mouse at the top edge: the bar comes back, the legend goes down under it
    a.mouse_moved(PhysicalPosition::new(600.0, 2.0));
    let r = a.zen_haut_rect().expect("the top bar shown");
    assert!(a.pointer.top_mouse.is_some(), "the bar has the mouse (its menus can be used)");
    assert_eq!(a.decalage_legende(), r.h, "the legend under it");
    let _ = a.to_chart();
    assert_eq!(argos_render::haut_legende(), r.h);
    // still there inside it, gone below it
    a.mouse_moved(PhysicalPosition::new(600.0, r.h - 3.0));
    assert!(a.zen_haut_rect().is_some(), "stays while the mouse is in it");
    a.mouse_moved(PhysicalPosition::new(600.0, r.h + 60.0));
    assert!(a.zen_haut_rect().is_none() && a.decalage_legende() == 0.0, "gone again");
    // the « Mode Zen » banner: under the legend's three lines, never over them
    let _ = a.to_chart();
    let b = argos_render::banner_rect(
        &a.chart.viewport,
        &argos_render::text::Text::default(),
        "Mode Zen : Échap ou le bouton Zen pour revenir",
    );
    assert!(b.y >= 24.0 + 2.0 * 16.0 + 8.0, "the banner under the legend: {b:?}");
    // out of Zen: as before
    a.toggle_zen();
    let _ = a.to_chart();
    assert_eq!(
        argos_render::banner_rect(&a.chart.viewport, &argos_render::text::Text::default(), "x").y,
        40.0,
        "out of Zen: at its place"
    );
    assert!(!a.ui.zen && !a.ui.zen_haut && a.top_h() > 0.0 && a.zen_haut_zone().is_none());
}

/// 🧘 Jee (17 Nov): in Zen on the PC, everything is laid out again from the top of the window by the one PC layout:
/// the title, O H L C, Vol · Buyers · Sellers and « ⌄ N » stay whole, nothing overlaps, and the top bar brought back
/// at the top edge never hides the legend (it goes down under it). At 1440 × 900 and 2368 × 1155, top rows open or
/// folded, rail, side panel, modules column, icon column, favourites bar, indicator list, top bar hidden or shown.
#[test]
fn rien_ne_se_chevauche_en_zen() {
    use argos_render::dock::DockPos;
    let mut defauts: std::collections::BTreeMap<String, (usize, String)> = Default::default();
    let mut n = 0;
    for (w, h) in [(1440u32, 900u32), (2368, 1155)] {
        for zen in [1, 2] {
            for haut in 0..2 {
                for rail in [false, true] {
                    for volet in 0..2 {
                        for dock in [None, Some(DockPos::Right), Some(DockPos::Bottom)] {
                            for col in [false, true] {
                                for (fav, liste) in [(false, false), (true, false), (false, true)] {
                                    let etat = format!(
                                        "{w}×{h}, Zen {}, haut {haut}, rail {rail}, volet {volet}, modules {dock:?}, icônes repliées {col}, sans favoris {fav}, liste {liste}",
                                        if zen == 2 { "barre du haut au survol" } else { "seul" }
                                    );
                                    for d in chevauchements_pc_zen(w, h, haut, rail, volet, dock, col, (0, 0), liste, zen, fav) {
                                        defauts.entry(d).or_insert((0, etat.clone())).0 += 1;
                                    }
                                    n += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let liste: Vec<String> = defauts.iter().map(|(d, (k, e))| format!("{d} — {k}/{n} états, ex. {e}")).collect();
    assert!(liste.is_empty(), "{} chevauchements en Zen :\n{}", liste.len(), liste.join("\n"));
}

/// 🔔 Jee (19 Nov, his TradingView capture): on the PC, the mouse on an alert line shows its label at once — no
/// click — named like TradingView (« BTCUSDC.P Croisement 63 584,0 »); it stays while the mouse goes along the line
/// onto the label (to reach 🗑), and goes when the mouse leaves both.
#[test]
fn etiquette_de_l_alerte_au_survol() {
    dossier_a_soi("alerte-survol");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    let _ = a.to_chart();
    let sc = a.scale_now().expect("a scale");
    let ph = a.chart.viewport.plot_h();
    let prix = sc.price_of(ph * 0.5);
    a.add_price_alert(prix);
    a.banner = None;
    let id = a.alerts.list.last().unwrap().id;
    let y = sc.y_of(prix);
    let bouger = |a: &mut App, x: f64, y: f64| a.mouse_moved(PhysicalPosition::new(x + a.rw(), y + a.top_h()));
    bouger(&mut a, 60.0, y - 80.0);
    assert!(a.etiquette_alerte().is_none(), "away from the line: no label");
    bouger(&mut a, 60.0, y + 3.0);
    let (i, nom, _, p) = a.etiquette_alerte().expect("on the line: its label, without a click");
    assert_eq!((i, a.ui.alerte_sel), (id, None), "shown by the mouse only");
    let px = argos_render::viewport::format_price(prix, argos_render::viewport::data_decimals(&a.chart.bars));
    let sym = a
        .top
        .symbol
        .rsplit(':')
        .next()
        .filter(|s| !s.is_empty())
        .map_or(a.symbol_label(), str::to_string);
    assert_eq!(nom, format!("{sym} Croisement {px}"), "named like TradingView");
    // onto 🗑 (the label is taller than the line's 6 points): still shown, and 🗑 answers
    bouger(&mut a, p.poubelle.cx(), p.poubelle.y + 1.0);
    assert!(a.etiquette_alerte().is_some(), "on the label: it stays");
    a.pointer.last_chart_click = None;
    let (wx, wy) = (p.poubelle.cx() + a.rw(), p.poubelle.y + 1.0 + a.top_h());
    clic(&mut a, wx, wy);
    assert!(a.ui.confirmer.is_some(), "🗑 under the mouse: « Supprimer cette alerte? »");
    touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
    // the mouse leaves: the label goes
    bouger(&mut a, 60.0, y - 80.0);
    assert!(a.etiquette_alerte().is_none(), "the mouse left: no label");
}

/// 🔔 Jee (17 Nov, « ok alerte »), mouse and finger: a click (or a touch) on an alert line shows its label (name
/// cut « … », 🗑, the round handle); the handle drags the alert to another price; 🗑 asks « Supprimer cette alerte? »
/// — « Annuler » keeps it, « Effacer » deletes it (alerts.json); Échap closes the box; the line itself still drags.
#[test]
fn etiquette_de_l_alerte() {
    for doigt in [false, true] {
        dossier_a_soi(if doigt { "alerte-etiquette-doigt" } else { "alerte-etiquette-souris" });
        let mut a = if doigt { cell_pilule() } else { app() };
        a.top.prefs.bienvenue_vue = true;
        let _ = a.to_chart();
        let sc = a.scale_now().expect("a scale");
        let ph = a.chart.viewport.plot_h();
        let prix = sc.price_of(ph * 0.5);
        a.add_price_alert(prix);
        a.banner = None;
        let id = a.alerts.list.last().unwrap().id;
        let y = sc.y_of(prix);
        let appui = |a: &mut App, x: f64, y: f64| {
            let (wx, wy) = (x + a.rw(), y + a.top_h());
            if doigt {
                toucher(a, wx, wy);
            } else {
                a.pointer.last_chart_click = None;
                clic(a, wx, wy);
            }
        };
        // a click / touch on the line: its label
        appui(&mut a, 60.0, y);
        assert_eq!(a.ui.alerte_sel, Some(id), "the label shows (doigt: {doigt})");
        let (_, nom, _, p) = a.etiquette_alerte().expect("its places");
        let sym = a
            .top
            .symbol
            .rsplit(':')
            .next()
            .filter(|s| !s.is_empty())
            .map_or(a.symbol_label(), str::to_string);
        assert!(nom.starts_with(&format!("{sym} Croisement ")), "its name, like TradingView: {nom}");
        assert!(p.poubelle.w >= 26.0, "🗑");
        // 🗑: the box; « Annuler » keeps the alert
        appui(&mut a, p.poubelle.cx(), p.poubelle.cy());
        let (c, _) = a.ui.confirmer.clone().expect("« Supprimer cette alerte? »");
        assert_eq!(c.titre, "Supprimer cette alerte?");
        assert!(c.texte.contains(&nom) && c.ok == "Effacer" && c.danger);
        let (w, h) = a.taille_points();
        let q = c.places(&argos_render::text::Text::default(), w, h);
        let (wx, wy) = (q.annuler.cx(), q.annuler.cy());
        if doigt {
            toucher(&mut a, wx, wy);
        } else {
            clic(&mut a, wx, wy);
        }
        assert!(a.ui.confirmer.is_none() && a.alerts.list.iter().any(|x| x.id == id), "Annuler: kept");
        // Échap closes the box too
        appui(&mut a, p.poubelle.cx(), p.poubelle.cy());
        assert!(a.ui.confirmer.is_some());
        touche(&mut a, Key::Named(NamedKey::Escape), ModifiersState::empty());
        assert!(a.ui.confirmer.is_none() && a.alerts.list.iter().any(|x| x.id == id), "Échap: kept");
        // the handle drags it to another price
        a.ui.alerte_sel = Some(id);
        let (_, _, _, p) = a.etiquette_alerte().expect("its places");
        let (gx, gy) = (p.poignee.cx() + a.rw(), p.poignee.cy() + a.top_h());
        a.mouse_moved(PhysicalPosition::new(gx, gy));
        a.left_button(ElementState::Pressed);
        a.mouse_moved(PhysicalPosition::new(gx, gy + 60.0));
        a.left_button(ElementState::Released);
        let nouveau = match a.alerts.list.iter().find(|x| x.id == id).unwrap().kind {
            argos_core::alerts::Kind::Price { price, .. } => price,
            _ => unreachable!(),
        };
        assert!(nouveau < prix, "dragged down: a lower price ({nouveau} < {prix})");
        // 🗑 then « Effacer »: gone, and saved
        a.ui.alerte_sel = Some(id);
        let (_, _, _, p) = a.etiquette_alerte().expect("its places");
        appui(&mut a, p.poubelle.cx(), p.poubelle.cy());
        let (c, _) = a.ui.confirmer.clone().unwrap();
        let q = c.places(&argos_render::text::Text::default(), w, h);
        if doigt {
            toucher(&mut a, q.ok.cx(), q.ok.cy());
        } else {
            clic(&mut a, q.ok.cx(), q.ok.cy());
        }
        assert!(a.ui.confirmer.is_none() && a.ui.alerte_sel.is_none(), "closed");
        assert!(!a.alerts.list.iter().any(|x| x.id == id), "Effacer: deleted");
        let gardees: argos_core::alerts::Alerts = App::read_prefs("alerts.json");
        assert!(!gardees.list.iter().any(|x| x.id == id), "and saved");
    }
}

/// 🤖 Argos AI level 1 (« ok Argos AI », Jee 18 Nov): without a Claude key, opening Argos AI (corner pill, phone
/// pill), the legends' ✨ and the Lens card's « Explique » open the « règles » mode: the explanation of the last closed
/// bar in the panel, its drawings on the chart (kept apart, never saved), gone with « Effacer l'explication » or when
/// the interval changes; with a key, opening Argos AI does not explain (Claude as before), « Explique » still does.
#[test]
fn argos_ai_regles() {
    use argos_render::assistant::KeyState;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos-ai-regles");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = vec![Ind::Lens];
    a.ui.frames = 5;
    a.refresh_indicators();
    // without a key: opening Argos AI explains with the rules
    assert_eq!(a.ai.key, KeyState::None);
    a.toggle_ai();
    assert!(a.ui.ai_open && a.ai.regles, "« règles » mode");
    let dernier = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
    assert!(dernier.starts_with("📏 Argos AI · règles"), "the rules' text: {dernier}");
    assert!(a.ui.explication.is_some(), "the explanation kept for this chart");
    assert!(a.ai.effacer, "« Effacer l'explication » shown");
    assert_eq!(a.to_chart().explication, a.explication_vue(), "drawn on the chart");
    // « Effacer l'explication »
    a.effacer_explication();
    assert!(a.ui.explication.is_none() && !a.ai.effacer && a.to_chart().explication.is_empty());
    // the legends' ✨ (no key): the rules again
    a.ai.msgs.clear();
    a.legend_click(argos_render::indleg::LegClick::Ai(Ind::Lens));
    assert!(a.ai.msgs.last().is_some_and(|m| m.1.starts_with("📏 Argos AI · règles")) && a.ui.explication.is_some());
    // another interval: the explanation goes
    a.set_interval(Interval(3600));
    a.refresh_indicators();
    assert!(a.ui.explication.is_none() && !a.ai.effacer, "another interval: gone");
    // nothing of it in the saved settings
    let dir = argos_core::dossiers::de_test("argos-ai-regles");
    for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let t = std::fs::read_to_string(f.path()).unwrap_or_default();
        assert!(!t.contains("explication") && !t.contains("Explanation"), "not saved: {:?}", f.path());
    }
    // with a key: opening Argos AI does not explain (Claude as before)
    a.toggle_ai();
    a.ai.key = KeyState::Ready;
    (a.ai.regles, a.ai.msgs) = (false, vec![]);
    a.toggle_ai();
    assert!(a.ui.ai_open && !a.ai.regles && a.ai.msgs.is_empty(), "with a key: as before");
}

/// 🤖 The Lens card's « Explique » pill, at the mouse (PC) and at the finger (40 points around it): always the rules.
#[test]
fn bouton_explique_de_la_carte_lens() {
    use argos_render::indicators::Ind;
    let mut appuis = 0;
    for doigt in [false, true] {
        dossier_a_soi(if doigt { "explique-doigt" } else { "explique-souris" });
        let mut a = if doigt { cell_pilule() } else { app() };
        a.top.prefs.bienvenue_vue = true;
        a.ind.on = vec![Ind::Lens];
        a.ui.frames = 5;
        a.ai.key = argos_render::assistant::KeyState::Ready; // (even with a key: « Explique » is the rules)
        a.refresh_indicators();
        let _ = argos_render::render(&a.to_chart());
        let Some(b) = (if doigt {
            argos_render::indicators::lens::bouton_explique_doigt()
        } else {
            argos_render::indicators::lens::bouton_explique()
        }) else {
            // (no photo on this chart: no pill, nothing to press)
            assert!(
                a.ind.out.lens.as_ref().is_none_or(|v| v.photo.is_none()),
                "the pill is there when the card has a photo"
            );
            continue;
        };
        if doigt {
            assert!(b.w >= 40.0 && b.h >= 40.0, "40 points at the finger");
        }
        let (wx, wy) = (b.cx() + a.rw(), b.cy() + a.top_h());
        if doigt {
            toucher(&mut a, wx, wy);
        } else {
            clic(&mut a, wx, wy);
        }
        assert!(a.ui.ai_open && a.ai.regles, "« Explique »: the rules (doigt: {doigt})");
        assert!(a.ai.msgs.last().is_some_and(|m| m.1.starts_with("📏 Argos AI · règles")));
        appuis += 1;
    }
    assert_eq!(appuis, 2, "the pill pressed at the mouse and at the finger");
}

/// 🤖 « ok lot 1 » (Jee 18 Nov), the legends' ✨: without a Claude key, Lens explains Lens; another indicator (RSI)
/// shows its name, its settings with their help and its rule-based reading — never a word of Lens; with a key,
/// Claude as before.
#[test]
fn etincelle_des_legendes() {
    use argos_render::assistant::KeyState;
    use argos_render::indicators::Ind;
    use argos_render::indleg::LegClick;
    dossier_a_soi("etincelle-legendes");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = vec![Ind::Rsi, Ind::Lens];
    a.ui.frames = 5;
    a.refresh_indicators();
    // ✨ Lens, no key: the Lens explanation, as before
    a.legend_click(LegClick::Ai(Ind::Lens));
    assert!(
        a.ai.regles && a.ai.msgs.last().is_some_and(|m| m.1.starts_with("📏 Argos AI · règles")),
        "✨ Lens: Lens"
    );
    // ✨ RSI, no key: its name, its settings with their help, its reading line; no word of Lens, nothing drawn for it
    a.effacer_explication();
    a.ai.msgs.clear();
    a.legend_click(LegClick::Ai(Ind::Rsi));
    let t = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
    assert!(t.starts_with(&format!("📏 {}", Ind::Rsi.name())), "its name first: {t}");
    assert!(!t.contains("Lens"), "never another card: {t}");
    let p = &Ind::Rsi.params().1[0];
    assert!(t.contains(p.label), "its settings: {t}");
    assert!(
        t.contains("Ce qu'il montre maintenant") && t.contains("Comment le lire") && t.contains("Ce qui changerait la lecture"),
        "RSI: its rules: {t}"
    );
    assert!(t.contains(argos_lens::explain::REMINDER), "the reminder: {t}");
    assert!(
        !t.contains("Couleur") && !t.contains("Épaisseur") && !t.contains("Type de ligne"),
        "no style settings: {t}"
    );
    // ✨ of an indicator without rules: its settings only
    a.ai.msgs.clear();
    let (t2, i) = lecture_sans_regles(&mut a);
    assert!(
        t2.starts_with(&format!("📏 {}", i.name())) && t2.contains("Lecture des réglages seulement"),
        "{i:?}: settings only: {t2}"
    );
    assert!(t2.contains(i.params().1[0].label) && !t2.contains("Lens"), "its settings, no Lens: {t2}");
    // ✨ ATR (lot 2): read by rules
    a.ind.on.push(Ind::Atr);
    a.refresh_indicators();
    a.ai.msgs.clear();
    a.legend_click(LegClick::Ai(Ind::Atr));
    let t3 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
    assert!(
        t3.contains("Ce qu'il montre maintenant") && !t3.contains("Lecture des réglages seulement"),
        "ATR: its rules: {t3}"
    );
    assert!(!t.contains("Lens") && a.ui.explication.is_none(), "never another card: {t}");
    assert!(a.ai.regles, "« règles » mode");
    assert_eq!(crate::actions::argos_ai_ecran::id_catalogue(Ind::Rsi), "Rsi");
    // ✨ RSI with a key: Claude as before (not the rules)
    a.ai.key = KeyState::Ready;
    (a.ai.regles, a.ai.msgs) = (false, vec![]);
    a.legend_click(LegClick::Ai(Ind::Rsi));
    assert!(
        !a.ai.regles && !a.ai.msgs.iter().any(|m| m.1.starts_with("📏")),
        "with a key: not the rules: {:?}",
        a.ai.msgs
    );
}

/// 🤖 Phone and tablet (Jee teste au doigt): the indicator's ⋯ menu has « ✨ Argos AI : lire <nom> » first, touched at
/// its place (a row of 40 points at least): the same as the legend's ✨ on the PC — without a key, the rules.
#[test]
fn etincelle_au_doigt_dans_le_menu() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::LegClick;
    dossier_a_soi("etincelle-doigt");
    let mut a = cell_pilule();
    a.ind.on = vec![Ind::Rsi];
    a.ui.frames = 5;
    a.refresh_indicators();
    a.ind.sel = Some(Ok(Ind::Rsi));
    a.legend_click(LegClick::Plus(Ind::Rsi));
    let menu = a.ind_plus_menu();
    assert_eq!(menu[0].items[0].id, 12, "first line of the menu");
    assert_eq!(menu[0].items[0].label, format!("Argos AI : lire {}", Ind::Rsi.name()));
    let r = a.ui.list.as_ref().expect("the ⋯ menu").1.rect;
    let mut ys = vec![];
    let mut y = r.y;
    while y < r.y + r.h {
        let mut m = a.ui.list.as_ref().unwrap().1.clone();
        if m.click(r.x + 60.0, y) == argos_render::listmenu::MenuClick::Pick(12) {
            ys.push(y);
        }
        y += 1.0;
    }
    let (haut, bas) = (ys.first().copied().expect("the line in the menu"), ys.last().copied().unwrap_or(0.0));
    assert!(bas - haut + 1.0 >= 40.0, "a row of 40 points at least: {}", bas - haut + 1.0);
    toucher(&mut a, r.x + 60.0, (haut + bas) / 2.0);
    assert!(a.ui.ai_open && a.ai.regles, "Argos AI « règles »");
    assert!(
        a.ai.msgs.last().is_some_and(|m| m.1.starts_with(&format!("📏 {}", Ind::Rsi.name()))),
        "the RSI reading"
    );
}

/// 🤖 Ideas 1, 2 and 4 of Argos AI (« ok fit en ordre 1 a 5 », Jee 18 Nov): in « règles » mode, « ✨ Lire tous les
/// indicateurs », « ✨ Accord des unités de temps » and « ✨ Volatilité du moment » at the top of the panel, 40 points high, pressed at the mouse
/// (PC) and at the finger (phone): the indicators shown (not hidden) read together, then the reminder; a 1-month
/// chart has no higher unit (the line is greyed, pressing it does nothing, the reading says so); no indicator shown:
/// a clear sentence. The settings read as the ⚙ shows them (« Show WaveTrend : oui », never « : 1 »).
#[test]
fn lire_tous_les_indicateurs_accord_et_volatilite() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::LegClick;
    let mut appuis = 0;
    for doigt in [false, true] {
        dossier_a_soi(if doigt { "ensemble-doigt" } else { "ensemble-souris" });
        let mut a = if doigt { cell_pilule() } else { app() };
        a.top.prefs.bienvenue_vue = true;
        a.ind.on = vec![Ind::Rsi, Ind::Macd, Ind::Cipher];
        a.ind.leg.1 = vec![Ind::Macd]; // (hidden by 👁)
        a.ui.frames = 5;
        a.refresh_indicators();
        // the panel in « règles » mode (no Claude key), opened by an indicator's reading
        a.legend_click(LegClick::Ai(Ind::Cipher));
        assert!(a.ui.ai_open && a.ai.regles);
        let t = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(
            t.contains("Show WaveTrend : oui") && !t.contains("Show WaveTrend : 1"),
            "a tick box: oui / non: {t}"
        );
        a.ai.unite_sup = argos_ai::higher_timeframe(a.chart.interval.secs()).map(argos_ai::timeframe_label);
        let r = a.ai_rect();
        let text = argos_render::text::Text::default();
        let lignes = a.ai.lignes_regles(&text, r.w, r.h);
        assert_eq!(lignes.len(), 4, "four lines in « règles » mode");
        assert!(lignes.iter().all(|l| l.h >= 40.0 && l.w >= 200.0), "40 points high: {lignes:?}");
        let appuyer = |a: &mut App, l: argos_render::shapes::Rect| {
            let (x, y) = (r.x + l.cx(), r.y + l.cy());
            if doigt {
                toucher(a, x, y);
            } else {
                clic(a, x, y);
            }
        };
        // 1. all the indicators shown: RSI and Cipher, not the hidden MACD
        a.ai.msgs.clear();
        appuyer(&mut a, lignes[0]);
        let t1 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t1.starts_with("✨ Tous les indicateurs de la chart"), "idea 1 (doigt: {doigt}): {t1}");
        assert!(t1.contains(Ind::Rsi.name()) && !t1.contains(Ind::Macd.name()), "the shown ones only: {t1}");
        assert!(t1.ends_with(argos_lens::explain::REMINDER), "the reminder last: {t1}");
        // 2. the timeframe agreement
        a.ai.msgs.clear();
        appuyer(&mut a, lignes[1]);
        let t2 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t2.starts_with("✨ Accord des unités de temps : "), "idea 2 (doigt: {doigt}): {t2}");
        assert!(t2.ends_with(argos_lens::explain::REMINDER), "the reminder last: {t2}");
        // 4. the volatility of the moment (idea 4)
        a.ai.msgs.clear();
        appuyer(&mut a, lignes[2]);
        let t5 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t5.starts_with("✨ Volatilité du moment"), "idea 4 (doigt: {doigt}): {t5}");
        assert!(t5.ends_with(argos_lens::explain::REMINDER), "the reminder last: {t5}");
        // 5. the day's levels (idea 5): yesterday's high and low and the day's VWAP, like the others
        a.ai.msgs.clear();
        appuyer(&mut a, lignes[3]);
        let t6 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t6.starts_with("✨ Niveaux du jour"), "idea 5 (doigt: {doigt}): {t6}");
        assert!(t6.contains("UTC"), "the day counted in UTC: {t6}");
        assert!(t6.ends_with(argos_lens::explain::REMINDER), "the reminder last: {t6}");
        // with a snapshot, its levels go on the chart (inside the window) with « Effacer l'explication »
        if argos_ai::read_day_levels(a.bougies_fermees()).snapshot.is_some() {
            assert!(a.ui.explication.is_some() && a.ai.effacer, "the day's levels drawn");
        }
        appuis += 1;
        // a 1-month chart: no higher unit, the line is greyed and does nothing; the reading says so
        a.ai.unite_sup = argos_ai::higher_timeframe(argos_core::types::MONTH_SECONDS).map(argos_ai::timeframe_label);
        assert!(a.ai.unite_sup.is_none(), "1M: no higher unit");
        a.ai.msgs.clear();
        appuyer(&mut a, lignes[1]);
        assert!(a.ai.msgs.is_empty(), "greyed: nothing");
        a.set_interval(argos_core::Interval(argos_core::types::MONTH_SECONDS));
        a.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::Accord);
        let t3 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t3.contains("Pas d'unité supérieure"), "1M: {t3}");
        // no indicator shown: a clear sentence
        a.ind.on.clear();
        a.refresh_indicators();
        a.lire_ensemble(crate::actions::argos_ai_ecran::Ensemble::Tout);
        let t4 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(t4.contains("Aucun indicateur affiché sur la chart"), "none shown: {t4}");
    }
    assert_eq!(appuis, 2, "pressed at the mouse and at the finger");
}

/// 🤖 Every indicator Argos AI reads by rules (`argos_ai::READ_BY_RULES`, Cipher included) is read by rules from the
/// screen with the catalogue's default settings, under the names the screen gives them: a key that moved would make
/// it fall back to « réglages seulement ».
#[test]
fn chaque_indicateur_lu_par_regles_l_est_depuis_l_ecran() {
    dossier_a_soi("lus-par-regles");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = argos_ai::READ_BY_RULES
        .iter()
        .map(|id| argos_render::catalogue::ind(id).unwrap_or_else(|| panic!("{id}: in the catalogue")))
        .collect();
    a.ui.frames = 5;
    a.refresh_indicators();
    assert!(
        a.bougies_fermees().len() > 50,
        "enough closed bars: {} of {}",
        a.bougies_fermees().len(),
        a.chart.bars.len()
    );
    for id in argos_ai::READ_BY_RULES {
        let i = argos_render::catalogue::ind(id).unwrap();
        assert_eq!(crate::actions::argos_ai_ecran::id_catalogue(i), id);
        let reglages = a.reglages_par_nom(i);
        let par_nom: Vec<(&str, f64)> = reglages.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        // (through `read_with`, with what the screen holds besides the bars, as the ✨ reads it)
        let r = a.lecture_ecran(i, id, &par_nom);
        assert_eq!(r.coverage, argos_ai::Coverage::Rules, "{id}: read by rules from the screen");
        let t = a.lecture_par_id(i, id, false);
        assert!(!t.contains("Lecture des réglages seulement"), "{id}: {t}");
    }
    // and every other indicator of the catalogue (but Lens, which has its card) says « réglages seulement » — no
    // number written here: nothing to change when Lens reads one more
    for i in argos_render::indicators::Ind::ALL
        .into_iter()
        .filter(|i| *i != argos_render::indicators::Ind::Lens)
    {
        let id = crate::actions::argos_ai_ecran::id_catalogue(i);
        if !argos_ai::READ_BY_RULES.contains(&id.as_str()) {
            let t = a.lecture_par_id(i, &id, false);
            assert!(t.contains("Lecture des réglages seulement"), "{id}: settings only: {t}");
        }
    }
}

/// 🤖 A screen with `on` computed on the test bars (closed bars enough for every reading).
fn ecran_avec(nom: &str, on: Vec<argos_render::indicators::Ind>) -> App {
    dossier_a_soi(nom);
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ind.on = on;
    a.ui.frames = 5;
    a.refresh_indicators();
    assert!(a.bougies_fermees().len() > 50);
    a
}

/// 🤖 The ✨ reading of `i` on the screen, all its useful settings.
fn lu_ecran(a: &mut App, i: argos_render::indicators::Ind) -> String {
    let id = crate::actions::argos_ai_ecran::id_catalogue(i);
    a.lecture_par_id(i, &id, true)
}

/// 🤖 The real orderflow of the last `k` closed bars, complete, one row each (as `App::orderflow_chart` gives it),
/// plus one for the bar under way (the screen keeps only the closed bars' for Argos AI).
fn reel_des_dernieres(a: &App, k: usize) -> Vec<argos_core::orderflow::FlowBar> {
    let f = a.bougies_fermees();
    let mut v: Vec<argos_core::orderflow::FlowBar> = f[f.len() - k..]
        .iter()
        .map(|b| argos_core::orderflow::FlowBar {
            time: b.time,
            pas: 1.0,
            rangees: vec![((b.close).round() as i64, b.volume * 0.7, b.volume * 0.3)],
            complete: true,
        })
        .collect();
    if let Some(b) = a.chart.bars.get(f.len()) {
        v.push(argos_core::orderflow::FlowBar {
            time: b.time,
            pas: 1.0,
            rangees: vec![],
            complete: false,
        });
    }
    v
}

/// 🤖 Argos CVD on the screen: the real orderflow the chart holds (closed bars only) reaches the reading, which says
/// how many bars have a true delta; without it, the delta is estimated from the bars, as the chart does.
#[test]
fn cvd_lu_avec_l_orderflow_de_l_ecran() {
    let i = argos_render::catalogue::ind("Cvd").unwrap();
    let mut a = ecran_avec("ai-cvd", vec![i]);
    assert!(a.ind.out.reel.is_empty(), "a file has no real orderflow");
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("pas d'orderflow réel pour ces bougies"), "{sans}");
    a.ind.out.reel = reel_des_dernieres(&a, 5);
    let c = a.calculs_ecran(i);
    let d = a.donnees_ecran(&c);
    assert_eq!(d.flow.map(<[_]>::len), Some(5), "the closed bars' orderflow only");
    assert_eq!(d.bar_secs, Some(1800));
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("Vrai delta (trades complets) sur 5 bougies"), "{avec}");
}

/// 🤖 Argos Footprint on the screen: the last closed bar from real trades when the screen has them, else estimated.
#[test]
fn footprint_lu_avec_l_orderflow_de_l_ecran() {
    let i = argos_render::catalogue::ind("Footprint").unwrap();
    let mut a = ecran_avec("ai-footprint", vec![i]);
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("estimée d'après la bougie (pas d'orderflow réel)"), "{sans}");
    a.ind.out.reel = reel_des_dernieres(&a, 3);
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("Dernière bougie fermée : vrais trades complets"), "{avec}");
}

/// 🤖 Argos Bubbles reads the real trades the chart draws them from: without them, it says they are missing.
#[test]
fn bulles_lues_d_apres_les_vrais_trades() {
    let i = argos_render::catalogue::ind("Bubbles").unwrap();
    let mut a = ecran_avec("ai-bulles", vec![i]);
    let sans = lu_ecran(&mut a, i);
    assert!(!sans.contains("Lecture des réglages seulement"), "{sans}");
    assert!(sans.contains("vrais trades absents pour ce marché ou cette unité"), "no real trades: {sans}");
    a.ind.out.reel = reel_des_dernieres(&a, 5);
    let avec = lu_ecran(&mut a, i);
    assert!(
        avec.contains("Vrais trades depuis") && !avec.contains("absents"),
        "5 bars of real trades: {avec}"
    );
}

/// 🤖 The Multi-UT table on the screen: the screen's time gives each row's « clôture dans » (none without it).
#[test]
fn tableau_multi_ut_lu_avec_l_heure_de_l_ecran() {
    let i = argos_render::catalogue::ind("MtfTable").unwrap();
    let mut a = ecran_avec("ai-mtf", vec![i]);
    let c = a.calculs_ecran(i);
    assert!(a.donnees_ecran(&c).now.is_some());
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("clôture dans"), "{avec}");
    let sans = argos_ai::read("MtfTable", a.bougies_fermees(), &[]);
    assert!(sans.blocks.iter().flat_map(|b| &b.sentences).all(|s| !s.text.contains("clôture dans")));
}

/// 🤖 Argos Tape on the screen: its live state (`IndOut::tape`) reaches the reading; without it, the data is missing.
#[test]
fn ruban_lu_depuis_l_etat_de_l_ecran() {
    let i = argos_render::catalogue::ind("Tape").unwrap();
    let mut a = ecran_avec("ai-ruban", vec![i]);
    assert!(a.ind.out.tape.is_some(), "the screen keeps the tape's state");
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("delta de volume de la bougie en cours"), "{avec}");
    a.ind.out.tape = None;
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("Donnée absente : le ruban (Argos Tape)"), "{sans}");
}

/// 🤖 The book heatmap on the screen: the walls computed as the chart draws them (middle price the last close) reach
/// the reading; without a book, the data is missing.
#[test]
fn murs_du_carnet_lus_comme_dessines() {
    let i = argos_render::catalogue::ind("BookHeat").unwrap();
    let mut a = ecran_avec("ai-carnet", vec![i]);
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("Donnée absente : le carnet d'ordres (ses murs)"), "{sans}");
    let mid = a.chart.bars.last().unwrap().close;
    let mut h = argos_render::bookheat::BookHeat::new("essai", argos_render::bookheat::tick_for(mid, false), 10);
    let t = a.chart.bars.last().unwrap().time;
    // (an ordinary book around the price, one big level just above it)
    let niveaux: Vec<(f64, f64)> = (1..=40)
        .flat_map(|k| [(mid * (1.0 + k as f64 * 0.0005), 1.0), (mid * (1.0 - k as f64 * 0.0005), 1.0)])
        .chain([(mid * 1.003, 200.0)])
        .collect();
    h.add(t, niveaux.into_iter(), mid);
    a.feeds.heat = Some(h);
    let c = a.calculs_ecran(i);
    let murs = c_murs(&a, &c);
    assert!(!murs.is_empty() && murs.iter().any(|w| w.sell), "{murs:?}");
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("plus gros mur au-dessus, étiquette « SELL »"), "{avec}");
}

/// 🤖 The walls `donnees_ecran` passes for these computations.
fn c_murs(a: &App, c: &crate::actions::argos_ai_ecran::Calculs) -> Vec<argos_ai::Wall> {
    a.donnees_ecran(c).walls.map(<[_]>::to_vec).unwrap_or_default()
}

/// 🤖 The liquidation map on the screen: its profile and messages reach the reading; without a map, data missing.
#[test]
fn carte_des_liquidations_lue_depuis_l_ecran() {
    let i = argos_render::catalogue::ind("LiqMap").unwrap();
    let mut a = ecran_avec("ai-liq", vec![i]);
    a.feeds.liq_view = None;
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("Donnée absente : la carte des liquidations"), "{sans}");
    let c = a.bougies_fermees().last().unwrap().close;
    a.feeds.liq_view = Some(argos_render::liqview::LiqView {
        bands: vec![],
        profile: vec![(c * 0.98, 3_000_000.0, 0.0), (c * 1.02, 0.0, 5_000_000.0)],
        bucket: c * 0.001,
        notes: vec!["historique en construction".into()],
    });
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("plus gros amas de shorts au-dessus"), "{avec}");
    assert!(avec.contains("plus gros amas de longs en dessous"), "{avec}");
    assert!(avec.contains("Message de la carte : historique en construction"), "{avec}");
}

/// 🤖 AI Trend's multi-timeframe dots on the screen: the smaller unit's bars the screen holds (`feeds.mtf_bars`)
/// reach the reading; a slot without its bars is said absent.
#[test]
fn points_multi_ut_d_ai_trend_lus_depuis_l_ecran() {
    let i = argos_render::catalogue::ind("AiTrend").unwrap();
    dossier_a_soi("ai-trend-mtf");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    // (slot 1 on, at 15 min under the chart's 30 min; the others off)
    let params = i.params().1;
    let mut v = argos_render::indicators::values(&a.ind.set, i);
    let k = |key: &str| params.iter().position(|p| p.key == key).unwrap_or_else(|| panic!("{key}"));
    for n in 1..=4 {
        v[k(&format!("m{n}_on"))] = if n == 1 { 1.0 } else { 0.0 };
    }
    v[k("m1_tf")] = argos_script::inputs::TF_SECS.iter().position(|s| *s == 900).unwrap() as f64;
    a.ind.set.insert(i, v);
    a.ind.on = vec![i];
    a.ui.frames = 5;
    a.refresh_indicators();
    a.feeds.mtf_bars.clear();
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("bougies de cette unité absentes"), "{sans}");
    a.feeds.mtf_bars.insert(900, a.bougies_fermees().to_vec());
    let c = a.calculs_ecran(i);
    assert!(a.donnees_ecran(&c).lower.is_some_and(|l| !l.is_empty()));
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("Points multi-UT") && !avec.contains("bougies de cette unité absentes"), "{avec}");
}

/// 🤖 Argos Anchored VWAP with another indicator's output as its source: the screen passes the series the chart uses
/// (cut to the closed bars); when that output is not computed, the reading says it does not have it.
#[test]
fn vwap_ancre_lu_avec_la_source_d_un_autre_indicateur() {
    use argos_render::indicators::Ind;
    let i = argos_render::catalogue::ind("AnchoredVwap").unwrap();
    let rsi = Ind::Rsi;
    dossier_a_soi("ai-avwap-ext");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    let params = i.params().1;
    let mut v = argos_render::indicators::values(&a.ind.set, i);
    let src = params.iter().position(|p| p.key == "src").unwrap();
    v[src] = argos_script::inputs::source_ext(Ind::ALL.iter().position(|x| *x == rsi).unwrap(), 0);
    a.ind.set.insert(i, v);
    a.ind.on = vec![rsi, i];
    a.ui.frames = 5;
    a.refresh_indicators();
    let c = a.calculs_ecran(i);
    let n = a.bougies_fermees().len();
    assert_eq!(a.donnees_ecran(&c).ext_source.map(<[f64]>::len), Some(n), "one value per closed bar");
    let avec = lu_ecran(&mut a, i);
    assert!(avec.contains("Valeur moyennée : la sortie d'un autre indicateur"), "{avec}");
    // (the RSI no longer computed: no series to pass)
    a.ind.on = vec![i];
    a.refresh_indicators();
    let c = a.calculs_ecran(i);
    assert!(a.donnees_ecran(&c).ext_source.is_none());
    let sans = lu_ecran(&mut a, i);
    assert!(sans.contains("Argos AI ne la lit pas encore (elle vient de l'écran)"), "{sans}");
}

/// 🤖 The reading of an indicator Argos AI does not read by rules (« réglages seulement »), whatever Lens reads: the
/// ✨ of the first built-in outside `argos_ai::READ_BY_RULES` (Lens aside), else Cipher's settings read under an id
/// argos_ai does not know. The reading is the panel's last message; returns it and the indicator.
fn lecture_sans_regles(a: &mut App) -> (String, argos_render::indicators::Ind) {
    use argos_render::indicators::Ind;
    let hors = Ind::ALL
        .into_iter()
        .find(|i| *i != Ind::Lens && !argos_ai::READ_BY_RULES.contains(&crate::actions::argos_ai_ecran::id_catalogue(*i).as_str()));
    match hors {
        Some(i) => {
            a.ind.on.push(i);
            a.refresh_indicators();
            a.legend_click(argos_render::indleg::LegClick::Ai(i));
            (a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default(), i)
        }
        None => {
            let t = a.lecture_par_id(Ind::Cipher, "TestSansRegles", false);
            a.ai.msgs.push((false, t.clone()));
            (t, Ind::Cipher)
        }
    }
}

/// 🎨 The ⚙ Style of a built-in written in Argos Script (« MA · couleur », « MA #1 · épaisseur »…) is never given to
/// Argos AI: the screen passes the inputs only, with their keys as the catalogue writes them.
#[test]
fn style_des_integres_cache_a_argos_ai() {
    dossier_a_soi("style-integres-ai");
    let a = app();
    for i in [argos_render::indicators::Ind::Sma, argos_render::indicators::Ind::MaRibbon] {
        let f = argos_render::catalogue::fiche(i);
        let n = argos_script::catalogue::entrees(argos_script::catalogue::source(f).unwrap()).len();
        assert!(i.params().1.len() > n, "{i:?}: style settings in its ⚙");
        assert!((0..n).all(|k| !argos_script::catalogue::est_style(f, k)) && argos_script::catalogue::est_style(f, n));
        let noms: Vec<String> = a.reglages_par_nom(i).into_iter().map(|(n, _)| n).collect();
        assert_eq!(noms.len(), n, "{i:?}: its inputs only: {noms:?}");
        assert!(
            !noms
                .iter()
                .any(|k| k.contains(" · couleur") || k.contains(" · épaisseur") || k.contains(" · type de ligne")),
            "{i:?}: {noms:?}"
        );
    }
}

/// 🤖 « Ok reglage court » (Jee 18 Nov): under a reading by rules, only the settings the rules read
/// (`argos_ai::keys_read`, in its order), and a « ⚙ Tous les réglages » button that opens the indicator's ⚙ (mouse
/// and finger); an indicator without rules keeps all its settings and has no such button.
#[test]
fn reglages_courts_sous_une_lecture() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::LegClick;
    for doigt in [false, true] {
        dossier_a_soi(if doigt { "reglages-courts-doigt" } else { "reglages-courts-souris" });
        let mut a = if doigt { cell_pilule() } else { app() };
        a.top.prefs.bienvenue_vue = true;
        a.ind.on = vec![Ind::Rsi, Ind::Cipher];
        a.ui.frames = 5;
        a.refresh_indicators();
        let reglages_lus = |t: &str| t.split("\nRéglages\n").nth(1).unwrap_or("").lines().filter(|l| l.starts_with("• ")).count();
        // Cipher: first only the settings the reading names (a role), then « ＋ N autres réglages utiles » shows all
        // its keys read, in the same message
        a.legend_click(LegClick::Ai(Ind::Cipher));
        let t = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        let roles = t
            .split("\nRéglages\n")
            .nth(1)
            .unwrap_or("")
            .lines()
            .filter(|l| l.starts_with("• ") && l.contains(" ("))
            .count();
        assert!(reglages_lus(&t) < argos_ai::keys_read("Cipher").len(), "Cipher: first the named ones only: {t}");
        assert_eq!(
            a.ai.autres_reglages,
            argos_ai::keys_read("Cipher").len() - reglages_lus(&t),
            "the others counted"
        );
        assert!(roles >= 1, "named settings: {t}");
        assert_eq!(a.ai.tous_reglages, Some(Ind::Cipher), "the button is there");
        let r = a.ai_rect();
        let text = argos_render::text::Text::default();
        let (ox, oy) =
            a.ai.bouton_autres_reglages(&text, r.w, r.h)
                .map(|b| (r.x + b.cx(), r.y + b.cy()))
                .expect("＋ N autres");
        let (b1, b2) = (
            a.ai.bouton_autres_reglages(&text, r.w, r.h).unwrap(),
            a.ai.bouton_tous_reglages(&text, r.w, r.h).unwrap(),
        );
        assert!(
            b1.x + b1.w <= b2.x || b2.x + b2.w <= b1.x,
            "the two buttons side by side, never one on the other: {b1:?} {b2:?}"
        );
        let (n, avant, autres) = (a.ai.msgs.len(), reglages_lus(&t), a.ai.autres_reglages);
        if doigt {
            toucher(&mut a, ox, oy);
        } else {
            clic(&mut a, ox, oy);
        }
        let t2 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert_eq!(a.ai.msgs.len(), n, "the same message, unfolded");
        assert_eq!(reglages_lus(&t2), avant + autres, "then all its keys read: {t2}");
        // every key the rules read is in the reading's settings (one of the catalogue's settings names)
        let par_nom = a.reglages_par_nom(Ind::Cipher);
        let par_nom: Vec<(&str, f64)> = par_nom.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        let lecture = argos_ai::read("Cipher", a.bougies_fermees(), &par_nom);
        let absentes: Vec<&str> = argos_ai::keys_read("Cipher")
            .iter()
            .copied()
            .filter(|k| !lecture.settings.iter().any(|r| r.key == *k))
            .collect();
        assert!(absentes.is_empty(), "keys_read not in Reading.settings: {absentes:?}");
        assert_eq!(a.ai.autres_reglages, 0, "nothing more to unfold");
        // TVD too: first the named ones only
        a.ind.on.push(Ind::Tvd);
        a.refresh_indicators();
        a.legend_click(LegClick::Ai(Ind::Tvd));
        let t3 = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        assert!(reglages_lus(&t3) < argos_ai::keys_read("Tvd").len(), "TVD: first the named ones only: {t3}");
        // RSI: length, high, low
        a.legend_click(LegClick::Ai(Ind::Rsi));
        let t = a.ai.msgs.last().map(|m| m.1.clone()).unwrap_or_default();
        let p = Ind::Rsi.params().1;
        assert_eq!(reglages_lus(&t), 3, "RSI: 3 settings: {t}");
        for k in ["len", "haut", "bas"] {
            let label = p.iter().find(|x| x.key == k).map(|x| x.label).unwrap_or(k);
            assert!(t.contains(&format!("• {label} : ")), "RSI: {k}: {t}");
        }
        // the button opens the indicator's ⚙
        let r = a.ai_rect();
        let text = argos_render::text::Text::default();
        let (bx, by) =
            a.ai.bouton_tous_reglages(&text, r.w, r.h)
                .map(|b| (r.x + b.cx(), r.y + b.cy()))
                .expect("the button");
        a.ui.ind_dlg = None;
        if doigt {
            toucher(&mut a, bx, by);
        } else {
            clic(&mut a, bx, by);
        }
        assert!(a.ui.ind_dlg.is_some(), "⚙ opened (doigt: {doigt})");
        a.ui.ind_dlg = None;
        // An indicator without rules: all its settings, no button
        let (t, _) = lecture_sans_regles(&mut a);
        assert!(
            t.contains("Lecture des réglages seulement") && a.ai.tous_reglages.is_none(),
            "settings only: {t}"
        );
        assert!(reglages_lus(&t) >= 4, "all its settings: {t}");
    }
}

/// 🔔 An app with a price alert in the middle of the chart, the mouse on its line: (app, id, price, line height).
fn alerte_au_milieu(dossier: &str) -> (App, u64, f64, f64) {
    dossier_a_soi(dossier);
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    let _ = a.to_chart();
    let sc = a.scale_now().expect("a scale");
    let prix = sc.price_of(a.chart.viewport.plot_h() * 0.5);
    a.add_price_alert(prix);
    a.banner = None;
    let id = a.alerts.list.last().unwrap().id;
    let y = sc.y_of(prix);
    a.mouse_moved(PhysicalPosition::new(60.0 + a.rw(), y + a.top_h()));
    (a, id, prix, y)
}

/// 🔔 Jee (19 Nov, his TradingView captures): a right-click (a long press on phone and tablet) on an alert line opens
/// its menu — ⏸ Mettre en pause (▶ Reprendre), Éditer…, 🗑 Effacer, then « Lignes d'alerte » (checked) and « Modifier
/// la couleur des alertes… »; each works: pause keeps it on the chart, grey, « (en pause) »; Éditer… opens the alert
/// window of drawings (« Enregistrer » edits the same alert, « Annuler » nothing); Effacer asks first; the colour and
/// the lines' setting are kept in topbar.json.
#[test]
fn menu_de_la_ligne_d_alerte() {
    let (mut a, id, prix, _) = alerte_au_milieu("alerte-menu");
    let labels = |a: &App| -> Vec<String> {
        let (_, m) = a.ui.list.as_ref().expect("a menu");
        m.groups.iter().flat_map(|g| g.items.iter().map(|i| i.label.clone())).collect()
    };
    a.right_click();
    assert!(matches!(a.ui.list, Some((ListKind::AlertLine(i), _)) if i == id), "its menu");
    assert!(a.ui.list.as_ref().unwrap().1.gris, "grey icons");
    assert_eq!(
        labels(&a),
        [
            "Mettre en pause",
            "Éditer…",
            "Son…",
            "Effacer",
            "Lignes d'alerte",
            "Modifier la couleur des alertes…"
        ]
    );
    let lignes = a.ui.list.as_ref().unwrap().1.groups[1].items[0].clone();
    assert!(lignes.checked && a.ui.list.as_ref().unwrap().1.groups.len() == 2, "checked, after a separator");
    let anchor = a.ui.list.as_ref().unwrap().1.anchor;
    a.ui.list = None;
    // ⏸ pause: still on the chart, grey; its label says so; the menu offers ▶ Reprendre
    a.ligne_alerte_choix(id, 1, anchor);
    assert!(!a.alerts.list.iter().find(|x| x.id == id).unwrap().on);
    let c = a.chart_with(false);
    assert!(c.alerts.is_empty() && c.alertes_vue.pause == vec![prix], "a grey line");
    assert!(a.etiquette_de(id).unwrap().1.ends_with("(en pause)"));
    a.right_click();
    assert_eq!(labels(&a)[0], "Reprendre");
    a.ui.list = None;
    // Éditer…: the window of drawings, « Annuler » changes nothing
    let avant = a.alerts.clone();
    a.ligne_alerte_choix(id, 2, anchor);
    let w = a.ui.draw_alert.as_ref().expect("the alert window").1.clone();
    let (r, bas) = (w.rect, w.rect.y + w.rect.h - 42.0);
    a.draw_alert_press(r.x + r.w - 24.0 - 124.0 - 106.0 + 48.0, bas);
    assert!(a.ui.draw_alert.is_none() && a.alerts == avant, "Annuler: nothing");
    // « Enregistrer » with a new price: the same alert, still paused
    a.ligne_alerte_choix(id, 2, anchor);
    let r = a.ui.draw_alert.as_ref().unwrap().1.rect;
    a.draw_alert_press(r.x + 383.0 + 96.0, r.y + 232.0);
    if let Some((_, w)) = a.ui.draw_alert.as_mut() {
        (0..20).for_each(|_| w.backspace());
        "1234.5".chars().for_each(|c| w.type_char(c));
    }
    a.draw_alert_press(r.x + r.w - 24.0 - 62.0, r.y + r.h - 42.0);
    assert!(a.ui.draw_alert.is_none(), "closed");
    assert_eq!(a.alerts.list.len(), avant.list.len(), "no duplicate");
    let al = a.alerts.list.iter().find(|x| x.id == id).unwrap();
    assert!(
        matches!(al.kind, argos_core::alerts::Kind::Price { price, dir: argos_core::alerts::Dir::Both } if price == 1234.5),
        "{:?}",
        al.kind
    );
    assert!(!al.on && al.msg.is_empty(), "its pause kept, its automatic name");
    a.ligne_alerte_choix(id, 1, anchor);
    // « Lignes d'alerte » unchecked, kept
    a.ligne_alerte_choix(id, 4, anchor);
    let p: argos_render::topbar::Prefs = App::read_prefs("topbar.json");
    assert!(!a.top.prefs.lignes_alerte && !p.lignes_alerte && a.chart_with(false).alertes_vue.courtes);
    a.ligne_alerte_choix(id, 4, anchor);
    assert!(a.top.prefs.lignes_alerte);
    // the colour: the drawing bar's palette, one colour for all alert lines, kept
    a.ligne_alerte_choix(id, 5, anchor);
    assert!(
        matches!(a.ui.colorpick, Some((i, ..)) if i == crate::actions::alerts::COULEUR_ALERTES),
        "the palette"
    );
    a.set_drawing_colour(crate::actions::alerts::COULEUR_ALERTES, argos_render::objdlg::Which::Line, "#ff2020".into());
    let p: argos_render::topbar::Prefs = App::read_prefs("topbar.json");
    assert_eq!(p.couleur_alertes, "#ff2020", "kept after reloading");
    assert_eq!(a.chart_with(false).alertes_vue.couleur, Some(argos_render::text::Rgb(0xff, 0x20, 0x20)));
    a.ui.colorpick = None;
    // 🗑 Effacer: « Supprimer cette alerte? » first, then gone
    a.ligne_alerte_choix(id, 3, anchor);
    let (c, quoi) = a.ui.confirmer.clone().expect("the box");
    assert_eq!(quoi, crate::actions::alerts::AConfirmer::SupprimerAlerte(id));
    let (w, h) = a.taille_points();
    let q = c.places(&argos_render::text::Text::default(), w, h);
    clic(&mut a, q.ok.cx(), q.ok.cy());
    assert!(!a.alerts.list.iter().any(|x| x.id == id), "Effacer: deleted");
}

/// 🔔 « Lignes d'alerte » unchecked: no full-width dashed line; the short form (name, 🗑, handle, a few dashes) stays
/// glued to the price scale, in the chosen colour, and its handle still drags the alert.
#[test]
fn forme_courte_des_alertes_sur_l_image() {
    let (mut a, id, prix, y) = alerte_au_milieu("alerte-courte");
    let s = argos_render::px::scale() as f64;
    let image = |a: &App| {
        let c = a.to_chart();
        let mut pm = argos_render::render(&c);
        a.dessiner_etiquette_alerte(&mut pm);
        pm
    };
    let compte = |pm: &tiny_skia::Pixmap, x0: f64, x1: f64, y: f64, ok: &dyn Fn(tiny_skia::PremultipliedColorU8) -> bool| {
        let py = (y.round() * s) as u32;
        ((x0 * s) as u32..(x1 * s) as u32).filter(|&x| pm.pixel(x, py).is_some_and(ok)).count()
    };
    let rouge = |p: tiny_skia::PremultipliedColorU8| p.red() > 170 && p.green() < 110 && p.blue() < 110;
    a.top.prefs.couleur_alertes = "#ff2020".into();
    a.mouse_moved(PhysicalPosition::new(60.0 + a.rw(), y - 120.0 + a.top_h()));
    let pm = image(&a);
    assert!(compte(&pm, 0.0, 60.0, y, &rouge) >= 20, "checked: the full-width line, red");
    a.top.prefs.lignes_alerte = false;
    let pm = image(&a);
    assert_eq!(compte(&pm, 0.0, 60.0, y, &rouge), 0, "unchecked: no line across the chart");
    let pw = a.chart.viewport.plot_w();
    assert!(
        compte(&pm, pw - argos_render::alerte_etiquette::TIRET, pw, y, &rouge) >= 6,
        "the dashes by the scale"
    );
    let (_, nom, _, p) = a.etiquette_de(id).expect("its short form");
    assert!(nom.contains("Croisement"));
    assert!(p.poignee.x + p.poignee.w <= pw - argos_render::alerte_etiquette::TIRET + 0.5 && p.boite.x + p.boite.w < p.poignee.x);
    let g = pm.pixel((p.poignee.cx() * s) as u32, (p.poignee.cy() * s) as u32).unwrap();
    assert!(rouge(g), "the handle in the chosen colour: {g:?}");
    // the mouse on the short form: it is the alert's (right-click menu); in the middle of the chart: nothing
    a.mouse_moved(PhysicalPosition::new(p.boite.cx() + a.rw(), y + a.top_h()));
    assert_eq!(a.alert_at(), Some(id));
    a.mouse_moved(PhysicalPosition::new(60.0 + a.rw(), y + a.top_h()));
    assert_eq!(a.alert_at(), None, "no line there any more");
    // its handle drags it
    let (hx, hy) = (p.poignee.cx() + a.rw(), p.poignee.cy() + a.top_h());
    a.mouse_moved(PhysicalPosition::new(hx, hy));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(hx, hy + 40.0));
    a.left_button(ElementState::Released);
    let nouveau = a.alerts.lignes(&a.symbol_label()).into_iter().find(|l| l.0 == id).unwrap().1;
    assert!(nouveau < prix, "dragged down by its handle ({nouveau} < {prix})");
}

/// 🔔 An alert that is off says why (coordinator, 19 Nov): paused by Jee « (en pause) », a once-only alert already
/// fired « (déclenchée) », past its validity « (expirée) »; « Reprendre » rearms it in every case.
#[test]
fn alerte_eteinte_dit_pourquoi() {
    use crate::actions::alerts::raison_arret;
    dossier_a_soi("alerte-raison");
    let mut a = app();
    let _ = a.to_chart();
    let prix = a.chart.bars.last().map_or(100.0, |b| b.close) * 1.5;
    a.add_price_alert(prix);
    let id = a.alerts.list.last().unwrap().id;
    let now = now_s();
    let cas = |a: &mut App, fired: Option<i64>, until: Option<i64>| {
        let x = a.alerts.list.iter_mut().find(|x| x.id == id).unwrap();
        (x.on, x.fired, x.every, x.until) = (false, fired, None, until);
        raison_arret(x, now)
    };
    assert_eq!(cas(&mut a, None, None), "(en pause)");
    assert_eq!(cas(&mut a, Some(now - 60), None), "(déclenchée)");
    assert_eq!(cas(&mut a, None, Some(now - 10)), "(expirée)");
    // « Reprendre » from the line's menu: on again, no past end of validity
    a.ligne_alerte_choix(id, crate::actions::alerts::AL_PAUSE, argos_render::shapes::Rect::new(0.0, 0.0, 0.0, 0.0));
    let x = a.alerts.list.iter().find(|x| x.id == id).unwrap();
    assert!(x.on && x.until.is_none() && x.fired.is_none(), "rearmed: {x:?}");
}

/// 🔔 A paused alert never fires; resumed, it fires again.
#[test]
fn alerte_en_pause_ne_se_declenche_pas() {
    dossier_a_soi("alerte-pause");
    let mut a = app();
    let last = a.chart.bars.last().map(|b| b.close).unwrap();
    a.add_price_alert(last + 2.0);
    let id = a.alerts.list[0].id;
    let r = Rect::new(10.0, 10.0, 0.0, 0.0);
    a.ligne_alerte_choix(id, 1, r);
    let prix = |a: &mut App, p: f64| {
        if let Some(b) = a.chart.bars.last_mut() {
            b.close = p;
        }
        a.alerts_watch();
    };
    prix(&mut a, last);
    prix(&mut a, last + 3.0);
    prix(&mut a, last);
    assert!(a.alerts.history.is_empty(), "paused: never fires");
    a.ligne_alerte_choix(id, 1, r);
    prix(&mut a, last);
    prix(&mut a, last + 3.0);
    assert_eq!(a.alerts.history.len(), 1, "resumed: it fires");
}

/// 🧠 Jee, 19 Nov: the memory menu of the Argos AI card opened behind the card (only its left end showed). It is now
/// laid over the card, whole; a menu not opened from the card keeps its place under it.
#[test]
fn menu_memoire_par_dessus_argos_ai() {
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.ui.ai_open = true;
    let (w, h) = (1440, 900);
    let (sans_menu, _) = image_ecran(&mut a, w, h);
    let r = a.ai_rect();
    a.open_list(ListKind::AiMemory, Rect::new(r.x + r.w - 300.0, r.y + 44.0, 260.0, 1.0));
    let (avec_menu, _) = image_ecran(&mut a, w, h);
    let m = a.ui.list.as_ref().expect("the memory menu is open").1.rect;
    let ordre = &a.ui.couches_test;
    let (ai, menu) = (ordre.iter().position(|c| *c == "argos_ai"), ordre.iter().position(|c| *c == "menu_ai"));
    assert!(ai.is_some() && menu > ai, "the memory menu is laid after the card: {ordre:?}");
    assert!(!ordre.contains(&"menu"), "not twice: {ordre:?}");
    // on the image: the menu shows inside the card (at least half of its pixels differ from the card alone), and it stays in
    // the window
    assert!(
        m.x >= 0.0 && m.y >= 0.0 && m.x + m.w <= w as f64 && m.y + m.h <= h as f64,
        "{m:?} in the window"
    );
    let (mut tous, mut changes) = (0, 0);
    for y in (m.y as u32 + 4)..((m.y + m.h) as u32).saturating_sub(4) {
        for x in (m.x as u32 + 4)..((m.x + m.w) as u32).saturating_sub(4) {
            let i = (y * w + x) as usize;
            tous += 1;
            changes += usize::from(avec_menu[i] != sans_menu[i]);
        }
    }
    assert!(changes * 2 >= tous, "the menu shows over the card ({changes} of {tous} pixels changed)");
    // a menu not opened from the card (the chart's interval menu, say) is laid as before
    a.ui.list = None;
    a.ui.ai_open = false;
    image_ecran(&mut a, w, h);
    assert!(!a.ui.couches_test.contains(&"menu_ai"));
}

/// 🖥📱 Jee (20 Nov, « jamais réglé »): the ticker row and the info row never cover the chart's legend (title, O H L C,
/// Vol, ⌄ N): rows unfolded, folded, or folded and open on hover over the chart (the legend goes down under them, then
/// up again); on the phone, under the top pill.
#[test]
fn legende_jamais_sous_les_barres_du_haut() {
    use argos_render::topbar::{HEIGHT, SANS_BANDEAUX_H};
    // (the title's letters: 24 under the chart's top plus the shift, about 13 high)
    let haut_legende = |a: &App| a.top_h() + a.decalage_legende() + 24.0 - 13.0;
    for (sans, survol) in [(false, false), (true, false), (true, true)] {
        let mut a = app();
        (a.top.sans_bandeaux, a.top.survol_bandeaux) = (sans, survol);
        // the bottom of what is drawn at the top: the bar, its folded band, or the rows open over the chart
        let bas = if sans && survol {
            HEIGHT as f64
        } else if let Some(b) = a.bande_haut() {
            b.y + b.h
        } else {
            a.top_h()
        };
        assert!(haut_legende(&a) >= bas, "folded {sans} hover {survol}: legend {} under {bas}", haut_legende(&a));
        if sans {
            assert_eq!(a.top_h(), SANS_BANDEAUX_H as f64);
        }
    }
    // the rows fold again: the legend goes back up
    let mut a = app();
    (a.top.sans_bandeaux, a.top.survol_bandeaux) = (true, true);
    let ouvert = a.decalage_legende();
    a.top.survol_bandeaux = false;
    assert!(a.decalage_legende() < ouvert, "back up");
    // 📱 phone: under the top pill
    let mut c = app();
    c.top.phone = true;
    let pilule = argos_render::ecrans::cell().place("demoband", c.win_w(), c.cell_h());
    if let Some(p) = pilule {
        assert!(haut_legende(&c) >= p.y + p.h - 0.5, "phone: legend under the pill");
    }
}

/// 🌐 « Tous les marchés » (Jee, 20 nov. 2026) : la ligne du menu des listes ouvre la vue, son fil lit seulement la
/// plateforme affichée, les lignes suivent « Trier par » (donnée absente en bas), changer de plateforme la demande
/// tout de suite, ☆ ajoute à « Mes favoris », et les menus des pilules n'écrivent aucun raccourci.
#[test]
fn watchlist_tous_les_marches() {
    use argos_data::tous::Ligne;
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    let tous = a.lists_menu()[0]
        .items
        .iter()
        .find(|i| i.id == ID_TOUS)
        .cloned()
        .expect("ligne du menu des listes");
    assert_eq!(tous.label, crate::actions::TOUS);
    a.lists_pick(ID_TOUS, Rect::new(0.0, 0.0, 1.0, 1.0));
    assert!(a.watch.prefs.tous);
    assert_eq!(*a.feeds.tous_wanted.lock().unwrap(), Some(Exchange::Hyperliquid), "plateforme par défaut");
    let l = |base: &str, vol: Option<f64>, high: Option<f64>| Ligne {
        base: base.into(),
        symbol: format!("{base}USDT"),
        price: 100.0,
        prev: 100.0,
        vol_usd: vol,
        high,
        low: high.map(|_| 90.0),
        ..Default::default()
    };
    a.tous_pick(ID_TOUS_BOURSE + 1);
    assert_eq!(*a.feeds.tous_wanted.lock().unwrap(), Some(Exchange::Binance), "Binance demandée tout de suite");
    a.feeds.tous = Some((
        Exchange::Binance,
        Ok(vec![l("ETH", Some(5.0), Some(120.0)), l("NEW", None, None), l("BTC", Some(9.0), Some(101.0))]),
    ));
    a.refresh_watch();
    let noms = |a: &App| a.watch.sections[0].rows.iter().map(|r| r.label.clone()).collect::<Vec<_>>().join(",");
    assert_eq!(noms(&a), "BTC,ETH,NEW", "volume 24 h, donnée absente en bas");
    assert_eq!(a.watch.sections[0].rows[0].coin, "binance:BTCUSDT", "le symbole du catalogue de la plateforme");
    let volat = argos_data::tous::Tri::ALL.iter().position(|t| *t == argos_data::tous::Tri::Volatilite).unwrap();
    a.tous_pick(ID_TOUS_TRI + volat as i64);
    assert_eq!(noms(&a), "ETH,BTC,NEW", "volatilité 24 h");
    assert!(a.watch.sections[0].rows[0].note.starts_with("Volat. 30"));
    // une autre plateforme : son relevé seulement (l'ancien n'est pas montré sous son nom)
    a.tous_pick(ID_TOUS_BOURSE + 2);
    assert!(a.watch.sections.iter().all(|s| s.rows.is_empty()));
    assert!(a.watch.note.contains("Bybit"));
    // ☆ : dans Mes favoris (une seule fois)
    a.ajouter_favori("binance:BTCUSDT");
    a.ajouter_favori("binance:BTCUSDT");
    let fav = a.favoris_id().unwrap();
    let items = a.store.as_ref().unwrap().items(fav).unwrap();
    assert_eq!(items.iter().filter(|(_, c)| c == "binance:BTCUSDT").count(), 1);
    assert!(a.watch.favoris.contains("binance:BTCUSDT"), "★ pleine sur la ligne");
    // Hyperliquid : même id que sa liste (« BTC ») ; ☆ le range comme « hl:BTC », ★ sur la ligne
    a.tous_pick(ID_TOUS_BOURSE);
    a.feeds.tous = Some((
        Exchange::Hyperliquid,
        Ok(vec![Ligne {
            symbol: "BTC".into(),
            ..l("BTC", Some(1.0), None)
        }]),
    ));
    a.refresh_watch();
    assert_eq!(a.watch.sections[0].rows[0].coin, "BTC");
    a.ajouter_favori("BTC");
    assert!(a.store.as_ref().unwrap().items(fav).unwrap().iter().any(|(_, c)| c == "hl:BTC"));
    assert!(a.watch.favoris.contains("BTC"));
    // 📱 menus des pilules : aucun raccourci écrit
    a.top.phone = true;
    for kind in [ListKind::TousBourse, ListKind::TousTri] {
        assert!(a.tous_menu(kind).iter().flat_map(|g| &g.items).all(|i| i.hint.is_empty()));
    }
    // retour à une liste : le fil s'arrête
    a.lists_pick(1, Rect::new(0.0, 0.0, 1.0, 1.0));
    assert!(!a.watch.prefs.tous);
    assert_eq!(*a.feeds.tous_wanted.lock().unwrap(), None);
}

/// ＋ de la watchlist (Jee, 20 nov. 2026) : il ouvre la recherche de symbole dans la fenêtre, avec un filtre pour
/// chaque plateforme (pas seulement les activées).
#[test]
fn plus_ouvre_la_recherche_toutes_plateformes() {
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Watchlist);
    a.refresh_watch();
    let h = a.area.1 as f64;
    let r = a.watch.add_rect();
    let v = a.volet_rect().unwrap_or_else(|| Rect::new(a.side_x(), a.top_h(), a.watch.pw(), h));
    clic(&mut a, v.x + r.cx(), v.y + r.cy());
    let p = a.ui.add.as_ref().expect("＋ ouvre la recherche de symbole");
    assert!(
        p.rect.x >= 0.0 && p.rect.x + p.rect.w <= a.win_w() + 0.5 && p.rect.y >= 0.0,
        "dans la fenêtre : {:?}",
        p.rect
    );
    for e in Exchange::ALL {
        assert!(p.chips.iter().any(|c| c.key == format!("ex:{}", e.id())), "filtre {}", e.name());
    }
}

/// Bug de Jee (20 nov. 2026) : un symbole OANDA (« OANDA:XAUUSD ») et un symbole Binance (« BINANCE:BTCUSDT »)
/// ne s'ajoutaient pas à « Mes favoris » ; ils s'y ajoutent depuis la recherche du ＋.
#[test]
fn favoris_oanda_et_binance() {
    let mut a = app();
    a.store = Some(Store::in_memory().unwrap());
    let btc = argos_core::Instrument {
        exchange: "binance".into(),
        symbol: "BTCUSDT".into(),
        base: "BTC".into(),
        quote: "USDT".into(),
        kind: argos_core::MarketKind::Perp,
        group: "Crypto".into(),
        price_decimals: 1,
        size_decimals: 3,
        growth: false,
        active: true,
    };
    a.store.as_mut().unwrap().save_catalog("binance", &[btc], now_s()).unwrap();
    a.load_lists();
    let fav = a.favoris_id().unwrap();
    a.lists_pick(a.lists.iter().position(|l| l.id == fav).unwrap() as i64 + 1, Rect::new(0.0, 0.0, 1.0, 1.0));
    a.ui.add = Some(argos_render::wlmenu::AddPanel::new(Rect::new(0.0, 300.0, 390.0, 400.0), a.add_chips()));
    for (q, id) in [
        ("OANDA:XAUUSD", argos_data::oanda::item("XAU_USD")),
        ("BINANCE:BTCUSDT", "binance:BTCUSDT".to_string()),
    ] {
        a.ui.add.as_mut().unwrap().query = q.into();
        a.search_add();
        let found = a.ui.add.as_ref().unwrap().results.iter().find(|f| f.id == id).cloned();
        assert!(found.as_ref().is_some_and(|f| !f.have), "{q} : {:?}", a.ui.add.as_ref().unwrap().results);
        a.add_symbol(&id);
        assert!(
            a.store.as_ref().unwrap().items(fav).unwrap().iter().any(|(_, c)| *c == id),
            "{q} dans Mes favoris"
        );
    }
}

/// 🔊 The open list menu's line ids at window point `y` (scanning the menu's middle column): the first line whose id is
/// `id`, as a window point.
fn ligne_du_menu(a: &App, id: i64) -> (f64, f64) {
    let (_, m) = a.ui.list.as_ref().expect("a menu");
    let r = m.rect;
    let mut y = r.y + 1.0;
    while y < r.y + r.h {
        if m.clone().click(r.cx(), y) == argos_render::listmenu::MenuClick::Pick(id) {
            return (r.cx(), y);
        }
        y += 2.0;
    }
    panic!("line {id} not in the menu")
}

/// 🔊 The labels of the open list menu.
fn lignes_du_menu(a: &App) -> Vec<String> {
    let (_, m) = a.ui.list.as_ref().expect("a menu");
    m.groups.iter().flat_map(|g| g.items.iter().map(|i| i.label.clone())).collect()
}

/// 🔊 Jee (20 Nov): one sound per alert. The menu of an alert line ▸ « Son… »: « Par défaut » first (checked), the 8
/// sounds, « Son coupé »; a choice is played, kept in alerts.json, and the alert rings with it; « Son coupé »: the
/// alert still shows its banner, with no sound at all; an alert without its own sound plays the settings' one.
#[test]
fn son_d_une_alerte_et_son_coupe() {
    use argos_core::alerts::SON_COUPE;
    dossier_a_soi("alerte-son");
    let mut a = app();
    let _ = platform::sons_joues();
    let prix = |a: &App| a.chart.bars.last().map(|b| b.close).unwrap();
    let franchir = |a: &mut App, de: f64| {
        a.alerts_watch(); // starting price
        if let Some(b) = a.chart.bars.last_mut() {
            b.close += de;
        }
        a.banner = None;
        a.alerts_watch();
    };
    let p = prix(&a);
    a.add_price_alert(p + 2.0);
    let id = a.alerts.list.last().unwrap().id;
    a.ligne_alerte_choix(id, 6, argos_render::shapes::Rect::new(300.0, 300.0, 0.0, 0.0));
    assert!(matches!(a.ui.list, Some((ListKind::SonAlerte(i), _)) if i == id), "« Son… »: its list");
    let l = lignes_du_menu(&a);
    assert_eq!(
        (l.len(), l[0].as_str(), l[1].as_str(), l[9].as_str()),
        (10, "Par défaut (Bip)", "Bip", "Son coupé")
    );
    assert!(a.ui.list.as_ref().unwrap().1.groups[0].items[0].checked, "« Par défaut » checked");
    // « Cloche »: played, kept, checked
    let (x, y) = ligne_du_menu(&a, 3);
    clic(&mut a, x, y);
    assert_eq!(a.alerts.list.iter().find(|x| x.id == id).unwrap().son.as_deref(), Some("Cloche"));
    let lu: argos_core::alerts::Alerts = App::read_prefs("alerts.json");
    assert_eq!(
        lu.list.iter().find(|x| x.id == id).unwrap().son.as_deref(),
        Some("Cloche"),
        "kept after reading back"
    );
    assert_eq!(platform::sons_joues(), ["Cloche"], "played to hear it");
    a.ligne_alerte_choix(id, 6, argos_render::shapes::Rect::new(300.0, 300.0, 0.0, 0.0));
    assert!(a.ui.list.as_ref().unwrap().1.groups[0].items[3].checked, "checked on its choice");
    a.ui.list = None;
    franchir(&mut a, 3.0);
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("🔔")), "fired");
    assert_eq!(platform::sons_joues(), ["Cloche"], "it rings with its own sound");
    // « Son coupé »: nothing played, neither to choose it nor when it fires
    let p = prix(&a);
    a.add_price_alert(p + 2.0);
    let id2 = a.alerts.list.last().unwrap().id;
    a.son_alerte_choix(id2, 9);
    assert_eq!(a.alerts.list.iter().find(|x| x.id == id2).unwrap().son.as_deref(), Some(SON_COUPE));
    franchir(&mut a, 3.0);
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("🔔")), "fired, its banner shown");
    assert!(platform::sons_joues().is_empty(), "« Son coupé »: no sound");
    // no sound of its own: the settings' sound
    a.top.prefs.son_alertes = "Gong doux".into();
    let p = prix(&a);
    a.add_price_alert(p + 2.0);
    franchir(&mut a, 3.0);
    assert_eq!(platform::sons_joues(), ["Gong doux"]);
}

/// 🔊 Réglages ▸ « Son des alertes »: its list — the 8 sounds (checked: « Bip »), ▶ Écouter, the volume − / + by
/// 10; it stays open after each choice; each is played, kept in topbar.json, and the row's summary says it.
#[test]
fn son_des_alertes_dans_les_reglages() {
    use argos_render::reglages::{Cible, RegClick};
    dossier_a_soi("reglages-son");
    let mut a = app();
    let _ = platform::sons_joues();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    a.reglages_ouvrir();
    a.placer(taille);
    let r = a.ui.reglages.clone().expect("volet Réglages").rect();
    let mut y = r.y + 60.0;
    while y < r.y + r.h && a.ui.reglages.as_ref().unwrap().click(r.x + 40.0, y) != RegClick::Ligne(Cible::Act(argos_render::sheet::Act::SonAlertes)) {
        y += 2.0;
    }
    assert!(y < r.y + r.h, "the row « Son des alertes »");
    clic(&mut a, r.x + 40.0, y);
    assert!(matches!(a.ui.list, Some((ListKind::SonAlertes, _))), "its list");
    assert!(a.ui.reglages.is_some(), "Réglages stays open");
    assert_eq!(
        lignes_du_menu(&a),
        [
            "Bip",
            "Carillon",
            "Cloche",
            "Triple bip",
            "Montée",
            "Descente",
            "Gong doux",
            "Clochette",
            "▶ Écouter",
            "− Moins fort",
            "+ Plus fort"
        ]
    );
    assert!(a.ui.list.as_ref().unwrap().1.groups[0].items[0].checked, "« Bip » by default");
    assert_eq!(a.ui.list.as_ref().unwrap().1.groups[1].title, "VOLUME : 60 %");
    let (x, y) = ligne_du_menu(&a, 2);
    clic(&mut a, x, y);
    assert!(matches!(a.ui.list, Some((ListKind::SonAlertes, _))), "it stays open");
    assert_eq!(platform::sons_joues(), ["Cloche"]);
    let (x, y) = ligne_du_menu(&a, 102);
    clic(&mut a, x, y);
    let (x, y) = ligne_du_menu(&a, 101);
    clic(&mut a, x, y);
    clic(&mut a, x, y);
    assert_eq!(a.ui.list.as_ref().unwrap().1.groups[1].title, "VOLUME : 50 %");
    let (x, y) = ligne_du_menu(&a, 100);
    clic(&mut a, x, y);
    assert_eq!(platform::sons_joues().len(), 4, "each choice played, ▶ too");
    let p: argos_render::topbar::Prefs = App::read_prefs("topbar.json");
    assert_eq!((p.son_alertes.as_str(), p.volume_alertes), ("Cloche", 50), "kept");
    a.top.prefs.volume_alertes = 0;
    a.son_alertes_choix(100);
    assert!(platform::sons_joues().is_empty(), "volume 0: silent");
    a.top.prefs.volume_alertes = 100;
    a.son_alertes_choix(102);
    assert_eq!(a.top.prefs.volume_alertes, 100, "100 at most");
    a.ui.list = None;
    a.placer(taille);
    let res = &a.ui.reglages.as_ref().unwrap().resumes;
    assert!(res.iter().any(|(n, t)| *n == "Son des alertes" && t == "Cloche · 100 %"), "{res:?}");
    // its list is laid over the panel, not under it
    a.open_list(ListKind::SonAlertes, Rect::new(r.x + 40.0, r.y + 300.0, 240.0, 1.0));
    image_ecran(&mut a, 1440, 900);
    let ordre = &a.ui.couches_test;
    let (reg, menu) = (ordre.iter().position(|c| *c == "reglages"), ordre.iter().position(|c| *c == "menu_reglages"));
    assert!(reg.is_some() && menu > reg, "over the panel: {ordre:?}");
}

/// 📱 Cell / tablet by touch: the sound menus and Réglages show no keyboard shortcut; « Son des alertes » is there.
#[test]
fn sons_sans_raccourci_sur_cell() {
    dossier_a_soi("son-cell");
    let mut a = app();
    a.top.phone = true;
    argos_render::actions::set_tactile(true);
    assert!(argos_render::reglages::lignes().iter().any(|l| l.1 == "Son des alertes"));
    // Réglages of the cell (the Panels sheet): its tile opens the same list
    a.sheet_act(argos_render::sheet::Act::SonAlertes);
    assert!(matches!(a.ui.list, Some((ListKind::SonAlertes, _))), "the list, by the finger");
    let p = a.chart.bars.last().map(|b| b.close).unwrap();
    a.add_price_alert(p + 2.0);
    let id = a.alerts.list.last().unwrap().id;
    let anchor = argos_render::shapes::Rect::new(100.0, 100.0, 0.0, 0.0);
    for kind in [ListKind::AlertLine(id), ListKind::SonAlerte(id), ListKind::SonAlertes] {
        a.ui.list = None;
        a.open_list(kind, anchor);
        let (_, m) = a.ui.list.as_ref().expect("a menu");
        for i in m.groups.iter().flat_map(|g| &g.items) {
            assert!(
                i.hint.is_empty() && !i.label.contains("Ctrl") && !i.label.contains("Alt"),
                "{kind:?}: {}",
                i.label
            );
        }
    }
    argos_render::actions::set_tactile(false);
}

// — 🧱 Argos Murs —

/// A book of `px` ± 2 % with 1 coin every `step` (the background) and `walls` (price, coins).
fn carnet_murs(px: f64, step: f64, walls: &[(f64, f64)]) -> argos_core::murs::VenueBook {
    let mut b = argos_core::murs::VenueBook {
        venue: "essai".into(),
        ..Default::default()
    };
    let k0 = (px / step).round() as i64;
    for k in (k0 - 40)..=(k0 + 40) {
        let p = k as f64 * step;
        let sz = 1.0 + walls.iter().filter(|w| w.0 == p).map(|w| w.1).sum::<f64>();
        if p > px {
            b.asks.push((p, sz));
        } else if p < px {
            b.bids.push((p, sz));
        }
    }
    b
}

/// 📖 Lot loin-1 B: a Binance perp level at +2 % known only from the diff depth stream lands in its bucket, with
/// Binance among the exchanges and in the per-exchange amounts; Binance's coverage stays its photo's span; setting off
/// (or no stream): Binance = its photo alone.
#[test]
fn argos_murs_carnet_binance_complet() {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs_flux");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.ind.on.push(Ind::ArgosMurs);
    a.murs_refresh();
    assert_eq!(a.feeds.murs_flux.as_deref(), Some("BTC"), "the stream follows the coin");
    assert!(a.feeds.murs.as_ref().unwrap().flux.is_some());
    let step = a.feeds.murs.as_ref().unwrap().step;
    let px = a.chart.bars.last().unwrap().close;
    // Binance's photo: three steps on each side of the price (about ±0.3 %), the others a wide book
    let k0 = (px / step).round();
    let etroit = argos_core::murs::VenueBook {
        venue: "Binance".into(),
        bids: (1..=3).map(|k| ((k0 - k as f64) * step, 1.0)).collect(),
        asks: (1..=3).map(|k| ((k0 + k as f64) * step, 1.0)).collect(),
    };
    let res = |b: &argos_core::murs::VenueBook| -> Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> {
        Carnet::PERP
            .iter()
            .map(|c| (*c, Ok(if *c == Carnet::BinancePerp { b.clone() } else { carnet_murs(px, step, &[]) })))
            .collect()
    };
    a.murs_recu("BTC".into(), res(&etroit), None, Some(100));
    // the stream: 400 coins at +2 %, beyond the photo
    let loin = ((px * 1.02) / step).round() * step;
    let gen = a.feeds.murs_flux_gen.load(std::sync::atomic::Ordering::SeqCst);
    a.murs_flux_recu(
        gen,
        argos_data::live::LiveEvent::Profondeur(Box::new(argos_data::murs::MajProfondeur {
            pu: 150,
            u: 200,
            asks: vec![(loin, 400.0)],
            ..Default::default()
        })),
    );
    a.murs_recu("BTC".into(), res(&etroit), None, Some(300));
    let m = a.feeds.murs.as_ref().unwrap();
    let d = &m.depth_perp;
    let i = d.noms.iter().position(|n| n == "Binance").expect("Binance read");
    let k = (loin / d.step).round() as i64;
    let c = d.asks.get(&k).expect("the bucket at +2 %");
    assert!(c.venues & (1 << i) != 0, "Binance among the exchanges there");
    assert!((c.par[i] - loin * 400.0).abs() < 1e-3, "Binance's amount: {:?}", c.par);
    let (lo, hi) = d.reach[i];
    assert_eq!((lo, hi), ((k0 - 3.0) * step, (k0 + 3.0) * step), "coverage: the photo's span");
    assert!(loin > hi, "the level is beyond it");
    assert_eq!(m.flux.as_ref().unwrap().carnet.hors_photo(), 1);
    // setting off: no stream, Binance = its photo alone
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    let kf = argos_render::murs::PARAMS.iter().position(|p| p.key == "binFlux").unwrap();
    assert_eq!(v[kf], 1.0, "on by default");
    v[kf] = 0.0;
    a.ind.set.insert(Ind::ArgosMurs, v);
    a.murs_refresh();
    assert!(a.feeds.murs_flux.is_none() && a.feeds.murs.as_ref().unwrap().flux.is_none());
    a.murs_recu("BTC".into(), res(&etroit), None, Some(400));
    let d = &a.feeds.murs.as_ref().unwrap().depth_perp;
    let i = d.noms.iter().position(|n| n == "Binance").unwrap();
    assert!(d.asks.get(&k).is_none_or(|c| c.venues & (1 << i) == 0), "photo alone");
    // replay: no stream
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    v[kf] = 1.0;
    a.ind.set.insert(Ind::ArgosMurs, v);
    a.replay = Some(argos_render::replay::Replay::new(now_s()));
    a.murs_refresh();
    assert!(a.feeds.murs_flux.is_none(), "replay: no stream");
    a.replay = None;
    a.murs_refresh();
    assert_eq!(a.feeds.murs_flux.as_deref(), Some("BTC"));
}

/// 🧱 REMPLI (Jee, 21 Nov): a wall eaten while the chart's real trades hit it gets the mark, is recorded, and gives
/// one « mur rempli » alert (option on); eaten without such trades: no mark, no alert. The chart's trades (Binance)
/// are weighed against Binance's own part of the wall (a fifth here), not against the wall summed over 5 books.
#[test]
fn argos_murs_rempli_et_alerte_une_fois() {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs_rempli");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.ind.on.push(Ind::ArgosMurs);
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    let k = |key: &str| argos_render::murs::PARAMS.iter().position(|p| p.key == key).unwrap();
    (v[k("holdMin")], v[k("alRempli")], v[k("rempliMin")], v[k("carnets")]) = (0.0, 1.0, 0.0, 0.0);
    a.ind.set.insert(Ind::ArgosMurs, v);
    a.murs_refresh();
    let step = a.feeds.murs.as_ref().unwrap().step;
    let cle = a.orderflow_cle().expect("Binance BTC: real trades");
    let rempli_alertes = |a: &App| a.alerts.history.iter().filter(|f| f.text.contains("REMPLI")).count();
    let manche = |a: &mut App, achats: f64| {
        let px = a.chart.bars.last().unwrap().close;
        let wall = (px / step).round() * step + 3.0 * step;
        let res = |px: f64, w: &[(f64, f64)]| -> Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> {
            Carnet::PERP.iter().map(|c| (*c, Ok(carnet_murs(px, step, w)))).collect()
        };
        a.murs_recu("BTC".into(), res(px, &[(wall, 50.0)]), None, None);
        a.murs_recu("BTC".into(), res(px, &[(wall, 50.0)]), None, None);
        assert!(a.murs_ecran().unwrap().vue.pills.iter().any(|p| p.sell && p.price == wall));
        // takers buy at the wall (or nothing traded), the price goes through, the wall is gone
        a.feeds.orderflow.clear();
        if achats > 0.0 {
            let pas = step / 10.0;
            a.feeds.orderflow.push(crate::actions::orderflow::FluxReel {
                cle: cle.clone(),
                agg: argos_core::orderflow::Agregateur::new(Some(pas)),
                minutes: vec![argos_core::orderflow::FlowBar {
                    time: now_s() / 60 * 60 - 60,
                    pas,
                    rangees: vec![((wall / pas).floor() as i64, achats, 0.0)],
                    complete: true,
                }],
                recus: (0, 0),
            });
        }
        let haut = wall + 2.0 * step;
        let b = a.chart.bars.last_mut().unwrap();
        (b.close, b.high) = (haut, b.high.max(haut));
        a.murs_recu("BTC".into(), res(haut, &[]), None, None);
        // the next photos: nothing new
        a.murs_recu("BTC".into(), res(haut, &[]), None, None);
        // back below for the next round
        let b = a.chart.bars.last_mut().unwrap();
        b.close = px;
        a.murs_recu("BTC".into(), res(px, &[]), None, None);
        wall
    };
    let wall = manche(&mut a, 1_000.0);
    let e = a.murs_ecran().unwrap();
    assert_eq!(e.vue.remplis.len(), 1, "{:?}", e.vue.remplis);
    assert_eq!((e.vue.remplis[0].price, e.vue.remplis[0].sell), (wall, true));
    assert!(e.vue.remplis[0].usd > 50.0 * wall, "its size before the hit");
    assert_eq!(rempli_alertes(&a), 1, "one alert for the wall");
    let ev = a.feeds.murs_db.as_ref().unwrap().read("BTC|murs", 0, i64::MAX).unwrap();
    assert_eq!(ev.iter().filter(|e| e.kind == argos_core::murs::Kind::Filled).count(), 1, "recorded");
    // the same thing without trades at the wall: eaten, but no REMPLI and no alert
    manche(&mut a, 0.0);
    assert_eq!(a.murs_ecran().unwrap().vue.remplis.len(), 1);
    assert_eq!(rempli_alertes(&a), 1);
    // Binance held 51 coins of the 255 summed: 31 bought there is 60 % of its part (12 % of the sum): REMPLI
    manche(&mut a, 31.0);
    assert_eq!(a.murs_ecran().unwrap().vue.remplis.len(), 2);
    assert_eq!(rempli_alertes(&a), 2);
    // 10 bought (20 % of its part): a pull, nothing
    manche(&mut a, 10.0);
    assert_eq!(a.murs_ecran().unwrap().vue.remplis.len(), 2);
    assert_eq!(rempli_alertes(&a), 2);
}

#[test]
fn argos_murs_murs_enregistres_et_relecture() {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    assert_eq!(a.book_coin().as_deref(), Some("BTC"));
    a.ind.on.push(Ind::ArgosMurs);
    // shown at once (no 3 min wait in a test)
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    let hold = argos_render::murs::PARAMS.iter().position(|p| p.key == "holdMin").unwrap();
    v[hold] = 0.0;
    a.ind.set.insert(Ind::ArgosMurs, v);
    a.murs_refresh();
    let step = a.feeds.murs.as_ref().expect("Argos Murs started").step;
    assert!(step > 0.0);
    let px = a.chart.bars.last().unwrap().close;
    let wall = ((px * 1.01) / step).round() * step;
    let spot_wall = ((px * 0.99) / step).round() * step;
    let mut res: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> =
        Carnet::PERP.iter().map(|c| (*c, Ok(carnet_murs(px, step, &[(wall, 50.0)])))).collect();
    // one exchange blocked: the count says 4/5, nothing is made up
    res[3].1 = Err("Bitunix : refusé".into());
    res.extend(Carnet::SPOT.iter().map(|c| (*c, Ok(carnet_murs(px, step, &[(spot_wall, 40.0)])))));
    a.murs_recu("BTC".into(), res.clone(), None, None);
    a.murs_recu("BTC".into(), res, None, None);
    let e = a.murs_ecran().expect("drawn on a crypto chart");
    assert!(e.vue.pills.iter().any(|p| p.sell && !p.spot && p.price == wall), "{:?}", e.vue.pills);
    assert!(e.vue.pills.iter().any(|p| !p.sell && p.spot && p.price == spot_wall), "{:?}", e.vue.pills);
    assert!(e.totals.is_some() && !e.depth.is_empty());
    // the depth bars are on a grid 20 times finer than the walls' (BTC: 5 $), handed to the drawing with its step
    let m = a.feeds.murs.as_ref().unwrap();
    assert!((m.bars_perp.step - step / 20.0).abs() < 1e-9 && m.bars_perp.venues > 0);
    assert!((e.depth_step - step / 20.0).abs() < 1e-9);
    let titre = a.murs_titre().unwrap();
    assert!(
        titre.long.contains("4/5") && titre.long.contains("Hyperliquid") && titre.carnets == "Perp + Comptant",
        "{titre:?}"
    );
    assert_eq!(titre.court, "4/5");
    // recorded (a memory base in the tests), with the heartbeat
    let ev = a.feeds.murs_db.as_ref().unwrap().read("BTC|murs", 0, i64::MAX).unwrap();
    assert!(ev.iter().any(|e| e.kind == argos_core::murs::Kind::Show));
    assert!(ev.iter().any(|e| e.kind == argos_core::murs::Kind::Alive));
    // replay now: the same walls as live; replay before the recording: nothing, and the note
    // (the BINANCE tag and the split by exchange are live only: the recorded events do not carry them)
    let sans_tag = |v: argos_core::murs::Vue| argos_core::murs::Vue {
        pills: v
            .pills
            .into_iter()
            .map(|p| argos_core::murs::Pill {
                tag: false,
                par: [0.0; argos_core::murs::PAR_BOOKS],
                ..p
            })
            .collect(),
        ..v
    };
    assert!(e.vue.pills.iter().any(|p| p.tag && !p.spot), "Binance's wall tagged: {:?}", e.vue.pills);
    let live = sans_tag(e.vue.clone()).sorted();
    a.replay = Some(argos_render::replay::Replay::new(now_s()));
    a.murs_refresh();
    assert_eq!(sans_tag(a.murs_ecran().unwrap().vue).sorted(), live);
    assert!(!a.murs_titre().unwrap().long.contains(crate::actions::murs::PAS_DE_CARNET));
    a.replay = Some(argos_render::replay::Replay::new(a.chart.bars[0].time));
    let e = a.murs_ecran().unwrap();
    assert!(e.vue.pills.is_empty() && e.totals.is_none());
    assert!(a.murs_titre().unwrap().long.contains(crate::actions::murs::PAS_DE_CARNET));
    // not a crypto chart: nothing
    a.replay = None;
    a.chart.source = Source::File("essai.json".into());
    a.murs_refresh();
    assert!(a.murs_ecran().is_none() && a.murs_titre().is_none());
}

/// 🧱 Lot loin-1: a far photo gives two far walls per side to the drawing (live), none in replay nor with « 0 »; a
/// failed far photo keeps them, three in a row clear them; a 429 skips the next two far photos.
#[test]
fn argos_murs_murs_lointains() {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs_loin");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.ind.on.push(Ind::ArgosMurs);
    a.murs_refresh();
    let step = a.feeds.murs.as_ref().expect("Argos Murs started").step;
    let px = a.chart.bars.last().unwrap().close;
    let res: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> = Carnet::PERP.iter().map(|c| (*c, Ok(carnet_murs(px, step, &[])))).collect();
    // groups of 2 % of the price: 0.96 overlaps the 3 % range, 0.86 is the last within 15 %, 0.84 beyond
    let g = |k: f64| px * (1.0 - 0.02 * k);
    let h = |k: f64| px * (1.0 + 0.02 * k);
    let loin = argos_core::murs::VenueBook {
        venue: "Hyperliquid".into(),
        bids: vec![(g(2.0), 900.0), (g(3.0), 10.0), (g(4.0), 30.0), (g(5.0), 20.0), (g(7.0), 5.0), (g(8.0), 999.0)],
        asks: vec![(h(1.0), 900.0), (h(2.0), 800.0), (h(3.0), 4.0), (h(4.0), 12.0), (h(6.0), 9.0), (h(9.0), 999.0)],
    };
    a.murs_recu("BTC".into(), res.clone(), Some(Ok(loin.clone())), None);
    let e = a.murs_ecran().unwrap();
    let px_loin: Vec<f64> = e.loin.iter().map(|l| l.price).collect();
    assert_eq!(px_loin, vec![h(6.0), h(4.0), g(4.0), g(5.0)], "two per side, highest first");
    // a failed far photo keeps them; three in a row clear them
    a.murs_recu("BTC".into(), res.clone(), Some(Err("réseau".into())), None);
    assert_eq!(a.murs_ecran().unwrap().loin.len(), 4);
    a.murs_recu("BTC".into(), res.clone(), Some(Err("réseau".into())), None);
    a.murs_recu("BTC".into(), res.clone(), Some(Err("réseau".into())), None);
    assert!(a.murs_ecran().unwrap().loin.is_empty(), "three failures: cleared");
    a.murs_recu("BTC".into(), res.clone(), Some(Ok(loin.clone())), None);
    assert_eq!(a.murs_ecran().unwrap().loin.len(), 4);
    // a 429: the next two far photos are skipped
    a.murs_recu("BTC".into(), res.clone(), Some(Err("Hyperliquid : HTTP 429".into())), None);
    assert_eq!(a.feeds.murs.as_ref().unwrap().loin_pause, crate::actions::murs::PAUSE_429);
    // replay: none
    a.replay = Some(argos_render::replay::Replay::new(now_s()));
    assert!(a.murs_ecran().unwrap().loin.is_empty(), "replay: no far wall");
    a.replay = None;
    // « Murs lointains par côté » 0: none, and they are dropped at the next photo
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    let k = argos_render::murs::PARAMS.iter().position(|p| p.key == "loinN").unwrap();
    assert_eq!(v[k], 2.0, "two per side by default");
    v[k] = 0.0;
    a.ind.set.insert(Ind::ArgosMurs, v);
    assert!(a.murs_ecran().unwrap().loin.is_empty(), "0: none");
}

#[test]
fn argos_murs_jeu_par_defaut_et_choix_garde() {
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs_defauts");
    let mut a = app();
    a.ind.on.clear();
    a.indicateurs_par_defaut(true);
    assert_eq!(a.ind.on, vec![Ind::ArgosMurs, Ind::ArgosVague, Ind::Tvd], "fresh install: Jee's set");
    a.toggle_indicator(Ind::ArgosMurs);
    a.toggle_indicator(Ind::ArgosVague);
    a.indicateurs_par_defaut(false);
    assert_eq!(a.ind.on, vec![Ind::Tvd], "removed: they do not come back");
    // a profile that already existed and never had them: Argos Murs and Argos Vague once, nothing else forced
    dossier_a_soi("argos_murs_defauts_2");
    let mut b = app();
    b.ind.on = vec![Ind::Rsi];
    b.indicateurs_par_defaut(false);
    assert_eq!(b.ind.on, vec![Ind::Rsi, Ind::ArgosMurs, Ind::ArgosVague]);
    b.ind.on.retain(|i| !matches!(i, Ind::ArgosMurs | Ind::ArgosVague));
    b.indicateurs_par_defaut(false);
    assert_eq!(b.ind.on, vec![Ind::Rsi]);
    // a profile that already got Argos Murs before Argos Vague existed: only Argos Vague is added, once
    dossier_a_soi("argos_murs_defauts_3");
    let mut c = app();
    c.ind.on = vec![Ind::Tvd];
    crate::App::write_prefs(crate::actions::murs::DEFAUTS_FICHIER, &vec![Ind::ArgosMurs, Ind::Tvd]);
    c.indicateurs_par_defaut(false);
    assert_eq!(c.ind.on, vec![Ind::Tvd, Ind::ArgosVague], "removed Argos Murs stays removed");
    c.indicateurs_par_defaut(false);
    assert_eq!(c.ind.on, vec![Ind::Tvd, Ind::ArgosVague]);
}

/// 📱 Jee (21 Nov, list 2): the lists' menu on the phone by the finger has no grey row and no list rows (they are the
/// chips under it); the PC keeps both.
#[test]
fn menu_des_listes_au_doigt() {
    let mut a = cell_pilule();
    a.store = Some(Store::in_memory().unwrap());
    a.load_lists();
    let gris = |g: &[MenuGroup]| g.iter().flat_map(|g| &g.items).any(|i| !i.enabled);
    let listes = |g: &[MenuGroup]| g.iter().flat_map(|g| &g.items).any(|i| i.id == ID_TOUS);
    argos_render::actions::set_tactile(false);
    let pc = a.lists_menu();
    assert!(gris(&pc) && listes(&pc), "PC: grey rows and lists");
    argos_render::actions::set_tactile(true);
    let doigt = a.lists_menu();
    assert!(!gris(&doigt) && !listes(&doigt), "finger: {doigt:?}");
    assert_eq!(doigt[0].title, "Voir :");
    argos_render::actions::set_tactile(false);
}

/// 🤖 Lot 7 du Simulateur (« ok maquette lot 7 », Jee 21 nov.) : un trade démo fermé ouvre la carte « Argos AI · ton
/// trade ». Les indicateurs de la chart sont lus à l'ouverture de la position et gardés avec elle (sur une montée, le
/// RSI et le Supertrend vont dans le sens d'un achat) ; la carte dit l'entrée, le plan, la leçon et le rappel ; ✕ la
/// ferme ; un toucher sur la ligne de l'Historique la rouvre ; une sortie partielle n'en ouvre pas ; 🖥 elle est en
/// bas à gauche de la chart, à gauche du volet ; 📱 le toucher ferme le volet et la carte prend la largeur.
#[test]
fn simulateur_argos_ai_commente_le_trade() {
    use argos_render::indicators::Ind;
    use argos_render::simulateur::SimClick;
    use argos_render::text::Text;
    for phone in [false, true] {
        dossier_a_soi(if phone { "simulateur_argos_ai_cell" } else { "simulateur_argos_ai_pc" });
        let mut a = if phone { cell_pilule() } else { app() };
        a.top.prefs.bienvenue_vue = true;
        a.chart.source = Source::Live {
            exchange: argos_data::Exchange::Binance,
            base: "BTC".into(),
        };
        a.ind.on = vec![Ind::Rsi, Ind::Supertrend];
        a.ui.frames = 5;
        a.refresh_indicators();
        a.rebuild();
        a.toggle_simulateur();
        a.demo.sans_frais = true;
        let t = a.chart.bars[150].time;
        a.sim_replay_depuis(t, None);
        // achat au marché, SL 1 % : la lecture est gardée avec la position
        a.ui.sim.as_mut().unwrap().sl = 3;
        a.sim_place();
        assert_eq!(a.demo.positions.len(), 1, "{:?}", a.banner);
        let p = a.demo.positions[0].clone();
        let l = p.lecture.clone().expect("lu à l'ouverture");
        assert!(l.pour.iter().any(|n| n.starts_with(Ind::Rsi.name())), "{l:?}");
        assert!(l.contre.is_empty() && l.pas_lue.is_empty(), "{l:?}");
        // ✂ la moitié à la main : une sortie partielle, pas de carte
        a.sim_faire(SimClick::Moitie(p.id));
        assert!(a.demo.history.last().unwrap().partiel);
        assert_eq!(a.ui.avis, None, "une sortie partielle n'ouvre pas de carte");
        // la suite fermée à la main : la carte du trade entier
        a.replay_step(1);
        a.sim_faire(SimClick::CloseTrade(p.id));
        let k = a.demo.history.len() - 1;
        assert_eq!(a.avis_index(), Some(k));
        let text = Text::default();
        let (avis, r) = a.avis_place(&text).expect("la carte s'ouvre seule");
        assert!(avis.entree.pastille.ends_with(" dans ton sens"), "{avis:?}");
        assert_eq!(avis.plan.pastille, "En partie", "{avis:?}");
        assert!(avis.plan.texte.starts_with("Sorti à la main"), "{avis:?}");
        assert_eq!((avis.sortie.as_str(), avis.rappel.as_str()), ("fermé à la main", argos_ai::TRADE_REMINDER));
        let (pm, _) = a.avis_draw(&text).expect("la carte se dessine");
        assert!(argos_render::px::lw(&pm) > 300.0);
        // sa place : sur la chart, au-dessus de son bas
        let (x0, y0) = (a.rw(), a.top_h());
        assert!(r.x >= x0 && r.y >= y0 && r.y + r.h <= y0 + a.chart.viewport.plot_h(), "{r:?}");
        // ⏪ la barre du replay reste visible et à portée : la carte ne la couvre pas (📱 elle se pose au-dessus)
        let (zone_w, ph) = a.replay_zone();
        let b = argos_render::replay::rect(zone_w, ph);
        let b = Rect::new(b.x + x0, b.y + y0, b.w, b.h);
        assert!(r.y + r.h <= b.y || r.x + r.w <= b.x || r.x >= b.x + b.w, "{r:?} sur la barre du replay {b:?}");
        if phone {
            assert_eq!((r.x, r.w), (16.0, a.win_w() - 32.0), "📱 toute la largeur moins 16 de chaque côté");
            assert!((r.y + r.h - (b.y - 8.0)).abs() < 1e-9, "📱 juste au-dessus de la barre du replay");
        } else {
            assert_eq!(r.x, x0 + 16.0, "🖥 en bas à gauche de la chart");
            assert!(r.x + r.w <= a.sim_rect().x, "🖥 à gauche du volet du Simulateur");
        }
        // 📱 le volet couvre l'écran (la carte est dessous) : elle se voit et se touche une fois revenu sur la chart
        if phone {
            a.toggle_simulateur();
        }
        // un clic sur la carte ne passe pas à la chart ; ✕ la ferme
        assert!(a.avis_sous(r.x + 40.0, r.y + 60.0));
        clic(&mut a, r.x + r.w - 17.0 - 9.0, r.y + 15.0 + 15.0);
        assert_eq!(a.ui.avis, None, "✕ la ferme");
        assert!(a.avis_draw(&text).is_none());
        // un toucher sur la ligne de l'Historique la rouvre (📱 le volet se ferme pour la montrer) ; encore : fermée
        if a.ui.sim.is_none() {
            a.toggle_simulateur();
        }
        a.sim_faire(SimClick::Avis(k));
        assert_eq!(a.avis_index(), Some(k));
        assert_eq!(a.ui.sim.is_none(), phone, "📱 le volet se ferme ; 🖥 il reste");
        a.sim_faire(SimClick::Avis(k));
        assert_eq!(a.ui.avis, None);
        // la sortie partielle n'a pas de carte ; un autre compte démo (ou une remise à zéro) : plus de carte
        a.sim_faire(SimClick::Avis(k - 1));
        assert_eq!(a.ui.avis, None);
        a.sim_faire(SimClick::Avis(k));
        a.demo.reset();
        assert!(a.avis_index().is_none() && a.avis_draw(&text).is_none());
    }
}

/// 🤖 Lot 7 : la carte « Argos AI · ton trade » est au-dessus de la chart, sous les menus et les fenêtres. Un menu
/// ouvert prend le clic avant elle ; un appui sur elle ne passe pas à la chart (ni son relâcher, ni le clic droit) ; la
/// ligne SL d'une position, attrapée sur la chart, se lâche aussi sur elle ; la flèche dessus, la main sur ✕, pas de
/// réticule.
#[test]
fn carte_argos_ai_sous_les_menus_au_dessus_de_la_chart() {
    use crate::types::Curseur::Icone;
    use argos_render::ordres::Prise;
    use winit::window::CursorIcon;
    dossier_a_soi("carte-argos-ai-couches");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.watch.panels.trading.guardian = false;
    a.watch.panels.trading.risk = 100.0;
    a.watch.panels.trading.tab = 0;
    a.watch.panels.trading.sl_pips = 4.0;
    // un achat fermé à la main : sa carte ; un autre achat ouvert, pour glisser son stop
    a.place_demo_order();
    let id = a.demo.positions[0].id;
    a.ordre_clic(Prise::Fermer(id));
    a.place_demo_order();
    let p = a.demo.positions[0].clone();
    let _ = a.to_chart();
    let (_, r) = a.avis_place(&argos_render::text::Text::default()).expect("la carte du trade fermé");
    let (rw, th) = (a.rw(), a.top_h());
    let (dans, croix) = ((r.x + 60.0, r.y + 60.0), (r.x + r.w - 26.0, r.y + 30.0));
    // la flèche sur la carte, la main sur ✕ ; pas de réticule sous elle
    a.mouse_moved(PhysicalPosition::new(dans.0, dans.1));
    assert_eq!(a.curseur_chart(dans.0 - rw, dans.1 - th), Icone(CursorIcon::Default));
    assert_eq!(a.curseur_chart(croix.0 - rw, croix.1 - th), Icone(CursorIcon::Pointer));
    assert!(a.to_chart().crosshair.is_none(), "pas de réticule sous la carte");
    // le clic droit sur elle : aucun menu de la chart
    a.right_click();
    assert!(!a.ui.context.is_open() && a.ui.list.is_none(), "pas de menu sous la carte");
    // le menu de la chart ouvert : il prend le clic (dehors, il se ferme), la carte reste
    a.mouse_moved(PhysicalPosition::new(rw + a.chart.viewport.plot_w() * 0.6, th + 40.0));
    a.right_click();
    assert!(a.ui.context.is_open(), "le menu de la chart");
    clic(&mut a, croix.0, croix.1);
    assert!(!a.ui.context.is_open() && a.ui.avis.is_some(), "le clic a fermé le menu, pas la carte");
    // la ligne SL attrapée sur la chart (à droite de la carte), lâchée sur la carte : posée à ce prix
    let sc = a.scale_now().expect("une échelle");
    let (x_ligne, y_sl) = (rw + a.chart.viewport.plot_w() * 0.7, th + sc.y_of(p.stop.unwrap()));
    assert!(x_ligne > r.x + r.w);
    a.mouse_moved(PhysicalPosition::new(x_ligne, y_sl));
    a.left_button(ElementState::Pressed);
    assert_eq!(a.pointer.order_drag.map(|(q, _)| q), Some(Prise::Sl(p.id)), "la ligne SL attrapée");
    a.mouse_moved(PhysicalPosition::new(dans.0, dans.1));
    a.left_button(ElementState::Released);
    assert!(a.pointer.order_drag.is_none(), "lâchée sur la carte");
    let stop = a.demo.positions[0].stop.unwrap();
    assert!((stop - sc.price_of(dans.1 - th)).abs() < 1e-6 && stop < p.stop.unwrap(), "{stop} {p:?}");
    assert_eq!(a.demo.positions[0].retouches.stop_eloigne, Some(stop));
    // un appui sur la carte (hors ✕) : rien dessous ne bouge, et son relâcher est avalé
    let vue = a.chart.viewport;
    a.mouse_moved(PhysicalPosition::new(dans.0, dans.1));
    a.left_button(ElementState::Pressed);
    a.mouse_moved(PhysicalPosition::new(dans.0 + 80.0, dans.1 - 40.0));
    a.left_button(ElementState::Released);
    assert!(a.chart.viewport == vue && a.pointer.order_drag.is_none() && a.ui.avis.is_some());
    // ✕ : fermée ; ensuite la chart reçoit tout de nouveau (le réticule revient)
    clic(&mut a, croix.0, croix.1);
    assert!(a.ui.avis.is_none());
    a.mouse_moved(PhysicalPosition::new(dans.0, dans.1));
    assert!(a.to_chart().crosshair.is_some());
}

/// 📅 Lot 8: a demo account with a trade of 40 days ago (made then: a month before), two of now (a win at the target,
/// a loss at the stop) and a short of the replay (its time: the market's, back then); the Simulator open, its
/// « 📅 Rapport du mois » page open, the screen laid out at `w` × `h`.
fn month_report_app(phone: bool, w: u32, h: u32) -> App {
    use argos_render::simulateur::SimClick;
    let mut a = if phone { cell_pilule() } else { app() };
    a.top.prefs.bienvenue_vue = true;
    a.toggle_simulateur();
    a.demo.sans_frais = true;
    let order = |base: &str, side| argos_core::portfolio::Order {
        base: base.into(),
        side,
        qty: 1.0,
        price: 100.0,
        stop: Some(if side == argos_core::Side::Long { 90.0 } else { 110.0 }),
        target: None,
        couts: None,
        perp: None,
    };
    let now = now_s();
    let id = a.demo.open(order("ETH", argos_core::Side::Long), now - 40 * 86_400 - 600).unwrap();
    a.demo.close(id, 105.0, now - 40 * 86_400, "objectif").unwrap();
    for (base, exit, reason) in [("BTC", 120.0, "objectif"), ("SOL", 90.0, "stop")] {
        let id = a.demo.open(order(base, argos_core::Side::Long), now - 600).unwrap();
        let c = a.demo.close(id, exit, now, reason).unwrap();
        a.demo_closed(&[c]);
    }
    let t0 = a.chart.bars[150].time;
    let id = a.demo.open(order("BTC", argos_core::Side::Short), t0).unwrap();
    let c = a.demo.close(id, 95.0, t0 + 1800, "fin du replay").unwrap();
    a.demo_closed(&[c]);
    assert!(a.ui.avis.is_some(), "the last trade's card");
    a.sim_faire(SimClick::MonthReport);
    if phone {
        // (the image is made with the phone's layout)
        a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    }
    let _ = image_ecran(&mut a, w, h);
    assert_eq!(a.top.phone, phone);
    a
}

/// 📅 The first of `points` (page coordinates) where a click on the page does what `want` says (given what the click
/// asks and the page after it), in window coordinates.
fn month_at(
    a: &App,
    points: impl IntoIterator<Item = (f64, f64)>,
    want: impl Fn(argos_render::month::MonthClick, &argos_render::month::MonthPage) -> bool,
) -> Option<(f64, f64)> {
    let text = argos_render::text::Text::default();
    let (page, v, r, f) = a.month_place()?;
    points
        .into_iter()
        .find(|&(x, y)| {
            let mut p = page.clone();
            let c = p.click(&v, &text, f, x, y);
            want(c, &p)
        })
        .map(|(x, y)| (r.x + x, r.y + y))
}

/// 📅 The first point (window) on the vertical line `x` of the page (its coordinates) where a click does what `want`
/// says, the page scrolled down to show it; the page is left at that scroll.
fn month_point(a: &mut App, x: f64, want: impl Fn(argos_render::month::MonthClick) -> bool) -> (f64, f64) {
    for _ in 0..10 {
        let (page, _, r, f) = a.month_place().expect("the page shows");
        if let Some(p) = month_at(a, (0..f.h as i32).step_by(3).map(|y| (x, f64::from(y))), |c, _| want(c)) {
            return p;
        }
        // not in view: further down
        a.mouse_moved(PhysicalPosition::new(r.x + x, r.y + 20.0));
        if !a.month_wheel(f.h * 0.7) || a.ui.sim.as_ref().and_then(|s| s.month.as_ref()).map(|p| p.scroll) == Some(page.scroll) {
            break;
        }
    }
    panic!("nothing found on the page at x = {x}");
}

/// 📅 Lot 8 of the Simulator (Jee's « ok maquette lot 8 », 21 Nov): « 📅 Rapport du mois » opens the month page of the
/// demo account: 🖥 over the chart, left of the panel, 40 under the top bar, as high as its content; 📱 the whole
/// screen. Its figures are the account's: a replay trade counts in the month it was really made (its real time is kept
/// at the exit, with its market date), a trade of 40 days ago in a month before; ‹ goes to it and › comes back. A click
/// on the page never reaches the chart (the arrow, no crosshair, no menu); the wheel and the finger scroll it; a trade's
/// round opens its lot 7 card (the page closes to show it); ✕, Escape and the button close it.
#[test]
fn simulator_month_report() {
    use crate::types::Curseur::Icone;
    use argos_render::month::MonthClick;
    use argos_render::simulateur::SimClick;
    use winit::window::CursorIcon;
    for phone in [false, true] {
        dossier_a_soi(if phone { "month_report_cell" } else { "month_report_pc" });
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        let mut a = month_report_app(phone, w, h);
        let now = now_s();
        let hist = &a.demo.history;
        assert!(hist[3].replay() && hist[3].real_close() >= now - 5, "{:?}", hist[3]);
        assert!(!hist[0].replay() && !hist[1].replay() && !hist[2].replay(), "the others keep their time");
        assert!(a.ui.avis.is_none(), "the card the page would cover closes");
        // the month: the 3 trades of now, the replay one with its market date; ‹ the month of 40 days ago, › none
        let ny = |t: i64| argos_core::month::date_of(t, argos_render::settings::tz_offset("America/New_York", t).unwrap());
        let ((y, m, _), (y0, m0, _)) = (ny(now), ny(now - 40 * 86_400));
        let (pg, v, r, f) = a.month_place().expect("the page shows");
        assert_eq!((v.year, v.month, v.trades.len(), v.zone.as_str()), (y, m, 3, "heure de New York"));
        assert_eq!((v.prev, v.next, v.account.as_str()), (Some((y0, m0)), None, a.demo.name.as_str()));
        assert_eq!(v.trades.iter().filter(|t| t.market.is_some()).count(), 1, "{:?}", v.trades);
        assert_eq!((v.winners, v.r_avg.map(|x| (x * 100.0).round())), (2, Some(50.0)), "+2 R, −1 R, +0,5 R");
        assert!(a.month_draw(&argos_render::text::Text::default()).is_some());
        // its place
        if phone {
            assert_eq!((r.x, r.y, r.w, r.h, f.sheet), (0.0, 0.0, 402.0, 874.0, true));
        } else {
            // 🖥 left of the Simulator (in the Trading volet), as high as its content: 40 under the top bar, higher
            // (down to 16) before it scrolls
            let s = a.sim_rect();
            assert!(
                a.sim_en_volet() && r.x >= a.rw() + 16.0 && r.x + r.w <= s.x - 16.0 && r.w <= 960.0,
                "{r:?} {s:?}"
            );
            assert!(r.y >= a.top_h() + 16.0 && r.y <= a.top_h() + 40.0 && r.y + r.h <= h as f64 - 16.0, "{r:?}");
            assert_eq!(r.h, pg.height(&v, &argos_render::text::Text::default(), f), "as high as its content");
        }
        // a press on the page (off its buttons) never reaches the chart: nothing moves, no crosshair, no menu
        let off = (r.x + r.w / 2.0, r.y + if phone { 150.0 } else { 160.0 });
        assert_eq!(a.month_hand(off.0, off.1), Some(false), "the arrow off its buttons");
        if !phone {
            assert_eq!(a.curseur_chart(off.0 - a.rw(), off.1 - a.top_h()), Icone(CursorIcon::Default));
        }
        let vue = a.chart.viewport;
        a.mouse_moved(PhysicalPosition::new(off.0, off.1));
        a.left_button(ElementState::Pressed);
        a.mouse_moved(PhysicalPosition::new(off.0 + 60.0, off.1 + 30.0));
        a.left_button(ElementState::Released);
        assert!(a.chart.viewport == vue && a.pointer.drag.is_none(), "the chart did not move");
        assert!(a.to_chart().crosshair.is_none(), "no crosshair under the page");
        a.top.prefs.estomper = true;
        assert!(phone || a.pointer.mouse.is_some());
        assert!(!a.barres_estompees(), "🖥 the bars do not fade while the mouse reads the page");
        a.mouse_moved(PhysicalPosition::new(off.0, off.1));
        a.right_click();
        assert!(!a.ui.context.is_open() && a.ui.list.is_none(), "no menu of the chart under the page");
        // ‹ the month before, › back (📱 the short month in its pill: ‹ at 16 + 22, 📱 75 / 🖥 42 down)
        let (lx, ly) = if phone { (38.0, 75.0) } else { (273.5, 42.0) };
        clic(&mut a, r.x + lx, r.y + ly);
        let (page, v, ..) = a.month_place().unwrap();
        assert_eq!((page.month, v.trades.len(), v.next), (Some((y0, m0)), 1, Some((y, m))), "‹");
        let row = (0..a.month_place().unwrap().3.w as i32).step_by(3).map(|x| (f64::from(x), ly));
        let rx = month_at(&a, row, |c, p| c == MonthClick::Changed && p.month == Some((y, m))).expect("› in the header");
        clic(&mut a, rx.0, rx.1);
        assert_eq!(a.month_place().unwrap().0.month, Some((y, m)), "›");
        // « Tous les détails » unfolds (the wheel brings it into view on the phone)
        let p = month_point(&mut a, 120.0, |c| c == MonthClick::Changed);
        let folded = a.month_place().unwrap().0;
        assert!(!folded.details);
        clic(&mut a, p.0, p.1);
        assert!(a.month_place().unwrap().0.details, "« Tous les détails » unfolded");
        // the wheel (or the finger) scrolls the page, never the chart under it
        let (_, _, r, _) = a.month_place().unwrap();
        a.mouse_moved(PhysicalPosition::new(r.x + 200.0, r.y + 100.0));
        let before = a.month_place().unwrap().0.scroll;
        a.mouse_wheel(winit::event::MouseScrollDelta::LineDelta(0.0, -3.0));
        let after = a.month_place().unwrap().0.scroll;
        assert!(after > before, "the wheel scrolls it: {before} → {after}");
        assert!(a.chart.viewport == vue, "not the chart");
        // a trade's round: its lot 7 card, the page closes to show it (📱 the panel too)
        let k = month_point(&mut a, if phone { 39.0 } else { 48.0 }, |c| matches!(c, MonthClick::Card(_)));
        clic(&mut a, k.0, k.1);
        assert!(a.ui.avis.is_some(), "the card of the trade");
        assert!(a.ui.sim.as_ref().is_none_or(|s| s.month.is_none()), "the page closed");
        assert_eq!(a.ui.sim.is_none(), phone, "📱 the panel closes, 🖥 it stays");
        // the button opens it again, ✕ closes it; Escape closes it before the panel
        if a.ui.sim.is_none() {
            a.toggle_simulateur();
        }
        a.sim_faire(SimClick::MonthReport);
        let (_, _, r, f) = a.month_place().unwrap();
        let x = if phone {
            (r.x + f.w - 28.0, r.y + 30.0)
        } else {
            (r.x + f.w - 35.4, r.y + 42.0)
        };
        assert_eq!(a.month_hand(x.0, x.1), Some(true), "the hand on ✕");
        clic(&mut a, x.0, x.1);
        assert!(a.month_place().is_none() && a.ui.sim.is_some(), "✕ closes the page, the panel stays");
        a.sim_faire(SimClick::MonthReport);
        a.sim_echap();
        assert!(a.month_place().is_none() && a.ui.sim.is_some(), "Escape: the page first");
        a.sim_echap();
        assert!(a.ui.sim.is_none(), "then the panel");
    }
}

/// 📅 Lot 8: on the phone, a real finger: a slide scrolls the page (no click on what it slides over), a tap on a button
/// works, a swipe to the right closes the page (the panel stays); 📱 without the single pill, the bars' handles under
/// the page do nothing.
#[test]
fn simulator_month_report_by_finger() {
    use argos_render::month::MonthClick;
    dossier_a_soi("month_report_finger");
    let mut a = month_report_app(true, 402, 874);
    let (_, _, r, _) = a.month_place().unwrap();
    // a finger slides up the page: it scrolls, nothing is clicked
    use winit::event::TouchPhase;
    let (x, y0) = (r.x + 200.0, r.y + 600.0);
    a.touch(3, TouchPhase::Started, (x, y0));
    for k in 1..=12 {
        a.touch(3, TouchPhase::Moved, (x, y0 - 30.0 * k as f64));
    }
    a.touch(3, TouchPhase::Ended, (x, y0 - 360.0));
    let p = a.month_place().unwrap().0;
    assert!(p.scroll > 100.0 && !p.details && p.month.is_none(), "{p:?}");
    // a tap on « Tous les détails »
    let d = month_point(&mut a, 120.0, |c| c == MonthClick::Changed);
    toucher(&mut a, d.0, d.1);
    assert!(a.month_place().unwrap().0.details, "a tap unfolds the details");
    // a swipe to the right closes the page; the panel stays
    a.touch(4, TouchPhase::Started, (60.0, 400.0));
    for k in 1..=8 {
        a.touch(4, TouchPhase::Moved, (60.0 + 35.0 * k as f64, 400.0));
    }
    a.touch(4, TouchPhase::Ended, (340.0, 400.0));
    assert!(a.month_place().is_none() && a.ui.sim.is_some(), "swiped away, the panel stays");
    // 📱 without the single pill (a screen of 600 or more): a touch on a bar's handle, under the page, goes to the page
    let mut b = cell_390();
    b.toggle_simulateur();
    b.sim_faire(argos_render::simulateur::SimClick::MonthReport);
    b.ui.form.kind = argos_render::formfactor::Kind::Phone;
    let _ = image_ecran(&mut b, 640, 900);
    assert!(!b.pilule() && b.month_place().is_some());
    let caches = (b.top.prefs.cell_haut_cache, b.top.prefs.cell_bas_cache);
    for r in [b.poignee_haut(), b.poignee_bas()] {
        toucher(&mut b, r.cx(), r.cy());
    }
    assert_eq!((b.top.prefs.cell_haut_cache, b.top.prefs.cell_bas_cache), caches, "the handles under the page");
    assert!(b.month_place().is_some(), "the page stays");
}

/// 📄 Lot 8: « Imprimer en PDF » on the computer: the month's light page goes to Downloads (a whole document that
/// prints itself once open), with the month's figures; the banner says where.
#[test]
fn simulator_month_report_printed() {
    use argos_render::month::MonthClick;
    dossier_a_soi("month_report_print");
    let mut a = month_report_app(false, 1440, 900);
    let (page, v, _, f) = a.month_place().unwrap();
    let p = month_point(&mut a, f.w - 120.0, |c| c == MonthClick::Print);
    clic(&mut a, p.0, p.1);
    let name = format!("argos-rapport-du-mois-{}-{:02}.html", v.year, v.month);
    let file = crate::platform::dossier_telechargements().join(&name);
    let html = std::fs::read_to_string(&file).expect("the page in Downloads");
    assert!(html.starts_with("<!doctype html>") && html.contains("print()"), "a document that prints itself");
    assert!(html.contains("Rapport du mois") && html.contains(&a.demo.name), "{html:.300}");
    assert!(a.banner.as_deref().is_some_and(|b| b.starts_with("📄 Rapport du mois dans ")), "{:?}", a.banner);
    assert_eq!(a.month_place().unwrap().0, page, "the page stays open");
    let _ = std::fs::remove_file(file);
}

/// 📅 Lot 8: the mock-up's month in a 1 440 × 900 window, whose left bar takes 55 of the chart's room and the Trading
/// volet (the Simulator in its 🧪 tab) 476: the side margins narrow (16) before the page does, the top one (40 → 16)
/// before it scrolls, so the page shows whole, nothing to scroll (the mock-up's). A wide window: the mock-up's 960,
/// at least 30 from the left bar and from the panel, 40 under the top bar.
#[test]
fn simulator_month_report_on_a_laptop() {
    dossier_a_soi("month_report_laptop");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.demo.start = argos_core::Money::from_cents(10_120_460);
    a.demo.history = mock_up_month();
    a.toggle_simulateur();
    a.sim_faire(argos_render::simulateur::SimClick::MonthReport);
    let _ = image_ecran(&mut a, 1440, 900);
    let (pg, v, r, f) = a.month_place().expect("the page shows");
    let s = a.sim_rect();
    assert_eq!((v.year, v.month, v.trades.len()), (2026, 11, 12));
    let near = |x: f64, y: f64| (x - y).abs() < 0.01;
    assert!(a.sim_en_volet() && near(r.x - a.rw(), 16.0) && near(s.x - r.x - r.w, 16.0), "{r:?} {s:?}");
    let tout = pg.height(&v, &argos_render::text::Text::default(), f);
    assert!(
        r.h == tout && r.y >= a.top_h() + 16.0 && r.y + r.h <= 900.0 - 16.0,
        "the whole page, nothing to scroll: {r:?}"
    );
    let _ = image_ecran(&mut a, 1920, 1080);
    let (_, _, r, _) = a.month_place().unwrap();
    let s = a.sim_rect();
    assert!(r.w == 960.0 && r.x - a.rw() >= 30.0 && s.x - r.x - r.w >= 30.0, "{r:?} {s:?}");
    assert_eq!(r.y, a.top_h() + 40.0, "room enough: the mock-up's 40 under the top bar");
}

/// 📸 Lot 7 : l'écran entier avec la carte « Argos AI · ton trade », pour le comparer à la maquette (PC 1440 × 900,
/// cell 402 × 874) : seulement si `ARGOS_CAPTURES` donne un dossier. 🖥 Un achat dont le stop est éloigné à la main,
/// sorti au stop (« Pas suivi », la leçon de l'Académie) ; 📱 un achat sorti à l'objectif (plan suivi).
#[test]
fn captures_lot7_pour_jee() {
    use argos_render::indicators::Ind;
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    for phone in [false, true] {
        dossier_a_soi(if phone { "captures_lot7_cell" } else { "captures_lot7_pc" });
        let mut a = if phone { cell_pilule() } else { app() };
        if phone {
            // (l'image se fait avec la mise en page du téléphone)
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        }
        a.top.prefs.bienvenue_vue = true;
        // 🖥 une montée puis une chute ; 📱 une montée
        let prix = |i: i64| {
            if phone || i <= 155 {
                100.0 + i as f64
            } else {
                255.0 - 2.0 * (i - 155) as f64
            }
        };
        a.chart.raw = (0..200)
            .map(|i| Bar {
                time: START + i * 1800,
                open: prix(i),
                high: prix(i).max(prix(i + 1)) + 1.0,
                low: prix(i).min(prix(i + 1)) - 1.0,
                close: prix(i + 1),
                volume: 10.0,
            })
            .collect();
        a.chart.source = Source::Live {
            exchange: argos_data::Exchange::Binance,
            base: "BTC".into(),
        };
        a.ind.on = vec![Ind::Rsi, Ind::Supertrend, Ind::Macd];
        a.ui.frames = 5;
        a.refresh_indicators();
        a.rebuild();
        a.toggle_simulateur();
        a.demo.sans_frais = true;
        let t = a.chart.bars[150].time;
        a.sim_replay_depuis(t, None);
        a.ui.sim.as_mut().unwrap().sl = 3;
        a.sim_place();
        let p = a.demo.positions[0].clone();
        if !phone {
            let loin = p.entry - (p.entry - p.stop.unwrap()) * 1.62;
            a.demo.set_level(p.id, true, loin).unwrap();
        }
        for _ in 0..40 {
            if a.demo.positions.is_empty() {
                break;
            }
            a.replay_step(1);
        }
        assert!(a.avis_index().is_some(), "{:?}", a.demo.history.last());
        if phone {
            a.toggle_simulateur();
        }
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        let (img, _) = image_ecran(&mut a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/lot7-ecran-{}.png", if phone { "cell" } else { "pc" })).unwrap();
    }
}

/// 📅 The mock-up's month (lot 8): 12 trades of November 2026, in New York time, 1 000 $ of risk each (day, time, symbol,
/// long, exit, R, costs, plan: S followed, P partly, X stop moved away, N not judged; ⏪ the market's date of a replay
/// trade), from a balance of 101 204,60 $ on the 1st.
fn mock_up_month() -> Vec<argos_core::portfolio::ClosedTrade> {
    use argos_core::portfolio::{ClosedTrade, Plan, Retouches};
    use argos_core::{Money, Side};
    const TRADES: [(i64, i64, &str, bool, &str, f64, i64, char, Option<i64>); 12] = [
        (2, 1042, "BTC", true, "objectif", 2.00, 5_820, 'S', None),
        (3, 1405, "ETH", false, "stop", -1.00, 4_130, 'S', None),
        (5, 958, "SOL", true, "objectif", 2.00, 3_560, 'S', None),
        (6, 1120, "BTC", true, "stop", -1.62, 9_620, 'X', None),
        (9, 1531, "BTC", false, "fermé à la main", 0.80, 5_210, 'P', None),
        (10, 1012, "ETH", true, "objectif", 2.00, 6_190, 'S', None),
        (12, 1347, "BTC", true, "break-even", 0.00, 4_810, 'S', None),
        // ⏪ 14 March 2025 and 2 April 2025, 15:00 UTC
        (13, 2015, "SOL", false, "stop", -1.00, 3_000, 'S', Some(1_741_964_400)),
        (16, 1005, "BTC", true, "stop", -1.40, 7_010, 'X', None),
        (18, 944, "SOL", true, "objectif", 2.50, 4_480, 'S', None),
        (19, 1602, "ETH", false, "fermé à la main", -0.45, 3_320, 'P', None),
        (20, 2130, "BTC", true, "fin du replay", 0.35, 2_240, 'N', Some(1_743_606_000)),
    ];
    // (November 1st 2026, 00:00 in New York: 05:00 UTC)
    let nov_1 = 1_793_491_200 + 5 * 3600;
    // the costs: 151,60 $ of spread and slippage over all the trades, 30 $ of funding on the replay SOL
    let mut slippage_left = 15_160;
    TRADES
        .iter()
        .enumerate()
        .map(|(k, &(day, hhmm, base, long, reason, r, costs, plan, market))| {
            let t = nov_1 + (day - 1) * 86_400 + hhmm / 100 * 3600 + hhmm % 100 * 60;
            let s = if long { 1.0 } else { -1.0 };
            let (entry, stop0, target0) = (100.0, 100.0 - 10.0 * s, 100.0 + 20.0 * s);
            let funding = if market == Some(1_741_964_400) { 3_000 } else { 0 };
            let slippage = if k == TRADES.len() - 1 {
                slippage_left
            } else {
                (costs - funding) * 2553 / 10_000
            };
            slippage_left -= slippage;
            let closed = market.unwrap_or(t);
            ClosedTrade {
                base: base.into(),
                side: if long { Side::Long } else { Side::Short },
                qty: 100.0,
                entry,
                exit: entry + r * 10.0 * s,
                pnl: Money::from_f64(r * 1000.0),
                fees: Money::from_cents(costs - funding),
                opened_at: closed - 3 * 3600,
                closed_at: closed,
                reason: reason.into(),
                risk: Money::from_f64(1000.0),
                partiel: false,
                glissement: Money::from_cents(slippage),
                funding: Money::from_cents(funding),
                levier: None,
                plan: Some(Plan {
                    stop0: Some(stop0),
                    target0: Some(target0),
                    stop: Some(stop0),
                    target: Some(target0),
                    retouches: Retouches {
                        stop_eloigne: (plan == 'X').then_some(entry + r * 10.0 * s),
                        ..Retouches::default()
                    },
                    lecture: None,
                }),
                real_closed_at: market.map(|_| t),
            }
        })
        .collect()
}

/// 📸 Lot 8: the whole screen with the « Rapport du mois » page of the mock-up's month, to compare with the mock-up
/// (🖥 1440 × 900: folded, then « Tous les détails » unfolded; 📱 402 × 874: its top, then its bottom), and its printed
/// page (HTML, to make into an image): only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_lot8_pour_jee() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    for phone in [false, true] {
        dossier_a_soi(if phone { "captures_lot8_cell" } else { "captures_lot8_pc" });
        let mut a = if phone { cell_pilule() } else { app() };
        if phone {
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        }
        a.top.prefs.bienvenue_vue = true;
        a.demo.name = "Démo Argos 100K".into();
        a.demo.start = argos_core::Money::from_cents(10_120_460);
        a.demo.history = mock_up_month();
        a.demo.balance = a.demo.start + a.demo.history.iter().map(|c| c.net()).sum::<argos_core::Money>();
        // (the panel's curve, for its « Drawdown max »)
        let mut balance = a.demo.start;
        a.demo.curve = a
            .demo
            .history
            .iter()
            .map(|c| {
                balance += c.net();
                (c.real_close(), balance)
            })
            .collect();
        a.toggle_simulateur();
        a.sim_faire(argos_render::simulateur::SimClick::MonthReport);
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        if !phone {
            // (🖥 the panel scrolled down to « 📅 Rapport du mois » and the history, as the mock-up shows it)
            let _ = image_ecran(&mut a, w, h);
            let s = a.sim_rect();
            a.mouse_moved(PhysicalPosition::new(s.x + 100.0, s.y + 300.0));
            a.sim_wheel(10_000.0);
        }
        let save = |a: &mut App, name: &str| {
            let (img, _) = image_ecran(a, w, h);
            let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
            for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
                *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
            }
            pm.save_png(format!("{dir}/lot8-{name}.png")).unwrap();
        };
        save(&mut a, if phone { "cell-haut" } else { "pc" });
        let page = a.ui.sim.as_mut().and_then(|s| s.month.as_mut()).unwrap();
        if phone {
            page.scroll = 10_000.0;
            save(&mut a, "cell-bas");
        } else {
            page.details = true;
            let (_, _, r, f) = a.month_place().unwrap();
            // (the unfolded part in view)
            a.mouse_moved(PhysicalPosition::new(r.x + 100.0, r.y + 100.0));
            a.month_wheel(f.h * 0.75);
            save(&mut a, "pc-details");
            let (_, v, ..) = a.month_place().unwrap();
            std::fs::write(
                format!("{dir}/lot8-imprime.html"),
                argos_render::month::printable_document(&v, "dim. 22 nov. 2026 à 10:00"),
            )
            .unwrap();
        }
    }
}

/// 🎫 The module 31 scene (mock-up): Binance BTC perpetual, 15 min, « Démo Argos 100K », the Trading ticket with a
/// Limit buy at 60 057,9 under the price, 5 % of the buying power, TP/SL ticked (TP 61 259,1 above the candles); 🖥
/// 1440 × 900 or 📱 402 × 874 with its sheet. Each test calls [`dossier_a_soi`] first.
fn scene_ticket(phone: bool) -> (App, u32, u32) {
    let mut a = if phone { cell_pilule() } else { pc_avec_volet(wl::Tool::Trading) };
    if phone {
        a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        // (the phone's first image, which arrives on the chart, already passed: the sheet then opened)
        a.ui.depart_vu = true;
        (a.watch.prefs.open, a.watch.prefs.panel) = (true, wl::Tool::Trading);
    }
    a.top.prefs.bienvenue_vue = true;
    // 15 min bars: up from 59 200 to 60 600, down to 59 500, up again to 60 360 (the mock-up's chart)
    let prix = |i: i64| {
        let x = i as f64;
        let base = if i < 70 {
            59_200.0 + 1_400.0 * x / 70.0
        } else if i < 140 {
            60_600.0 - 1_100.0 * (x - 70.0) / 70.0
        } else {
            59_500.0 + 860.0 * (x - 140.0) / 59.0
        };
        ((base + 90.0 * (x * 1.7).sin()) * 10.0).round() / 10.0
    };
    a.chart.raw = (0..200)
        .map(|i| Bar {
            time: START + i * 900,
            open: prix(i),
            high: prix(i).max(prix(i + 1)) + 35.0,
            low: prix(i).min(prix(i + 1)) - 35.0,
            close: prix(i + 1),
            volume: 10.0,
        })
        .collect();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    (a.chart.interval, a.chart.file_interval) = (Interval(900), Interval(900));
    a.title = argos_data::Exchange::Binance.chart_title("BTC", "15m");
    a.ind.on.clear();
    a.ui.frames = 5;
    a.refresh_indicators();
    a.rebuild();
    // (the price scale fitted to these bars)
    a.start_free();
    a.demo = argos_core::portfolio::Portfolio::default();
    let t = &mut a.watch.panels.trading;
    (t.guardian, t.tab, t.px, t.tpsl) = (false, 1, 60_057.9, true);
    // (the buying power comes with the first image; then 5 % of it)
    let (w, h) = if phone { (402, 874) } else { (1440, 900) };
    let _ = image_ecran(&mut a, w, h);
    a.watch.panels.trading.set_pct(5.0);
    (a, w, h)
}

/// 📸 Module 31: the whole screen with the Trading sheet's ticket as the mock-up shows it (Binance BTC perpetual, 15 min,
/// « Démo Argos 100K », a Limit buy under the price, 5 % of the buying power, TP/SL ticked), to compare with the
/// mock-up (🖥 1440 × 900, then with the price scale zoomed out by hand to see the three lines; 📱 402 × 874: its top,
/// then its bottom with « Compte » unfolded): only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_module31_pour_jee() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    for phone in [false, true] {
        dossier_a_soi(if phone { "captures_m31_cell" } else { "captures_m31_pc" });
        let (mut a, w, h) = scene_ticket(phone);
        let save = |a: &mut App, name: &str| {
            let (img, _) = image_ecran(a, w, h);
            let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
            for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
                *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
            }
            pm.save_png(format!("{dir}/module31-{name}.png")).unwrap();
        };
        save(&mut a, if phone { "cell-haut" } else { "pc" });
        if !phone {
            // (🖥 the price scale zoomed out by hand, as the mock-up's: Limite, TP and SL in view)
            a.chart.settings.scale.manual = Some((56_700.0, 63_200.0));
            save(&mut a, "pc-3-lignes");
        } else {
            a.watch.panels.trading.compte_ouvert = true;
            // (scrolled to the bottom: the drawing keeps it to the sheet's end)
            a.watch.wheel(-1_000.0, 0.0);
            while a.watch.glide() {}
            save(&mut a, "cell-bas");
        }
    }
}

/// 🎫 📏 Zoom auto (mock-up of 22 Nov 2026, « ok maquette zoom auto »): the ticket's TP above the candles comes into
/// view by itself (the chart in its usual free mode); the ticket closed, the scale from before comes back; a ticket
/// line dragged keeps the scale still (the line stays under the mouse), the scale follows once let go; a scale moved
/// by hand during the ticket is left alone, and given back at the close.
#[test]
fn zoom_auto_du_ticket() {
    use argos_render::ordres::Prise;
    dossier_a_soi("zoom_auto_ticket");
    let (mut a, w, h) = scene_ticket(false);
    let _ = image_ecran(&mut a, w, h);
    let o = a.watch.panels.trading.ordre(true);
    let (tp, sl) = (o.tp.unwrap(), o.sl.unwrap());
    let haut = a.chart.bars.iter().map(|b| b.high).fold(f64::MIN, f64::max);
    assert!(tp > haut + 300.0, "the scene's TP is above the candles ({tp} vs {haut})");
    let ph = a.chart.viewport.plot_h();
    let vu = |a: &App, p: f64| a.scale_now().is_some_and(|s| s.y_of(p) > 10.0 && s.y_of(p) < ph - 10.0);
    assert!(vu(&a, tp) && vu(&a, sl) && vu(&a, 60_057.9), "Limite, TP and SL in view");
    // the ticket closed: the scale from before (the candles), the TP above the top
    a.watch.prefs.open = false;
    let _ = image_ecran(&mut a, w, h);
    assert!(!vu(&a, tp), "no ticket: the scale from before");
    let avant = a.chart.settings.scale.manual;
    a.watch.prefs.open = true;
    let _ = image_ecran(&mut a, w, h);
    assert!(vu(&a, tp), "next ticket: the zoom auto again");
    // the TP dragged higher: the scale holds still while it moves, then takes it in
    let fixe = a.scale_now().map(|s| (s.min, s.max)).unwrap();
    a.ordre_clic(Prise::TkTp);
    a.ordre_glisse(62_400.0);
    let _ = image_ecran(&mut a, w, h);
    assert_eq!(a.scale_now().map(|s| (s.min, s.max)), Some(fixe), "held while dragged");
    a.ordre_lache();
    let _ = image_ecran(&mut a, w, h);
    let tp2 = a.watch.panels.trading.ordre(true).tp.unwrap();
    assert!((tp2 - 62_400.0).abs() < 1.0 && vu(&a, tp2), "let go: the new TP comes into view ({tp2})");
    // moved by hand during the ticket: left alone (even with the TP out), given back at the close
    a.chart.settings.scale.manual = Some((59_900.0, 60_500.0));
    let _ = image_ecran(&mut a, w, h);
    assert_eq!(a.scale_now().map(|s| (s.min, s.max)), Some((59_900.0, 60_500.0)));
    a.watch.prefs.open = false;
    let _ = image_ecran(&mut a, w, h);
    assert_eq!(a.chart.settings.scale.manual, avant, "the scale from before the ticket");
    // automatic before the ticket: automatic again after it
    a.chart.settings.scale.manual = None;
    a.watch.prefs.open = true;
    let _ = image_ecran(&mut a, w, h);
    assert!(vu(&a, tp2) && a.chart.settings.scale.manual.is_some());
    a.watch.prefs.open = false;
    let _ = image_ecran(&mut a, w, h);
    assert!(a.chart.settings.scale.auto());
}

/// 🎫 📏 📱 Zoom auto on a phone: the Trading sheet covers the chart's bottom and the legend its top (all the width);
/// the ticket's Limite, TP and SL are drawn in the part between them.
#[test]
fn zoom_auto_du_ticket_sur_cell() {
    dossier_a_soi("zoom_auto_ticket_cell");
    let (mut a, w, h) = scene_ticket(true);
    let _ = image_ecran(&mut a, w, h);
    let (feuille, _) = a.ui.rows.expect("the sheet over the chart");
    assert!(feuille < a.chart.viewport.plot_h(), "the chart keeps its height under the glass sheet");
    let legende = argos_render::indleg::bas_legende(&a.ind.out, a.ind.leg.0, true);
    let o = a.watch.panels.trading.ordre(true);
    let s = a.scale_now().unwrap();
    for p in [o.tp.unwrap(), o.sl.unwrap(), 60_057.9] {
        let y = s.y_of(p);
        assert!(
            y > legende && y < feuille,
            "{p} under the legend and above the sheet ({legende} < {y} < {feuille})"
        );
    }
}

/// 📸 Zoom auto (mock-up of 22 Nov): 🖥 the module 31 scene, the three lines in view by themselves; 📱 the same with
/// its sheet: only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_zoom_auto_pour_jee() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    for phone in [false, true] {
        dossier_a_soi(if phone { "captures_zoom_cell" } else { "captures_zoom_pc" });
        let (mut a, w, h) = scene_ticket(phone);
        let (img, _) = image_ecran(&mut a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/zoom-auto-{}.png", if phone { "cell" } else { "pc" })).unwrap();
    }
}

/// 📖 🔥 Jee, 21 Nov: on a Binance chart the heatmap and the order-book panel read Binance's own book (not
/// Hyperliquid's), summed when several are ticked, named in the legend and the panel; Hyperliquid alone keeps its live
/// stream; a Hyperliquid stream book does not overwrite the chosen ones.
#[test]
fn heatmap_et_carnet_lisent_la_bourse_de_la_chart() {
    use argos_data::murs::{Carnet, Monnaie};
    use argos_render::dock::CarnetSource;
    use argos_render::indicators::Ind;
    let mut a = ecran_avec("carnets-choisis", vec![Ind::BookHeat]);
    a.chart.source = Source::Live {
        exchange: Exchange::Hyperliquid,
        base: "BTC".into(),
    };
    assert!(!a.carnets_instantanes(), "Hyperliquid chart: its live book");
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    assert!(a.carnets_instantanes());
    let ch = a.carnets_choix().unwrap();
    assert_eq!(ch.carnets, vec![(Carnet::BinancePerp, Monnaie::Usdt)]);
    // the books answered: summed into one, into the heatmap and the panel, named
    a.feeds.flow_coin = Some("BTC".into());
    a.carnets_refresh();
    let key = a.feeds.carnets.as_ref().unwrap().key.clone();
    let px = a.chart.bars.last().unwrap().close;
    let vb = argos_core::murs::VenueBook {
        venue: "Binance".into(),
        bids: vec![(px - 10.0, 2.0), (px - 200.0, 50.0)],
        asks: vec![(px + 10.0, 1.0), (px + 300.0, 40.0)],
    };
    a.on_user_event(UserEvent::Carnets(key, vec![(Carnet::BinancePerp, Monnaie::Usdt, Ok(vb))]));
    let b = a.feeds.book.clone().unwrap();
    assert_eq!(b.coin, "BTC");
    assert!(b.bids.iter().map(|l| l.sz).sum::<f64>() == 52.0 && b.asks.iter().map(|l| l.sz).sum::<f64>() == 41.0);
    assert!(a.feeds.heat.as_ref().is_some_and(|h| !h.cols.is_empty()), "the heatmap got the photo");
    assert_eq!(a.carnets_titre().as_deref(), Some("Binance USDT 1/1"));
    a.refresh_indicators();
    assert!(argos_render::indicators::legend_title(Ind::BookHeat, &a.ind.out).starts_with("Argos Heatmap du carnet · Binance USDT 1/1 ("));
    assert_eq!(a.carnet_source(), CarnetSource::Choisis("Binance USDT 1/1".into()));
    // Hyperliquid's stream does not replace the chosen book
    let hl = argos_data::book::Book {
        coin: "BTC".into(),
        time_ms: 1,
        bids: vec![argos_data::book::Level { px: 1.0, sz: 1.0, n: 1 }],
        asks: vec![argos_data::book::Level { px: 2.0, sz: 1.0, n: 1 }],
    };
    a.on_user_event(UserEvent::Flow(argos_data::live::LiveEvent::Book(hl)));
    assert_eq!(a.feeds.book.as_ref().unwrap().bids.iter().map(|l| l.sz).sum::<f64>(), 52.0);
}

/// 🧱 Lot 3c (Jee, 21 Nov 20:33 UTC): drawn from 1 min to 4 h, hidden above (margin released, grey legend), the books
/// still read; setting off: drawn everywhere. The top pill lists the base books first, then « + » the others.
#[test]
fn argos_murs_cache_au_dessus_de_4h_et_legende() {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    dossier_a_soi("argos_murs_4h");
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.ind.on.push(Ind::ArgosMurs);
    // before the first books: « Lecture des carnets… » against the price column
    a.murs_refresh();
    assert_eq!(a.murs_ecran().unwrap().etat.as_deref(), Some(crate::actions::murs::LECTURE));
    let step = a.feeds.murs.as_ref().unwrap().step;
    let px = a.chart.bars.last().unwrap().close;
    let wall = ((px * 1.01) / step).round() * step;
    let res: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> =
        Carnet::PERP.iter().map(|c| (*c, Ok(carnet_murs(px, step, &[(wall, 50.0)])))).collect();
    a.murs_recu("BTC".into(), res.clone(), None, None);
    let e = a.murs_ecran().unwrap();
    assert!(e.etat.is_none() && !e.vue.pills.is_empty() && !e.reach.is_empty());
    assert_eq!(a.murs_titre().unwrap().long, "Binance · OKX · Hyperliquid + Bybit · Bitunix 5/5");
    for (tf, cache) in [(60, false), (300, false), (3_600, false), (14_400, false), (86_400, true), (604_800, true)] {
        a.chart.interval = Interval(tf);
        assert_eq!(a.murs_ecran().is_none(), cache, "{tf}");
        assert_eq!(a.murs_marge() == 0.0, cache, "{tf}: margin");
        let t = a.murs_titre().unwrap();
        assert_eq!(t.cache, cache, "{tf}");
        if cache {
            assert_eq!(t.long, "· caché au-dessus de 4 h");
            assert_eq!(t.court, "· caché > 4 h");
        }
    }
    // hidden, the books are still read and followed
    let avant = a.feeds.murs.as_ref().unwrap().mem.len();
    a.murs_recu("BTC".into(), res.clone(), None, None);
    assert!(a.feeds.murs.as_ref().unwrap().perp.shown().iter().any(|w| w.price == wall));
    assert!(a.feeds.murs.as_ref().unwrap().mem.len() >= avant);
    // setting off: drawn at 1D too
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::ArgosMurs);
    let k = |key: &str| argos_render::murs::PARAMS.iter().position(|p| p.key == key).unwrap();
    v[k("cache4h")] = 0.0;
    // a base book unticked is not listed
    v[k("exOkx")] = 0.0;
    a.ind.set.insert(Ind::ArgosMurs, v);
    assert!(a.murs_ecran().is_some() && a.murs_marge() > 0.0);
    assert!(
        a.murs_titre().unwrap().long.starts_with("Binance · Hyperliquid + Bybit · Bitunix "),
        "{:?}",
        a.murs_titre()
    );
    // no book for this coin anywhere: said (lot 3e: once the last good snapshots are older than CARNET_GARDE_MS;
    // younger, they stand in for a failed request)
    for d in a.feeds.murs.as_mut().unwrap().derniers.iter_mut() {
        d.1 -= crate::actions::murs::CARNET_GARDE_MS + 1;
    }
    let rien: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> = Carnet::PERP.iter().map(|c| (*c, Err("pas de marché".to_string()))).collect();
    a.murs_recu("BTC".into(), rien, None, None);
    assert_eq!(a.murs_ecran().unwrap().etat.as_deref(), Some(crate::actions::murs::PAS_DE_CARNET_PUBLIC));
}

/// 📸 The Simulator in the Trading volet (PC 1440 × 900, Binance BTC perpetual 15 min, a buy with its SL at 0,5 %), to
/// compare with the Interface thread's capture of the defect: only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_simulateur_dans_le_volet() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    dossier_a_soi("captures_simulateur_volet");
    let mut a = pc_avec_volet(wl::Tool::Trading);
    a.top.prefs.bienvenue_vue = true;
    let prix = |i: i64| {
        let x = i as f64;
        let base = if i < 70 {
            59_200.0 + 1_400.0 * x / 70.0
        } else if i < 140 {
            60_600.0 - 1_100.0 * (x - 70.0) / 70.0
        } else {
            59_500.0 + 860.0 * (x - 140.0) / 59.0
        };
        ((base + 90.0 * (x * 1.7).sin()) * 10.0).round() / 10.0
    };
    a.chart.raw = (0..200)
        .map(|i| Bar {
            time: START + i * 900,
            open: prix(i),
            high: prix(i).max(prix(i + 1)) + 35.0,
            low: prix(i).min(prix(i + 1)) - 35.0,
            close: prix(i + 1),
            volume: 10.0,
        })
        .collect();
    a.chart.source = Source::Live {
        exchange: argos_data::Exchange::Binance,
        base: "BTC".into(),
    };
    (a.chart.interval, a.chart.file_interval) = (Interval(900), Interval(900));
    a.title = argos_data::Exchange::Binance.chart_title("BTC", "15m");
    a.ind.on.clear();
    a.ui.frames = 5;
    a.refresh_indicators();
    a.rebuild();
    a.start_free();
    a.demo = argos_core::portfolio::Portfolio::default();
    a.watch.panels.trading.guardian = false;
    let (w, h) = (1440, 900);
    let save = |a: &mut App, name: &str| {
        let (img, _) = image_ecran(a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/simulateur-volet-{name}.png")).unwrap();
    };
    let _ = image_ecran(&mut a, w, h);
    save(&mut a, "ticket");
    let v = a.volet_rect().unwrap();
    clic(&mut a, v.x + 93.5 + 50.0, v.y + 93.0);
    a.ui.sim.as_mut().unwrap().sl = 2;
    a.sim_place();
    save(&mut a, "pc");
}

/// 📍 The high / low pills keep clear of the floating toolbar laid on the chart (not when it is hidden), and of the
/// legend's unfolded list on PC.
#[test]
fn plus_haut_plus_bas_evitent_la_barre_d_outils() {
    let mut a = app();
    let text = argos_render::text::Text::default();
    let (bar, btns) = argos_render::toolbar::layout(&a.tools, &text);
    assert!(!btns.is_empty());
    assert!(a.hilo_garder().contains(&bar));
    assert_eq!(a.chart_with(false).garder, a.hilo_garder());
    a.tools.hidden = true;
    assert!(!a.hilo_garder().contains(&bar));
    // the legend's list unfolded (one indicator on the price): its lines are kept clear too
    a.tools.hidden = false;
    a.ind.on.push(argos_render::indicators::Ind::Tvd);
    a.refresh_indicators();
    a.ind.leg.0 = true;
    assert!(a.hilo_garder().len() > 1, "{:?}", a.hilo_garder());
}

/// 📟 Tablet 820 × 1180 by the finger: Argos Murs' two bottom pills sit above the floating symbol pill and ƒx (none
/// touches them); the lowest's price pill keeps clear of their new place. Phone and PC: unchanged.
#[test]
fn tablette_pilules_d_argos_murs_au_dessus_des_boutons() {
    let mut a = cell_390();
    a.area = (820, (1180.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
    (a.chart.viewport.width, a.chart.viewport.height) = (a.area.0, a.area.1);
    argos_render::actions::set_tactile(true);
    let y = a.boutons_bas_y().expect("the buttons float on the tablet");
    let c = a.chart_with(false);
    assert_eq!(c.boutons_bas, Some(y));
    let murs = argos_render::murs_pilules_bas(&c);
    let m = argos_render::ecrans::cell();
    for id in ["pg", "pd"] {
        let r = m.place(id, a.win_w(), a.cell_h()).unwrap();
        let r = Rect::new(r.x, r.y - a.top_h(), r.w, r.h);
        let touche = murs.x < r.x + r.w && r.x < murs.x + murs.w && murs.y < r.y + r.h && r.y < murs.y + murs.h;
        assert!(!touche, "{id}: {r:?} / {murs:?}");
        assert!(murs.y + murs.h <= r.y - 4.0, "{id} just under: {r:?} / {murs:?}");
    }
    // phone (its single pill) and PC: nothing floats there, the pills stay at the plot's bottom
    let mut p = cell_390();
    p.watch.phone = Some(390);
    assert!(p.pilule() && p.boutons_bas_y().is_none());
    argos_render::actions::set_tactile(false);
    p.top.phone = false;
    assert!(p.boutons_bas_y().is_none());
    assert!(app().chart_with(false).boutons_bas.is_none());
}

/// ➕ « Nouvelle mise en page » (Jee, 21 Nov): the screen left is kept first (indicators and split); the new one
/// opens on the same market and time frame, one chart, the default indicators, each with « mes défauts » when there
/// are some, else its original settings. 📄 A copy keeps everything.
#[test]
fn nouvelle_mise_en_page_aux_indicateurs_par_defaut() {
    use crate::actions::layouts::{Layouts, FICHIER};
    use argos_render::indicators::{values, Ind};
    dossier_a_soi("layout-defauts");
    let mut a = app();
    a.top.prefs.layout_name = "Chargée".into();
    a.top.prefs.auto_save = true;
    a.split.layout = "4g".into();
    a.ind.on = vec![Ind::Rsi, Ind::Tvd, Ind::ArgosMurs];
    // settings changed on the loaded screen
    let mut murs = values(&Default::default(), Ind::ArgosMurs);
    murs[0] += 1.0;
    a.ind.set.insert(Ind::ArgosMurs, murs.clone());
    // « mes défauts » of TVD
    let mut tvd = values(&Default::default(), Ind::Tvd);
    tvd[0] += 2.0;
    App::enregistrer_mes_defauts(Ind::Tvd, &tvd, &Default::default());
    let (iv, sym) = (a.chart.interval, a.symbol_label());
    a.nouvelle_layout(false);
    assert_eq!(a.top.prefs.layout_name, "Mise en page");
    assert_eq!(a.ind.on, crate::actions::murs::DEFAUTS.to_vec());
    assert_eq!(a.split.layout, "1", "one chart");
    assert_eq!((a.chart.interval, a.symbol_label()), (iv, sym), "same market and time frame");
    assert_eq!(values(&a.ind.set, Ind::Tvd), tvd, "mes défauts");
    assert_eq!(
        values(&a.ind.set, Ind::ArgosMurs),
        values(&Default::default(), Ind::ArgosMurs),
        "original settings"
    );
    assert!(a.banner.as_deref().unwrap_or("").contains("« Mise en page » créée"));
    // the layout left keeps its indicators, its settings and its split
    let all: Layouts = App::read_prefs(FICHIER);
    let vieille = all.liste.iter().find(|l| l.nom == "Chargée").expect("kept");
    assert_eq!(vieille.indicateurs, serde_json::to_value([Ind::Rsi, Ind::Tvd, Ind::ArgosMurs]).unwrap());
    assert_eq!(vieille.split["layout"], "4g");
    let set: argos_render::indicators::Settings = serde_json::from_value(vieille.reglages.clone()).unwrap();
    assert_eq!(values(&set, Ind::ArgosMurs), murs);
    let neuve = all.liste.iter().find(|l| l.nom == "Mise en page").expect("saved");
    assert_eq!(neuve.split["layout"], "1");
    // 📄 a copy keeps the screen as it is
    a.ind.on.push(Ind::Rsi);
    a.nouvelle_layout(true);
    assert_eq!(a.top.prefs.layout_name, "Mise en page (copie)");
    // (the default set and the Rsi added: Argos Vague joined the defaults on argos-vague-1)
    assert_eq!(a.ind.on.len(), crate::actions::murs::DEFAUTS.len() + 1, "the copy keeps everything");
}

/// 🗂 Layouts, non-regression (Jee, 21 Nov): create, open from a tile, rename (open and not), ★ then off, duplicate,
/// delete one not open, delete the open one (another opens: the 1st favourite, else the 1st of the list), the only
/// one (« Supprimer… » greyed); a tile's index k always aims at the right layout after ★ and after a deletion; the
/// banners of the Accueil's actions show over it.
#[test]
fn mises_en_page_non_regression() {
    use crate::actions::layouts::{Layout, Layouts, FICHIER};
    dossier_a_soi("layouts-non-regression");
    let mut a = app();
    a.top.prefs.bienvenue_vue = true;
    a.top.prefs.auto_save = true;
    a.top.prefs.layout_name = "Argos".into();
    let l = |nom: &str| Layout {
        nom: nom.into(),
        ..Default::default()
    };
    App::write_prefs(
        FICHIER,
        &Layouts {
            ouverte: "Argos".into(),
            liste: vec![l("Argos"), l("Or"), l("Nuit")],
        },
    );
    let noms = |a: &App| a.liste_layouts().into_iter().map(|x| x.0).collect::<Vec<_>>();
    let r = Rect::new(500.0, 300.0, 28.0, 28.0);
    a.open_accueil();
    // ➕ create (from the Accueil: the chart opens with the banner)
    a.accueil_result(argos_render::accueil::AccClick::NouvelleLayout);
    assert_eq!(a.top.prefs.layout_name, "Mise en page");
    assert_eq!(noms(&a), ["Argos", "Or", "Nuit", "Mise en page"]);
    // open from a tile
    a.open_accueil();
    a.accueil_result(argos_render::accueil::AccClick::Layout(1));
    assert_eq!(a.top.prefs.layout_name, "Or");
    assert!(a.ui.accueil.is_none());
    // ✏️ rename the open one, then one not open
    a.open_accueil();
    a.renommer_layout_de("Or", "Or 4h".into());
    assert_eq!(a.top.prefs.layout_name, "Or 4h");
    a.renommer_layout_de("Nuit", "Nuit 1h".into());
    assert_eq!(a.top.prefs.layout_name, "Or 4h");
    assert_eq!(noms(&a), ["Argos", "Or 4h", "Nuit 1h", "Mise en page"]);
    // ★ then off: the tiles keep their order, k still aims at the same layout
    a.accueil_result(argos_render::accueil::AccClick::Favori(2));
    assert_eq!(a.favoris_layouts(), ["Nuit 1h"]);
    assert_eq!(a.nom_layout(2).as_deref(), Some("Nuit 1h"));
    assert!(a.banner.as_deref().unwrap_or("").contains("favoris"));
    a.accueil_result(argos_render::accueil::AccClick::Favori(2));
    assert!(a.favoris_layouts().is_empty());
    assert_eq!(a.nom_layout(2).as_deref(), Some("Nuit 1h"));
    // 📄 duplicate
    a.tuile_pick(ListKind::TuileLayout(0), 3, r);
    assert_eq!(noms(&a), ["Argos", "Or 4h", "Nuit 1h", "Mise en page", "Argos (copie)"]);
    // 🗑 one not open: after it, k aims at the next one
    a.tuile_pick(ListKind::SupprimerLayout(3), 6, r);
    assert_eq!(noms(&a), ["Argos", "Or 4h", "Nuit 1h", "Argos (copie)"]);
    assert_eq!(a.nom_layout(3).as_deref(), Some("Argos (copie)"));
    assert_eq!(a.top.prefs.layout_name, "Or 4h");
    // the banner shows over the Accueil, in a glass pill at the bottom
    assert!(a.ui.accueil.is_some());
    let b = a.banner.clone().expect("a banner");
    assert!(b.contains("« Mise en page » supprimée"), "{b}");
    let (_, br) = argos_render::bandeau_verre(&argos_render::text::Text::default(), &b, 1440.0, 900.0).expect("drawn");
    assert!(br.y > 800.0 && br.x > 0.0 && br.x + br.w < 1440.0, "{br:?}");
    // 🗑 the open one: the 1st favourite opens (else the 1st of the list)
    a.accueil_result(argos_render::accueil::AccClick::Favori(2));
    a.tuile_pick(ListKind::SupprimerLayout(1), 6, r);
    assert_eq!(a.top.prefs.layout_name, "Nuit 1h", "the favourite");
    assert_eq!(noms(&a), ["Argos", "Nuit 1h", "Argos (copie)"]);
    assert_eq!(a.banner.as_deref(), Some("🗑 « Or 4h » supprimée · « Nuit 1h » ouverte"));
    let all: Layouts = App::read_prefs(FICHIER);
    assert_eq!(all.ouverte, "Nuit 1h");
    a.accueil_result(argos_render::accueil::AccClick::Favori(1));
    a.tuile_pick(ListKind::SupprimerLayout(1), 6, r);
    assert_eq!(a.top.prefs.layout_name, "Argos", "no favourite: the 1st of the list");
    a.tuile_pick(ListKind::SupprimerLayout(1), 6, r);
    assert_eq!(noms(&a), ["Argos"]);
    // the only one: « Supprimer… » greyed (the rest of the menu unchanged), and refused
    a.ui.list = None;
    a.menu_tuile(0, r);
    let (_, m) = a.ui.list.as_ref().expect("the menu");
    let items: Vec<(String, bool)> = m.groups.iter().flat_map(|g| &g.items).map(|i| (i.label.clone(), i.enabled)).collect();
    assert_eq!(items.len(), 5);
    assert!(items.iter().all(|(l, on)| *on == (l != "Supprimer…")), "{items:?}");
    a.ui.list = None;
    a.supprimer_layout("Argos");
    assert_eq!(noms(&a), ["Argos"]);
    // with two again: enabled
    a.tuile_pick(ListKind::TuileLayout(0), 3, r);
    a.menu_tuile(0, r);
    let (_, m) = a.ui.list.as_ref().expect("the menu");
    assert!(m.groups.iter().flat_map(|g| &g.items).all(|i| i.enabled));
}

/// 🗑 Deleting the open layout opens another one, whose market reloads: when its candles arrive, the banner
/// « « X » supprimée · « Y » ouverte » stays (a load's own banner still wins; another banner still goes).
#[test]
fn bandeau_de_mise_en_page_garde_au_chargement() {
    let charge = |a: &mut App, banner: Option<String>| {
        let gen = a.gen.load(Ordering::SeqCst);
        a.on_user_event(UserEvent::Loaded(
            gen,
            Ok(crate::run::Candles {
                bars: bougies(),
                title: "essai".into(),
                banner,
                primary: false,
                a_reparer: vec![],
                secours: None,
            }),
        ));
    };
    let mut a = app();
    let b = "🗑 « Or » supprimée · « Argos » ouverte".to_string();
    a.banner = Some(b.clone());
    charge(&mut a, None);
    assert_eq!(a.banner.as_deref(), Some(b.as_str()));
    a.banner = Some("🗂 Mise en page « Argos » ouverte".into());
    charge(&mut a, Some("source de secours".into()));
    assert_eq!(a.banner.as_deref(), Some("source de secours"));
    a.banner = Some("autre chose".into());
    charge(&mut a, None);
    assert!(a.banner.is_none());
}

// — 📜 editeur-pine-1 : un éditeur honnête —

/// The last line written in the editor's terminal, and its kind.
fn derniere_ligne(a: &App) -> (String, argos_render::editor::LogKind) {
    let ed = a.ui.editor.as_ref().expect("l'éditeur");
    let (_, m, k) = ed.log.last().expect("une ligne");
    (m.clone(), *k)
}

/// Opens a draft editor with `src` and presses ▶.
fn tester_dans_l_editeur(a: &mut App, src: &str) {
    a.editor_on(None, Some(src));
    a.editor_run(false);
}

/// A0: a user script gets what the built-in scripts get — `barstate.isconfirmed` false on a last bar still forming
/// (the options of the refresh), the chart's symbol in `syminfo`.
#[test]
fn les_scripts_de_l_utilisateur_ont_les_options_des_integres() {
    dossier_a_soi("editeur-pine-1-a0");
    let mut a = app();
    a.scripts.push(SavedScript {
        name: "c".into(),
        src: "//@version=6\nindicator(\"c\")\nplot(barstate.isconfirmed ? 1 : 0, \"c\")\n".into(),
        on: true,
        ..Default::default()
    });
    let fermee = a.options_scripts(&[]);
    assert!(!fermee.last_open, "old bars: the last one is closed");
    let v = a.run_scripts(vec![], &fermee);
    assert_eq!(v[0].out.plots[0].values.last(), Some(&1.0));
    let mut b = *a.chart.bars.last().unwrap();
    b.time = now_s().div_euclid(1800) * 1800;
    a.chart.bars.push(b);
    let ouverte = a.options_scripts(&[]);
    assert!(ouverte.last_open, "a last bar of now is still forming");
    let v = a.run_scripts(vec![], &ouverte);
    let c = &v[0].out.plots[0].values;
    assert_eq!((c[c.len() - 2], c[c.len() - 1]), (1.0, 0.0));
}

/// A1 + A3: ▶ with an unknown function on line 7 (after a `//@version=6` header, comments and blank lines): the red
/// « ❌ Ligne 7 », the line marked, nothing saved nor added to the chart.
#[test]
fn erreur_a_la_ligne_de_l_editeur_rien_n_est_ajoute() {
    dossier_a_soi("editeur-pine-1-a1-erreur");
    let mut a = app();
    let src = "//@version=6\n// un commentaire\nindicator(\"e\")\n\n// encore\nx = close\ny = ta.inconnue(x)\nplot(y)\n";
    tester_dans_l_editeur(&mut a, src);
    let (m, k) = derniere_ligne(&a);
    assert!(m.starts_with("❌ Ligne 7 : "), "{m}");
    assert_eq!(k, argos_render::editor::LogKind::Err);
    assert_eq!(a.ui.editor.as_ref().unwrap().err_line, Some(7));
    assert!(a.scripts.is_empty());
}

/// A1: no bar on the chart yet: the amber « ⏳ », nothing saved.
#[test]
fn pas_encore_de_bougies() {
    dossier_a_soi("editeur-pine-1-a1-bougies");
    let mut a = app();
    a.chart.bars.clear();
    tester_dans_l_editeur(&mut a, argos_render::editor::TEMPLATE);
    assert_eq!(
        derniere_ligne(&a),
        (
            "⏳ Pas encore de bougies sur la chart : réessaie dans un instant.".to_string(),
            argos_render::editor::LogKind::Warn
        )
    );
    assert!(a.scripts.is_empty());
}

/// A1: Jee's paste cut after its 79th line (declaration, inputs, helpers, no plot): the red « ne trace rien », the
/// grey line count, nothing saved — not an empty « Vague » pane any more (photo-10).
#[test]
fn un_script_qui_ne_trace_rien_n_est_pas_ajoute() {
    dossier_a_soi("editeur-pine-1-a1-rien");
    let mut a = app();
    let v4 = include_str!("../../../../pine/argos/argos-vague-tradingview.pine");
    let debut: String = v4.lines().take(79).map(|l| format!("{l}\n")).collect();
    tester_dans_l_editeur(&mut a, &debut);
    let ed = a.ui.editor.as_ref().unwrap();
    let n = ed.log.len();
    assert_eq!(
        ed.log[n - 2].1,
        "❌ Ce script ne trace rien : il lui faut au moins un plot(), un plotshape() ou un dessin."
    );
    assert_eq!(ed.log[n - 1].1, "Le code a 79 lignes : vérifie qu'il a été collé en entier.");
    assert_eq!(ed.log[n - 1].2, argos_render::editor::LogKind::Note);
    assert!(a.scripts.is_empty());
    // (the whole file: added)
    tester_dans_l_editeur(&mut a, v4);
    assert_eq!(a.scripts.len(), 1);
    assert!(a.ui.editor.as_ref().unwrap().log.iter().any(|l| l.1.starts_with("✅ Compilé")));
}

/// A1: output calls that draw nothing on these bars: « ✅ Compilé » then the amber « ⚠ Rien à tracer ».
#[test]
fn rien_a_tracer_pour_l_instant() {
    dossier_a_soi("editeur-pine-1-a1-vide");
    let mut a = app();
    tester_dans_l_editeur(
        &mut a,
        "//@version=6\nindicator(\"v\")\nif bar_index < 0\n    label.new(bar_index, high, \"x\")\n",
    );
    let ed = a.ui.editor.as_ref().unwrap();
    assert!(ed.log.iter().any(|l| l.1.starts_with("✅ Compilé")));
    assert!(ed
        .log
        .iter()
        .any(|l| l.1 == "⚠ Rien à tracer sur ces bougies pour l'instant." && l.2 == argos_render::editor::LogKind::Warn));
    assert_eq!(a.scripts.len(), 1);
}

/// A2: on the chart, a saved script that draws nothing (Jee's « Vague ») or that never ran gets no pane: one line
/// of the price legend, « ⚠ ne trace rien » / « ⚠ erreur ligne n »; one that ran keeps its drawing and says
/// « ⚠ erreur ligne n »; opening it puts the cursor on that line, the error in red.
#[test]
fn legende_d_un_script_en_erreur_sans_panneau_vide() {
    dossier_a_soi("editeur-pine-1-a2");
    let mut a = app();
    let v4 = include_str!("../../../../pine/argos/argos-vague-tradingview.pine");
    let debut: String = v4.lines().take(79).map(|l| format!("{l}\n")).collect();
    for (name, src) in [
        ("Vague", debut.as_str()),
        ("Cassé", "//@version=6\nindicator(\"k\")\nplot(ta.inconnue(close))\n"),
    ] {
        a.scripts.push(SavedScript {
            name: name.into(),
            src: src.into(),
            on: true,
            ..Default::default()
        });
    }
    let opts = a.options_scripts(&[]);
    let v = a.run_scripts(vec![], &opts);
    assert!(v.iter().all(|s| s.out.overlay), "no pane");
    assert_eq!(v[0].alerte().as_deref(), Some("⚠ ne trace rien"));
    assert_eq!(v[1].alerte().as_deref(), Some("⚠ erreur ligne 3"));
    // (a first run before the bars arrive, then a failing one: still no pane)
    let bougies = std::mem::take(&mut a.chart.bars);
    let avant = a.run_scripts(vec![], &opts);
    a.chart.bars = bougies;
    let v = a.run_scripts(avant, &opts);
    assert!(v[1].out.overlay, "no empty pane after a run without bars");
    assert_eq!(v[1].alerte().as_deref(), Some("⚠ erreur ligne 3"));
    // (a script that ran, then fails: its drawing stays, the legend says why)
    a.scripts[1].src = "//@version=6\nindicator(\"k\")\nplot(close)\n".into();
    let bon = a.run_scripts(vec![], &opts);
    assert!(!bon[1].out.overlay && bon[1].alerte().is_none());
    a.scripts[1].src = "//@version=6\nindicator(\"k\")\nplot(ta.inconnue(close))\n".into();
    let v = a.run_scripts(bon, &opts);
    assert!(!v[1].out.overlay && !v[1].out.plots.is_empty(), "the drawing before stays");
    assert_eq!(v[1].alerte().as_deref(), Some("⚠ erreur ligne 3"));
    a.ind.out.scripts = v;
    a.editor_on(Some(1), None);
    assert_eq!(a.ui.editor.as_ref().unwrap().err_line, Some(3));
    let (m, k) = derniere_ligne(&a);
    assert!(m.starts_with("❌ Ligne 3 : "), "{m}");
    assert_eq!(k, argos_render::editor::LogKind::Err);
}

/// A4 + A5: everything selected (Ctrl + A), then a paste from the page (`paste_text`) replaces everything; a whole
/// script pasted into the untouched « Nouveau script » template replaces it.
#[test]
fn tout_selectionne_puis_coller_remplace_tout() {
    dossier_a_soi("editeur-pine-1-a5");
    let mut a = app();
    a.editor_on(None, None);
    a.ui.editor.as_mut().unwrap().tout_selectionner();
    a.paste_text("plot(close)".into());
    let ed = a.ui.editor.as_ref().unwrap();
    assert_eq!(ed.text(), "plot(close)");
    assert!(!ed.tout);
    a.editor_on(None, None);
    let script = "//@version=6\nindicator(\"Mien\")\nplot(close)\n";
    a.paste_text(script.into());
    assert_eq!(a.ui.editor.as_ref().unwrap().text(), script, "no second indicator() in one text");
}

// — 🔬 editeur-pine-1, part D : les petites bougies des scripts —

/// D: merging by time keeps the older bars and puts the new ones after (the newest win), 10 000 at most.
#[test]
fn fusion_des_petites_bougies() {
    use crate::actions::indicators::{fusionner, unite_petites, MAX_PETITES};
    let b = |t: i64, c: f64| Bar {
        time: t,
        open: c,
        high: c,
        low: c,
        close: c,
        volume: 1.0,
    };
    let mut v: Vec<Bar> = (0..5).map(|i| b(i * 60, 1.0)).collect();
    assert_eq!(fusionner(&mut v, vec![b(180, 2.0), b(240, 2.0), b(300, 2.0)]), 6);
    assert_eq!(
        v.iter().map(|x| (x.time, x.close)).collect::<Vec<_>>()[2..],
        [(120, 1.0), (180, 2.0), (240, 2.0), (300, 2.0)]
    );
    let mut grand: Vec<Bar> = (0..MAX_PETITES as i64).map(|i| b(i * 60, 1.0)).collect();
    fusionner(&mut grand, vec![b(MAX_PETITES as i64 * 60, 3.0)]);
    assert_eq!((grand.len(), grand[0].time), (MAX_PETITES, 60), "the oldest goes");
    assert_eq!(
        (unite_petites(60), unite_petites(300), unite_petites(3600), unite_petites(86_400)),
        ("1 min".into(), "5 min".into(), "1 h".into(), "1 j".into())
    );
}

/// 🔒 Reliable data: the weeks of Hyperliquid, Gate.io, Kraken and KuCoin begin on Thursday (24 Nov. 2026); AI
/// Trend's slots and the scripts' lower timeframes make a week of days, from Monday, like the chart's, on every
/// exchange; the rest as before (the exchange's interval that divides it).
#[test]
fn the_lower_timeframes_make_their_weeks_of_days_like_the_chart() {
    use crate::actions::indicators::lower_source;
    for sec in [7 * 86_400, 14 * 86_400] {
        assert_eq!(lower_source(sec), Some(Timeframe::D1), "{sec} s: days, from Monday");
        assert_eq!(lower_source(sec), Interval(sec).source(), "the chart's own rule");
    }
    assert_eq!(lower_source(5 * 3600), Some(Timeframe::H1), "5 h = 1 h regrouped");
    assert_eq!(lower_source(4 * 3600), Some(Timeframe::H4));
    assert_eq!(lower_source(86_400), Some(Timeframe::D1));
    assert_eq!(lower_source(60), Some(Timeframe::M1));
    assert_eq!(lower_source(30), None, "seconds: none");
    let monday = argos_core::bucket_start(1_795_498_960, 7 * 86_400);
    assert_eq!((monday - 4 * 86_400).rem_euclid(7 * 86_400), 0, "a week of the chart begins on Monday");
}

/// D3 + D5 + D6: the bars arrive for the current key → the scripts get them (`options_scripts`), the editor open on
/// the script says « ✅ … chargées » ; an answer for an old key is dropped; a failure → « ⚠ … indisponibles », counted.
#[test]
fn les_petites_bougies_arrivent_aux_scripts() {
    use crate::state::{EtatPetites, ScriptMtf};
    dossier_a_soi("editeur-pine-1-d");
    let mut a = app();
    let src = "//@version=6\nindicator(\"p\")\nc = request.security_lower_tf(syminfo.tickerid, \"1\", close)\nplot(array.size(c), \"n\")\n";
    tester_dans_l_editeur(&mut a, src);
    // (a file chart: nothing to load, the script stays on its fallback and the terminal says so)
    assert!(a
        .ui
        .editor
        .as_ref()
        .unwrap()
        .log
        .iter()
        .any(|l| l.1 == "⚠ Bougies 1 min indisponibles sur ce marché : le script reste sur son calcul de repli."));
    a.refresh_indicators();
    assert_eq!(a.ind.out.scripts[0].out.lower_tf_asked, [60]);
    assert!(a.feeds.script_mtf.is_none());
    a.feeds.script_mtf = Some(ScriptMtf {
        key: "k".into(),
        exchange: argos_data::Exchange::Hyperliquid,
        at: None,
        en_cours: true,
        echecs: 0,
        etats: [(60, EtatPetites::Chargement)].into_iter().collect(),
    });
    let derniere = *a.chart.bars.last().unwrap();
    let minutes: Vec<Bar> = (0..90)
        .map(|i| Bar {
            time: derniere.time - 1800 * 2 + i * 60,
            ..derniere
        })
        .collect();
    a.script_bars_recues("vieille".into(), vec![(60, Ok(minutes.clone()))]);
    assert!(a.feeds.script_bars.is_empty(), "an old key: dropped");
    a.script_bars_recues("k".into(), vec![(60, Ok(minutes))]);
    assert_eq!(a.feeds.script_mtf.as_ref().unwrap().etats[&60], EtatPetites::Chargees(90));
    assert!(a
        .ui
        .editor
        .as_ref()
        .unwrap()
        .log
        .iter()
        .any(|l| l.1 == "✅ Bougies 1 min chargées (90) : le script utilise les vraies petites bougies."));
    // (R2: the 60 s refresh of the newest bars writes no new line)
    let plus: Vec<Bar> = (0..2)
        .map(|i| Bar {
            time: derniere.time + 60 * (30 + i),
            ..derniere
        })
        .collect();
    a.script_bars_recues("k".into(), vec![(60, Ok(plus))]);
    let n_lignes = a.ui.editor.as_ref().unwrap().log.iter().filter(|l| l.1.starts_with("✅ Bougies 1 min")).count();
    assert_eq!(n_lignes, 1, "once per loading");
    a.script_bars_recues("k".into(), vec![(60, Ok(vec![]))]);
    let opts = a.options_scripts(&[]);
    assert_eq!(opts.intrabars[&60].len(), 92);
    let v = a.run_scripts(vec![], &opts);
    let n = &v[0].out.plots[0].values;
    assert_eq!(
        &n[n.len() - 3..],
        [30.0, 30.0, 32.0],
        "30 minutes each, the bar in progress its 2 newer ones too"
    );
    assert!(v[0].out.lower_tf_missing.is_empty());
    a.feeds.script_mtf.as_mut().unwrap().etats.insert(300, EtatPetites::Chargement);
    a.script_bars_recues("k".into(), vec![(300, Err("réseau".into()))]);
    let m = a.feeds.script_mtf.as_ref().unwrap();
    assert_eq!((m.etats[&300], m.echecs), (EtatPetites::Indisponibles, 1));
}

/// R2 (editeur-pine-1): after ▶ the verdict stays in the terminal's visible lines: a script that logs 5 alerts ends
/// its run with « ✅ Compilé… », after the 📋 lines.
#[test]
fn le_verdict_reste_la_derniere_ligne() {
    dossier_a_soi("editeur-pine-1-r2");
    let mut a = app();
    tester_dans_l_editeur(
        &mut a,
        "//@version=6\nindicator(\"a\")\nplot(close)\nif bar_index >= last_bar_index - 4\n    alert(\"bougie \" + str.tostring(bar_index))\n",
    );
    let ed = a.ui.editor.as_ref().unwrap();
    let n = ed.log.len();
    assert!(ed.log[n - 1].1.starts_with("✅ Compilé"), "{:?}", ed.log[n - 1]);
    assert!(ed.log[n - 6..n - 1].iter().all(|l| l.1.starts_with("📋 bougie")));
}

/// 📜 R1 (editeur-pine-1): the top edge of a script's pane is dragged like a native band's; the height is kept in the
/// chart settings by script id (saved with them), and forgotten when the script is deleted.
#[test]
fn panneau_de_script_tire_garde_et_oublie() {
    dossier_a_soi("editeur-pine-1-r1");
    let mut a = app();
    tester_dans_l_editeur(&mut a, "//@version=6\nindicator(\"p\")\nplot(close)\n");
    a.refresh_indicators();
    let _ = a.chart_with(false); // (a drawing sets the heights the panes read)
    argos_render::render(&a.chart_with(false));
    let ph = a.chart.viewport.plot_h();
    let (_, top) = a.ind.out.script_bands(ph)[0];
    assert!(a.script_bord(top + 1.0));
    assert!(a.script_bord_appui(top));
    assert!(a.script_bord_glisse(top - 60.0));
    a.pointer.script_drag = None;
    let id = a.scripts[0].id.clone();
    assert_eq!(a.chart.settings.script_bands.get(&id).copied(), Some(argos_render::indicators::OSC_H + 60.0));
    // (kept with the chart settings: written and read back)
    let txt = serde_json::to_string(&a.chart.settings).unwrap();
    let relu: argos_render::settings::ChartSettings = serde_json::from_str(&txt).unwrap();
    assert_eq!(relu.script_bands.get(&id).copied(), Some(argos_render::indicators::OSC_H + 60.0));
    // 🗑️ deleting the script drops its height
    a.editor_on(Some(0), None);
    a.editor_click(argos_render::editor::EdClick::Item(argos_render::editor::EdItem::Delete));
    a.editor_click(argos_render::editor::EdClick::Item(argos_render::editor::EdItem::Delete));
    assert!(a.scripts.is_empty());
    assert!(a.chart.settings.script_bands.is_empty());
}

// — 🧱 Argos Murs, lot pilules-1 (Jee, 22 Nov: the wall pills covered the last candles on a phone) —

/// 🧱 Binance BTCUSDT perp 5 min, Argos Murs on, a sell and a buy wall at the price of the last candles.
fn murs_aux_dernieres_bougies(mut a: App) -> App {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.chart.interval = Interval(300);
    a.ind.on.push(Ind::ArgosMurs);
    a.murs_refresh();
    let step = a.feeds.murs.as_ref().unwrap().step;
    let px = a.chart.bars.last().unwrap().close;
    let k0 = (px / step).round();
    let (sell, buy) = ((k0 + 2.0) * step, (k0 - 2.0) * step);
    let res: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> = Carnet::PERP
        .iter()
        .map(|c| (*c, Ok(carnet_murs(px, step, &[(sell, 60.0), (buy, 60.0)]))))
        .collect();
    a.murs_recu("BTC".into(), res, None, None);
    assert!(a.murs_ecran().is_some_and(|e| e.vue.pills.len() >= 2), "two walls read");
    a
}

fn croisent(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// 🧱 Every wall pill of the last drawing: on no candle, no table, no rect of the app over the chart; every wall
/// drawn (none dropped for room), the tooltip of each says its amount and its exchange.
fn pilules_libres(a: &mut App, w: u32, h: u32, tactile: bool) -> Vec<(Rect, String)> {
    let _ = image_ecran(a, w, h);
    // (the test lays the screen out as a PC window: the finger's view is set again after it)
    argos_render::actions::set_tactile(tactile);
    let c = a.chart_with(false);
    let _ = argos_render::render(&c);
    let pilules = argos_render::murs::pilules_dessinees();
    let e = a.murs_ecran().unwrap();
    // (the two walls of the test at least; the Binance-tagged wall, when there is one, never dropped)
    assert!(pilules.len() >= 2, "{w}: the walls drawn: {pilules:?}");
    let bougies = argos_render::murs::bougies();
    assert!(!bougies.is_empty());
    for (r, s) in &pilules {
        for b in &bougies {
            assert!(!croisent(r, b), "{w}: pill {s:?} {r:?} on the candle {b:?}");
        }
        for t in argos_render::indicators::tableaux() {
            assert!(!croisent(r, &t), "{w}: pill {s:?} {r:?} on the table {t:?}");
        }
        for g in &c.garder {
            assert!(!croisent(r, g), "{w}: pill {s:?} {r:?} under {g:?}");
        }
    }
    let (_, layer) = argos_render::render_fond(&c);
    let layer = layer.expect("mouse layer");
    for (r, s) in &pilules {
        let l = layer.murs_infobulle(r.cx(), r.cy()).join(" | ");
        assert!(l.contains(" $"), "{w}: amount in {l:?} ({s:?})");
        assert!(l.contains("Binance") || l.contains("BINANCE"), "{w}: exchange in {l:?}");
        assert!(l.contains("PERP"), "{w}: perp in {l:?}");
    }
    if e.vue.pills.iter().any(|p| p.tag) {
        let dit = pilules.iter().any(|(r, _)| layer.murs_infobulle(r.cx(), r.cy()).join(" ").contains("BINANCE"));
        assert!(dit, "{w}: the Binance wall drawn");
    }
    pilules
}

/// 📱 Cell 390 × 844 by the finger: each wall pill writes only its amount (F1i: its price is on the price scale, in
/// the axis' format) and sits on no candle; a tap on one opens its tooltip (inside the screen), the next tap closes it.
#[test]
fn murs_pilules_prix_seul_au_doigt() {
    dossier_a_soi("murs_pilules_cell");
    let mut a = murs_aux_dernieres_bougies(cell_390());
    argos_render::actions::set_tactile(true);
    let pilules = pilules_libres(&mut a, 390, 844, true);
    let e = a.murs_ecran().unwrap();
    for (_, s) in pilules.iter().filter(|(_, s)| !s.is_empty()) {
        assert!(
            e.vue.pills.iter().any(|p| argos_render::murs::montant_court(p.usd) == *s),
            "{s:?}: a wall's amount, short"
        );
    }
    assert!(pilules.iter().any(|(_, s)| !s.is_empty()), "at least one amount pill");
    let axe = argos_render::murs::etiquettes_axe();
    // (each wall drawn has its price on the scale; walls left out for room have none)
    assert!(axe.len() >= 2, "{axe:?}");
    assert!(
        axe.iter()
            .all(|t| e.vue.pills.iter().any(|p| t.texte == argos_render::murs::prix_axe(p.price, e.step))),
        "each tag is a wall's price: {axe:?}"
    );
    // a tap on a pill: its tooltip, open after the finger is lifted
    let (r, _) = pilules[0];
    argos_render::murs::set_bulle(None);
    let (rw, th) = (a.rw(), a.top_h());
    toucher(&mut a, r.cx() + rw, r.cy() + th);
    let b = argos_render::murs::bulle().expect("tooltip open");
    let c = a.chart_with(false);
    let (_, layer) = argos_render::render_fond(&c);
    let l = layer.unwrap().murs_infobulle(b.0, b.1);
    assert!(!l.is_empty());
    let t = argos_render::text::Text::default();
    let w = l.iter().map(|s| t.width(s, 10.0) as f64).fold(0.0, f64::max) + 20.0;
    assert!(w <= 390.0 - 16.0, "the tooltip fits the phone: {w} {l:?}");
    // a tap elsewhere closes it
    toucher(&mut a, rw + 40.0, th + 300.0);
    assert!(argos_render::murs::bulle().is_none());
    argos_render::actions::set_tactile(false);
}

/// 🖥 PC 1440 × 900 and 📟 tablet 820 × 1180: no pill on a candle, a table or the app's rects; the walls all there.
#[test]
fn murs_pilules_pc_et_tablette_hors_des_bougies() {
    use argos_render::indicators::Ind;
    dossier_a_soi("murs_pilules_pc");
    argos_render::actions::set_tactile(false);
    let mut a = murs_aux_dernieres_bougies(app());
    // the multi-UT table (one of `tableaux`): the pills keep clear of it too
    a.ind.on.push(Ind::MtfTable);
    let p = pilules_libres(&mut a, 1440, 900, false);
    assert!(p.iter().any(|(_, s)| s.contains(" $")), "PC: a full pill where there is room: {p:?}");
    let mut t = murs_aux_dernieres_bougies(cell_390());
    t.area = (820, (1180.0 - t.top_h() - argos_render::ecrans::dock_h()) as u32);
    argos_render::actions::set_tactile(true);
    let _ = pilules_libres(&mut t, 820, 1180, true);
    argos_render::actions::set_tactile(false);
}

/// 🧱 [`murs_aux_dernieres_bougies`] with the spot books too: a spot wall at the perp sell wall's price (its twin) and
/// a spot buy wall alone. Returns the twin's price.
fn murs_jumeaux(mut a: App) -> (App, f64) {
    use argos_data::murs::Carnet;
    use argos_render::indicators::Ind;
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    a.chart.interval = Interval(300);
    a.ind.on.push(Ind::ArgosMurs);
    a.murs_refresh();
    let step = a.feeds.murs.as_ref().unwrap().step;
    let px = a.chart.bars.last().unwrap().close;
    let k0 = (px / step).round();
    let (sell, buy, spot_buy) = ((k0 + 2.0) * step, (k0 - 2.0) * step, (k0 - 5.0) * step);
    let mut res: Vec<(Carnet, Result<argos_core::murs::VenueBook, String>)> = Carnet::PERP
        .iter()
        .map(|c| (*c, Ok(carnet_murs(px, step, &[(sell, 60.0), (buy, 60.0)]))))
        .collect();
    res.extend(Carnet::SPOT.iter().map(|c| (*c, Ok(carnet_murs(px, step, &[(sell, 40.0), (spot_buy, 40.0)])))));
    a.murs_recu("BTC".into(), res, None, None);
    let e = a.murs_ecran().expect("walls read");
    assert!(e.vue.pills.iter().any(|p| p.price == sell), "{:?}", e.vue.pills);
    (a, sell)
}

/// 🧱 F1k / F1l / F1m (coordinator, 23 Nov, images v1): at cell 390, tablet 820 and PC 1440 — no label of the price
/// scale under a wall's price tag (« 60 800 » half under « ▼ 60 400 »); each tag writes its price exactly as the scale
/// does, the thousands apart (« 66 000 », not « 66000 »); a perp wall and a spot wall at the same price: one tag in
/// two colours (tablet and PC as on the cell).
#[test]
fn murs_etiquettes_de_l_echelle() {
    dossier_a_soi("murs_echelle");
    let gras = argos_render::text::Text::graisse(argos_render::text::Graisse::Bold);
    let t = argos_render::text::Text::default();
    for (w, h, tactile) in [(390u32, 844u32, true), (820, 1180, true), (1440, 900, false)] {
        argos_render::actions::set_tactile(false);
        let (mut a, jumeau) = murs_jumeaux(if w < 1000 { cell_390() } else { app() });
        if (560..1000).contains(&w) {
            a.area = (w, (h as f64 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
        }
        let _ = image_ecran(&mut a, w, h);
        argos_render::actions::set_tactile(tactile);
        a.murs_refresh();
        // (⇕ framed on the walls: their tags spread over the scale, between its labels — not all on the last price's)
        a.murs_cadrer();
        let c = a.chart_with(false);
        let _ = argos_render::render(&c);
        let (labels, tags) = argos_render::echelle_dessinee();
        assert!(
            !tags.is_empty(),
            "{w}: the walls' price tags drawn: {:?} murs {} labels {labels:?} pills {:?}",
            argos_render::murs::etiquettes_axe(),
            c.murs.is_some(),
            argos_render::murs::pilules_dessinees()
        );
        assert!(!labels.is_empty(), "{w}: the scale's labels drawn");
        for l in &labels {
            for g in &tags {
                assert!(!croisent(l, g), "{w}: the scale's label {l:?} under the tag {g:?}");
            }
        }
        // F1l: the price as the scale writes it, the thousands apart as on its labels
        let e = a.murs_ecran().unwrap();
        let axe = argos_render::murs::etiquettes_axe();
        for q in &axe {
            assert_eq!(q.texte, argos_render::viewport::format_price(q.price, 0), "{w}: {q:?}");
            // (the test's prices are under 1 000: the gap is checked on « 66 000 » in argos-render's own test)
            assert_eq!(q.texte.contains('\u{202f}'), q.price >= 999.5, "{w}: thousands apart in {:?}", q.texte);
            let gap = argos_render::murs::espace_milliers(&t, 12.0);
            assert!(gap >= 2.0 && gap >= gras.width("\u{202f}", 12.0) as f64, "{w}: a gap you see ({gap})");
            let tw = argos_render::murs::largeur_prix(&gras, &t, &q.texte, 12.0);
            assert!(
                tw + 8.0 + 12.0 <= a.chart.viewport.price_axis as f64 + 1.0,
                "{w}: {:?} fits the scale ({tw})",
                q.texte
            );
        }
        // F1m: the twin's tag in the perp colour and the spot colour, on every form
        let p = argos_render::indicators::values(&a.ind.set, argos_render::indicators::Ind::ArgosMurs);
        let inp = argos_script::inputs::Inputs {
            params: &argos_render::murs::PARAMS,
            vals: &p,
        };
        let cols = argos_render::murs::couleurs(&inp);
        let deux = axe
            .iter()
            .find(|q| q.price == jumeau && q.bord.is_none())
            .unwrap_or_else(|| panic!("{w}: the twin's tag {axe:?}"));
        assert_eq!(
            deux.couleurs,
            vec![argos_render::palette::on_chart(cols[0]), argos_render::palette::on_chart(cols[2])],
            "{w}: perp red and spot orange (step {})",
            e.step
        );
        assert_eq!(
            axe.iter().filter(|q| q.texte == deux.texte && q.bord.is_none()).count(),
            1,
            "{w}: one tag for the price, not one per wall"
        );
        argos_render::actions::set_tactile(false);
    }
}

/// F1h / F1i (22 Nov): the PC pills write no price (it is on the price scale). 🧱 H7 (Jee, 24 Nov: « la chart et
/// l'indicateur bloquent à cause d'Argos Murs qui crée un bloc noir »): no band is kept for Argos Murs any more —
/// with it on, the candles and every pane go up to the price scale (bars_w = plot_w) at the phone (402), the tablet
/// (820) and the PC (1440), the last candle is where it is with Argos Murs off, and the pills are drawn over the
/// chart exactly where they were with the band of before.
#[test]
fn murs_keeps_no_band_right_of_the_candles() {
    dossier_a_soi("murs_no_band");
    for (w, h, doigt) in [(402u32, 874u32, true), (820, 1180, true), (1440, 900, false)] {
        let fresh = || {
            let mut t = if doigt { cell_390() } else { app() };
            if doigt {
                t.area = (w, (h as f64 - t.top_h() - argos_render::ecrans::dock_h()) as u32);
                t.ui.form.kind = if w < 560 {
                    argos_render::formfactor::Kind::Phone
                } else {
                    argos_render::formfactor::Kind::TabletPortrait
                };
            }
            t
        };
        // Argos Murs off: where the last candle is
        argos_render::actions::set_tactile(doigt);
        let mut off = fresh();
        let _ = image_ecran(&mut off, w, h);
        argos_render::actions::set_tactile(doigt);
        let x_off = off.chart.viewport.x_of(off.chart.bars.len() - 1, &off.chart.slots);
        // Argos Murs on: no band, the last candle at the same place
        let mut t = murs_aux_dernieres_bougies(fresh());
        let _ = image_ecran(&mut t, w, h);
        argos_render::actions::set_tactile(doigt);
        t.murs_refresh();
        let vp = t.chart.viewport;
        assert!(t.murs_marge() > 0.0, "{w}: Argos Murs drawn");
        assert_eq!(vp.bars_w(), vp.plot_w(), "{w}: the candles go up to the price scale");
        let x_on = vp.x_of(t.chart.bars.len() - 1, &t.chart.slots);
        assert!((x_on - x_off).abs() < 1e-9, "{w}: last candle at {x_on}, {x_off} with Argos Murs off");
        let _ = argos_render::render(&t.chart_with(false));
        let now = argos_render::murs::pilules_dessinees();
        assert!(now.len() >= 2, "{w}: {now:?}");
        let e = t.murs_ecran().unwrap();
        if !doigt {
            for (_, s) in &now {
                assert!(
                    !e.vue.pills.iter().any(|p| s.contains(&argos_render::murs::prix_axe(p.price, e.step))),
                    "PC: no price in {s:?}"
                );
            }
        }
        // the band of before (the widest pill and the depth bars): the pills were drawn at the same place
        let vals = t.murs_vals();
        let p = argos_script::inputs::Inputs {
            params: &argos_render::murs::PARAMS,
            vals: &vals,
        };
        let band = argos_render::murs::marge_juste(&e, &p, vp.plot_w(), t.murs_marge(), doigt);
        t.chart.viewport.marge = band;
        let _ = argos_render::render(&t.chart_with(false));
        let before = argos_render::murs::pilules_dessinees();
        assert_eq!(now, before, "{w}: the pills stay where they were");
        for (r, s) in &now {
            assert!(
                r.x + r.w <= vp.plot_w() + 0.5 && r.x >= vp.plot_w() - t.murs_marge() - 1.0,
                "{w}: {s:?} {r:?} against the price scale"
            );
        }
        argos_render::actions::set_tactile(false);
    }
}

/// ≫ F1j (Jee's iPad, 22 Nov: 46 % of the plot in the future, no way back by the finger): dragged far right the empty
/// space stops at 5 candles + 25 % of the candles' width; off live the ≫ button shows (on no candle), a click on it
/// goes back to live; a double click on the time axis too; live: no button.
#[test]
fn direct_bouton_et_double_clic_axe_du_temps() {
    dossier_a_soi("f1j_direct");
    argos_render::actions::set_tactile(false);
    let mut a = murs_aux_dernieres_bougies(app());
    let _ = image_ecran(&mut a, 1440, 900);
    argos_render::actions::set_tactile(false);
    let live = argos_render::viewport::Viewport::default().right_offset;
    a.chart.viewport.right_offset = live;
    let _ = argos_render::render(&a.chart_with(false));
    assert!(argos_render::bouton_direct().is_none(), "live: no ≫");
    let s = a.chart.slots.clone();
    a.chart.viewport.scroll_px(-1e9, &s);
    let vp = a.chart.viewport;
    assert!((vp.right_offset - vp.futur_max()).abs() < 1e-6, "{} / {}", vp.right_offset, vp.futur_max());
    let _ = argos_render::render(&a.chart_with(false));
    let r = argos_render::bouton_direct().expect("≫ off live");
    for b in argos_render::murs::bougies() {
        assert!(!croisent(&r, &b), "≫ {r:?} on the candle {b:?}");
    }
    assert!(argos_render::murs::zones_dessinees().iter().all(|z| !croisent(&r, z)), "≫ on a pill");
    let (rw, th) = (a.rw(), a.top_h());
    a.mouse_moved(PhysicalPosition::new(r.cx() + rw, r.cy() + th));
    a.left_button(ElementState::Pressed);
    a.left_button(ElementState::Released);
    assert_eq!(a.chart.viewport.right_offset, live, "≫: back to live");
    // double click on the time axis
    a.chart.viewport.scroll_px(-1e9, &s);
    let (x, y) = (300.0 + rw, vp.plot_h() + vp.time_axis as f64 / 2.0 + th);
    for _ in 0..2 {
        a.mouse_moved(PhysicalPosition::new(x, y));
        a.left_button(ElementState::Pressed);
        a.left_button(ElementState::Released);
    }
    assert_eq!(a.chart.viewport.right_offset, live, "double click on the time axis: back to live");
}

/// 👆 F1j: by the finger an Argos Murs tooltip opens only on a real tap on a pill (never where a drag ends), closes
/// after 4 s and with a drag; placed under the pill against the price scale, never on a candle — 390, 820, 1024.
#[test]
fn murs_bulle_au_vrai_toucher() {
    use winit::event::TouchPhase;
    dossier_a_soi("f1j_bulle");
    for (w, h) in [(390u32, 844u32), (820, 1180), (1024, 1366)] {
        let mut a = murs_aux_dernieres_bougies(cell_390());
        a.area = (w, (h as f64 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
        let _ = image_ecran(&mut a, w, h);
        argos_render::actions::set_tactile(true);
        let _ = argos_render::render(&a.chart_with(false));
        let pilules = argos_render::murs::pilules_dessinees();
        let (r, _) = pilules.first().cloned().expect("a pill");
        let (rw, th) = (a.rw(), a.top_h());
        let (px, py) = (r.cx() + rw, r.cy() + th);
        argos_render::murs::set_bulle(None);
        // a drag that ends on the pill: nothing opens (60 points: not a swipe to the right)
        a.touch(3, TouchPhase::Started, (px - 60.0, py));
        for k in 1..=6 {
            a.touch(3, TouchPhase::Moved, (px - 60.0 + k as f64 * 10.0, py));
        }
        a.touch(3, TouchPhase::Ended, (px, py));
        assert!(argos_render::murs::bulle().is_none(), "{w}: a drag opens nothing");
        // a tap: open; 4 s later: closed
        toucher(&mut a, px, py);
        assert!(argos_render::murs::bulle().is_some(), "{w}: a tap opens it");
        let t = argos_render::journal::now_ms();
        assert!(
            argos_render::murs::bulle_a(t + argos_render::murs::BULLE_MS + 500).is_none(),
            "{w}: closed after 4 s"
        );
        // a drag closes it
        a.touch(4, TouchPhase::Started, (px - 200.0, py + 40.0));
        for k in 1..=6 {
            a.touch(4, TouchPhase::Moved, (px - 200.0 - k as f64 * 20.0, py + 40.0));
        }
        a.touch(4, TouchPhase::Ended, (px - 320.0, py + 40.0));
        assert!(argos_render::murs::bulle().is_none(), "{w}: a drag closes it");
        // placed under / above the pill, against the price scale, never on a candle
        let _ = argos_render::render(&a.chart_with(false));
        let vp = a.chart.viewport;
        for (r, _) in argos_render::murs::pilules_dessinees() {
            let b = argos_render::murs::cadre_toucher(190.0, 96.0, r, vp.plot_w(), vp.plot_h());
            for c in argos_render::murs::bougies() {
                assert!(!croisent(&b, &c), "{w}: tooltip {b:?} on the candle {c:?}");
            }
            assert!(b.x + b.w <= vp.plot_w() - 3.5, "{w}: against the price scale");
        }
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 R4: the multi-UT table, in its default corner with Argos Murs on, sits above Argos Murs' bottom pills (and its
/// wall pills), never under them — PC 1440 × 900 and cell 390 × 844.
#[test]
fn table_multi_ut_au_dessus_des_pilules_d_argos_murs() {
    use argos_render::indicators::Ind;
    dossier_a_soi("murs_multi_ut");
    for (cell, w, h) in [(false, 1440u32, 900u32), (true, 390, 844)] {
        let mut a = murs_aux_dernieres_bougies(if cell { cell_390() } else { app() });
        argos_render::actions::set_tactile(cell);
        a.ind.on.push(Ind::MtfTable);
        let _ = image_ecran(&mut a, w, h);
        let c = a.chart_with(false);
        let _ = argos_render::render(&c);
        let tables = argos_render::indicators::tableaux();
        assert!(!tables.is_empty(), "{w}: the multi-UT table is drawn");
        let murs = argos_render::murs::zones_dessinees();
        assert!(murs.len() >= 2, "{w}: Murs drew its pills");
        for t in &tables {
            for m in &murs {
                assert!(!croisent(t, m), "{w}: table {t:?} on the Murs pill {m:?}");
            }
        }
        argos_render::actions::set_tactile(false);
    }
}

/// ⇕ « Cadrer sur les murs » on the cell (Jee, 22 Nov: « il ne me fonctionne pas »): a tap frames the walls, says so
/// in a short notice and closes the action bar; nothing to frame: the notice says why, one sentence per case.
#[test]
fn murs_cadrer_dit_ce_qu_il_fait() {
    use argos_render::indicators::Ind;
    use argos_render::indleg::{Cell, LegClick};
    dossier_a_soi("murs_cadrer");
    let mut a = murs_aux_dernieres_bougies(cell_390());
    argos_render::actions::set_tactile(true);
    (a.ind.leg.0, a.ind.sel) = (true, Some(Ok(Ind::ArgosMurs)));
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let t = argos_render::text::Text::default();
    let cell = Cell {
        actif: true,
        sel: a.ind.sel,
        pw,
    };
    let z = argos_render::indleg::zones_de(&t, &a.ind.on, &a.ind.out, a.ind.leg.0, ph, pw, cell);
    let (r, _) = z.iter().find(|(_, k)| matches!(k, LegClick::Cadrer(_))).expect("⇕ in the action bar");
    a.banner = None;
    // (the button under the finger at its place, then what its tap does)
    let k = a.legende_bouton(r.cx(), r.cy()).expect("a legend button there");
    assert!(matches!(k, LegClick::Cadrer(_)));
    a.legend_click(k);
    let b = a.banner.clone().unwrap_or_default();
    assert!(b.starts_with("Murs cadrés : "), "{b:?}");
    assert!(a.ind.sel.is_none(), "the action bar closed");
    let (lo, hi) = a.chart.settings.scale.manual.expect("framed");
    let e = a.murs_ecran().unwrap();
    for p in &e.vue.pills {
        assert!((lo..=hi).contains(&p.price), "{} in {lo}..{hi}", p.price);
    }
    argos_render::actions::set_tactile(false);
    // the reasons
    let raison = |a: &mut App| {
        a.banner = None;
        a.murs_cadrer();
        a.banner.clone().unwrap_or_default()
    };
    a.ind.out.hidden.push(Ind::ArgosMurs);
    assert_eq!(raison(&mut a), "Argos Murs est masqué");
    a.ind.out.hidden.clear();
    a.chart.interval = Interval(86_400);
    assert_eq!(raison(&mut a), "Argos Murs se cache au-dessus de 4 h : passe en 4 h ou moins");
    dossier_a_soi("murs_cadrer_rien");
    let mut n = app();
    n.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTCUSDT".into(),
    };
    n.chart.interval = Interval(300);
    n.ind.on.push(Ind::ArgosMurs);
    assert_eq!(raison(&mut n), "Carnets pas encore lus : réessaie dans un instant");
    n.murs_refresh();
    assert_eq!(raison(&mut n), "Carnets pas encore lus : réessaie dans un instant");
    let step = n.feeds.murs.as_ref().unwrap().step;
    let px = n.chart.bars.last().unwrap().close;
    let rien: Vec<(argos_data::murs::Carnet, Result<argos_core::murs::VenueBook, String>)> = argos_data::murs::Carnet::PERP
        .iter()
        .map(|c| {
            // (a book whose every level is 5 % away or more: no wall within ±3 %)
            let loin = argos_core::murs::VenueBook {
                venue: "essai".into(),
                asks: (0..5).map(|k| (((px * 1.05) / step).round() * step + k as f64 * step, 1.0)).collect(),
                bids: (0..5).map(|k| (((px * 0.95) / step).round() * step - k as f64 * step, 1.0)).collect(),
            };
            (*c, Ok(loin))
        })
        .collect();
    n.murs_recu("BTC".into(), rien, None, None);
    let avant = n.chart.settings.scale.manual;
    assert_eq!(raison(&mut n), "Aucun mur à ±3 % du prix");
    assert_eq!(n.chart.settings.scale.manual, avant, "nothing framed");
}

/// F1 (22 Nov, images cell-cadrer-apres-v3 / cell-cadrer-raison-v3): on the cell the ⇕ notice sat on the
/// « O … H … L … C … » line and its long reason was cut with « … ». Now, for every sentence it can say: under the
/// legend's lines (never on the title, O H L C, Vol or « ⌄ N »), two lines at most inside the screen less 32 points,
/// the whole text written.
#[test]
fn avis_cadrer_sous_la_legende_au_doigt() {
    dossier_a_soi("avis_cadrer_390");
    let mut a = murs_aux_dernieres_bougies(cell_390());
    argos_render::actions::set_tactile(true);
    let t = argos_render::text::Text::default();
    let raisons = [
        "Murs cadrés : 64 850 à 65 320",
        "Argos Murs n'est pas sur la chart",
        "Argos Murs est masqué",
        "Argos Murs se cache au-dessus de 4 h : passe en 4 h ou moins",
        "Pas de carnet public pour ce symbole",
        "Carnets pas encore lus : réessaie dans un instant",
        "Pas encore de prix : réessaie dans un instant",
        "Aucun mur à ±3 % du prix",
        "Rien à cadrer : réessaie dans un instant",
    ];
    let c = a.to_chart();
    let _ = argos_render::render(&c);
    let legende = argos_render::legende_zones(&c);
    assert!(legende.len() >= 2, "title and O H L C at least: {legende:?}");
    let ww = a.chart.viewport.width as f64;
    let croise = |p: &argos_render::shapes::Rect, q: &argos_render::shapes::Rect| p.x < q.x + q.w && q.x < p.x + p.w && p.y < q.y + q.h && q.y < p.y + p.h;
    // F1n (cell-cadrer-apres-v1: « ▲ 26 M » cut by the notice): never on a wall pill
    let pilules = argos_render::murs::pilules_dessinees();
    assert!(!pilules.is_empty(), "the walls' pills drawn");
    for r in raisons {
        let (b, lignes, size) = argos_render::banner_lignes(&a.chart.viewport, &t, r);
        assert!((1..=2).contains(&lignes.len()), "{r}: {lignes:?}");
        assert_eq!(lignes.join(" "), r, "the whole text, never « … »");
        for l in &lignes {
            assert!(t.width(l, size) as f64 <= b.w - 34.0 + 0.5, "{r}: {l:?} in {b:?}");
        }
        assert!(b.x >= 15.5 && b.x + b.w <= ww - 15.5, "{r}: inside the screen less 32: {b:?}");
        for z in &legende {
            assert!(!croise(z, &b), "{r}: {b:?} on the legend {z:?}");
        }
        for (p, s) in &pilules {
            assert!(!croise(p, &b), "{r}: {b:?} on the pill {s:?} {p:?}");
        }
    }
    // the framed range's notice itself, after a real ⇕ (the walls framed, so the pills spread over the height)
    a.murs_cadrer();
    let c = a.to_chart();
    let _ = argos_render::render(&c);
    let b = a.banner.clone().expect("the notice");
    let (r, lignes, _) = argos_render::banner_lignes(&a.chart.viewport, &t, &b);
    assert_eq!(lignes.join(" "), b, "never cut");
    assert!(lignes.len() <= 2, "{lignes:?}");
    for (p, s) in &argos_render::murs::pilules_dessinees() {
        assert!(!croise(p, &r), "{r:?} on the pill {s:?} {p:?}");
    }
    // the legend's list unfolded (« ⌄ 3 » touched): the notice goes under its rows (image cell-cadrer-raison-v1)
    a.ind.leg.0 = true;
    let _ = image_ecran(&mut a, 390, 844);
    argos_render::actions::set_tactile(true);
    let bas = argos_render::indleg::bas_legende(&a.ind.out, true, true);
    for r in raisons {
        let (b, _, _) = argos_render::banner_lignes(&a.chart.viewport, &t, r);
        assert!(b.y >= bas, "{r}: {b:?} on the unfolded legend (its bottom {bas})");
    }
    argos_render::actions::set_tactile(false);
}

/// 📋🧱 Coordinator's rule (22 Nov): with Argos Murs on, the Argos TVD Pro dashboard (default: top right) goes to the
/// top left, below the legend's lines, under no pill and no app rect — PC 1440 × 900 and cell 390 × 844 by the
/// finger; Argos Murs off: back at the top right.
#[test]
fn tableau_tvd_a_gauche_avec_argos_murs() {
    use argos_render::indicators::Ind;
    dossier_a_soi("murs_tvd");
    for (cell, w, h) in [(false, 1440u32, 900u32), (true, 390, 844)] {
        let mut a = murs_aux_dernieres_bougies(if cell { cell_390() } else { app() });
        if !a.ind.on.contains(&Ind::Tvd) {
            a.ind.on.push(Ind::Tvd);
        }
        let _ = image_ecran(&mut a, w, h);
        argos_render::actions::set_tactile(cell);
        a.ind.key = Default::default();
        let c = a.chart_with(false);
        let _ = argos_render::render(&c);
        let tables = argos_render::indicators::tableaux();
        assert!(!tables.is_empty(), "{w}: the TVD Pro dashboard drawn");
        let pw = c.viewport.plot_w();
        let legende = argos_render::legende_zones(&c);
        let pilules: Vec<Rect> = argos_render::murs::pilules_dessinees().into_iter().map(|(r, _)| r).collect();
        for t in &tables {
            assert!(t.x <= 12.0, "{w}: at the left corner with the walls: {t:?} (plot {pw})");
            for r in legende.iter().chain(&c.garder).chain(&pilules).chain(&argos_render::murs::zones_dessinees()) {
                assert!(!croisent(t, r), "{w}: table {t:?} on {r:?}");
            }
        }
        // Argos Murs off: the dashboard back at its own corner (top right)
        a.ind.on.retain(|i| *i != Ind::ArgosMurs);
        let _ = image_ecran(&mut a, w, h);
        argos_render::actions::set_tactile(cell);
        let c = a.chart_with(false);
        let _ = argos_render::render(&c);
        let t = argos_render::indicators::tableaux();
        assert!(t.iter().any(|t| t.x + t.w >= pw - 12.0), "{w}: back at the right corner: {t:?} (plot {pw})");
        argos_render::actions::set_tactile(false);
    }
}

/// 📅🧱 Jee (22 Nov, iPhone): Argos Murs' bottom pills covered the economic calendar's flags. With rounds at the
/// bottom of the price pane, no Murs pill touches one — cell 390 × 844, tablet 820 × 1180, PC 1440 × 900.
#[test]
fn pilules_d_argos_murs_au_dessus_des_drapeaux() {
    dossier_a_soi("murs_drapeaux");
    for (cell, w, h) in [(true, 390u32, 844u32), (true, 820, 1180), (false, 1440, 900)] {
        let mut a = murs_aux_dernieres_bougies(if cell { cell_390() } else { app() });
        if w == 820 {
            a.area = (820, (1180.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
        }
        let _ = image_ecran(&mut a, w, h);
        argos_render::actions::set_tactile(cell);
        let mut c = a.chart_with(false);
        let n = c.bars.len();
        c.events = [n - 3, n - 12, n - 25]
            .iter()
            .zip(["EUR", "GBP", "USD"])
            .map(|(&i, cur)| argos_render::econ::Mark {
                ts: c.bars[i].time,
                currency: cur.into(),
                impact: 2,
                title: "Essai".into(),
                numbers: String::new(),
                window_s: 0,
                ..Default::default()
            })
            .collect();
        let _ = argos_render::render(&c);
        let ronds = argos_render::econ::ronds_dessines();
        assert!(!ronds.is_empty(), "{w}: the rounds drawn");
        let murs = argos_render::murs::zones_dessinees();
        assert!(murs.len() >= 2, "{w}: Murs drawn");
        for r in &ronds {
            for m in &murs {
                assert!(!croisent(r, m), "{w}: round {r:?} under the Murs pill {m:?}");
            }
        }
        argos_render::actions::set_tactile(false);
    }
}

/// 📸 Base of the multi-exchange footprint mock-up (not the feature): today's Footprint on a BTC 1 min chart with real
/// minutes (zoomed so the numbers show), 🖥 1440 × 900 and 📱 402 × 874; the mock-up adds its new parts over it. Only
/// when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_base_footprint_multi() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    for phone in [false, true] {
        dossier_a_soi(if phone { "captures_fpm_cell" } else { "captures_fpm_pc" });
        let mut a = if phone { cell_pilule() } else { pc_avec_volet(wl::Tool::Trading) };
        if phone {
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
            a.ui.depart_vu = true;
        } else {
            a.watch.prefs.open = false;
        }
        a.top.prefs.bienvenue_vue = true;
        let t0 = (now_s() / 60 - 140) * 60;
        let prix = |i: i64| {
            let x = i as f64;
            ((62_040.0 + 150.0 * (x / 9.0).sin() + 40.0 * (x * 1.3).sin()) * 10.0).round() / 10.0
        };
        a.chart.raw = (0..140)
            .map(|i| Bar {
                time: t0 + i * 60,
                open: prix(i),
                high: prix(i).max(prix(i + 1)) + 22.0 + 10.0 * ((i * 7 % 5) as f64),
                low: prix(i).min(prix(i + 1)) - 20.0 - 9.0 * ((i * 3 % 4) as f64),
                close: prix(i + 1),
                volume: 60.0,
            })
            .collect();
        a.chart.source = Source::Live {
            exchange: argos_data::Exchange::Binance,
            base: "BTC".into(),
        };
        (a.chart.interval, a.chart.file_interval) = (Interval(60), Interval(60));
        a.title = argos_data::Exchange::Binance.chart_title("BTC", "1m");
        // real minutes for the last 30 bars: rows of 5 $ from the low to the high, more volume near the close
        let pas = 5.0;
        let minutes: Vec<argos_core::orderflow::FlowBar> = a.chart.raw[110..]
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let (r0, r1) = ((b.low / pas).floor() as i64, (b.high / pas).floor() as i64);
                let rangees = (r0..=r1)
                    .map(|r| {
                        let p = r as f64 * pas;
                        let pres = (-((p - b.close) / 40.0).powi(2)).exp();
                        let haut = b.close >= b.open;
                        let s = ((r * 37 + k as i64 * 11) % 13) as f64 / 13.0;
                        let a_ = (0.4 + 6.0 * pres + 1.8 * s) * if haut { 1.25 } else { 0.8 };
                        let v_ = (0.4 + 6.0 * pres + 1.8 * (1.0 - s)) * if haut { 0.8 } else { 1.25 };
                        (r, (a_ * 10.0).round() / 10.0, (v_ * 10.0).round() / 10.0)
                    })
                    .collect();
                argos_core::orderflow::FlowBar {
                    time: b.time,
                    pas,
                    rangees,
                    complete: k > 0,
                }
            })
            .collect();
        a.feeds.orderflow = vec![crate::actions::orderflow::FluxReel {
            cle: argos_core::orderflow::Cle {
                exchange: "binance".into(),
                symbole: "BTC".into(),
            },
            agg: argos_core::orderflow::Agregateur::new(Some(pas)),
            minutes,
            recus: (0, 0),
        }];
        a.ind.on = vec![argos_render::indicators::Ind::Footprint];
        a.ui.frames = 5;
        a.refresh_indicators();
        a.rebuild();
        a.start_free();
        a.chart.viewport.bar_spacing = if phone { 66.0 } else { 86.0 };
        a.chart.viewport.right_offset = if phone { 0.6 } else { 1.2 };
        a.chart.settings.scale.manual = Some(if phone { (61_975.0, 62_235.0) } else { (61_965.0, 62_330.0) });
        // (the mouse away from the floating bar: it hides, as on the PC)
        a.pointer.mouse = Some((1_200.0, 820.0));
        a.tools.retracted = true;
        a.demo = argos_core::portfolio::Portfolio::default();
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        let _ = image_ecran(&mut a, w, h);
        let (img, _) = image_ecran(&mut a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/base-footprint-{}.png", if phone { "cell" } else { "pc" })).unwrap();
    }
}

// — 🏦 footprint multi-bourses —

/// 🏦 Test scene: a BTC 1 min chart of Binance perps (140 bars ending at the last minute) with the Footprint on, and
/// real minutes for its last 30 bars on each exchange of `bourses`: (exchange, share of Binance's volume, price gap
/// (its prices = Binance's + the gap), size of a contract (1: sizes in coins)). `coupee`: (exchange, bars from the
/// end) whose minute is cut short. The markets are put in place directly (nothing read from the cache).
fn fp_scene(a: &mut App, bourses: &[(Exchange, f64, f64, f64)], coupee: Option<(Exchange, usize)>) {
    let t0 = (now_s() / 60 - 140) * 60;
    let prix = |i: i64| {
        let x = i as f64;
        ((62_040.0 + 150.0 * (x / 9.0).sin() + 40.0 * (x * 1.3).sin()) * 10.0).round() / 10.0
    };
    a.chart.raw = (0..140)
        .map(|i| Bar {
            time: t0 + i * 60,
            open: prix(i),
            high: prix(i).max(prix(i + 1)) + 22.0 + 10.0 * ((i * 7 % 5) as f64),
            low: prix(i).min(prix(i + 1)) - 20.0 - 9.0 * ((i * 3 % 4) as f64),
            close: prix(i + 1),
            volume: 60.0,
        })
        .collect();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "BTC".into(),
    };
    (a.chart.interval, a.chart.file_interval) = (Interval(60), Interval(60));
    a.title = Exchange::Binance.chart_title("BTC", "1m");
    let pas = 5.0;
    let n = a.chart.raw.len();
    a.feeds.orderflow = bourses
        .iter()
        .map(|&(ex, part, ecart, ctval)| {
            let minutes = a.chart.raw[n - 30..]
                .iter()
                .enumerate()
                .map(|(k, b)| {
                    let (r0, r1) = (((b.low + ecart) / pas).floor() as i64, ((b.high + ecart) / pas).floor() as i64);
                    let rangees = (r0..=r1)
                        .map(|r| {
                            let p = r as f64 * pas - ecart;
                            let pres = (-((p - b.close) / 40.0).powi(2)).exp();
                            let haut = b.close >= b.open;
                            let s = ((r * 37 + k as i64 * 11) % 13) as f64 / 13.0;
                            let a_ = (0.4 + 6.0 * pres + 1.8 * s) * if haut { 1.25 } else { 0.8 };
                            let v_ = (0.4 + 6.0 * pres + 1.8 * (1.0 - s)) * if haut { 0.8 } else { 1.25 };
                            let q = |x: f64| (x * part / ctval * 10.0).round() / 10.0;
                            (r, q(a_), q(v_))
                        })
                        .collect();
                    argos_core::orderflow::FlowBar {
                        time: b.time,
                        pas,
                        rangees,
                        complete: k > 0 && coupee != Some((ex, 30 - k)),
                    }
                })
                .collect();
            crate::actions::orderflow::FluxReel {
                cle: argos_core::orderflow::Cle {
                    exchange: ex.id().into(),
                    symbole: "BTC".into(),
                },
                agg: argos_core::orderflow::Agregateur::new(Some(pas)),
                minutes,
                recus: (0, 0),
            }
        })
        .collect();
    a.ind.on = vec![argos_render::indicators::Ind::Footprint];
}

/// 🏦 The five exchanges of the mock-up (Binance, Bybit, OKX in contracts of 0,01 BTC, Coinbase 12 $ higher,
/// Hyperliquid 3 $ lower).
const FP_CINQ: [(Exchange, f64, f64, f64); 5] = [
    (Exchange::Binance, 1.0, 0.0, 1.0),
    (Exchange::Bybit, 0.45, 0.0, 1.0),
    (Exchange::Okx, 0.33, 0.0, 0.01),
    (Exchange::Coinbase, 0.22, 12.0, 1.0),
    (Exchange::Hyperliquid, 0.16, -3.0, 1.0),
];

/// Recalcule les indicateurs tout de suite.
fn fp_recalcul(a: &mut App) {
    a.ui.frames = 5;
    a.rebuild();
    a.ind.key = Default::default();
    a.refresh_indicators();
}

/// 🏦 Footprint on, a Binance BTC chart: its market and, in « Auto », the 4 biggest other exchanges are followed
/// (one key per exchange and coin); unticking every other exchange follows the chart's market alone.
#[test]
fn footprint_multi_suit_les_bourses_d_auto() {
    let mut a = app();
    a.chart.source = Source::Live {
        exchange: Exchange::Binance,
        base: "FPSUIVI".into(),
    };
    a.orderflow_suivre();
    assert_eq!(a.feeds.orderflow.len(), 1, "Footprint off: the chart's market only");
    a.ind.on = vec![argos_render::indicators::Ind::Footprint];
    a.orderflow_suivre();
    let suivis: Vec<(String, String)> = a.feeds.orderflow.iter().map(|f| (f.cle.exchange.clone(), f.cle.symbole.clone())).collect();
    let attendu: Vec<(String, String)> = [Exchange::Binance, Exchange::Bybit, Exchange::Okx, Exchange::Coinbase, Exchange::Hyperliquid]
        .iter()
        .map(|e| (e.id().to_string(), "FPSUIVI".to_string()))
        .collect();
    assert_eq!(suivis, attendu);
    // Hyperliquid through the book's connection (its pieces), the others on their own connection
    assert_eq!(a.orderflow_pieces(Exchange::Hyperliquid), vec!["FPSUIVI".to_string()]);
    // unticked: the chart's exchange alone
    let mut v = argos_render::indicators::values(&a.ind.set, argos_render::indicators::Ind::Footprint);
    let k = |c: &str| argos_script::indicators::flow::FOOTPRINT_PARAMS.iter().position(|p| p.key == c).unwrap();
    v[k("auto")] = 0.0;
    for c in argos_script::indicators::flow::FOOTPRINT_CASES {
        v[k(c)] = 0.0;
    }
    a.ind.set.insert(argos_render::indicators::Ind::Footprint, v);
    a.orderflow_suivre();
    assert_eq!(a.feeds.orderflow.len(), 1, "every other exchange unticked: the chart's market only");
    assert!(a.fp_cles().is_empty(), "one exchange: today's footprint");
    effacer_series(&["binance-fpsuivi.", "bybit-fpsuivi.", "okx-fpsuivi.", "coinbase-fpsuivi.", "hl-fpsuivi."]);
}

/// 🏦 The real trades of 5 exchanges summed price by price in the Footprint: each complete bar is real, its rows hold
/// the volume of all of them (OKX's contracts in BTC), the parts are each exchange's volume, Coinbase's prices are
/// moved onto Binance's (gap read from the trades), a bar where one exchange missed a bit keeps what was received
/// (orange) and says which one. The CVD stays on the chart's exchange. In $: sizes × price.
#[test]
fn footprint_multi_additionne_les_vrais_trades() {
    use argos_render::indicators::Ind;
    use argos_script::indicators::flow::Origine;
    argos_data::murs::okx_ctval_noter("BTC", 0.01);
    let mut a = app();
    fp_scene(&mut a, &FP_CINQ, Some((Exchange::Hyperliquid, 2)));
    fp_recalcul(&mut a);
    let out = &a.ind.out;
    let m = out.fp_multi.clone().expect("multi-exchange footprint");
    let noms: Vec<&str> = m.bourses.iter().map(|b| b.0.as_str()).collect();
    assert_eq!(noms, ["Binance", "Bybit", "OKX", "Coinbase", "Hyperliquid"]);
    assert_eq!(argos_render::indicators::legend_title(Ind::Footprint, out), "Argos Footprint · 5 bourses");
    // Coinbase 12 $ higher: moved down by about 12 $ (within one row of 5 $)
    assert!((m.ecarts[3] + 12.0).abs() < 2.5, "{:?}", m.ecarts);
    assert!((m.ecarts[4] - 3.0).abs() < 2.5, "{:?}", m.ecarts);
    let f = out.footprint.as_ref().unwrap();
    let n = a.chart.bars.len();
    let volume_de = |ex: Exchange, t: i64| -> f64 {
        let k = if ex == Exchange::Okx { 0.01 } else { 1.0 };
        let fl = a.feeds.orderflow.iter().find(|f| f.cle.exchange == ex.id()).unwrap();
        fl.minutes
            .iter()
            .find(|b| b.time == t)
            .map_or(0.0, |b| b.rangees.iter().map(|r| (r.1 + r.2) * k).sum())
    };
    for back in [5usize, 2] {
        let i = n - back;
        let t = a.chart.bars[i].time;
        let b = f.bars.iter().find(|b| b.i == i).unwrap();
        let total: f64 = b.rows.iter().map(|r| r.1 + r.2).sum();
        let attendu: f64 = FP_CINQ.iter().map(|x| volume_de(x.0, t)).sum();
        assert!((total - attendu).abs() < 1e-6, "bar {back} from the end: {total} vs {attendu}");
        let parts = m.parts_de(t).unwrap();
        assert!((parts.iter().sum::<f64>() - attendu).abs() < 1e-6);
        assert!((parts[2] - volume_de(Exchange::Okx, t)).abs() < 1e-6, "OKX in BTC");
        if back == 2 {
            assert_eq!(b.origine, Origine::Incomplet, "Hyperliquid cut short: orange, with what was received");
            assert_eq!(m.incompletes(t), vec!["Hyperliquid"]);
        } else {
            assert_eq!(b.origine, Origine::Reel);
        }
    }
    // the CVD and the bubbles: the chart's exchange only
    let t = a.chart.bars[n - 5].time;
    let cvd_bar = out.reel.iter().find(|b| b.time == t).unwrap();
    assert!((cvd_bar.rangees.iter().map(|r| r.1 + r.2).sum::<f64>() - volume_de(Exchange::Binance, t)).abs() < 1e-6);
    // in $: each row × its price
    let attendu: f64 = FP_CINQ.iter().map(|x| volume_de(x.0, t)).sum::<f64>() * a.chart.bars[n - 5].close;
    let mut v = argos_render::indicators::values(&a.ind.set, Ind::Footprint);
    let k = argos_script::indicators::flow::FOOTPRINT_PARAMS.iter().position(|p| p.key == "unite").unwrap();
    v[k] = 1.0;
    a.ind.set.insert(Ind::Footprint, v);
    fp_recalcul(&mut a);
    let f = a.ind.out.footprint.as_ref().unwrap();
    let b = f.bars.iter().find(|b| b.i == n - 5).unwrap();
    let total: f64 = b.rows.iter().map(|r| r.1 + r.2).sum();
    assert!((total / attendu - 1.0).abs() < 0.01, "{total} $ vs {attendu} $");
    assert_eq!(a.ind.out.fp_multi.as_ref().unwrap().unite, "$");
}

/// 🏦 The row under the mouse: its bubble gives each exchange's sells and buys (in the order of the legend), their sum
/// is the cell's; an empty place gives none.
#[test]
fn footprint_multi_bulle_du_survol() {
    argos_data::murs::okx_ctval_noter("BTC", 0.01);
    let mut a = app();
    fp_scene(&mut a, &FP_CINQ, None);
    fp_recalcul(&mut a);
    a.chart.viewport.bar_spacing = 86.0;
    a.chart.viewport.right_offset = 1.2;
    a.chart.settings.scale.manual = Some((61_965.0, 62_330.0));
    let n = a.chart.bars.len();
    let f = a.ind.out.footprint.clone().unwrap();
    let b = f.bars.iter().find(|b| b.i == n - 3).unwrap().clone();
    let row = *b.rows.iter().max_by(|x, y| (x.1 + x.2).total_cmp(&(y.1 + y.2))).unwrap();
    let c = a.chart_with(false);
    let (_, ret) = argos_render::render_fond(&c);
    let ret = ret.unwrap();
    let fr = a.frame_now().unwrap();
    let g = fr.geo();
    let (x, y) = (g.x_of(a.chart.bars[n - 3].time), g.y_of((row.0 as f64 + 0.5) * f.step));
    let bulle = ret.fp_bulle(x, y).expect("a bubble on the row");
    assert_eq!(bulle.lignes.len(), 5);
    assert_eq!(bulle.lignes[0].0, "Binance");
    let (ventes, achats): (f64, f64) = bulle.lignes.iter().fold((0.0, 0.0), |s, l| (s.0 + l.2, s.1 + l.3));
    assert!(
        (achats - row.1).abs() < 1e-6 && (ventes - row.2).abs() < 1e-6,
        "the cell's {row:?} = the sum of the exchanges"
    );
    assert!(bulle.heure.starts_with("1 min · "), "{}", bulle.heure);
    assert!(ret.fp_bulle(x, 5.0).is_none(), "nothing at the top of the chart");
}

/// 🏦 The Footprint's ⚙ (legend): the exchanges panel opens; a chip in « Auto » keeps what is shown and adds the
/// exchange; the chart's exchange stays; $ and the switches change their setting; « ⚙ Tous les réglages » opens the
/// full window and closes the panel.
#[test]
fn footprint_multi_engrenage_des_bourses() {
    use argos_render::fpmulti::Clic;
    use argos_render::indicators::Ind;
    let mut a = app();
    fp_scene(&mut a, &FP_CINQ, None);
    fp_recalcul(&mut a);
    a.ind.leg.0 = true;
    a.legend_click(argos_render::indleg::LegClick::Settings(Ind::Footprint));
    assert!(a.ui.fp_reglages, "the panel opens");
    assert!(a.ui.ind_dlg.is_none(), "not the full window");
    let text = argos_render::text::Text::default();
    let clic = |a: &mut App, c: Clic| {
        let (r, q) = a.fp_place(&text).expect("panel shown");
        let z = r.zones(&text, q.w, a.top.phone).into_iter().find(|(_, x)| *x == c).expect("zone").0;
        let (x, y) = (q.x + z.cx(), q.y + z.cy());
        assert!(a.fp_sous(x, y));
        a.fp_click(x, y);
    };
    let vals = |a: &App| argos_render::indicators::values(&a.ind.set, Ind::Footprint);
    let on = |a: &App, c: &str| {
        let k = argos_script::indicators::flow::FOOTPRINT_PARAMS.iter().position(|p| p.key == c).unwrap();
        vals(a)[k]
    };
    clic(&mut a, Clic::Bourse(5)); // ＋ Bitget
    assert_eq!(on(&a, "auto"), 0.0, "a chip: « Auto » off");
    for c in ["exBinance", "exBybit", "exOkx", "exCoinbase", "exHyperliquid", "exBitget"] {
        assert_eq!(on(&a, c), 1.0, "{c}");
    }
    clic(&mut a, Clic::Bourse(0)); // Binance: the chart's exchange
    assert_eq!(on(&a, "exBinance"), 1.0, "the chart's exchange stays");
    assert!(a.banner.as_deref().is_some_and(|b| b.contains("bourse de la chart")));
    clic(&mut a, Clic::Bourse(1)); // Bybit off
    assert_eq!(on(&a, "exBybit"), 0.0);
    clic(&mut a, Clic::Unite(true));
    assert_eq!(on(&a, "unite"), 1.0);
    clic(&mut a, Clic::Aligne);
    assert_eq!(on(&a, "aligne"), 0.0);
    clic(&mut a, Clic::Parts);
    assert_eq!(on(&a, "parts"), 0.0);
    clic(&mut a, Clic::Auto);
    assert_eq!(on(&a, "auto"), 1.0);
    clic(&mut a, Clic::Tous);
    assert!(!a.ui.fp_reglages);
    assert!(a.ui.ind_dlg.as_ref().is_some_and(|d| d.ind == Ind::Footprint), "the full window");
}

/// 📸 Side by side with the mock-up v1 (simulateur-demo/2026-11-22-maquette-footprint-multi-v1-*.png): 🖥 the chart
/// with the legend open and the bubble of a row under the mouse, 🖥 the ⚙ open, 📱 the chart and 📱 the ⚙ sheet. Only
/// when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_footprint_multi() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    argos_data::murs::okx_ctval_noter("BTC", 0.01);
    for (phone, reglages, appui) in [
        (false, false, false),
        (false, true, false),
        (true, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        dossier_a_soi(if phone { "captures_fpm2_cell" } else { "captures_fpm2_pc" });
        let mut a = if phone { cell_pilule() } else { pc_avec_volet(wl::Tool::Trading) };
        if phone {
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
            a.ui.depart_vu = true;
        } else {
            a.watch.prefs.open = false;
        }
        a.top.prefs.bienvenue_vue = true;
        fp_scene(&mut a, &FP_CINQ, Some((Exchange::Coinbase, 3)));
        fp_recalcul(&mut a);
        a.start_free();
        a.chart.viewport.bar_spacing = if phone { 66.0 } else { 86.0 };
        a.chart.viewport.right_offset = if phone { 0.6 } else { 1.2 };
        a.chart.settings.scale.manual = Some(if phone { (61_975.0, 62_235.0) } else { (61_965.0, 62_330.0) });
        a.ind.leg.0 = true;
        a.pointer.mouse = Some((1_200.0, 820.0));
        a.tools.retracted = true;
        a.demo = argos_core::portfolio::Portfolio::default();
        let (w, h) = if phone { (402, 874) } else { (1440, 900) };
        let _ = image_ecran(&mut a, w, h);
        if reglages {
            a.ui.fp_reglages = true;
        } else if !phone || appui {
            // the mouse (📱 the finger) on the busiest row of the second bar from the end
            let n = a.chart.bars.len();
            let f = a.ind.out.footprint.clone().unwrap();
            let b = f.bars.iter().find(|b| b.i == n - 2).unwrap().clone();
            let row = *b.rows.iter().max_by(|x, y| (x.1 + x.2).total_cmp(&(y.1 + y.2))).unwrap();
            let fr = a.frame_now().unwrap();
            let g = fr.geo();
            let (x, y) = (g.x_of(a.chart.bars[n - 2].time) + 10.0, g.y_of((row.0 as f64 + 0.5) * f.step));
            if appui {
                a.ui.croix = Some((x, y, (row.0 as f64 + 0.5) * f.step));
            } else {
                a.pointer.mouse = Some((x, y));
            }
        }
        let _ = image_ecran(&mut a, w, h);
        let (img, _) = image_ecran(&mut a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        let nom = format!(
            "fpm-{}{}",
            if phone { "cell" } else { "pc" },
            if reglages {
                "-reglages"
            } else if appui {
                "-appui"
            } else {
                ""
            }
        );
        pm.save_png(format!("{dir}/{nom}.png")).unwrap();
    }
}

/// ⚖️ The report's scene: the HYPE 50 min bars of the scripts' fixtures (504) and the guide's strategy (EMA 9 / 21, its
/// stop and target), backtested, its report page open. Each test calls [`dossier_a_soi`] first.
fn scene_rapport_hype() -> App {
    let v: serde_json::Value = serde_json::from_str(include_str!("../../argos-script/tests/fixtures/hype_hl_50m.json")).unwrap();
    let bars: Vec<Bar> = v
        .as_array()
        .unwrap()
        .iter()
        .map(|b| Bar {
            time: b["time"].as_i64().unwrap(),
            open: b["open"].as_f64().unwrap(),
            high: b["high"].as_f64().unwrap(),
            low: b["low"].as_f64().unwrap(),
            close: b["close"].as_f64().unwrap(),
            volume: b["volume"].as_f64().unwrap(),
        })
        .collect();
    let iv = Interval(3000);
    let mut a = App::blank(Source::File("HYPE.json".into()), iv, iv, vec![], "HYPE · 50m · fichier".into());
    a.chart.raw = bars;
    a.rebuild();
    a.start_free();
    a.top.prefs.bienvenue_vue = true;
    a.scripts = vec![SavedScript {
        name: "Exemple EMA 9 / 21".into(),
        src: r#"//@version=6
strategy("Exemple EMA 9 / 21", overlay = true, initial_capital = 10000, default_qty_type = strategy.percent_of_equity, default_qty_value = 10, commission_type = strategy.commission.percent, commission_value = 0.05)
rapide = ta.ema(close, input.int(9, "EMA rapide"))
lente = ta.ema(close, input.int(21, "EMA lente"))
plot(rapide, "EMA rapide", color.aqua)
plot(lente, "EMA lente", color.orange)
if ta.crossover(rapide, lente)
    strategy.entry("Achat", strategy.long)
    strategy.exit("Sortie achat", "Achat", stop = close * 0.98, limit = close * 1.04)
if ta.crossunder(rapide, lente)
    strategy.entry("Vente", strategy.short)
    strategy.exit("Sortie vente", "Vente", stop = close * 1.02, limit = close * 0.96)
"#
        .into(),
        on: true,
        ..Default::default()
    }];
    a.ids_des_scripts();
    a.ind.key = Default::default();
    a.refresh_indicators();
    a.ui.bt_prefs.script = a.scripts[0].id.clone();
    a.run_backtest();
    assert!(a.ui.bt.as_ref().is_some_and(|v| v.script && v.detail.lignes.len() >= 10));
    a.ui.bt_page = Some(argos_render::btpage::BtPage::new());
    a
}

/// TradingView's list of the report's trades, as TradingView writes it (« Liste des transactions » exported in CSV,
/// in New York time while the chart is in UTC), with trade `plus_tot` leaving a bar earlier (its price and result
/// changed with it).
fn liste_tradingview(a: &App, plus_tot: Option<usize>) -> String {
    let v = a.ui.bt.as_ref().unwrap();
    let mut l = v.detail.lignes.clone();
    if let Some(k) = plus_tot {
        let b = &a.chart.bars[l[k].bar_out - 1];
        let k_sens = if l[k].side == argos_core::Side::Long { 1.0 } else { -1.0 };
        let plus = (b.open - l[k].prix_out) * l[k].qty * k_sens;
        l[k].net += argos_core::Money::from_f64(plus);
        (l[k].t_out, l[k].prix_out) = (b.time, b.open);
    }
    argos_script::lang::rapport::csv(&l, v.start, "USD", -5 * 3600)
}

/// ⚖️ ⬆ Comparer avec TradingView: TradingView's file (the latest CSV of the Downloads that Argos did not write) read
/// and compared, the ⚖️ TradingView tab opens on it; every trade the same, then one exit a bar earlier there: one ✗
/// that says so; a file that is not TradingView's list: the banner says why, the page stays as it was.
#[test]
fn comparer_le_rapport_avec_tradingview() {
    dossier_a_soi("comparer-tv");
    let mut a = scene_rapport_hype();
    a.chart.settings.timezone = "UTC".into();
    let n = a.ui.bt.as_ref().unwrap().detail.lignes.len();
    let fichier = platform::dossier_telechargements().join("Exemple_EMA_9_21_liste_des_trades.csv");
    std::fs::write(&fichier, liste_tradingview(&a, None)).unwrap();
    a.rapport_tv();
    let _ = std::fs::remove_file(&fichier);
    let p = a.ui.bt_page.as_ref().unwrap();
    let tv = p.tv.as_ref().unwrap_or_else(|| panic!("{:?}", a.banner));
    assert_eq!(p.tab, 2, "the ⚖️ TradingView tab opens");
    assert_eq!(tv.fichier, "Exemple_EMA_9_21_liste_des_trades.csv");
    assert_eq!(
        (tv.c.pareils, tv.c.differents, tv.c.fuseau),
        (n, 0, Some(-5 * 3600)),
        "{:?}",
        tv.c.rangees.iter().find(|r| !r.pareil())
    );
    assert!(a.banner.as_deref().is_some_and(|b| b.contains(&format!("{n} pareils, 0 différent sur {n}"))));
    // TradingView leaves trade 7 a bar earlier
    a.rapport_tv_lire("tv.csv", liste_tradingview(&a, Some(6)).as_bytes());
    let c = &a.ui.bt_page.as_ref().unwrap().tv.as_ref().unwrap().c;
    assert_eq!((c.pareils, c.differents), (n - 1, 1));
    let r = c.rangees.iter().find(|r| !r.pareil()).unwrap();
    assert_eq!((r.argos, r.ecarts[0]), (Some(6), argos_script::lang::comparer::Ecart::Sortie(-1)));
    // the page draws it (its words: argos-render's btpage tests)
    let _ = image_ecran(&mut a, 1440, 900);
    let _ = image_ecran(&mut a, 402, 874);
    // another file: the banner says why, the comparison stays
    a.rapport_tv_lire("bougies.csv", b"Date,Open,High,Low,Close\n2026-11-18,1,2,0,1\n");
    assert!(a
        .banner
        .as_deref()
        .is_some_and(|b| b.starts_with("⚠️ bougies.csv : ce n'est pas la liste des trades")));
    assert_eq!(a.ui.bt_page.as_ref().unwrap().tv.as_ref().unwrap().c.differents, 1);
}

/// 📸 ⚖️ The report's ⚖️ TradingView tab (HYPE 50 min, the guide's EMA 9 / 21, one exit a bar earlier at TradingView),
/// to compare with the mock-up (🖥 1440 × 900, 📱 402 × 874): only when `ARGOS_CAPTURES` gives a folder.
#[test]
fn captures_comparer_tv_pour_jee() {
    let Ok(dir) = std::env::var("ARGOS_CAPTURES") else { return };
    dossier_a_soi("captures-comparer-tv");
    let mut a = scene_rapport_hype();
    a.chart.settings.timezone = "UTC".into();
    let n = a.ui.bt.as_ref().unwrap().detail.lignes.len();
    let k = n.saturating_sub(7);
    a.rapport_tv_lire("Liste_des_trades_EMA_9_21.csv", liste_tradingview(&a, Some(k)).as_bytes());
    for (nom, w, h) in [("tv-pc", 1440u32, 900u32), ("tv-cell", 402, 874)] {
        if w < 900 {
            // (📱 the page scrolled down to its table)
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
            let p = a.ui.bt_page.as_mut().unwrap();
            p.wheel(10_000.0, &argos_render::text::Text::default(), w as f64, h as f64, 0);
        }
        let (img, _) = image_ecran(&mut a, w, h);
        let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
        for (px, v) in pm.pixels_mut().iter_mut().zip(&img) {
            *px = tiny_skia::ColorU8::from_rgba((v >> 16) as u8, (v >> 8) as u8, *v as u8, 255).premultiply();
        }
        pm.save_png(format!("{dir}/{nom}.png")).unwrap();
    }
}

/// 🔔 Alerts panel (mock-up « alertes v1 »), PC: 🔔 opens the panel; a new alert keeps its market, time frame and
/// creation time; a click on an active alert opens the menu of before (Mettre en pause, Éditer…, Son…, Effacer) and
/// pausing makes its row pale; nothing of before is lost (+ opens the alerts menu, ⚙ every setting and the feed).
#[test]
fn panneau_alertes_pc_actives() {
    use argos_render::alertes::{AZone, Etat};
    dossier_a_soi("alertes-panneau-pc");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.watch.prefs.open = false;
    a.ui.frames = 5;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    let avant = now_s();
    a.add_price_alert(123.0);
    let al = a.alerts.list.last().unwrap().clone();
    assert_eq!((al.coin.as_str(), al.tf), (a.main_coin().as_str(), Some(1800)));
    assert!(al.created.is_some_and(|t| t >= avant));
    a.notifs_pop(Rect::new(260.0, 12.0, 32.0, 36.0));
    a.placer(taille);
    let n = a.ui.notifs.clone().expect("🔔 opens the panel");
    assert!(n.volet && !n.fil, "the alerts panel, not the feed");
    let d = a.alertes_donnees();
    assert_eq!(d.actives.len(), 1);
    assert_eq!((d.actives[0].ut.as_str(), d.actives[0].frequence.as_str()), ("30m", "Une fois"));
    let t = argos_render::text::Text::default();
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let r = n.alertes_place(w, h);
    assert_eq!(Some(r), a.volet_rect(), "beside the right column, as the volet");
    let zone = |a: &App, e: &Etat, z: AZone| e.zones(&t, r, &a.alertes_donnees(), None).into_iter().find(|(_, x)| *x == z).unwrap().0;
    let q = zone(&a, &n.alertes, AZone::Ligne(al.id));
    clic(&mut a, q.cx(), q.cy());
    assert!(matches!(a.ui.list, Some((ListKind::AlertLine(i), _)) if i == al.id), "its menu of before");
    let labels: Vec<String> = a.ui.list.as_ref().unwrap().1.groups[0].items.iter().map(|i| i.label.clone()).collect();
    assert_eq!(labels, ["Mettre en pause", "Éditer…", "Son…", "Effacer"]);
    let anchor = a.ui.list.as_ref().unwrap().1.anchor;
    a.ui.list = None;
    a.ligne_alerte_choix(al.id, crate::actions::alerts::AL_PAUSE, anchor);
    assert!(a.alertes_donnees().actives[0].pause, "paused: the row is pale");
    assert!(a.ui.notifs.is_some(), "the panel stays");
    // + : the alerts menu of before
    let n = a.ui.notifs.clone().unwrap();
    let q = zone(&a, &n.alertes, AZone::Nouvelle);
    clic(&mut a, q.cx(), q.cy());
    assert!(matches!(a.ui.list, Some((ListKind::Alerts, _))), "+ : the alerts menu");
    a.ui.list = None;
    // ⚙ : every alert setting, and the feed of notifications
    let q = zone(&a, &n.alertes, AZone::Reglages);
    clic(&mut a, q.cx(), q.cy());
    let (k, m) = a.ui.list.clone().expect("⚙ : its menu");
    assert_eq!(k, ListKind::ReglagesAlertes);
    let labels: Vec<String> = m.groups.iter().flat_map(|g| g.items.iter().map(|i| i.label.clone())).collect();
    for l in [
        "Son des alertes…",
        "Lignes d'alerte",
        "Modifier la couleur des alertes…",
        "Déclenchement, validité, mes alertes…",
    ] {
        assert!(labels.iter().any(|x| x == l), "{l} in {labels:?}");
    }
    a.ui.list = None;
    a.reglages_alertes_choix(5, m.anchor);
    assert!(a.ui.notifs.as_ref().unwrap().fil, "the feed of notifications");
    assert_eq!(a.fil_non_lus(), 0, "opened: nothing unread");
    // the search field takes the keyboard (Ctrl + V on the page too)
    a.ui.notifs.as_mut().unwrap().fil = false;
    let q = zone(&a, &a.ui.notifs.clone().unwrap().alertes, AZone::Champ);
    clic(&mut a, q.cx(), q.cy());
    assert!(a.open_now().alertes_recherche, "the field takes the keyboard");
    a.on_user_event(UserEvent::Paste("zz".into()));
    assert_eq!(a.ui.notifs.as_ref().unwrap().alertes.recherche.as_deref(), Some("zz"));
    assert!(a.alertes_donnees().actives.len() == 1 && a.ui.notifs.as_ref().unwrap().alertes.montrees(&a.alertes_donnees()).is_empty());
}

/// 🔔 Historique: « Trade » loads that market on the chart and opens the demo Trading ticket; « Vider l'historique »
/// asks first — Annuler empties nothing, Vider empties it (the alerts stay).
#[test]
fn panneau_alertes_trade_et_vider() {
    use argos_core::alerts::{Declenchement, Sens};
    use argos_render::alertes::{AClic, AZone};
    dossier_a_soi("alertes-panneau-histo");
    let mut a = app();
    a.appliquer_cadre(&crate::actions::Interface2::default());
    a.top.prefs.bienvenue_vue = true;
    a.ui.frames = 5;
    let taille = winit::dpi::PhysicalSize::new(1440, 900);
    a.placer(taille);
    a.add_price_alert(100.0);
    for (i, coin) in ["synth:XAUEUR", "synth:XAUGBP"].iter().enumerate() {
        a.alerts.declenchements.push(Declenchement {
            id: 1,
            ts: now_s() - 60 * i as i64,
            sym: coin.trim_start_matches("synth:").into(),
            coin: coin.to_string(),
            tf: Some(3600),
            sens: Sens::Down,
            target: argos_core::alerts::decimal(2_000.0),
            price: argos_core::alerts::decimal(1_999.5),
            every: false,
        });
    }
    a.notifs_pop(Rect::new(260.0, 12.0, 32.0, 36.0));
    a.placer(taille);
    let t = argos_render::text::Text::default();
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let n = a.ui.notifs.as_mut().unwrap();
    n.alertes.onglet = 1;
    let r = n.alertes_place(w, h);
    let d = a.alertes_donnees();
    assert_eq!(d.historique[0].declenche, Some(1_999.5), "the true price");
    assert_eq!(d.historique[0].prix, Some(2_000.0));
    // the confirmation: Annuler empties nothing
    let n = a.ui.notifs.clone().unwrap();
    let vider = n.alertes.zones(&t, r, &d, None).into_iter().find(|(_, z)| *z == AZone::Vider).unwrap().0;
    clic(&mut a, vider.cx(), vider.cy());
    let (c, quoi) = a.ui.confirmer.clone().expect("« Vider l'historique? »");
    assert_eq!(quoi, crate::actions::alerts::AConfirmer::ViderHistorique);
    assert!(c.texte.contains("(2)"), "the panel's firings counted: {}", c.texte);
    let (cw, ch) = a.taille_points();
    let q = c.places(&t, cw, ch);
    clic(&mut a, q.annuler.cx(), q.annuler.cy());
    assert!(a.ui.confirmer.is_none() && a.alerts.declenchements.len() == 2, "Annuler: nothing emptied");
    // a history row under the mouse: Trade on the second row's market (XAUGBP)
    let mut e = a.ui.notifs.clone().unwrap().alertes;
    let lq = e.zones(&t, r, &d, None).into_iter().find(|(_, z)| *z == AZone::Ligne(1)).unwrap().0;
    let tr = e
        .zones(&t, r, &d, Some((lq.cx(), lq.cy())))
        .into_iter()
        .find(|(_, z)| *z == AZone::Trade(1))
        .unwrap()
        .0;
    assert_eq!(e.click(&t, r, &d, tr.cx(), tr.cy()), AClic::Trade(1));
    // (a click on the row lights it, like a touch: « Trade » under it)
    clic(&mut a, lq.cx(), lq.cy());
    assert_eq!(a.ui.notifs.as_ref().unwrap().alertes.touche, Some(1));
    let tr =
        a.ui.notifs
            .clone()
            .unwrap()
            .alertes
            .zones(&t, r, &d, None)
            .into_iter()
            .find(|(_, z)| *z == AZone::Trade(1))
            .expect("Trade")
            .0;
    clic(&mut a, tr.cx(), tr.cy());
    assert!(matches!(&a.chart.source, Source::Synth { sym } if sym == "XAUGBP"), "that market on the chart");
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Trading, "the demo Trading ticket");
    assert!(a.ui.notifs.is_none());
    // Vider, confirmed: empty; the alerts stay
    a.notifs_pop(Rect::new(260.0, 12.0, 32.0, 36.0));
    a.placer(taille);
    a.ui.notifs.as_mut().unwrap().alertes.onglet = 1;
    clic(&mut a, vider.cx(), vider.cy());
    let (c, _) = a.ui.confirmer.clone().expect("the box");
    let q = c.places(&t, cw, ch);
    clic(&mut a, q.ok.cx(), q.ok.cy());
    assert!(a.alerts.declenchements.is_empty() && a.alerts.list.len() == 1, "emptied, the alerts stay");
    let back: argos_core::alerts::Alerts = App::read_prefs("alerts.json");
    assert!(back.declenchements.is_empty() && back.list.len() == 1, "saved");
}

/// 📱 Alerts panel on the iPhone 390 × 844: 🔔 opens the bottom sheet (handle), the footer clear of the round Argos AI,
/// a touch on a history row shows Trade; a touch above the sheet closes it.
#[test]
fn panneau_alertes_cell() {
    use argos_render::alertes::AZone;
    use argos_render::ecrans::CellHit;
    let mut a = cell_390();
    a.ui.frames = 5;
    a.alerts.declenchements.push(argos_core::alerts::Declenchement {
        ts: now_s() - 30,
        sym: "XAUEUR".into(),
        coin: "synth:XAUEUR".into(),
        price: argos_core::alerts::decimal(2_001.0),
        ..Default::default()
    });
    let z = argos_render::ecrans::zones(a.win_w(), a.cell_h(), &a.cell_etat());
    let cloche = z.iter().find(|(_, c)| *c == CellHit::Alertes).unwrap().0;
    toucher(&mut a, cloche.cx(), cloche.cy());
    a.draw();
    let n = a.ui.notifs.clone().expect("🔔 opens the sheet");
    assert!(n.alertes.feuille && !n.fil);
    let (w, h) = (a.win_w(), a.top_h() + a.area.1 as f64 + argos_render::ecrans::dock_h());
    let r = n.alertes_place(w, h);
    assert!(r.x == 0.0 && (r.w - w).abs() < 0.5 && r.y + r.h <= h + 0.5, "{r:?}");
    let t = argos_render::text::Text::default();
    let d = a.alertes_donnees();
    let onglet = n.alertes.zones(&t, r, &d, None).into_iter().find(|(_, z)| *z == AZone::Onglet(1)).unwrap().0;
    toucher(&mut a, onglet.cx(), onglet.cy());
    let n = a.ui.notifs.clone().unwrap();
    assert_eq!(n.alertes.onglet, 1);
    let l = n.alertes.zones(&t, r, &d, None).into_iter().find(|(_, z)| *z == AZone::Ligne(0)).unwrap().0;
    toucher(&mut a, l.cx(), l.cy());
    let n = a.ui.notifs.clone().unwrap();
    assert_eq!(n.alertes.touche, Some(0), "the touched row is lit");
    assert!(n.alertes.zones(&t, r, &d, None).iter().any(|(_, z)| *z == AZone::Trade(0)), "Trade shown");
    toucher(&mut a, 200.0, r.y - 60.0);
    assert!(a.ui.notifs.is_none(), "above the sheet: closed");
}

/// 📟 A touch tablet (Jee, 23 Nov: « ok tablette »): the PC bars (not the phone view), big targets, no keyboard hint
/// (« (T = changer de type) », « Ctrl+K »), bars never faded by the finger; the phone view by the finger is unchanged.
#[test]
fn tablette_au_doigt_barres_du_pc() {
    use argos_render::formfactor::{choose, Kind, Screen};
    let ecran = |w: f64, h: f64| Screen {
        w,
        h,
        scale: 2.0,
        coarse: true,
        ..Default::default()
    };
    for (w, h, kind) in [
        (1366.0, 1024.0, Kind::Desktop),
        (1024.0, 1366.0, Kind::TabletPortrait),
        (820.0, 1180.0, Kind::TabletPortrait),
    ] {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.top.prefs.estomper = true;
        a.ui.form = choose(&ecran(w, h));
        a.placer(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        assert_eq!(a.ui.form.kind, kind, "{w} × {h}");
        assert!(!a.top.phone, "{w} × {h}: the PC bars, not the phone view");
        assert!(argos_render::actions::tactile(), "{w} × {h}: touch mode");
        assert!(!argos_render::actions::montrer_raccourcis(), "{w} × {h}: no keyboard hint");
        let titre = a.to_chart().title.unwrap_or_default();
        assert!(!titre.contains("T = changer de type"), "{titre}");
        assert!(!a.barres_estompees(), "{w} × {h}: never faded by the finger");
    }
    // phones by the finger: the phone view, as before
    for (w, h) in [(390.0, 844.0), (430.0, 932.0)] {
        let mut a = app();
        a.ui.form = choose(&ecran(w, h));
        a.placer(winit::dpi::PhysicalSize::new(w as u32, h as u32));
        assert!(a.top.phone && argos_render::actions::tactile(), "{w} × {h}: the phone view");
    }
    // the mouse: shortcuts shown, as before
    let mut a = app();
    a.ui.form = choose(&Screen {
        w: 1366.0,
        h: 1024.0,
        scale: 2.0,
        ..Default::default()
    });
    a.placer(winit::dpi::PhysicalSize::new(1366, 1024));
    assert!(!argos_render::actions::tactile() && a.to_chart().title.unwrap_or_default().contains("T = changer de type"));
}

/// 🧭 The Argos ▾ menu: « Marchés », « Marketplace » and the « Gardien » pill each open their module (they closed the
/// menu and opened nothing, /app2/ 22 Nov).
#[test]
fn menu_argos_ouvre_ses_modules() {
    use argos_render::topmenu::TopAct;
    let ancre = Rect::new(10.0, 10.0, 100.0, 40.0);
    let mut a = app();
    a.watch.prefs.open = false;
    a.top_act(TopAct::Marches as i64, ancre);
    assert!(a.watch.prefs.open && a.watch.prefs.panel == wl::Tool::Market, "Marchés: the market panel");
    let mut a = app();
    a.top_act(TopAct::Marketplace as i64, ancre);
    assert!(a.ui.mk_page.is_some(), "Marketplace: its page");
    let mut a = app();
    a.top_act(TopAct::Gardien as i64, ancre);
    assert!(a.ui.sim.is_some(), "Gardien: the PropFirm simulator");
}

/// 🔔 The alerts panel names every market « SOL/USDT » (base, then quote), whatever the exchange and however it
/// writes the symbol: Bybit's « SOLUSDT », « BTC-USDT », « BTC_USDT », « BTC/USDT », « BTCUSDC.P », OKX's -SWAP;
/// Hyperliquid's « HYPE » keeps its USDC.
#[test]
fn panneau_alertes_paire_toujours_coupee() {
    use argos_core::alerts::{Declenchement, Sens};
    dossier_a_soi("alertes-paire-coupee");
    let mut a = app();
    let marches = [
        ("bybit:SOLUSDT", "SOLUSDT"),
        ("", "BTC-USDT"),
        ("", "BTC_USDT"),
        ("", "BTC/USDT"),
        ("", "BTCUSDC.P"),
        ("okx:ETH-USDT-SWAP", "ETH-USDT-SWAP"),
        ("hl:HYPE", "HYPE"),
    ];
    for (coin, sym) in marches {
        a.alerts.declenchements.push(Declenchement {
            id: 99,
            ts: now_s(),
            sym: sym.into(),
            coin: coin.into(),
            tf: Some(3600),
            sens: Sens::Up,
            target: argos_core::alerts::decimal(1.0),
            price: argos_core::alerts::decimal(1.0),
            every: false,
        });
    }
    let d = a.alertes_donnees();
    let noms: Vec<(String, String)> = d.historique.iter().map(|l| (l.base.clone(), l.quote.clone())).collect();
    let attendu = [
        ("SOL", "USDT"),
        ("BTC", "USDT"),
        ("BTC", "USDT"),
        ("BTC", "USDT"),
        ("BTC", "USDC"),
        ("ETH", "USDT"),
        ("HYPE", "USDC"),
    ];
    assert_eq!(noms.len(), attendu.len());
    for (b, q) in attendu {
        assert!(noms.contains(&(b.to_string(), q.to_string())), "{b}/{q} in {noms:?}");
    }
    assert!(
        noms.iter().all(|(b, q)| !q.is_empty() && !b.contains(['-', '_', '/', '.'])),
        "always base/quote: {noms:?}"
    );
}

// ───── 📋 tableaux-1: movable / resizable tables ─────

/// 📋 The chart drawn once more as the app draws it (its tables placed), `tactile`: phone view.
fn tab_dessin(a: &App, tactile: bool) {
    argos_render::actions::set_tactile(tactile);
    let c = a.chart_with(false);
    let _ = argos_render::render(&c);
}

/// 📋 T5 / T8: a table dropped on the legend, on a wall pill, on the floating bar or outside the price pane snaps to
/// the nearest free place (its size kept), never under any of them — cell 390 × 844, tablet 820 × 1180, PC 1440 ×
/// 900; `tableaux()` gives its real rect.
#[test]
fn tableau_lache_sur_une_place_prise_va_a_la_place_libre() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, TableLayout};
    dossier_a_soi("tableaux_lache");
    for (cell, w, h) in [(true, 390u32, 844u32), (true, 820, 1180), (false, 1440, 900)] {
        let mut a = murs_aux_dernieres_bougies(if cell { cell_390() } else { app() });
        if cell {
            // (the phone view, as on the cell and the touch tablet)
            a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        }
        if w == 820 {
            a.area = (820, (1180.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
        }
        a.ind.on.push(Ind::Tvd);
        let _ = image_ecran(&mut a, w, h);
        a.ind.key = Default::default();
        tab_dessin(&a, cell);
        // (twice: the wall pills at full length are the last frame's)
        tab_dessin(&a, cell);
        let key = tb::cle(Ind::Tvd, "");
        let t0 = tb::pose(&key).unwrap_or_else(|| panic!("{w}: the TVD Pro dashboard drawn"));
        let c = a.chart_with(false);
        let legende = argos_render::legende_zones(&c);
        let garder = c.garder.clone();
        drop(c);
        let pilules = argos_render::murs::pilules_pleines();
        assert!(!pilules.is_empty(), "{w}: the wall pills");
        let mut cibles = vec![legende[0], pilules[0], Rect::new(t0.area.w + 400.0, t0.area.h + 400.0, 1.0, 1.0)];
        cibles.extend(garder.iter().copied());
        for cible in cibles {
            let r = Rect::new(cible.cx() - t0.rect.w / 2.0, cible.cy() - t0.rect.h / 2.0, t0.rect.w, t0.rect.h);
            a.ind.tables.insert(key.clone(), TableLayout::default().ancre(r, t0.area, None));
            tab_dessin(&a, cell);
            let t = tb::pose(&key).unwrap_or_else(|| panic!("{w}: dropped on {cible:?}: still drawn"));
            assert!(argos_render::indicators::tableaux().contains(&t.rect), "{w}: tableaux() gives the real rect");
            assert!((t.rect.w - t0.rect.w).abs() < 1.0, "{w}: its size kept: {:?} / {:?}", t.rect, t0.rect);
            let a0 = t.area;
            assert!(t.rect.x >= a0.x - 0.5 && t.rect.y >= a0.y - 0.5, "{w}: in the price pane: {:?}", t.rect);
            assert!(
                t.rect.x + t.rect.w <= a0.x + a0.w + 0.5 && t.rect.y + t.rect.h <= a0.y + a0.h + 0.5,
                "{w}: {:?}",
                t.rect
            );
            for o in legende.iter().chain(&garder).chain(&argos_render::murs::pilules_pleines()) {
                assert!(!croisent(&t.rect, o), "{w}: dropped on {cible:?}, the table {:?} is under {o:?}", t.rect);
            }
        }
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 T5: `tableaux()` returns the real rect of a moved, resized table for the multi-UT table and a user script's
/// `table.new` too (the wall pills and the high / low pills avoid exactly that).
#[test]
fn tableaux_deplaces_du_multi_ut_et_d_un_script() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, TableLayout};
    dossier_a_soi("tableaux_mtf_script");
    let mut a = app();
    a.ind.on.push(Ind::MtfTable);
    a.scripts.push(SavedScript {
        name: "tab".into(),
        src: "//@version=6\nindicator(\"tab\", overlay = true)\nvar t = table.new(position.top_right, 2, 1, bgcolor = color.new(color.black, 30))\nif barstate.islast\n    table.cell(t, 0, 0, \"RSI\", text_color = color.white)\n    table.cell(t, 1, 0, str.tostring(ta.rsi(close, 14), \"#\"), text_color = color.white)\nplot(close)\n".into(),
        on: true,
        ..Default::default()
    });
    a.ids_des_scripts();
    let _ = image_ecran(&mut a, 1440, 900);
    a.ind.key = Default::default();
    let _ = image_ecran(&mut a, 1440, 900);
    tab_dessin(&a, false);
    let cles: Vec<String> = tb::poses().into_iter().map(|p| p.key).collect();
    let mtf = tb::cle(Ind::MtfTable, "");
    let script = cles
        .iter()
        .find(|k| k.starts_with("script:"))
        .cloned()
        .unwrap_or_else(|| panic!("the script's table: {cles:?}"));
    for key in [mtf, script] {
        let t0 = tb::pose(&key).unwrap();
        let s = 1.3;
        let r = Rect::new(t0.area.x + 40.0, t0.area.y + t0.area.h * 0.45, t0.rect.w * s, t0.rect.h * s);
        a.ind.tables.insert(
            key.clone(),
            TableLayout {
                scale: s,
                ..Default::default()
            }
            .ancre(r, t0.area, None),
        );
        tab_dessin(&a, false);
        let t = tb::pose(&key).unwrap();
        assert!(
            (t.rect.x - r.x).abs() < 1.0 && (t.rect.y - r.y).abs() < 1.0,
            "{key}: moved there: {:?} / {r:?}",
            t.rect
        );
        assert!(
            (t.rect.w - r.w).abs() < 2.0 && (t.rect.h - r.h).abs() < 2.0,
            "{key}: 130 %: {:?} / {r:?}",
            t.rect
        );
        assert!(argos_render::indicators::tableaux().contains(&t.rect), "{key}: in tableaux()");
        a.ind.tables.remove(&key);
    }
}

/// 📋 T2 / T4 / T8 (PC, mouse): hovering a table shows it; dragging its body moves it; a corner scales it around the
/// opposite corner; the place is saved with the indicator settings and read back after a reload; a double-click
/// puts it back; removing the indicator forgets it.
#[test]
fn tableau_a_la_souris_garde_au_rechargement() {
    use argos_render::catalogue::ReglagesParNom;
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, Prise};
    dossier_a_soi("tableaux_souris");
    let mut a = app();
    a.ind.on.push(Ind::Tvd);
    let _ = image_ecran(&mut a, 1440, 900);
    tab_dessin(&a, false);
    let key = tb::cle(Ind::Tvd, "");
    let t0 = tb::pose(&key).expect("the TVD Pro dashboard").rect;
    let (rw, th) = (a.rw(), a.top_h());
    let va = |a: &mut App, x: f64, y: f64| a.mouse_moved(PhysicalPosition::new(x + rw, y + th));
    va(&mut a, t0.cx(), t0.cy());
    assert_eq!(a.pointer.tab.survol.as_ref().map(|s| s.1), Some(Prise::Corps), "hovered: its body");
    // the body dragged 300 left, 200 down
    a.left_button(ElementState::Pressed);
    va(&mut a, t0.cx() - 150.0, t0.cy() + 100.0);
    va(&mut a, t0.cx() - 300.0, t0.cy() + 200.0);
    a.left_button(ElementState::Released);
    let t1 = tb::pose(&key).unwrap().rect;
    assert!(
        (t1.x - (t0.x - 300.0)).abs() < 1.0 && (t1.y - (t0.y + 200.0)).abs() < 1.0,
        "moved: {t1:?} from {t0:?}"
    );
    assert!(a.ind.tables.get(&key).is_some_and(|l| l.moved));
    // the bottom right corner pulled 60 to the right: bigger, the top left corner held
    a.pointer.tab.dernier = None;
    va(&mut a, t1.x + t1.w + tb::MARGE, t1.y + t1.h + tb::MARGE);
    assert_eq!(a.pointer.tab.survol.as_ref().map(|s| s.1), Some(Prise::Coin(2)));
    a.left_button(ElementState::Pressed);
    va(&mut a, t1.x + t1.w + tb::MARGE + 60.0, t1.y + t1.h + tb::MARGE);
    a.left_button(ElementState::Released);
    let t2 = tb::pose(&key).unwrap();
    assert!(
        t2.scale > 1.1 && (t2.rect.x - t1.x).abs() < 1.0 && (t2.rect.y - t1.y).abs() < 1.0,
        "scaled around its top left: {t2:?}"
    );
    // saved with the indicator settings, read back after a reload
    let lu: ReglagesParNom = App::read_prefs(crate::actions::indicators::REGLAGES_NOMS);
    assert_eq!(lu.tableaux.get(&key), a.ind.tables.get(&key), "in indicator_settings.json");
    let mut b = app();
    b.ind.tables = App::read_prefs::<ReglagesParNom>(crate::actions::indicators::REGLAGES_NOMS).tableaux;
    b.ind.on.push(Ind::Tvd);
    let _ = image_ecran(&mut b, 1440, 900);
    tab_dessin(&b, false);
    assert_eq!(tb::pose(&key).unwrap().rect, t2.rect, "the same place after a reload");
    // a double-click: back in place
    a.pointer.tab.dernier = None;
    let c = t2.rect;
    clic(&mut a, c.cx() + rw, c.cy() + th);
    clic(&mut a, c.cx() + rw, c.cy() + th);
    assert!(!a.ind.tables.contains_key(&key), "back in place");
    tab_dessin(&a, false);
    assert_eq!(tb::pose(&key).unwrap().rect, t0, "at its own corner again");
    // removing the indicator forgets its layout
    a.ind.tables.insert(key.clone(), Default::default());
    a.toggle_indicator(Ind::Tvd);
    assert!(!a.ind.tables.contains_key(&key));
}

/// 📋 T2: a right-click on a table opens its menu: « Masquer le tableau » turns off the indicator's own table setting
/// (TVD Pro « Dashboard »); « ↺ Remettre à sa place » forgets its layout.
#[test]
fn menu_du_tableau() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, MenuLigne};
    dossier_a_soi("tableaux_menu");
    let mut a = app();
    a.ind.on.push(Ind::Tvd);
    let _ = image_ecran(&mut a, 1440, 900);
    tab_dessin(&a, false);
    let key = tb::cle(Ind::Tvd, "");
    let t = tb::pose(&key).unwrap();
    assert_eq!(t.interrupteur, Some("showTable"));
    let (rw, th) = (a.rw(), a.top_h());
    a.ind
        .tables
        .insert(key.clone(), argos_render::tableaux::TableLayout::default().ancre(t.rect, t.area, None));
    a.mouse_moved(PhysicalPosition::new(t.rect.cx() + rw, t.rect.cy() + th));
    a.right_click();
    let m = a.pointer.tab.menu.clone().expect("its menu");
    tab_dessin(&a, false);
    let lignes = tb::menu_lignes(&m);
    assert_eq!(
        lignes.iter().map(|(_, l)| *l).collect::<Vec<_>>(),
        [MenuLigne::Titre, MenuLigne::Remettre, MenuLigne::Taille, MenuLigne::Masquer]
    );
    let (r, _) = lignes[1];
    clic(&mut a, r.cx() + rw, r.cy() + th);
    assert!(!a.ind.tables.contains_key(&key) && a.pointer.tab.menu.is_none(), "↺ back in place, menu closed");
    a.mouse_moved(PhysicalPosition::new(t.rect.cx() + rw, t.rect.cy() + th));
    a.right_click();
    let m = a.pointer.tab.menu.clone().unwrap();
    let (r, _) = tb::menu_lignes(&m)[3];
    clic(&mut a, r.cx() + rw, r.cy() + th);
    let vals = argos_render::indicators::values(&a.ind.set, Ind::Tvd);
    let k = Ind::Tvd.params().1.iter().position(|p| p.key == "showTable").unwrap();
    assert_eq!(vals[k], 0.0, "« Dashboard » turned off");
}

/// 📋 T3 / T8 (cell, by the finger): a drag that starts off a table pans the chart; a tap on the table selects it;
/// then the chart no longer pans, one finger moves the table, a pinch on it scales it; a tap elsewhere ends it.
#[test]
fn tableau_au_doigt_sur_cell() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    use winit::event::TouchPhase;
    dossier_a_soi("tableaux_doigt");
    let mut a = cell_390();
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ind.on.push(Ind::Tvd);
    let _ = image_ecran(&mut a, 390, 844);
    tab_dessin(&a, true);
    let key = tb::cle(Ind::Tvd, "");
    let t0 = tb::pose(&key).expect("the TVD Pro dashboard").rect;
    let (rw, th) = (a.rw(), a.top_h());
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    // a point of the chart off the table
    let (fx, fy) = (pw * 0.5, ph * 0.75);
    assert!(!t0.contains(fx, fy));
    let glisser = |a: &mut App, x: f64, y: f64, dx: f64, dy: f64| {
        a.touch(1, TouchPhase::Started, (x + rw, y + th));
        for k in 1..=6 {
            a.touch(1, TouchPhase::Moved, (x + rw + dx * k as f64 / 6.0, y + th + dy * k as f64 / 6.0));
        }
        a.touch(1, TouchPhase::Ended, (x + rw + dx, y + th + dy));
        tab_dessin(a, true);
    };
    let toucher = |a: &mut App, x: f64, y: f64| {
        a.pointer.last_chart_click = None;
        a.touch(1, TouchPhase::Started, (x + rw, y + th));
        a.touch(1, TouchPhase::Ended, (x + rw, y + th));
        tab_dessin(a, true);
    };
    let v0 = a.chart.viewport;
    glisser(&mut a, fx, fy, -120.0, 0.0);
    assert_ne!(a.chart.viewport, v0, "nothing selected: the finger pans the chart");
    assert!(a.pointer.tab.choisi.is_none());
    let t0 = tb::pose(&key).unwrap().rect;
    toucher(&mut a, t0.cx(), t0.cy());
    assert_eq!(a.pointer.tab.choisi.as_deref(), Some(key.as_str()), "a tap selects the table");
    // selected: a drag off the table does not pan the chart
    let v1 = a.chart.viewport;
    glisser(&mut a, fx, fy, -120.0, 0.0);
    assert_eq!(a.chart.viewport, v1, "selected: the chart does not pan");
    assert!(a.pointer.tab.choisi.is_some(), "a drag elsewhere is not a tap: still selected");
    // one finger on the table moves it
    glisser(&mut a, t0.cx(), t0.cy(), 0.0, 120.0);
    let t1 = tb::pose(&key).unwrap().rect;
    assert!(t1.y > t0.y + 60.0, "moved down: {t1:?} from {t0:?}");
    // a pinch on it scales it
    let (cx, cy) = (t1.cx() + rw, t1.cy() + th);
    a.touch(1, TouchPhase::Started, (cx - 30.0, cy));
    a.touch(2, TouchPhase::Started, (cx + 30.0, cy));
    a.touch(2, TouchPhase::Moved, (cx + 60.0, cy));
    a.touch(2, TouchPhase::Ended, (cx + 60.0, cy));
    a.touch(1, TouchPhase::Ended, (cx - 30.0, cy));
    assert!(
        a.ind.tables.get(&key).is_some_and(|l| l.scale > 1.2),
        "pinched bigger: {:?}",
        a.ind.tables.get(&key)
    );
    // a tap elsewhere: done (and kept)
    toucher(&mut a, fx, fy);
    assert!(a.pointer.tab.choisi.is_none(), "a tap elsewhere ends it");
    let lu: argos_render::catalogue::ReglagesParNom = App::read_prefs(crate::actions::indicators::REGLAGES_NOMS);
    assert!(lu.tableaux.get(&key).is_some_and(|l| l.scale > 1.2), "kept");
    argos_render::actions::set_tactile(false);
}

/// 📋 T6: on the cell, the legend's list unfolded (Argos TVD Pro, Argos Murs, a moving average) is never drawn over
/// the TVD Pro table: the table sits below its lines, at its own place and when dropped on them.
#[test]
fn cell_legende_ouverte_le_tableau_dessous() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, TableLayout};
    dossier_a_soi("tableaux_legende");
    let mut a = murs_aux_dernieres_bougies(cell_390());
    a.ui.form.kind = argos_render::formfactor::Kind::Phone;
    a.ind.on.extend([Ind::Tvd, Ind::Sma]);
    a.ind.leg.0 = true;
    let _ = image_ecran(&mut a, 390, 844);
    a.ind.key = Default::default();
    let _ = image_ecran(&mut a, 390, 844);
    tab_dessin(&a, true);
    let (haut, _) = argos_render::indleg::lignes_liste(&a.ind.out).expect("the list unfolded");
    // (the cell's rows as drawn: 34 px apart, each name 32 px high — the last one ends at `bas_legende`)
    let bas = argos_render::indleg::bas_legende(&a.ind.out, true, true);
    let liste = Rect::new(0.0, haut, a.chart.viewport.plot_w(), bas - haut);
    let key = tb::cle(Ind::Tvd, "");
    let t = tb::pose(&key).expect("the TVD Pro dashboard");
    assert!(!croisent(&t.rect, &liste), "the table {:?} under the legend {liste:?}", t.rect);
    a.ind.tables.insert(
        key.clone(),
        TableLayout::default().ancre(Rect::new(0.0, haut, t.rect.w, t.rect.h), t.area, None),
    );
    tab_dessin(&a, true);
    let t = tb::pose(&key).expect("still drawn");
    assert!(!croisent(&t.rect, &liste), "dropped on the legend: {:?} / {liste:?}", t.rect);
    argos_render::actions::set_tactile(false);
}

/// 📋 Phone and tablet held upright, the legend folded (its lines « O H L C » and « Vol · Acheteurs · Vendeurs », the
/// « ⌄ N » pill), Argos Murs and Argos TVD Pro on as on a new profile: the TVD Pro table at its automatic place is on
/// none of the legend's lines, on no wall pill and on nothing the app lays over the chart. (Seen on the images of
/// 22 Nov: on the phone the table covered the end of the O H L C line and « Vendeurs ».) 502 × 972: Jee's phone.
#[test]
fn tactile_legende_repliee_le_tableau_hors_de_ses_lignes() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    dossier_a_soi("tableaux_legende_repliee");
    for (w, h) in [(390u32, 844u32), (502, 972), (820, 1180)] {
        let a = tab_app(w, h, &[Ind::Tvd]);
        assert!(!a.ind.leg.0, "{w}: the legend's list folded");
        let t = tb::pose(&tb::cle(Ind::Tvd, "")).unwrap_or_else(|| panic!("{w}: TVD Pro's table drawn"));
        let c = a.chart_with(false);
        let mut pris = argos_render::legende_zones(&c);
        assert!(pris.len() >= 4, "{w}: title, O H L C, Vol and « ⌄ N »: {pris:?}");
        pris.extend(c.garder.iter().copied());
        drop(c);
        pris.extend(argos_render::murs::pilules_pleines());
        pris.extend(argos_render::murs::zones_dessinees());
        for o in &pris {
            assert!(!croisent(&t.rect, o), "{w}: the table {:?} on {o:?}", t.rect);
        }
        argos_render::actions::set_tactile(false);
    }
}

/// { } B1 on the cell: the row's ⋯ menu has « Code de … » for every indicator; for a Rust one it opens its Rust
/// source, read only.
#[test]
fn code_depuis_le_menu_du_cell() {
    use argos_render::indicators::Ind;
    let mut a = cell_390();
    a.ind.on = vec![Ind::ArgosMurs];
    a.ind.sel = Some(Ok(Ind::ArgosMurs));
    let menu = a.ind_plus_menu();
    let code = menu.iter().flat_map(|g| &g.items).find(|it| it.id == 13).expect("« Code de … »");
    assert!(code.enabled && code.label.contains("Argos Murs"), "{}", code.label);
    a.ind_plus_pick(13);
    let ed = a.ui.editor.as_ref().expect("its code");
    assert!(ed.lecture_seule && ed.rust, "Rust, read only");
}

// ───── 📋 tableaux-1, tour 2 (fixes seen on the v1 images) ─────

/// 📋 A finger's tap at (`x`, `y`) (chart coordinates), then the chart drawn again (phone view).
fn tab_toucher(a: &mut App, x: f64, y: f64) {
    let (rw, th) = (a.rw(), a.top_h());
    a.pointer.last_chart_click = None;
    a.touch(1, winit::event::TouchPhase::Started, (x + rw, y + th));
    a.touch(1, winit::event::TouchPhase::Ended, (x + rw, y + th));
    tab_dessin(a, true);
}

/// 📋 The app at `w` wide in the phone view (cell 390, touch tablet 820) or PC, Binance BTCUSDT 5m, Argos Murs with
/// two walls, `inds` on, drawn.
fn tab_app(w: u32, h: u32, inds: &[argos_render::indicators::Ind]) -> App {
    let tactile = w < 1000;
    let mut a = murs_aux_dernieres_bougies(if tactile { cell_390() } else { app() });
    if tactile {
        a.ui.form.kind = argos_render::formfactor::Kind::Phone;
        a.area = (w, (h as f64 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
    }
    a.ind.on.extend(inds.iter().copied());
    let _ = image_ecran(&mut a, w, h);
    a.ind.key = Default::default();
    let _ = image_ecran(&mut a, w, h);
    // (twice: the wall pills at full length are the last frame's)
    tab_dessin(&a, tactile);
    tab_dessin(&a, tactile);
    a
}

/// 📋 F5 (PC 1440 × 900 and 1920 × 1080, TVD Pro + Argos Murs + multi-UT): both tables are drawn, on neither each
/// other, the legend, the floating bar, the walls' pills nor the bottom pills.
#[test]
fn tour2_pc_les_deux_tableaux_visibles() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    dossier_a_soi("tableaux2_pc");
    for (w, h) in [(1440u32, 900u32), (1920, 1080)] {
        let a = tab_app(w, h, &[Ind::Tvd, Ind::MtfTable]);
        let tvd = tb::pose(&tb::cle(Ind::Tvd, "")).unwrap_or_else(|| panic!("{w}: TVD Pro's table drawn"));
        let mtf = tb::pose(&tb::cle(Ind::MtfTable, "")).unwrap_or_else(|| panic!("{w}: the multi-UT table drawn"));
        assert!(!croisent(&tvd.rect, &mtf.rect), "{w}: {:?} / {:?}", tvd.rect, mtf.rect);
        let c = a.chart_with(false);
        let mut haut = argos_render::legende_zones(&c);
        haut.extend(c.garder.iter().copied());
        drop(c);
        let mut pilules = argos_render::murs::pilules_pleines();
        pilules.extend(argos_render::murs::zones_dessinees());
        for t in [&tvd, &mtf] {
            for o in haut.iter().chain(&pilules) {
                assert!(!croisent(&t.rect, o), "{w}: {} {:?} under {o:?}", t.key, t.rect);
            }
        }
    }
}

/// 📋 F3: PC, no crosshair while the mouse is over a table (its move cursor shows), back outside; touch (390, 820),
/// the tap that selects a table lays no crosshair, and none shows while it is selected.
#[test]
fn tour2_pas_de_reticule_sur_un_tableau() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    dossier_a_soi("tableaux2_reticule");
    let mut a = tab_app(1440, 900, &[Ind::Tvd]);
    let t = tb::pose(&tb::cle(Ind::Tvd, "")).expect("the dashboard").rect;
    let (rw, th) = (a.rw(), a.top_h());
    a.mouse_moved(PhysicalPosition::new(t.cx() + rw, t.cy() + th));
    assert!(a.chart_with(false).crosshair.is_none(), "over the table: no crosshair");
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let (x, y) = (pw * 0.55, ph * 0.5);
    assert!(tb::pose_sous(x, y, tb::MARGE + 6.0).is_none());
    a.mouse_moved(PhysicalPosition::new(x + rw, y + th));
    assert!(a.chart_with(false).crosshair.is_some(), "outside: the crosshair is back");
    for (w, h) in [(390u32, 844u32), (820, 1180)] {
        let mut a = tab_app(w, h, &[Ind::Tvd]);
        let t = tb::pose(&tb::cle(Ind::Tvd, "")).expect("the dashboard").rect;
        tab_toucher(&mut a, t.cx(), t.cy());
        assert!(a.pointer.tab.choisi.is_some(), "{w}: selected");
        assert!(a.chart_with(false).crosshair.is_none() && a.ui.croix.is_none(), "{w}: no crosshair");
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 F2 (390 and 820, by the finger): a table selected, the hint pill takes the row of « Carnets en direct » (its
/// height, 22 / 32), that pill is hidden meanwhile and nothing is under the hint pill (table, wall pill, high / low
/// pill); « OK » ends the selection and « Carnets » is back.
#[test]
fn tour2_bulle_au_doigt_dans_la_rangee_des_carnets() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    dossier_a_soi("tableaux2_bulle");
    for (w, h) in [(390u32, 844u32), (820, 1180)] {
        let mut a = tab_app(w, h, &[Ind::Tvd, Ind::MtfTable]);
        let carnets = argos_render::murs::zones_dessinees()
            .into_iter()
            .find(|r| r.x == 8.0)
            .unwrap_or_else(|| panic!("{w}: « Carnets en direct »"));
        let t = tb::pose(&tb::cle(Ind::Tvd, "")).expect("the dashboard").rect;
        tab_toucher(&mut a, t.cx(), t.cy());
        tab_dessin(&a, true);
        let (b, _) = tb::bulle_doigt();
        assert!(
            (b.y - carnets.y).abs() < 0.5 && (b.h - carnets.h).abs() < 0.5,
            "{w}: {b:?} in the row of {carnets:?}"
        );
        assert!(b.h <= carnets.h + 0.5, "{w}: never taller");
        assert!(
            !argos_render::murs::zones_dessinees().iter().any(|r| r.x == 8.0),
            "{w}: « Carnets » hidden meanwhile"
        );
        for o in argos_render::indicators::tableaux()
            .into_iter()
            .chain(argos_render::murs::zones_dessinees())
            .chain(argos_render::hilo_dessines())
        {
            assert!(!croisent(&b, &o), "{w}: {o:?} under the hint pill {b:?}");
        }
        let ok = tb::ok_cible();
        tab_toucher(&mut a, ok.cx(), ok.cy());
        assert!(a.pointer.tab.choisi.is_none(), "{w}: « OK » ends it");
        tab_dessin(&a, true);
        assert!(argos_render::murs::zones_dessinees().iter().any(|r| r.x == 8.0), "{w}: « Carnets » back");
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 F6 (PC 1440): the right-click menu opens beside the table, never over it nor over a wall pill.
#[test]
fn tour2_menu_a_cote_du_tableau() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    dossier_a_soi("tableaux2_menu");
    let mut a = tab_app(1440, 900, &[Ind::Tvd]);
    let t = tb::pose(&tb::cle(Ind::Tvd, "")).expect("the dashboard").rect;
    let (rw, th) = (a.rw(), a.top_h());
    a.mouse_moved(PhysicalPosition::new(t.cx() + rw, t.cy() + th));
    a.right_click();
    let m = a.pointer.tab.menu.clone().expect("its menu");
    tab_dessin(&a, false);
    let c = tb::menu_rect(&m).expect("drawn");
    let autour = Rect::new(t.x - tb::MARGE, t.y - tb::MARGE, t.w + 2.0 * tb::MARGE, t.h + 2.0 * tb::MARGE);
    assert!(!croisent(&c, &autour), "the menu {c:?} over the table {t:?}");
    for o in argos_render::murs::pilules_pleines() {
        assert!(!croisent(&c, &o), "the menu {c:?} over a wall pill {o:?}");
    }
    assert!(c.x >= 0.0 && c.x + c.w <= a.chart.viewport.plot_w() + 0.5);
}

/// 📋 F10 (cell 390): the legend's list unfolded, no high / low pill under its rows (re-laid out when it opens).
#[test]
fn tour2_cell_legende_ouverte_sans_etiquette_haut_bas_dessous() {
    use argos_render::indicators::Ind;
    dossier_a_soi("tableaux2_hilo");
    let mut a = tab_app(390, 844, &[Ind::Tvd, Ind::Sma]);
    a.ind.leg.0 = true;
    let _ = image_ecran(&mut a, 390, 844);
    tab_dessin(&a, true);
    let (haut, _) = argos_render::indleg::lignes_liste(&a.ind.out).expect("the list unfolded");
    let bas = argos_render::indleg::bas_legende(&a.ind.out, true, true);
    let liste = Rect::new(0.0, haut, a.chart.viewport.plot_w(), bas - haut);
    for p in argos_render::hilo_dessines() {
        assert!(!croisent(&p, &liste), "the high / low pill {p:?} under the legend {liste:?}");
    }
    argos_render::actions::set_tactile(false);
}

/// 📋 F9 (cell 390): the code view, the 📖 Doc and the editor start at the very top of the chart's area (over the
/// chart's title): no title cut in half above them.
#[test]
fn tour2_cell_editeur_depuis_le_haut() {
    use argos_render::indicators::Ind;
    let mut a = cell_390();
    a.code_indicateur(Ind::ArgosMurs);
    assert!(a.ui.editor.is_some());
    let r = a.ed_rect();
    assert_eq!((r.x, r.y, r.w), (a.rw(), a.top_h(), a.area.0 as f64), "the code view from the top");
    a.ui.editor.as_mut().unwrap().basculer_doc();
    assert_eq!(a.ed_rect().y, a.top_h(), "📖 Doc");
    a.ui.editor = None;
    a.editor_on(None, None);
    assert_eq!(a.ed_rect().y, a.top_h(), "the editor");
}

// ───── 📋 tableaux-1, tour 3 (Jee after trying v2) ─────

/// 📋 A script of the user whose small table asks for the top left of the chart, where the legend's list unfolds.
const TABLEAU_EN_HAUT_A_GAUCHE: &str = r#"//@version=6
indicator("Essai tableau", overlay = true)
var t = table.new(position.top_left, 2, 1, bgcolor = color.new(color.black, 30), frame_color = color.gray, frame_width = 1)
table.cell(t, 0, 0, "Essai", text_color = color.white)
table.cell(t, 1, 0, str.tostring(close), text_color = color.white)
"#;

/// 📋 H1 (PC 1440 × 900 and 1920 × 1080): the legend's list unfolded (TVD Pro, multi-UT, a moving average, Argos
/// Murs, a script whose table asks for the top left): no table (at its own place or dropped on the list) and no floating bar at its default place lies on a row
/// of the list, its name included (the legend's zones are only its buttons).
#[test]
fn tour3_pc_liste_ouverte_rien_dessus() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, TableLayout};
    dossier_a_soi("tableaux3_liste");
    for (w, h) in [(1440u32, 900u32), (1920, 1080)] {
        let mut a = tab_app(w, h, &[Ind::Tvd, Ind::MtfTable, Ind::Sma]);
        // (and the user's script whose small table asks for the top left, where the list unfolds)
        a.scripts.push(crate::types::SavedScript {
            id: "e55a1e55a1e55a7f".into(),
            name: "Essai tableau".into(),
            src: TABLEAU_EN_HAUT_A_GAUCHE.into(),
            on: true,
            ..Default::default()
        });
        a.ind.key = Default::default();
        a.refresh_indicators();
        // (the floating bar never moved by the hand: at its default place)
        a.ui.barre_par_defaut.set(true);
        a.ind.leg.0 = true;
        let _ = image_ecran(&mut a, w, h);
        tab_dessin(&a, false);
        let (haut, bas) = argos_render::indleg::lignes_liste(&a.ind.out).expect("the list unfolded");
        let text = argos_render::text::Text::default();
        let zones = argos_render::indleg::zones(&text, &a.ind.on, &a.ind.out, true, a.chart.viewport.plot_h());
        // (each row: from the chart's left edge to the end of its last button, 28 apart)
        let n = ((bas - haut) / 28.0).round() as usize;
        let rangees: Vec<Rect> = (0..n)
            .map(|k| {
                let y = haut + k as f64 * 28.0;
                let droite = zones
                    .iter()
                    .filter(|(r, _)| r.y >= y - 1.0 && r.y < y + 27.0)
                    .map(|(r, _)| r.x + r.w)
                    .fold(0.0f64, f64::max);
                Rect::new(0.0, y, droite, 24.0)
            })
            .collect();
        assert!(n >= 5 && rangees.iter().all(|r| r.w > 100.0), "{w}: the rows {rangees:?}");
        assert!(tb::poses().iter().any(|t| t.key.starts_with("script:")), "{w}: the script's table drawn");
        let bar = argos_render::toolbar::layout(&a.tools, &text).0;
        assert!(bar.w > 0.0, "{w}: the floating bar");
        for r in &rangees {
            assert!(!croisent(&bar, r), "{w}: the floating bar {bar:?} on the row {r:?}");
            for t in tb::poses() {
                assert!(!croisent(&t.rect, r), "{w}: {} {:?} on the row {r:?}", t.key, t.rect);
            }
        }
        // the dashboard dropped at the top left, on the rows: below them, never under the floating bar
        let key = tb::cle(Ind::Tvd, "");
        let t = tb::pose(&key).expect("the dashboard");
        a.ind.tables.insert(
            key.clone(),
            TableLayout::default().ancre(Rect::new(8.0, haut, t.rect.w, t.rect.h), t.area, None),
        );
        tab_dessin(&a, false);
        let t = tb::pose(&key).expect("still drawn");
        for r in rangees.iter().chain([&bar]) {
            assert!(!croisent(&t.rect, r), "{w}: dropped on the list: {:?} on {r:?}", t.rect);
        }
    }
}

/// 📋 H5 (PC): no grip and no handle any more; the mouse takes a table by its body, a corner (just inside it and up
/// to 10 points outside) or anywhere along an edge, with no crosshair; nothing is taken 20 points above it.
#[test]
fn tour3_souris_zones_invisibles() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, Prise};
    dossier_a_soi("tableaux3_souris");
    let mut a = tab_app(1440, 900, &[Ind::Tvd]);
    let t = tb::pose(&tb::cle(Ind::Tvd, "")).expect("the dashboard").rect;
    let attendu = [
        ((t.cx(), t.cy()), Prise::Corps),
        ((t.x - 3.0, t.y - 3.0), Prise::Coin(0)),
        ((t.x + t.w + 8.0, t.y - 2.0), Prise::Coin(1)),
        ((t.x + t.w + 2.0, t.y + t.h + 2.0), Prise::Coin(2)),
        ((t.x + t.w + 4.0, t.y + t.h * 0.3), Prise::Cote(1)),
        ((t.x + t.w * 0.25, t.y + t.h + 4.0), Prise::Cote(2)),
        ((t.x - 5.0, t.cy()), Prise::Cote(3)),
    ];
    for ((x, y), p) in attendu {
        assert_eq!(a.tab_prise_sous(x, y).map(|(_, q)| q), Some(p), "({x}, {y}) of {t:?}");
    }
    assert!(a.tab_prise_sous(t.cx(), t.y - 20.0).is_none(), "no grip above the table any more");
    let (rw, th) = (a.rw(), a.top_h());
    a.mouse_moved(PhysicalPosition::new(t.x + t.w + 4.0 + rw, t.cy() + th));
    assert_eq!(a.pointer.tab.survol.as_ref().map(|s| s.1), Some(Prise::Cote(1)), "hovered by its edge");
    assert!(a.chart_with(false).crosshair.is_none(), "no crosshair on its edge");
}

/// 📋 H6 (cell 390, tablet 820, by the finger): a selected table is scaled by one finger from 10 points inside a
/// corner (no handle to aim at), and moved, its size kept, from its middle.
#[test]
fn tour3_doigt_coin_invisible() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux as tb;
    use winit::event::TouchPhase;
    dossier_a_soi("tableaux3_doigt");
    for (w, h) in [(390u32, 844u32), (820, 1180)] {
        let mut a = tab_app(w, h, &[Ind::Tvd]);
        let key = tb::cle(Ind::Tvd, "");
        let t = tb::pose(&key).expect("the dashboard").rect;
        tab_toucher(&mut a, t.cx(), t.cy());
        assert_eq!(a.pointer.tab.choisi.as_deref(), Some(key.as_str()), "{w}: selected");
        let (rw, th) = (a.rw(), a.top_h());
        let glisser = |a: &mut App, x: f64, y: f64, dx: f64, dy: f64| {
            a.touch(1, TouchPhase::Started, (x + rw, y + th));
            for k in 1..=6 {
                a.touch(1, TouchPhase::Moved, (x + rw + dx * k as f64 / 6.0, y + th + dy * k as f64 / 6.0));
            }
            a.touch(1, TouchPhase::Ended, (x + rw + dx, y + th + dy));
            tab_dessin(a, true);
        };
        // (the corner facing the chart's middle, 10 points inside, pulled away from the table)
        let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
        let (droite, bas) = (t.cx() < pw / 2.0, t.cy() < ph / 2.0);
        let cx = if droite { t.x + t.w - 10.0 } else { t.x + 10.0 };
        let cy = if bas { t.y + t.h - 10.0 } else { t.y + 10.0 };
        glisser(&mut a, cx, cy, if droite { 40.0 } else { -40.0 }, if bas { 30.0 } else { -30.0 });
        let s1 = a.ind.tables.get(&key).map_or(1.0, |l| l.scale);
        assert!(s1 > 1.05, "{w}: a drag from inside a corner scales it: {s1}");
        let t1 = tb::pose(&key).expect("drawn").rect;
        glisser(&mut a, t1.cx(), t1.cy(), if droite { 30.0 } else { -30.0 }, 20.0);
        let t2 = tb::pose(&key).expect("drawn").rect;
        let s2 = a.ind.tables.get(&key).map_or(1.0, |l| l.scale);
        assert!((s2 - s1).abs() < 1e-9 && t2 != t1, "{w}: moved, its size kept: {t1:?} -> {t2:?}");
        assert!(a.pointer.tab.choisi.is_some(), "{w}: still selected");
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 H8: a double-click puts a moved table back in its place, away from the mouse: it is no longer hovered and the
/// crosshair is back. H9 (cell 390, tablet 820): « OK » on the finger's hint pill lays no crosshair.
#[test]
fn tour3_double_clic_et_ok_sans_reste() {
    use argos_render::indicators::Ind;
    use argos_render::tableaux::{self as tb, TableLayout};
    dossier_a_soi("tableaux3_double");
    let mut a = tab_app(1440, 900, &[Ind::Tvd]);
    let key = tb::cle(Ind::Tvd, "");
    let t = tb::pose(&key).expect("the dashboard");
    let (pw, ph) = (a.chart.viewport.plot_w(), a.chart.viewport.plot_h());
    let r = Rect::new(pw * 0.4, ph * 0.4, t.rect.w, t.rect.h);
    a.ind.tables.insert(key.clone(), TableLayout::default().ancre(r, t.area, None));
    tab_dessin(&a, false);
    let m = tb::pose(&key).expect("moved").rect;
    let (rw, th) = (a.rw(), a.top_h());
    a.pointer.tab.dernier = None;
    clic(&mut a, m.cx() + rw, m.cy() + th);
    clic(&mut a, m.cx() + rw, m.cy() + th);
    assert!(!a.ind.tables.contains_key(&key), "back in its place");
    let r0 = tb::pose(&key).expect("drawn").rect;
    assert!(!r0.contains(m.cx(), m.cy()), "away from the mouse: {r0:?}");
    assert!(a.pointer.tab.survol.is_none(), "not hovered any more");
    assert!(a.chart_with(false).crosshair.is_some(), "the crosshair is back");
    for (w, h) in [(390u32, 844u32), (820, 1180)] {
        let mut a = tab_app(w, h, &[Ind::Tvd]);
        let t = tb::pose(&key).expect("the dashboard").rect;
        tab_toucher(&mut a, t.cx(), t.cy());
        assert!(a.pointer.tab.choisi.is_some(), "{w}: selected");
        let ok = tb::ok_cible();
        tab_toucher(&mut a, ok.cx(), ok.cy());
        assert!(a.pointer.tab.choisi.is_none(), "{w}: « OK » ends it");
        assert!(a.ui.croix.is_none() && a.pointer.mouse.is_none(), "{w}: no crosshair where « OK » was");
        assert!(a.chart_with(false).crosshair.is_none(), "{w}: none drawn");
        argos_render::actions::set_tactile(false);
    }
}

/// 📋 H4: the indicator's ⋯ menu elides « de »: « Code d'Argos Murs », but « Code de MACD ».
#[test]
fn tour3_elision_du_menu() {
    use argos_render::indicators::Ind;
    assert!(Ind::Macd.name().starts_with('M'), "{}", Ind::Macd.name());
    let macd = format!("Code de {}", Ind::Macd.name());
    for (ind, texte) in [(Ind::ArgosMurs, "Code d'Argos Murs"), (Ind::Macd, macd.as_str())] {
        let mut a = cell_390();
        a.ind.on = vec![ind];
        a.ind.sel = Some(Ok(ind));
        let menu = a.ind_plus_menu();
        let code = menu.iter().flat_map(|g| &g.items).find(|it| it.id == 13).expect("« Code … »");
        assert!(code.label.starts_with(texte), "{} / {texte}", code.label);
    }
}

/// 🟠 📟 The tablet standing (819 points by the finger), the legend's list unfolded: the DÉMO pill is never over one
/// of its rows (it went over the first one): beside them, else under them, and never gone.
#[test]
fn tablette_debout_pilule_demo_hors_de_la_liste() {
    use argos_render::formfactor::{choose, Screen};
    use argos_render::indicators::Ind;
    for inds in [vec![Ind::Sma], vec![Ind::Sma, Ind::Tvd, Ind::MtfTable, Ind::Macd]] {
        let mut a = app();
        a.top.prefs.bienvenue_vue = true;
        a.ui.form = choose(&Screen {
            w: 1024.0,
            h: 1260.0,
            scale: 2.0,
            coarse: true,
            ..Default::default()
        });
        // (upright: its panel is a drawer, folded when the app first reads its screen)
        a.watch.prefs.open = false;
        a.ind.on.extend(inds.iter().copied());
        a.ind.leg.0 = true;
        // (its size in points: 1024 × 1260 px at the finger's zoom 1.25)
        let _ = image_ecran(&mut a, 819, 1008);
        a.ind.key = Default::default();
        let _ = image_ecran(&mut a, 819, 1008);
        assert!(!a.top.phone, "the PC bars");
        let (r, _) = a.bandeau_place();
        assert!(r.w > 0.0, "{inds:?}: the pill has its place");
        let text = argos_render::text::Text::default();
        let l = argos_render::indleg::liste_rect(&text, &a.ind.on, &a.ind.out, a.chart.viewport.plot_h()).expect("the list unfolded");
        let l = Rect::new(a.rw() + l.x, a.top_h() + l.y, l.w, l.h);
        assert!(!croisent(&r, &l), "{inds:?}: the pill {r:?} on the list {l:?}");
    }
    argos_render::actions::set_tactile(false);
}
