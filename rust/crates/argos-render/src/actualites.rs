//! 📰 The Actualités module (Jee's mock-up « actualités v1 », 22 Nov 2026), in the panel and in the column: header
//! « 📰 Actualités » ⚙ ✕, the field « Chercher dans les nouvelles », the crypto filter pill « BTC ✕ » (opened by
//! « Voir tout › » or by a ⚡), the tabs Toutes / Marché / ETF crypto / Régulation / Technologie, the news (coin
//! badges · time · date · source, then the title in bold; mouse over a row: « Ouvrir ↗ »), and at the bottom « Sources
//! n · à jour il y a x min » and the language. Under ⚙ « Tous les réglages »: each source (tick / untick), the
//! language (FR + EN, FR, EN) and « ⚡ sur la chart » yes / no.

use crate::eco::{Eco, EcoClick};
use crate::jetons as j;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_data::actu::{category_of, coins_of, Category};
use argos_data::eco::News;
use tiny_skia::Pixmap;

/// The languages offered (index kept in the settings).
pub const LANGUES: [&str; 3] = ["FR + EN", "FR", "EN"];
/// The tabs (0: every news).
pub const ONGLETS: [&str; 5] = ["Toutes", "Marché", "ETF crypto", "Régulation", "Technologie"];

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
const CHAMP_H: f64 = 34.0;
const ONGLET_H: f64 = 30.0;
const PIED: f64 = 40.0;
const TITRE: f32 = 14.5;
/// ⚙'s place from the right edge: left of the panel's ⬓ ⤢ ✕.
const REGLAGES_X: f64 = 140.0;

/// Where a click lands in the module.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NZone {
    Reglages,
    Champ,
    Filtre,
    Onglet(usize),
    /// ‹ (false) or › (true): slides the row of tabs.
    Defiler(bool),
    Article(String),
    Source(String),
    Langue(usize),
    Eclairs,
}

impl Eco {
    /// The news kept by the language, the sources ticked, the crypto filter, the tab and the search.
    pub fn news_shown(&self) -> Vec<&News> {
        let q = self.recherche.as_deref().map(|q| crate::i18n::plain(q.trim())).unwrap_or_default();
        self.news
            .iter()
            .filter(|n| match self.langue {
                1 => n.lang == "fr",
                2 => n.lang != "fr",
                _ => true,
            })
            .filter(|n| !self.sources_off.contains(&n.source))
            .filter(|n| self.filtre_coin.as_deref().is_none_or(|c| coins_of(&n.title).contains(&c)))
            .filter(|n| self.onglet == 0 || Category::ALL.get(self.onglet - 1) == Some(&category_of(&n.title)))
            .filter(|n| q.is_empty() || crate::i18n::plain(&n.title).contains(&q))
            .collect()
    }

