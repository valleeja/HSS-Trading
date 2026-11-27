//! 🔎 Argos Lens dans l'app, au toucher sur cell (390 × 844) : l'indicateur s'ajoute, sa carte se calcule sur les
//! bougies clôturées, et de vrais touchers sur la légende la cachent puis la remontrent (👁).

use crate::*;
use argos_render::indicators::Ind;
use argos_render::indleg::{zones_de, Cell, LegClick};

/// 300 bougies de 30 min en hausse avec des vagues (finies depuis longtemps : toutes clôturées).
fn cell_avec_lens() -> App {
    let iv = Interval(1800);
    let mut a = App::blank(Source::File("essai.json".into()), iv, iv, vec![], "essai.json · 30m · fichier".into());
    a.chart.raw = (0..300)
        .map(|i| {
            let c = 2000.0 + i as f64 * 1.2 + (i as f64 / 3.0).sin() * 4.0;
            Bar {
                time: 14_400 * 118_052 + i * 1800,
                open: c - 1.5,
                high: c + 3.0,
                low: c - 3.5,
                close: c,
                volume: 100.0 + (i % 7) as f64 * 20.0,
            }
        })
        .collect();
    a.rebuild();
    a.start_free();
    a.top.phone = true;
    a.top.prefs.bienvenue_vue = true;
    a.area = (390, (844.0 - a.top_h() - argos_render::ecrans::dock_h()) as u32);
    a.ui.frames = 5;
    a.ind.on = vec![Ind::Lens];
    a.refresh_indicators();
    a
}

/// Un vrai toucher (doigt posé puis levé) à ce point de la fenêtre.
fn toucher(a: &mut App, x: f64, y: f64) {
    use winit::event::TouchPhase;
    a.touch(7, TouchPhase::Started, (x, y));
    a.touch(7, TouchPhase::Ended, (x, y));
}

/// La zone de la légende qui fait `c`, en coordonnées de la fenêtre.
fn zone(a: &App, c: LegClick) -> Option<(f64, f64)> {
    let t = argos_render::text::Text::default();
    let (ph, pw) = (a.chart.viewport.plot_h(), a.chart.viewport.plot_w());
    let cell = Cell {
        actif: true,
        sel: a.ind.sel,
        pw,
    };
    let z = zones_de(&t, &a.ind.on, &a.ind.out, true, ph, pw, cell);
    z.iter().find(|(_, x)| *x == c).map(|(r, _)| (r.cx() + a.rw(), r.cy() + a.top_h()))
}

#[test]
fn lens_sur_cell_au_toucher() {
    let mut a = cell_avec_lens();
    let v = a.ind.out.lens.clone().expect("Lens calculé");
    let (i, photo) = v.photo.expect("une photo de la dernière bougie clôturée");
    assert!(i < a.chart.bars.len() && photo.regime.is_some() && v.erreur.is_none());
    // la légende des indicateurs ouverte (⌄), un vrai toucher sur « Argos Lens » ouvre sa barre 👁 ⚙ 🗑 ⋯
    a.ind.leg.0 = true;
    a.tools.hidden = true; // la barre des favoris n'est pas sur la légende
    let (x, y) = zone(&a, LegClick::Nom(Ind::Lens)).expect("le nom d'Argos Lens dans la légende");
    toucher(&mut a, x, y);
    assert_eq!(a.ind.sel, Some(Ok(Ind::Lens)), "toucher le nom ouvre sa barre");
    // 👁 : la carte se cache, puis revient
    let (x, y) = zone(&a, LegClick::Eye(Ind::Lens)).expect("👁");
    toucher(&mut a, x, y);
    assert!(a.ind.out.hidden.contains(&Ind::Lens), "carte cachée");
    let (x, y) = zone(&a, LegClick::Eye(Ind::Lens)).expect("👁");
    toucher(&mut a, x, y);
    assert!(!a.ind.out.hidden.contains(&Ind::Lens), "carte revenue");
}
