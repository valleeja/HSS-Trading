//! 📐 Toutes les zones de l'écran PC (fenêtre) au même endroit : les cases fixes (1re rangée, rail, colonne de modules,
//! zone des prix, panneaux, axe du temps, échelle, volet, colonne d'icônes) et ce qui flotte par-dessus (bande et
//! rangées du haut, pied, Argos AI, DÉMO, barre de favoris, carte Lens, légendes des panneaux). Sert au test « rien ne se
//! chevauche » qui parcourt les états des barres.

use crate::*;

/// Bas de la 1re rangée de la barre du haut (8 + 44, et l'écart de 4 avec la 2e).
const BAS_RANGEE1: f64 = 8.0 + argos_render::formfactor::RANGEE1_PC + 4.0;

/// 🧘 The top bar brought back over the chart in Zen.
pub(crate) const ZEN_HAUT: &str = "barre du haut (Zen)";

/// Une zone de l'écran PC.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Genre {
    /// Case fixe : aucune case n'en croise une autre.
    Case,
    /// Flotte sur la chart (zone des prix, panneaux, axe du temps) : ne croise ni une autre case ni un autre flottant.
    Flotte,
    /// Flotte dans la 1re rangée (pilule DÉMO).
    DansLeHaut,
    /// Légende d'un panneau (son nom) : seule la barre de favoris ne doit pas la cacher.
    Legende,
}

impl App {
    /// 📐 Les zones de l'écran PC (coordonnées de la fenêtre), d'après la dernière mise en page (`placer`) et, pour la
    /// carte Lens, le dernier dessin de la chart.
    pub(crate) fn zones_pc(&self) -> Vec<(String, Genre, Rect)> {
        use Genre::*;
        let mut z: Vec<(String, Genre, Rect)> = vec![];
        let mut met = |nom: &str, g: Genre, r: Rect| {
            if r.w > 0.5 && r.h > 0.5 {
                z.push((nom.to_string(), g, r));
            }
        };
        let (rw, top_h) = (self.rw(), self.top_h());
        let vp = &self.chart.viewport;
        let (pw, ph) = (vp.plot_w(), vp.plot_h());
        let osc = self.ind.out.osc_h();
        // 1re rangée (jusqu'à 56), puis les rangées 2 et 3 quand elles restent ouvertes (entre leurs bords : le volet
        // flottant commence à 66, à droite d'elles)
        met("1re rangée", Case, Rect::new(0.0, 0.0, self.top_w(), top_h.min(BAS_RANGEE1)));
        if top_h > BAS_RANGEE1 {
            if let Some((g, d)) = self.rangs_haut() {
                met("rangées 2 et 3", Case, Rect::new(g, BAS_RANGEE1, d - g, top_h - BAS_RANGEE1));
            }
        }
        if let Some(b) = self.bande_haut() {
            met("bande du haut", Flotte, b);
        }
        // 🧘 Zen: the top bar, brought back over the chart at the top edge
        if let Some(r) = self.zen_haut_rect() {
            met(ZEN_HAUT, Flotte, r);
        }
        if self.top.sans_bandeaux && self.top.survol_bandeaux {
            if let Some((x0, x1)) = self.rangs_haut() {
                met(
                    "rangées ouvertes",
                    Flotte,
                    Rect::new(x0, top_h, x1 - x0, argos_render::topbar::HEIGHT as f64 - top_h),
                );
            }
        }
        met("rail", Case, Rect::new(0.0, top_h, rw - self.dock.left, self.area.1 as f64));
        if let Some(d) = self.dock.rect {
            met("colonne de modules", Case, d);
        }
        met("zone des prix", Case, Rect::new(rw, top_h, pw, ph - osc));
        for (i, top) in self.ind.out.bands(ph) {
            met(&format!("panneau {i:?}"), Case, Rect::new(rw, top_h + top, pw, i.band_h()));
        }
        for (n, r) in self.legendes() {
            met(&n, Legende, r);
        }
        met("axe du temps", Case, Rect::new(rw, top_h + ph, pw, vp.time_axis as f64));
        met("échelle de prix", Case, Rect::new(rw + pw, top_h, vp.price_axis as f64, ph));
        // la case du coin (sous l'échelle, à droite de l'axe du temps) : celle du bouton Argos AI
        met("case du coin", Case, Rect::new(rw + pw, top_h + ph, vp.price_axis as f64, vp.time_axis as f64));
        let sx = self.side_x();
        if self.watch.prefs.open && !self.ui.zen {
            let v = self.volet_rect().unwrap_or(Rect::new(sx, top_h, self.watch.bar_x(), self.area.1 as f64));
            met("volet", Case, v);
        }
        if let Some(c) = self.watch.col_cadre() {
            met("colonne d'icônes", Case, Rect::new(sx + c.x, top_h + c.y, c.w, c.h));
        }
        if let Some(p) = self.pied_rect() {
            met("pied", Flotte, Rect::new(rw + p.x, top_h + p.y, p.w, p.h));
        }
        // ✨ Argos AI : le rond, centré dans la case du coin (toute la case se clique)
        met("Argos AI", Flotte, self.ai_rond());
        // 🕐 l'horloge, au bout à droite de l'axe du temps (même calcul que son dessin dans argos-render)
        if vp.footer == 0 {
            let t = argos_render::text::Text::default();
            let w = t.width("00:00:00 UTC+10:30", 12.0) as f64 + 16.0;
            met("horloge", Flotte, Rect::new(rw + pw - w, top_h + ph, w, vp.time_axis as f64));
        }
        // (🧘 in Zen the DÉMO pill is in the hidden top bar: there only while the bar is brought back)
        if !self.ui.zen || self.ui.zen_haut {
            met("DÉMO", DansLeHaut, self.bandeau_place().0);
        }
        if !self.tools.hidden {
            let (b, _) = toolbar::layout(&self.tools, &argos_render::text::Text::default());
            met("favoris", Flotte, Rect::new(rw + b.x, top_h + b.y, b.w, b.h));
        }
        if !self.top.phone {
            let p = argos_render::navbar::rect(vp, ph, true);
            met("pilule de navigation", Flotte, Rect::new(rw + p.x, top_h + p.y, p.w, p.h));
        }
        if let Some(l) = argos_render::indicators::carte_lens() {
            met("Lens", Flotte, Rect::new(rw + l.x, top_h + l.y, l.w, l.h));
        }
        z
    }
}

