//! 📶 Stats Hyperliquid (Jee's mock-ups « stats hyperliquid v1 », PC and phone, 22 Nov 2026): the whole exchange at a
//! glance. Header « 📶 Stats Hyperliquid » ⚙, 4 tiles (Volume 24 h, Open interest in dollars, Funding moyen weighted by
//! the open interest per hour, Marchés with how many HIP-3), tabs Marchés / Funding / HIP-3, and the table: Marché,
//! OI (amount and a bar, its share of the total), Vol. 24 h, Funding 1 h, 24 h. A row opens that market's chart; the
//! sort is chosen at the bottom (OI / Volume / Funding / 24 h) or under ⚙.
//!
//! 🚦 No request of its own: the figures are the app's price list (`metaAndAssetCtxs` of Hyperliquid and of the HIP-3
//! dexes already read for the watchlist), taken again every 30 s while the panel is open.

use crate::hlexplorer::{couleur_affichee, court_dollars, pourcent};
use crate::jetons as j;
use crate::shapes::{round_fill, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_data::watchlist::Quote;
use tiny_skia::Pixmap;

pub const ONGLETS: [&str; 3] = ["Marchés", "Funding", "HIP-3"];
pub const TRIS: [&str; 4] = ["Open interest", "Volume", "Funding", "24 h"];
/// Rows shown in the Marchés and HIP-3 tabs (the rest is counted at the end).
pub const MAX_LIGNES: usize = 40;
/// The figures are taken again from the price list every 30 s while the panel is open.
pub const RELECTURE_S: i64 = 30;

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
const TUILE_H: f64 = 70.0;
const ONGLET_H: f64 = 36.0;
const LIGNE: f64 = 46.0;
const SECTION: f64 = 28.0;
const PIED: f64 = 40.0;
const REGLAGES_X: f64 = 140.0;

/// One market's figures.
#[derive(Debug, Clone, PartialEq)]
pub struct Ligne {
    pub coin: String,
    /// Open interest in dollars.
    pub oi: f64,
    pub volume: f64,
    /// Hourly funding (fraction).
    pub funding: f64,
    /// 24 h change (fraction; `None`: no price of yesterday).
    pub var: Option<f64>,
}

impl Ligne {
    pub fn of(q: &Quote) -> Ligne {
        let mark = q.mark.parse::<f64>().unwrap_or(0.0);
        Ligne {
            coin: q.coin.clone(),
            oi: q.oi * mark,
            volume: q.volume,
            funding: q.funding,
            var: (q.prev > 0.0 && mark > 0.0).then(|| mark / q.prev - 1.0),
        }
    }

    pub fn hip3(&self) -> bool {
        self.coin.contains(':')
    }
}

/// The exchange's totals.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Totaux {
    pub volume: f64,
    pub oi: f64,
    /// Hourly funding, weighted by each market's open interest in dollars.
    pub funding: f64,
    pub marches: usize,
    pub hip3: usize,
}

pub fn totaux(l: &[Ligne]) -> Totaux {
    let oi: f64 = l.iter().map(|x| x.oi).sum();
    Totaux {
        volume: l.iter().map(|x| x.volume).sum(),
        oi,
        funding: if oi > 0.0 {
            l.iter().map(|x| x.funding * x.oi).sum::<f64>() / oi
        } else {
            0.0
        },
        marches: l.len(),
        hip3: l.iter().filter(|x| x.hip3()).count(),
    }
}

/// Sorted by `tri` (index in [`TRIS`]), the biggest first.
pub fn trier(l: &mut [Ligne], tri: usize) {
    let cle = |x: &Ligne| match tri {
        1 => x.volume,
        2 => x.funding,
        3 => x.var.unwrap_or(f64::MIN),
        _ => x.oi,
    };
    l.sort_by(|a, b| cle(b).total_cmp(&cle(a)).then_with(|| a.coin.cmp(&b.coin)));
}

