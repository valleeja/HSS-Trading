//! ⛶ Full screen of the chart or of one indicator panel (Jee, 17 Nov; preview images 1 to 4 of
//! apercus/2026-11-17-plein-ecran-*). Phone and tablet: a double tap on the chart or on a panel, a double tap again
//! to come back; the PC: three clicks (the third undoes what the double-click did), the last line of the chart's
//! right-click menu, and back with a double-click, Échap or « Quitter le plein écran ». The main chart only (no
//! split).

use crate::*;
use argos_render::indicators::Ind;

/// What is full screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum PleinEcran {
    /// The chart of the prices, without its panels.
    Prix,
    /// One indicator panel alone, the whole height, its own scale.
    Panneau(Ind),
}

/// 🖥 What the third click undoes before going full screen: the double-click's auto ⇄ free switch, or its panel
/// enlarged ⇄ normal (the panel's height before it).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Defaire {
    Echelle,
    Panneau(Ind, Option<f64>),
}

/// How long the « Double tap pour revenir » bubble stays, and how many times it is shown at most.
const BULLE_MS: u128 = 2000;
const BULLE_FOIS: u8 = 3;

impl App {
    /// ⛶ Full screen is possible: the main chart alone (no split), no full page over it.
    pub(crate) fn plein_possible(&self) -> bool {
        self.panes.is_empty() && self.ui.accueil.is_none()
    }

    /// ⛶ What goes full screen from the height `y` of the chart: the panel there, else the chart.
    pub(crate) fn plein_cible_en(&self, y: f64) -> PleinEcran {
        match self.ind.out.band_at(self.chart.viewport.plot_h(), y) {
            Some(i) => PleinEcran::Panneau(i),
            None => PleinEcran::Prix,
        }
    }

    /// ⛶ Its name in the menu line « ⛶ Plein écran : <name> ».
    fn plein_nom(&self, c: PleinEcran) -> String {
        match c {
            PleinEcran::Prix => self.symbol_label(),
            PleinEcran::Panneau(i) => i.name().to_string(),
        }
    }

    /// ⛶ Full screen of `c`: 🖥 the bars go as in Zen (the PC's « barres cachées »), 📱 the top row and the pill fold
    /// away; the panels go (the chart) or the panel takes the whole height (a panel). The bubble the first 3 times.
    pub(crate) fn entrer_plein_ecran(&mut self, c: PleinEcran) {
        if !self.plein_possible() {
            return self.redraw();
        }
        if self.ui.plein_ecran.is_none() && !self.top.phone {
            self.ui.plein_zen_avant = self.ui.zen;
            self.ui.zen = true;
        }
        self.ui.plein_ecran = Some(c);
        (self.ui.context, self.ui.list, self.ui.croix, self.ui.pfeuille) = (Default::default(), None, None, None);
        self.ind.key = Default::default(); // the panels again (gone for the chart, one alone for a panel)
        let mut fois = 0;
        Self::interface2_changer(|i| {
            fois = i.plein_ecran_bulles;
            i.plein_ecran_bulles = i.plein_ecran_bulles.saturating_add(1).min(BULLE_FOIS);
        });
        if fois < BULLE_FOIS {
            // (👆 the tablet by the finger too: no double click, no Escape key)
            let t = if self.top.phone || argos_render::actions::tactile() {
                "Double tap pour revenir"
            } else {
                "Double-clic ou Échap pour revenir"
            };
            self.ui.bulle_plein = Some((t, Instant::now()));
        }
        self.redraw();
    }

    /// ⛶ Back from full screen: everything as before (🖥 Zen as it was).
    pub(crate) fn quitter_plein_ecran(&mut self) {
        if self.ui.plein_ecran.take().is_none() {
            return self.redraw();
        }
        if !self.top.phone {
            self.ui.zen = self.ui.plein_zen_avant;
        }
        self.ui.bulle_plein = None;
        self.ind.key = Default::default();
        self.redraw();
    }

    /// ⛶ The menu line: full screen of what was under the right-click (or the finger's crosshair), or back from it.
    pub(crate) fn basculer_plein_ecran(&mut self) {
        if self.ui.plein_ecran.is_some() {
            return self.quitter_plein_ecran();
        }
        let c = self.ui.plein_cible.unwrap_or(PleinEcran::Prix);
        self.entrer_plein_ecran(c);
    }