/// 📐 Les chevauchements interdits parmi `zones` dans une fenêtre `w` × `h` (une ligne par défaut trouvé).
pub(crate) fn chevauchements(zones: &[(String, Genre, Rect)], w: f64, h: f64) -> Vec<String> {
    use Genre::*;
    let croise = |a: &Rect, b: &Rect| a.x < b.x + b.w - 0.5 && b.x < a.x + a.w - 0.5 && a.y < b.y + b.h - 0.5 && b.y < a.y + a.h - 0.5;
    let dedans = |a: &Rect, b: &Rect| a.x >= b.x - 0.5 && a.y >= b.y - 0.5 && a.x + a.w <= b.x + b.w + 0.5 && a.y + a.h <= b.y + b.h + 0.5;
    let fenetre = Rect::new(0.0, 0.0, w, h);
    // les cases où un flottant peut se poser
    let chart = |n: &str| n == "zone des prix" || n == "axe du temps" || n.starts_with("panneau ");
    let mut out = vec![];
    for (i, (na, ga, a)) in zones.iter().enumerate() {
        if !dedans(a, &fenetre) {
            out.push(format!("{na} sort de la fenêtre {a:?}"));
        }
        for (nb, gb, b) in zones.iter().skip(i + 1) {
            if !croise(a, b) {
                continue;
            }
            let permis = match (ga, gb) {
                (Case, Case) => false,
                (Legende, _) | (_, Legende) => {
                    // la légende est sur son panneau ; seule la barre de favoris ne doit pas la cacher
                    let autre = if *ga == Legende { nb } else { na };
                    // (🧘 nor the top bar brought back in Zen: the legend goes down under it)
                    autre != "favoris" && autre != "bande du haut" && autre != ZEN_HAUT
                }
                (DansLeHaut, Case) => nb == "1re rangée" || (nb != "1re rangée" && zones.iter().any(|(n, _, _)| n == ZEN_HAUT) && chart(nb)),
                (Case, DansLeHaut) => na == "1re rangée" || (na != "1re rangée" && zones.iter().any(|(n, _, _)| n == ZEN_HAUT) && chart(na)),
                (DansLeHaut, Flotte) => nb == ZEN_HAUT,
                (Flotte, DansLeHaut) => na == ZEN_HAUT,
                // (🧘 the top bar brought back in Zen lies over the top of the chart and of its price scale)
                (Flotte, Case) => chart(nb) || (na == "Argos AI" && nb == "case du coin") || (na == ZEN_HAUT && nb == "échelle de prix"),
                (Case, Flotte) => chart(na) || (nb == "Argos AI" && na == "case du coin") || (nb == ZEN_HAUT && na == "échelle de prix"),
                // la bande et les rangées du haut sont l'une ou l'autre
                // (les rangées ouvertes au survol passent par-dessus ce qui flotte en haut de la chart, le temps du survol)
                // (le pied, montré au survol du bas, passe par-dessus l'horloge le temps du survol : déjà le cas avant ;
                // and over the navigation pill too — Jee, 17 Nov)
                (Flotte, Flotte) => {
                    let pied = |a: &str, b: &str| a == "pied" && (b == "horloge" || b == "pilule de navigation");
                    // (🧘 the top bar brought back in Zen passes over what floats at the top of the chart, while hovered)
                    na == "rangées ouvertes" || nb == "rangées ouvertes" || na == ZEN_HAUT || nb == ZEN_HAUT || pied(na, nb) || pied(nb, na)
                }
                _ => false,
            };
            if !permis {
                out.push(format!("{na} × {nb}"));
            }
        }
    }
    // ✨ le rond Argos AI : dans la case du coin, à 2 points au moins de chaque bord
    if let (Some((_, _, r)), Some((_, _, c))) = (
        zones.iter().find(|(n, _, _)| n == "Argos AI"),
        zones.iter().find(|(n, _, _)| n == "case du coin"),
    ) {
        if r.x < c.x + 2.0 - 0.01 || r.y < c.y + 2.0 - 0.01 || r.x + r.w > c.x + c.w - 2.0 + 0.01 || r.y + r.h > c.y + c.h - 2.0 + 0.01 {
            out.push(format!("Argos AI pas à 2 points dans la case du coin ({r:?} / {c:?})"));
        }
    }
    // la carte Lens : entière dans la zone des prix
    if let (Some((_, _, l)), Some((_, _, p))) = (zones.iter().find(|(n, _, _)| n == "Lens"), zones.iter().find(|(n, _, _)| n == "zone des prix")) {
        if !dedans(l, p) {
            out.push(format!("Lens pas entière dans la zone des prix ({l:?} / {p:?})"));
        }
    }
    out
}