/// The Funding tab: the 10 highest, then the 10 lowest (lowest first), markets with open interest only.
pub fn extremes(l: &[Ligne]) -> (Vec<Ligne>, Vec<Ligne>) {
    let mut v: Vec<Ligne> = l.iter().filter(|x| x.oi > 0.0).cloned().collect();
    trier(&mut v, 2);
    let hauts: Vec<Ligne> = v.iter().take(10).cloned().collect();
    let bas: Vec<Ligne> = v.iter().rev().take(10.min(v.len().saturating_sub(hauts.len()))).cloned().collect();
    (hauts, bas)
}

/// The module's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Stats {
    pub lignes: Vec<Ligne>,
    /// Time the figures were taken (ms), and now.
    pub lu_ms: Option<i64>,
    pub now_ms: i64,
    pub onglet: usize,
    pub tri: usize,
    pub reglages: bool,
}

#[derive(Debug, Clone, PartialEq)]
enum Zone {
    Reglages,
    Onglet(usize),
    Marche(String),
    Tri(usize),
    TriSuivant,
}

/// What a click asks of the app.
#[derive(Debug, Clone, PartialEq)]
pub enum StClick {
    None,
    Changed,
    Chart(String),
}

/// What is drawn in the table, top to bottom.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Titre(&'static str),
    Ligne(Ligne),
    Reste(usize),
}