    /// The sources the panel knows (every feed), in the feeds' order.
    pub fn news_sources() -> Vec<&'static str> {
        argos_data::eco::FEEDS.iter().map(|f| f.0).collect()
    }

    /// The tabs' widths on one line: roomy, else tighter (no name changed); `true` if they still do not fit (the row
    /// then slides sideways).
    fn onglets_largeurs(text: &Text, pw: f64) -> (Vec<f64>, f64, bool) {
        let place = pw - 2.0 * PAD;
        let mesure = |pad: f64, gap: f64| -> (Vec<f64>, f64) {
            let w: Vec<f64> = ONGLETS.iter().map(|l| text.width(l, 13.0) as f64 + pad).collect();
            (w, gap)
        };
        let total = |w: &[f64], gap: f64| w.iter().sum::<f64>() + gap * (w.len() - 1) as f64;
        for (pad, gap) in [(24.0, 4.0), (16.0, 2.0)] {
            let (w, gap) = mesure(pad, gap);
            if total(&w, gap) <= place {
                return (w, gap, false);
            }
        }
        let (w, gap) = mesure(16.0, 2.0);
        (w, gap, true)
    }

    /// The row's full length and the slide's maximum.
    fn onglets_max(text: &Text, pw: f64) -> f64 {
        let (w, gap, _) = Self::onglets_largeurs(text, pw);
        (w.iter().sum::<f64>() + gap * (w.len() - 1) as f64 - (pw - 2.0 * PAD)).max(0.0)
    }

    /// The tabs on one line (slid by `onglets_x` when they do not fit; a tab out of the row is left out).
    fn onglets(&self, text: &Text, pw: f64) -> Vec<(Rect, usize)> {
        let (largeurs, gap, _) = Self::onglets_largeurs(text, pw);
        let decal = self.onglets_x.clamp(0.0, Self::onglets_max(text, pw));
        let (mut x, y) = (PAD - decal, HEAD + CHAMP_H + 14.0);
        let mut out = vec![];
        for (k, w) in largeurs.into_iter().enumerate() {
            if x + w > PAD && x < pw - PAD {
                out.push((Rect::new(x, y, w, ONGLET_H), k));
            }
            x += w + gap;
        }
        out
    }

    /// Slides the row so tab `k` is in full view (with a bit of its neighbours).
    fn onglet_en_vue(&mut self, text: &Text, pw: f64, k: usize) {
        let (largeurs, gap, deborde) = Self::onglets_largeurs(text, pw);
        if !deborde {
            self.onglets_x = 0.0;
            return;
        }
        let debut: f64 = largeurs[..k].iter().map(|w| w + gap).sum();
        let place = pw - 2.0 * PAD;
        let w = largeurs.get(k).copied().unwrap_or(0.0);
        self.onglets_x = (debut - (place - w) / 2.0).clamp(0.0, Self::onglets_max(text, pw));
    }

    fn haut_liste(&self, text: &Text, pw: f64) -> f64 {
        let _ = (text, pw);
        HEAD + CHAMP_H + 14.0 + ONGLET_H + 10.0
    }

    fn row_h(&self, text: &Text, n: &News, pw: f64) -> f64 {
        let gras = Text::graisse(Graisse::Bold);
        let _ = text;
        let l = crate::assistant::wrap(&gras, &n.title, TITRE, pw - 2.0 * PAD).len().clamp(1, 2);
        30.0 + l as f64 * 19.5 + 12.0
    }

    /// The module's zones (content points: scrolling already counted).
    pub(crate) fn news_zones(&self, text: &Text, pw: f64) -> Vec<(Rect, NZone)> {
        // (⚙ left of the panel's own ⬓ ⤢ ✕, which close it: Jee's mock-up « ⚙ ✕ »)
        let mut z = vec![(Rect::new(pw - REGLAGES_X, 9.0, 30.0, 30.0), NZone::Reglages)];
        let filtre_w = self.filtre_coin.as_ref().map_or(0.0, |c| text.width(c, 13.0) as f64 + 62.0);
        z.push((
            Rect::new(PAD, HEAD + 4.0, pw - 2.0 * PAD - filtre_w - if filtre_w > 0.0 { 6.0 } else { 0.0 }, CHAMP_H),
            NZone::Champ,
        ));
        if filtre_w > 0.0 {
            z.push((Rect::new(pw - PAD - filtre_w, HEAD + 4.0, filtre_w, CHAMP_H), NZone::Filtre));
        }
        if self.reglages_news {
            let mut y = HEAD + CHAMP_H + 44.0;
            for s in Self::news_sources() {
                z.push((Rect::new(PAD, y, pw - 2.0 * PAD, 32.0), NZone::Source(s.to_string())));
                y += 34.0;
            }
            y += 30.0;
            let mut x = PAD;
            for (k, l) in LANGUES.iter().enumerate() {
                let w = text.width(l, 13.0) as f64 + 24.0;
                z.push((Rect::new(x, y, w, ONGLET_H), NZone::Langue(k)));
                x += w + 6.0;
            }
            y += ONGLET_H + 40.0;
            z.push((Rect::new(PAD, y, pw - 2.0 * PAD, 34.0), NZone::Eclairs));
            return z;
        }
        z.extend(self.onglets(text, pw).into_iter().map(|(r, k)| (r, NZone::Onglet(k))));
        // (the row slides: ‹ › at its ends while there is more on that side, over the tabs)
        let max = Self::onglets_max(text, pw);
        if max > 0.0 {
            let y = HEAD + CHAMP_H + 14.0;
            let decal = self.onglets_x.clamp(0.0, max);
            if decal > 0.5 {
                z.push((Rect::new(PAD - 4.0, y, 30.0, ONGLET_H), NZone::Defiler(false)));
            }
            if decal < max - 0.5 {
                z.push((Rect::new(pw - PAD - 26.0, y, 30.0, ONGLET_H), NZone::Defiler(true)));
            }
        }
        let mut y = self.haut_liste(text, pw);
        for n in self.news_shown() {
            let h = self.row_h(text, n, pw);
            z.push((Rect::new(6.0, y, pw - 12.0, h), NZone::Article(n.link.clone())));
            y += h;
        }
        z
    }

    /// The module's height (it scrolls).
    pub fn news_h(&self, text: &Text, pw: f64) -> f64 {
        let bas = self.news_zones(text, pw).iter().map(|(r, _)| r.y + r.h).fold(0.0, f64::max);
        bas.max(self.haut_liste(text, pw) + 60.0) + PIED + 10.0 + crate::eco::marge_ai()
    }

    /// A click in the module (content points).
    pub(crate) fn news_click(&mut self, text: &Text, pw: f64, x: f64, y: f64) -> EcoClick {
        let zones = self.news_zones(text, pw);
        // (‹ › lie over the tabs' ends: they win)
        let z = zones
            .iter()
            .find(|(r, z)| matches!(z, NZone::Defiler(_)) && r.contains(x, y))
            .or_else(|| zones.iter().find(|(r, _)| r.contains(x, y)))
            .map(|(_, z)| z.clone());
        let Some(z) = z else {
            self.recherche = self.recherche.take().filter(|q| !q.is_empty());
            return EcoClick::None;
        };
        if z != NZone::Champ {
            self.recherche = self.recherche.take().filter(|q| !q.is_empty());
        }
        match z {
            NZone::Reglages => self.reglages_news = !self.reglages_news,
            NZone::Champ => {
                self.recherche.get_or_insert_with(String::new);
            }
            NZone::Filtre => self.filtre_coin = None,
            NZone::Onglet(k) => {
                self.onglet = k;
                self.onglet_en_vue(text, pw, k);
            }
            NZone::Defiler(droite) => {
                let pas = (pw - 2.0 * PAD) * 0.6;
                self.onglets_x = (self.onglets_x + if droite { pas } else { -pas }).clamp(0.0, Self::onglets_max(text, pw));
            }
            NZone::Article(l) => return EcoClick::Open(l),
            NZone::Source(s) => match self.sources_off.iter().position(|x| *x == s) {
                Some(i) => {
                    self.sources_off.remove(i);
                }
                None => self.sources_off.push(s),
            },
            NZone::Langue(k) => self.langue = k,
            NZone::Eclairs => self.eclairs = !self.eclairs,
        }
        EcoClick::Changed
    }

    /// Keyboard in the search field (`None`: backspace).
    pub fn news_key(&mut self, c: Option<char>) {
        if let Some(q) = self.recherche.as_mut() {
            match c {
                Some(c) if !c.is_control() => q.push(c),
                None => {
                    q.pop();
                }
                _ => {}
            }
        }
    }

    /// The row of tabs, cut at the margins (a tab half out of the row is cut, not drawn over the edge), and ‹ › at the
    /// ends while the row slides.
    fn onglets_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, survol: &dyn Fn(&Rect) -> bool) {
        let gras = Text::graisse(Graisse::Bold);
        let y0 = HEAD + CHAMP_H + 14.0;
        let place = pw - 2.0 * PAD;
        let Some(mut row) = crate::px::int_rect(PAD, y0, place.max(1.0), ONGLET_H).and_then(|r| pm.clone_rect(r)) else {
            return;
        };
        let fond = row
            .pixel(0, 0)
            .map(|p| p.demultiply())
            .map_or(Rgb(0, 0, 0), |p| Rgb(p.red(), p.green(), p.blue()));
        for (r, z) in self.news_zones(text, pw) {
            let l = Rect::new(r.x - PAD, r.y - y0, r.w, r.h);
            match z {
                NZone::Onglet(k) => {
                    let on = k == self.onglet;
                    if on || survol(&r) {
                        round_fill(&mut row, l, l.h / 2.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.3 } else { 0.06 });
                    }
                    let f = if on { &gras } else { text };
                    f.draw(
                        &mut row,
                        ONGLETS[k],
                        (l.cx() as f32, (l.cy() + 4.5) as f32),
                        13.0,
                        if on { j::TEXTE } else { j::DIM },
                        Align::Center,
                    );
                }
                NZone::Defiler(droite) => {
                    // the row's background over the tab's end, then a small round ‹ / ›
                    let b = Rect::new(if droite { l.x + l.w - 26.0 } else { l.x + 4.0 }, l.y + 2.0, 22.0, l.h - 4.0);
                    let (a, z) = if droite { (b.x - 6.0, place) } else { (0.0, b.x + b.w + 6.0) };
                    crate::fill(&mut row, a, 0.0, z - a, l.h, fond, 0.92);
                    round_fill(&mut row, b, b.h / 2.0, j::BLANC, if survol(&r) { 0.18 } else { 0.10 });
                    text.draw(
                        &mut row,
                        if droite { "›" } else { "‹" },
                        (b.cx() as f32, (b.cy() + 5.0) as f32),
                        15.0,
                        j::TEXTE,
                        Align::Center,
                    );
                }
                _ => {}
            }
        }
        crate::px::put(pm, PAD, y0, &row);
    }

    /// Draws the module (`pw` wide; content points, `pm` as tall as [`Eco::news_h`]).
    pub(crate) fn news_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, now: i64, mouse: Option<(f64, f64)>, tz: i64) {
        let gras = Text::graisse(Graisse::Bold);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        gras.draw(pm, "📰 Actualités", (PAD as f32, 31.0), 17.0, j::TEXTE, Align::Left);
        let shown = self.news_shown();
        for (r, z) in self.news_zones(text, pw) {
            match z {
                NZone::Reglages => {
                    let on = self.reglages_news;
                    round_fill(
                        pm,
                        r,
                        r.h / 2.0,
                        if on { j::ACCENT } else { j::BLANC },
                        if on || survol(&r) { 0.2 } else { 0.07 },
                    );
                    crate::phosphor::draw(pm, "gear", r.cx(), r.cy(), 16, j::TEXTE);
                }
                NZone::Champ => {
                    round_fill(pm, r, r.h / 2.0, j::CHAMP.0, j::CHAMP.1);
                    round_stroke(pm, r, r.h / 2.0, j::sur_fond(j::LISERE), 1.0);
                    let (s, c) = match self.recherche.as_deref() {
                        Some(q) => (format!("{q}|"), j::TEXTE),
                        None => ("🔎  Chercher dans les nouvelles".to_string(), j::DIM),
                    };
                    text.draw(
                        pm,
                        &text.fit(&s, 13.5, r.w - 28.0),
                        ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        c,
                        Align::Left,
                    );
                }
                NZone::Filtre => {
                    let c = self.filtre_coin.clone().unwrap_or_default();
                    round_fill(pm, r, r.h / 2.0, j::ACCENT, 0.18);
                    round_stroke(pm, r, r.h / 2.0, j::ACCENT, 0.9);
                    crate::watchlist::coin_badge(pm, text, &c, None, r.x + 18.0, r.cy(), 10.0);
                    gras.draw(pm, &c, ((r.x + 34.0) as f32, (r.cy() + 5.0) as f32), 13.0, j::TEXTE, Align::Left);
                    text.draw(pm, "✕", ((r.x + r.w - 14.0) as f32, (r.cy() + 5.0) as f32), 12.0, j::DIM, Align::Center);
                }
                // (the tabs and ‹ › are drawn below, in their row cut at the margins)
                NZone::Onglet(_) | NZone::Defiler(_) => {}
                NZone::Article(l) => {
                    let Some(n) = shown.iter().find(|n| n.link == l) else { continue };
                    if survol(&r) {
                        round_fill(pm, r, j::R_LIGNE, j::BLANC, 0.05);
                        text.draw(pm, "Ouvrir ↗", ((r.x + r.w - 8.0) as f32, (r.y + 22.0) as f32), 12.5, j::ACCENT, Align::Right);
                    }
                    let coins = coins_of(&n.title);
                    let w = crate::eclair::pastilles(pm, text, &coins, PAD, r.y + 17.0, 9.0);
                    let quand = crate::eclair::quand(n, tz);
                    text.draw(
                        pm,
                        &text.fit(&quand, 12.5, r.w - w - 90.0),
                        ((PAD + w + 8.0) as f32, (r.y + 21.5) as f32),
                        12.5,
                        j::DIM,
                        Align::Left,
                    );
                    let l = crate::assistant::wrap(&gras, &n.title, TITRE, pw - 2.0 * PAD);
                    for (i, s) in l.iter().take(2).enumerate() {
                        let s = if i == 1 && l.len() > 2 {
                            gras.fit(&format!("{s}…"), TITRE, pw - 2.0 * PAD)
                        } else {
                            s.clone()
                        };
                        gras.draw(pm, &s, (PAD as f32, (r.y + 44.0 + i as f64 * 19.5) as f32), TITRE, j::TEXTE, Align::Left);
                    }
                }
                NZone::Source(s) => {
                    let on = !self.sources_off.contains(&s);
                    let b = Rect::new(r.x, r.cy() - 9.0, 18.0, 18.0);
                    round_fill(pm, b, 5.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.9 } else { 0.08 });
                    if on {
                        text.draw(pm, "✓", (b.cx() as f32, (b.cy() + 4.5) as f32), 12.0, j::BLANC, Align::Center);
                    }
                    let fr = argos_data::eco::FEEDS.iter().find(|f| f.0 == s).is_some_and(|f| f.2 == "fr");
                    text.draw(pm, &s, ((r.x + 28.0) as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Left);
                    text.draw(
                        pm,
                        if fr { "FR" } else { "EN" },
                        ((r.x + r.w) as f32, (r.cy() + 5.0) as f32),
                        12.0,
                        j::DIM,
                        Align::Right,
                    );
                }
                NZone::Langue(k) => {
                    let on = k == self.langue;
                    round_fill(pm, r, r.h / 2.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.3 } else { 0.06 });
                    text.draw(pm, LANGUES[k], (r.cx() as f32, (r.cy() + 4.5) as f32), 13.0, j::TEXTE, Align::Center);
                }
                NZone::Eclairs => {
                    text.draw(pm, "⚡ sur la chart", ((r.x) as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Left);
                    let t = Rect::new(r.x + r.w - 44.0, r.cy() - 11.0, 44.0, 22.0);
                    round_fill(
                        pm,
                        t,
                        11.0,
                        if self.eclairs { j::ACCENT } else { j::BLANC },
                        if self.eclairs { 0.85 } else { 0.15 },
                    );
                    let cx = if self.eclairs { t.x + t.w - 11.0 } else { t.x + 11.0 };
                    round_fill(pm, Rect::new(cx - 8.0, t.cy() - 8.0, 16.0, 16.0), 8.0, j::BLANC, 1.0);
                    text.draw(
                        pm,
                        if self.eclairs { "Oui" } else { "Non" },
                        ((t.x - 8.0) as f32, (t.cy() + 4.5) as f32),
                        12.5,
                        j::DIM,
                        Align::Right,
                    );
                }
            }
        }
        if !self.reglages_news {
            self.onglets_draw(pm, text, pw, &survol);
        }
        if self.reglages_news {
            let y0 = HEAD + CHAMP_H + 30.0;
            gras.draw(pm, "TOUS LES RÉGLAGES · SOURCES", (PAD as f32, y0 as f32), 11.5, j::DIM, Align::Left);
            let yl = y0 + 14.0 + Self::news_sources().len() as f64 * 34.0 + 22.0;
            gras.draw(pm, "LANGUE", (PAD as f32, yl as f32), 11.5, j::DIM, Align::Left);
        } else if shown.is_empty() {
            let msg = if self.news.is_empty() {
                if self.news_note.is_empty() {
                    "Chargement des nouvelles…"
                } else {
                    self.news_note.as_str()
                }
            } else if self.onglet > 0 && self.recherche.as_deref().is_none_or(str::is_empty) {
                "Aucune nouvelle dans cet onglet"
            } else {
                "Aucune nouvelle avec ce filtre"
            };
            let y = self.haut_liste(text, pw) + 24.0;
            text.draw(pm, &text.fit(msg, 13.0, pw - 2.0 * PAD), (PAD as f32, y as f32), 13.0, j::DIM, Align::Left);
        }
        let _ = now;
    }

    /// 📰 The module's bottom « Sources n · à jour il y a x min » and the language, on the `fond` band ending at `bas`
    /// (content points): the hosts draw it at the bottom of what is in view, the list scrolls above it.
    pub fn news_pied(&self, pm: &mut Pixmap, text: &Text, pw: f64, bas: f64, now: i64, fond: Rgb) {
        // (📱 the band goes down to `bas`, the text stays above the round Argos AI button)
        crate::fill(pm, 0.0, bas - PIED - crate::eco::marge_ai(), pw, PIED + crate::eco::marge_ai(), fond, 1.0);
        let h = bas - crate::eco::marge_ai();
        let y = h - PIED / 2.0 + 5.0;
        crate::fill(pm, PAD, h - PIED, pw - 2.0 * PAD, 1.0, j::BLANC, 0.08);
        let actives = Self::news_sources().iter().filter(|s| !self.sources_off.iter().any(|o| o == *s)).count();
        let age = self.news_at.map(|t| (now - t).max(0) / 60);
        let pied = match age {
            Some(m) => format!("Sources {actives} · à jour il y a {m} min"),
            None => format!("Sources {actives}"),
        };
        text.draw(pm, &text.fit(&pied, 12.5, pw * 0.62), (PAD as f32, y as f32), 12.5, j::DIM, Align::Left);
        text.draw(
            pm,
            &format!("Langue {}", LANGUES[self.langue.min(2)]),
            ((pw - PAD) as f32, y as f32),
            12.5,
            j::DIM,
            Align::Right,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(t: &str, ts: i64, src: &str, lang: &str) -> News {
        News {
            title: t.into(),
            link: format!("https://x/{ts}"),
            ts,
            source: src.into(),
            lang: lang.into(),
        }
    }

    fn eco() -> Eco {
        Eco {
            news: vec![
                n("Bitcoin : le BTC repasse au-dessus de 61 000 $", 5, "Cointribune", "fr"),
                n("Les ETF bitcoin et ether reçoivent 420 M$", 4, "Journal du Coin", "fr"),
                n("La SEC repousse sa décision sur solana", 3, "Invezz FR", "fr"),
                n("Ethereum network upgrade goes live", 2, "Decrypt", "en"),
                n("Crypto market adds $60 billion", 1, "CoinDesk", "en"),
            ],
            eclairs: true,
            ..Default::default()
        }
    }

    fn titres(e: &Eco) -> Vec<i64> {
        e.news_shown().iter().map(|n| n.ts).collect()
    }

    /// 📰 Filter on a crypto, tabs, language, search.
    #[test]
    fn filtres_du_module() {
        let mut e = eco();
        assert_eq!(titres(&e), [5, 4, 3, 2, 1]);
        e.filtre_coin = Some("BTC".into());
        assert_eq!(titres(&e), [5, 4]);
        e.filtre_coin = None;
        e.onglet = 2; // ETF crypto
        assert_eq!(titres(&e), [4]);
        e.onglet = 4; // Technologie
        assert_eq!(titres(&e), [2]);
        e.onglet = 1; // Marché
        assert_eq!(titres(&e), [5, 1]);
        e.onglet = 0;
        e.langue = 1;
        assert_eq!(titres(&e), [5, 4, 3]);
        e.langue = 2;
        assert_eq!(titres(&e), [2, 1]);
        e.langue = 0;
        e.recherche = Some("ETHEREUM".into());
        assert_eq!(titres(&e), [2], "no case");
        e.recherche = None;
        e.sources_off = vec!["Cointribune".into()];
        assert_eq!(titres(&e), [4, 3, 2, 1]);
    }

    /// 📰 Clicks: a tab, the filter's ✕, ⚙ and its settings, an article, an empty tab says so.
    #[test]
    fn clics_du_module() {
        let t = Text::default();
        let mut e = eco();
        e.filtre_coin = Some("BTC".into());
        let pw = 390.0;
        let zone = |e: &Eco, k: &NZone| e.news_zones(&t, pw).into_iter().find(|(_, z)| z == k).unwrap().0;
        let r = zone(&e, &NZone::Filtre);
        assert_eq!(e.news_click(&t, pw, r.cx(), r.cy()), EcoClick::Changed);
        assert!(e.filtre_coin.is_none(), "✕ removes the filter");
        let r = zone(&e, &NZone::Onglet(3));
        e.news_click(&t, pw, r.cx(), r.cy());
        assert_eq!(e.onglet, 3);
        assert_eq!(titres(&e), [3]);
        for (r, _) in e.onglets(&t, pw) {
            assert!(r.x + r.w <= pw - PAD + 0.1, "tabs inside a phone's width");
        }
        let a = zone(&e, &NZone::Article("https://x/3".into()));
        assert_eq!(e.news_click(&t, pw, a.cx(), a.cy()), EcoClick::Open("https://x/3".into()));
        let r = zone(&e, &NZone::Reglages);
        e.news_click(&t, pw, r.cx(), r.cy());
        assert!(e.reglages_news);
        let r = zone(&e, &NZone::Source("CoinDesk".into()));
        e.news_click(&t, pw, r.cx(), r.cy());
        assert_eq!(e.sources_off, ["CoinDesk"]);
        let r = zone(&e, &NZone::Langue(1));
        e.news_click(&t, pw, r.cx(), r.cy());
        assert_eq!(e.langue, 1);
        let r = zone(&e, &NZone::Eclairs);
        e.news_click(&t, pw, r.cx(), r.cy());
        assert!(!e.eclairs, "⚡ sur la chart: no");
        // the field takes the keyboard
        e.reglages_news = false;
        let r = zone(&e, &NZone::Champ);
        e.news_click(&t, pw, r.cx(), r.cy());
        for c in "sec".chars() {
            e.news_key(Some(c));
        }
        assert_eq!(e.recherche.as_deref(), Some("sec"));
        // drawn at every width
        for pw in [300.0, 390.0, 480.0] {
            let mut pm = crate::px::new(pw as u32, e.news_h(&t, pw) as u32).unwrap();
            e.news_draw(&mut pm, &t, pw, 100, Some((50.0, 200.0)), 0);
        }
    }
}