    /// ⛶ Each image: the panel full screen (argos-render) and the chart menu's last line.
    pub(crate) fn plein_ecran_par_image(&self) {
        let panneau = match self.ui.plein_ecran {
            Some(PleinEcran::Panneau(i)) => Some(i),
            _ => None,
        };
        argos_render::indicators::set_plein_ecran(panneau);
        // 📱 none on the phone: one chart on the screen, its full screen changes almost nothing (Jee, 21 Nov, list 2)
        let ligne = (self.plein_possible() && !self.pilule()).then(|| {
            let c = self.ui.plein_ecran.or(self.ui.plein_cible).unwrap_or(PleinEcran::Prix);
            (self.plein_nom(c), self.ui.plein_ecran.is_some())
        });
        argos_render::context_menu::set_plein_ecran(ligne);
    }

    /// ⛶ The chart full screen: its panels go (their indicators stay on, back with them). A panel full screen whose
    /// indicator left the chart: back from full screen (no bubble, no ghost state).
    pub(crate) fn plein_ecran_sans_panneaux(&mut self) {
        if let Some(PleinEcran::Panneau(i)) = self.ui.plein_ecran {
            if !self.ind.out.bands(1.0).iter().any(|(b, _)| *b == i) {
                self.ui.plein_ecran = None;
                self.ui.bulle_plein = None;
                if !self.top.phone {
                    self.ui.zen = self.ui.plein_zen_avant;
                }
            }
        }
        if self.ui.plein_ecran == Some(PleinEcran::Prix) {
            let o = &mut self.ind.out;
            (o.cipher, o.ai, o.tbo) = (None, None, None);
            o.plots.retain(|p| !p.sub);
            o.scripts.retain(|s| s.out.overlay);
        }
    }

    /// 🖥 The third click (350 ms, 24 points, like the double-click): undoes what the double-click did, then full
    /// screen of what is under the mouse. `true` if it was one.
    pub(crate) fn triple_clic(&mut self, now: Instant) -> bool {
        let Some((t, d, c)) = self.pointer.triple.take() else { return false };
        if !self.est_double(Some(t), now) {
            return false;
        }
        match d {
            Defaire::Echelle => {
                self.run_action(ChartAction::AutoScale);
                self.banner = None;
            }
            Defaire::Panneau(i, avant) => match avant {
                Some(h) => _ = self.chart.settings.bands.insert(i, h),
                None => _ = self.chart.settings.bands.remove(&i),
            },
        }
        self.pointer.last_chart_click = None;
        self.entrer_plein_ecran(c);
        true
    }

    /// ⛶ The bubble « Double tap pour revenir » (2 s, the first 3 times), at the top of the chart in the middle.
    pub(crate) fn bulle_plein_ecran(&self, pm: &mut tiny_skia::Pixmap) {
        let Some((t, _)) = self.ui.bulle_plein.filter(|(_, d)| d.elapsed().as_millis() < BULLE_MS) else {
            return;
        };
        let text = argos_render::text::Text::default();
        let w = text.width(t, 13.0) as f64 + 28.0;
        // (📱 under the legend's lines and its ⌄; 🖥 at the top, beside them)
        let y = if self.top.phone { 104.0 + argos_render::haut_legende() } else { 58.0 };
        let r = Rect::new(((self.chart.viewport.plot_w() - w) / 2.0).max(8.0), y, w, 30.0);
        argos_render::shapes::round_fill(pm, r, r.h / 2.0, argos_render::jetons::FOND, 0.92);
        argos_render::shapes::round_stroke(pm, r, r.h / 2.0, argos_render::jetons::sur_fond(argos_render::jetons::LISERE), 1.0);
        text.draw(
            pm,
            t,
            (r.cx() as f32, (r.cy() + 4.5) as f32),
            13.0,
            argos_render::jetons::TEXTE,
            argos_render::text::Align::Center,
        );
        if let Some(w) = &self.window {
            w.request_redraw(); // it goes on its own after 2 s
        }
    }
}