impl Stats {
    /// Takes the figures again from the app's price list (every [`RELECTURE_S`] while open).
    pub fn prendre<'a>(&mut self, quotes: impl Iterator<Item = &'a Quote>, now_ms: i64) {
        self.lignes = quotes.map(Ligne::of).collect();
        self.lignes.sort_by(|a, b| a.coin.cmp(&b.coin));
        self.lignes.dedup_by(|a, b| a.coin == b.coin);
        self.lu_ms = Some(now_ms);
    }

    /// Due for a new look (never taken, or 30 s ago).
    pub fn du(&self, now_ms: i64) -> bool {
        self.lu_ms.is_none_or(|t| now_ms - t >= RELECTURE_S * 1000)
    }

    fn items(&self) -> Vec<Item> {
        match self.onglet {
            1 => {
                let (h, b) = extremes(&self.lignes);
                let mut v = vec![Item::Titre("LES 10 PLUS HAUTS")];
                v.extend(h.into_iter().map(Item::Ligne));
                v.push(Item::Titre("LES 10 PLUS BAS"));
                v.extend(b.into_iter().map(Item::Ligne));
                v
            }
            k => {
                let mut l: Vec<Ligne> = self.lignes.iter().filter(|x| k != 2 || x.hip3()).cloned().collect();
                trier(&mut l, self.tri);
                let reste = l.len().saturating_sub(MAX_LIGNES);
                let mut v: Vec<Item> = l.into_iter().take(MAX_LIGNES).map(Item::Ligne).collect();
                if reste > 0 {
                    v.push(Item::Reste(reste));
                }
                v
            }
        }
    }

    fn tuiles(pw: f64) -> Vec<Rect> {
        let large = pw >= 520.0 && !crate::actions::tactile();
        let (n, g) = (if large { 4 } else { 2 }, 8.0);
        let w = (pw - 2.0 * PAD - g * (n as f64 - 1.0)) / n as f64;
        (0..4)
            .map(|k| Rect::new(PAD + (k % n) as f64 * (w + g), HEAD + 8.0 + (k / n) as f64 * (TUILE_H + g), w, TUILE_H))
            .collect()
    }

    fn haut_onglets(pw: f64) -> f64 {
        Self::tuiles(pw).last().map_or(HEAD, |r| r.y + r.h) + 12.0
    }

    fn haut_tableau(pw: f64) -> f64 {
        Self::haut_onglets(pw) + ONGLET_H + 10.0 + 22.0
    }

    fn lignes_y(&self, pw: f64) -> Vec<(Rect, Item)> {
        let mut y = Self::haut_tableau(pw);
        self.items()
            .into_iter()
            .map(|it| {
                let h = match it {
                    Item::Titre(_) => SECTION,
                    Item::Reste(_) => 30.0,
                    Item::Ligne(_) => LIGNE,
                };
                let r = Rect::new(6.0, y, pw - 12.0, h);
                y += h;
                (r, it)
            })
            .collect()
    }

    fn zones(&self, pw: f64, h: f64) -> Vec<(Rect, Zone)> {
        let mut z = vec![(Rect::new(pw - REGLAGES_X, 9.0, 30.0, 30.0), Zone::Reglages)];
        if self.reglages {
            let mut y = HEAD + 40.0;
            for k in 0..TRIS.len() {
                z.push((Rect::new(PAD, y, pw - 2.0 * PAD, 34.0), Zone::Tri(k)));
                y += 38.0;
            }
            return z;
        }
        let yo = Self::haut_onglets(pw);
        let wo = (pw - 2.0 * PAD - 8.0) / 3.0;
        for k in 0..3 {
            z.push((Rect::new(PAD + 4.0 + k as f64 * wo, yo + 4.0, wo, ONGLET_H - 8.0), Zone::Onglet(k)));
        }
        for (r, it) in self.lignes_y(pw) {
            if let Item::Ligne(l) = it {
                z.push((r, Zone::Marche(l.coin)));
            }
        }
        if self.onglet != 1 {
            z.push((
                Rect::new(PAD - 4.0, h - PIED - crate::eco::marge_ai() + 4.0, 200.0, PIED - 8.0),
                Zone::TriSuivant,
            ));
        }
        z
    }

    /// The content's height (it scrolls).
    pub fn content_h(&self, pw: f64) -> f64 {
        if self.reglages {
            return HEAD + 40.0 + TRIS.len() as f64 * 38.0 + 30.0 + PIED;
        }
        let bas = self.lignes_y(pw).last().map_or(Self::haut_tableau(pw) + 60.0, |(r, _)| r.y + r.h);
        bas + 12.0 + PIED + crate::eco::marge_ai()
    }

    /// A click (content points; `h`: the content's height).
    pub fn click(&mut self, pw: f64, h: f64, x: f64, y: f64) -> StClick {
        let Some(z) = self.zones(pw, h).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) else {
            return StClick::None;
        };
        match z {
            Zone::Reglages => self.reglages = !self.reglages,
            Zone::Onglet(k) => self.onglet = k,
            Zone::Marche(c) => return StClick::Chart(c),
            Zone::Tri(k) => {
                self.tri = k;
                self.reglages = false;
            }
            Zone::TriSuivant => self.tri = (self.tri + 1) % TRIS.len(),
        }
        StClick::Changed
    }

    /// Draws the module (`pw` wide, `pm` as tall as [`Stats::content_h`]).
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>, fond: Rgb) {
        let gras = Text::graisse(Graisse::Bold);
        let h = crate::px::lh(pm);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        crate::fill(pm, 0.0, 0.0, pw, h, fond, 1.0);
        gras.draw(pm, "📶 Stats Hyperliquid", (PAD as f32, 31.0), 17.0, j::TEXTE, Align::Left);
        let zones = self.zones(pw, h);
        if let Some((r, _)) = zones.first() {
            round_fill(
                pm,
                *r,
                r.h / 2.0,
                if self.reglages { j::ACCENT } else { j::BLANC },
                if self.reglages || survol(r) { 0.2 } else { 0.07 },
            );
            crate::phosphor::draw(pm, "gear", r.cx(), r.cy(), 16, j::TEXTE);
        }
        if self.reglages {
            gras.draw(pm, "TRIER LES MARCHÉS PAR", (PAD as f32, (HEAD + 28.0) as f32), 11.5, j::DIM, Align::Left);
            for (r, z) in &zones {
                if let Zone::Tri(k) = z {
                    let on = *k == self.tri;
                    round_fill(
                        pm,
                        *r,
                        j::R_LIGNE,
                        if on { j::ACCENT } else { j::BLANC },
                        if on {
                            0.25
                        } else if survol(r) {
                            0.08
                        } else {
                            0.04
                        },
                    );
                    text.draw(pm, TRIS[*k], ((r.x + 14.0) as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Left);
                }
            }
            return;
        }
        // tiles
        let t = totaux(&self.lignes);
        let vide = self.lignes.is_empty();
        let tuiles: [(&str, String, String); 4] = [
            ("Volume 24 h", court_dollars(t.volume, 1), String::new()),
            ("Open interest", court_dollars(t.oi, 1), String::new()),
            ("Funding moyen", pourcent(t.funding, 4), "par heure".into()),
            ("Marchés", t.marches.to_string(), format!("dont {} HIP-3", t.hip3)),
        ];
        for (r, (l, v, s)) in Self::tuiles(pw).iter().zip(tuiles) {
            round_fill(pm, *r, j::R_TUILE * 0.75, j::BLANC, 0.04);
            crate::shapes::round_stroke(pm, *r, j::R_TUILE * 0.75, j::sur_fond(j::LISERE), 1.0);
            text.draw(pm, l, ((r.x + 12.0) as f32, (r.y + 21.0) as f32), 12.0, j::DIM, Align::Left);
            let v = if vide { "—".to_string() } else { v };
            gras.draw(
                pm,
                &gras.fit(&v, 19.0, r.w - 20.0),
                ((r.x + 12.0) as f32, (r.y + 45.0) as f32),
                19.0,
                j::TEXTE,
                Align::Left,
            );
            if !s.is_empty() && !vide {
                text.draw(
                    pm,
                    &text.fit(&s, 12.0, r.w - 20.0),
                    ((r.x + 12.0) as f32, (r.y + 62.0) as f32),
                    12.0,
                    j::DIM,
                    Align::Left,
                );
            }
        }
        // tabs
        let yo = Self::haut_onglets(pw);
        round_fill(pm, Rect::new(PAD, yo, pw - 2.0 * PAD, ONGLET_H), ONGLET_H / 2.0, j::BLANC, 0.05);
        for (r, z) in &zones {
            if let Zone::Onglet(k) = z {
                let on = *k == self.onglet;
                if on || survol(r) {
                    round_fill(pm, *r, r.h / 2.0, if on { j::ACCENT } else { j::BLANC }, if on { 0.35 } else { 0.06 });
                }
                (if on { &gras } else { text }).draw(
                    pm,
                    ONGLETS[*k],
                    (r.cx() as f32, (r.cy() + 4.5) as f32),
                    13.5,
                    if on { j::TEXTE } else { j::DIM },
                    Align::Center,
                );
            }
        }
        // table
        let r0 = pw - PAD;
        let (x24, xf, xv, xo) = (r0, r0 - 64.0, r0 - 150.0, r0 - 226.0);
        let yt = Self::haut_tableau(pw) - 8.0;
        text.draw(pm, "Marché", (PAD as f32, yt as f32), 12.0, j::DIM, Align::Left);
        for (x, l) in [(xo, "OI"), (xv, "Vol. 24 h"), (xf, "Funding 1 h"), (x24, "24 h")] {
            text.draw(pm, l, (x as f32, yt as f32), 12.0, j::DIM, Align::Right);
        }
        if vide {
            text.draw(
                pm,
                "Lecture des marchés d'Hyperliquid…",
                (PAD as f32, (yt + 30.0) as f32),
                12.5,
                j::DIM,
                Align::Left,
            );
        }
        let oi_max = self.lignes.iter().map(|l| l.oi).fold(0.0, f64::max).max(1.0);
        for (r, it) in self.lignes_y(pw) {
            match it {
                Item::Titre(s) => {
                    gras.draw(pm, s, (PAD as f32, (r.y + 19.0) as f32), 11.5, j::DIM, Align::Left);
                }
                Item::Reste(n) => {
                    text.draw(
                        pm,
                        &format!("… et {n} autres marchés"),
                        (PAD as f32, (r.y + 20.0) as f32),
                        12.5,
                        j::DIM,
                        Align::Left,
                    );
                }
                Item::Ligne(l) => {
                    if survol(&r) {
                        round_fill(pm, Rect::new(r.x, r.y + 2.0, r.w, r.h - 4.0), j::R_LIGNE, j::BLANC, 0.05);
                    }
                    let cy = r.y + r.h / 2.0;
                    crate::watchlist::coin_badge(pm, text, &l.coin, None, PAD + 11.0, cy, 11.0);
                    let (dex, nom) = l.coin.split_once(':').map_or(("", l.coin.as_str()), |(d, n)| (d, n));
                    let place = xo - 70.0 - (PAD + 28.0);
                    if dex.is_empty() {
                        gras.draw(
                            pm,
                            &gras.fit(nom, 14.0, place),
                            ((PAD + 28.0) as f32, (cy + 5.0) as f32),
                            14.0,
                            j::TEXTE,
                            Align::Left,
                        );
                    } else {
                        gras.draw(
                            pm,
                            &gras.fit(nom, 14.0, place),
                            ((PAD + 28.0) as f32, (cy - 1.0) as f32),
                            14.0,
                            j::TEXTE,
                            Align::Left,
                        );
                        text.draw(
                            pm,
                            &text.fit(dex, 11.0, place),
                            ((PAD + 28.0) as f32, (cy + 14.0) as f32),
                            11.0,
                            j::DIM,
                            Align::Left,
                        );
                    }
                    text.draw(pm, &court_dollars(l.oi, 2), (xo as f32, (cy + 1.0) as f32), 13.5, j::TEXTE, Align::Right);
                    let bw = (70.0 * l.oi / oi_max).max(3.0);
                    round_fill(pm, Rect::new(xo - bw, cy + 8.0, bw, 4.0), 2.0, j::ACCENT, 0.85);
                    text.draw(pm, &court_dollars(l.volume, 2), (xv as f32, (cy + 5.0) as f32), 13.5, j::TEXTE, Align::Right);
                    // (sign and colour from the value written: « 0,0 % » white, no sign)
                    let f = pourcent(l.funding, 4);
                    text.draw(pm, &f, (xf as f32, (cy + 5.0) as f32), 13.0, couleur_affichee(&f, l.funding), Align::Right);
                    if let Some(v) = l.var {
                        let p = pourcent(v, 1);
                        gras.draw(pm, &p, (x24 as f32, (cy + 5.0) as f32), 13.5, couleur_affichee(&p, v), Align::Right);
                    }
                }
            }
        }
        // bottom: the sort and the freshness
        let hb = h - crate::eco::marge_ai();
        let y = hb - PIED / 2.0 + 5.0;
        crate::fill(pm, PAD, hb - PIED, pw - 2.0 * PAD, 1.0, j::BLANC, 0.08);
        if self.onglet != 1 {
            text.draw(pm, "Trié par", (PAD as f32, y as f32), 12.5, j::DIM, Align::Left);
            let w = text.width("Trié par ", 12.5) as f64;
            gras.draw(
                pm,
                &format!("{} ▾", TRIS[self.tri.min(3)]),
                ((PAD + w) as f32, y as f32),
                12.5,
                j::TEXTE,
                Align::Left,
            );
        }
        if let Some(t) = self.lu_ms {
            text.draw(
                pm,
                &format!("à jour il y a {} s", ((self.now_ms - t).max(0) / 1000)),
                ((pw - PAD) as f32, y as f32),
                12.5,
                j::DIM,
                Align::Right,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(coin: &str, mark: &str, prev: f64, volume: f64, oi: f64, funding: f64) -> Quote {
        Quote {
            coin: coin.into(),
            mark: mark.into(),
            prev,
            volume,
            growth: false,
            oracle: 0.0,
            oi,
            funding,
        }
    }

    fn quotes() -> Vec<Quote> {
        vec![
            q("BTC", "60000", 59_000.0, 3.1e9, 20_000.0, 0.0001),
            q("ETH", "2000", 2010.0, 1.48e9, 300_000.0, 0.000125),
            q("xyz:NVDA", "180", 175.0, 50e6, 100_000.0, -0.0002),
            q("ZEC", "500", 0.0, 1e6, 0.0, 0.01),
        ]
    }

    /// 📶 Totals and the funding weighted by the open interest in dollars, on an example list.
    #[test]
    fn totaux_et_moyenne_ponderee() {
        let mut s = Stats::default();
        s.prendre(quotes().iter(), 1_000);
        let t = totaux(&s.lignes);
        // OI $: BTC 1,2 Md, ETH 600 M, NVDA 18 M, ZEC 0
        assert!((t.oi - 1.818e9).abs() < 1.0);
        assert!((t.volume - (3.1e9 + 1.48e9 + 50e6 + 1e6)).abs() < 1.0);
        let attendu = (0.0001 * 1.2e9 + 0.000125 * 6e8 - 0.0002 * 1.8e7) / 1.818e9;
        assert!((t.funding - attendu).abs() < 1e-12, "ZEC (no OI) weighs nothing");
        assert_eq!((t.marches, t.hip3), (4, 1));
        assert_eq!(court_dollars(t.oi, 1), "1,8 Md$");
        assert!(!s.du(1_000 + 29_000) && s.du(1_000 + 30_000), "30 s");
    }

    #[test]
    fn tri_et_onglets() {
        let mut s = Stats::default();
        s.prendre(quotes().iter(), 0);
        let noms = |s: &Stats| {
            s.items()
                .into_iter()
                .filter_map(|i| if let Item::Ligne(l) = i { Some(l.coin) } else { None })
                .collect::<Vec<_>>()
        };
        assert_eq!(noms(&s), ["BTC", "ETH", "xyz:NVDA", "ZEC"], "by OI");
        s.tri = 1;
        assert_eq!(noms(&s), ["BTC", "ETH", "xyz:NVDA", "ZEC"], "by volume");
        s.tri = 3;
        assert_eq!(noms(&s)[0], "xyz:NVDA", "by 24 h (+2,9 %)");
        s.onglet = 2;
        assert_eq!(noms(&s), ["xyz:NVDA"], "HIP-3 only");
        // Funding tab: the highest, then the lowest (markets with OI only)
        s.onglet = 1;
        let it = s.items();
        assert_eq!(it[0], Item::Titre("LES 10 PLUS HAUTS"));
        assert_eq!(noms(&s), ["ETH", "BTC", "xyz:NVDA"]);
        let (h, b) = extremes(&s.lignes);
        assert_eq!((h.len(), b.len()), (3, 0), "3 markets: no « lowest » twice");
    }

    #[test]
    fn clic_sur_une_ligne_et_tri_du_bas() {
        let t = Text::default();
        let mut s = Stats::default();
        s.prendre(quotes().iter(), 0);
        let pw = 402.0;
        let h = s.content_h(pw);
        let (r, _) = s.zones(pw, h).into_iter().find(|(_, z)| *z == Zone::Marche("ETH".into())).unwrap();
        assert_eq!(s.click(pw, h, r.cx(), r.cy()), StClick::Chart("ETH".into()));
        let (r, _) = s.zones(pw, h).into_iter().find(|(_, z)| *z == Zone::TriSuivant).unwrap();
        s.click(pw, h, r.cx(), r.cy());
        assert_eq!(s.tri, 1);
        for pw in [320.0, 402.0, 640.0] {
            for k in 0..3 {
                s.onglet = k;
                let mut pm = crate::px::new(pw as u32, s.content_h(pw) as u32).unwrap();
                s.draw(&mut pm, &t, pw, None, Rgb(0, 0, 0));
            }
        }
    }
}
