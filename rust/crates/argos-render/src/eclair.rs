//! ⚡ The news of the chart's crypto (Jee's mock-ups « actualités v1 », 22 Nov 2026): the violet glass ⚡ in the row
//! of the calendar flags, and what a click on one opens:
//! - PC: the glass card « Dernières mises à jour » (≈ 380 px, a small tip towards the ⚡, above the row, never off
//!   the chart nor over the price scale): 3 news of that crypto at most, the clicked one first, then « BTC · n
//!   nouvelles aujourd'hui » and « Voir tout › »;
//! - phone and tablet: the sheet « Gros titres pour BTCUSDT.P » from the bottom (blurred header that stays, the list
//!   scrolls, the newest first).
//!
//! A click on a news opens the article; « Voir tout › » the Actualités module filtered on the crypto.

use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_data::eco::News;
use tiny_skia::{FillRule, Paint, PathBuilder, Pixmap};

/// The ⚡'s glass: fill, edge, bolt (mock-up colours).
pub const FOND: (Rgb, f32) = (Rgb(124, 77, 255), 0.22);
pub const BORD: (Rgb, f32) = (Rgb(168, 122, 255), 0.85);
pub const ECLAIR: Rgb = Rgb(0xd4, 0xb8, 0xff);
/// The PC card's width.
pub const CARTE_W: f64 = 380.0;
/// News in the PC card at most.
pub const CARTE_MAX: usize = 3;

fn paint(c: Rgb, a: f32) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, (a * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

/// The bolt alone (a polygon: the font has no ⚡), centred on (`cx`, `cy`), `s` high.
pub fn bolt(pm: &mut Pixmap, cx: f64, cy: f64, s: f64, c: Rgb) {
    let k = s / 16.0;
    let pts = [(2.0, -8.0), (-5.0, 1.5), (-0.5, 1.5), (-2.0, 8.0), (5.0, -1.5), (0.5, -1.5)];
    let mut pb = PathBuilder::new();
    for (i, (x, y)) in pts.iter().enumerate() {
        let (x, y) = ((cx + x * k) as f32, (cy + y * k) as f32);
        if i == 0 {
            pb.move_to(x, y);
        } else {
            pb.line_to(x, y);
        }
    }
    pb.close();
    if let Some(p) = pb.finish() {
        pm.fill_path(&p, &paint(c, 1.0), FillRule::Winding, crate::px::t(), None);
    }
}

/// ⚡ One round of the row (radius `r`); `choisi`: the clicked one (stronger fill, white bolt, a halo).
pub fn rond(pm: &mut Pixmap, cx: f64, cy: f64, r: f64, choisi: bool) {
    let rr = |d: f64| Rect::new(cx - r - d, cy - r - d, 2.0 * (r + d), 2.0 * (r + d));
    if choisi {
        round_fill(pm, rr(5.0), r + 5.0, BORD.0, 0.18);
        round_fill(pm, rr(2.5), r + 2.5, BORD.0, 0.22);
    }
    round_fill(pm, rr(0.0), r, crate::jetons::FOND, 0.55);
    round_fill(pm, rr(0.0), r, FOND.0, if choisi { 0.55 } else { FOND.1 });
    round_stroke(pm, rr(0.0), r, BORD.0, BORD.1);
    bolt(pm, cx, cy, r * 1.05, if choisi { Rgb(255, 255, 255) } else { ECLAIR });
}

const MOIS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

/// « 14:30 · 22 nov. · Cointribune » (`tz` : offset of the time shown, s).
pub fn quand(n: &News, tz: i64) -> String {
    use chrono::{Datelike, TimeZone};
    let d = chrono::Utc.timestamp_opt(n.ts + tz, 0).single().unwrap_or_default();
    let s = (n.ts + tz).rem_euclid(86_400);
    format!(
        "{:02}:{:02} · {} {} · {}",
        s / 3600,
        s % 3600 / 60,
        d.day(),
        MOIS[d.month0() as usize],
        n.source
    )
}

/// The coin badges of a news (its cryptos, else the blue ◎ « Marché »), overlapping a little; returns their width.
pub fn pastilles(pm: &mut Pixmap, text: &Text, coins: &[&str], x: f64, cy: f64, r: f64) -> f64 {
    if coins.is_empty() {
        let c = Rect::new(x, cy - r, 2.0 * r, 2.0 * r);
        round_fill(pm, c, r, Rgb(0x3b, 0x5b, 0xdb), 1.0);
        text.draw(
            pm,
            "◎",
            ((x + r) as f32, (cy + r * 0.42) as f32),
            (r * 1.2) as f32,
            Rgb(255, 255, 255),
            Align::Center,
        );
        return 2.0 * r;
    }
    let n = coins.len().min(3);
    for (k, c) in coins.iter().take(n).enumerate() {
        crate::watchlist::coin_badge(pm, text, c, None, x + r + k as f64 * r * 1.45, cy, r);
    }
    2.0 * r + (n - 1) as f64 * r * 1.45
}

/// What a click did.
#[derive(Debug, Clone, PartialEq)]
pub enum EclairClick {
    None,
    Fermer,
    /// Open this article.
    Ouvrir(String),
    /// The Actualités module filtered on this crypto.
    VoirTout(String),
}

/// ⚡ The open card (PC) or sheet (phone).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Eclair {
    /// The chart's crypto (« BTC ») and its symbol as shown (« BTCUSDT.P »).
    pub coin: String,
    pub symbole: String,
    /// Every news of that crypto, the newest first.
    pub news: Vec<News>,
    /// The link of the ⚡ clicked (its news first in the card).
    pub lien: String,
    /// The ⚡'s centre (chart points) and the top of the row; the chart's width without the price scale.
    pub ancre: (f64, f64),
    pub haut_rangee: f64,
    pub plot_w: f64,
    /// Sheet from the bottom (phone, tablet) rather than the card.
    pub feuille: bool,
    pub scroll: f64,
    /// Time shown (offset, s) and now (s).
    pub tz: i64,
    pub now: i64,
}

const PAD: f64 = 14.0;
const TETE: f64 = 56.0;
const PIED: f64 = 44.0;

#[derive(Debug, Clone, PartialEq)]
enum Zone {
    Fermer,
    Ligne(String),
    VoirTout,
}

impl Eclair {
    /// The news of the card: the clicked one first, then the newest, 3 at most.
    pub fn carte_news(&self) -> Vec<&News> {
        let mut v: Vec<&News> = self.news.iter().filter(|n| n.link == self.lien).take(1).collect();
        v.extend(self.news.iter().filter(|n| n.link != self.lien));
        v.truncate(CARTE_MAX);
        v
    }

    /// News of the crypto today (time shown).
    pub fn aujourd_hui(&self) -> usize {
        let day = |t: i64| (t + self.tz).div_euclid(86_400);
        self.news.iter().filter(|n| day(n.ts) == day(self.now)).count()
    }

    fn lignes(&self, text: &Text, w: f64, taille: f32) -> Vec<(f64, Vec<String>)> {
        let gras = Text::graisse(Graisse::Bold);
        let _ = text;
        let liste: Vec<&News> = if self.feuille { self.news.iter().collect() } else { self.carte_news() };
        liste
            .iter()
            .map(|n| {
                let mut l = crate::assistant::wrap(&gras, &n.title, taille, w - 2.0 * PAD - 4.0);
                if !self.feuille && l.len() > 2 {
                    l.truncate(2);
                    l[1] = gras.fit(&format!("{}…", l[1]), taille, w - 2.0 * PAD - 4.0);
                }
                let h = 30.0 + l.len() as f64 * (taille as f64 * 1.35) + 14.0;
                (h, l)
            })
            .collect()
    }

    /// The card's place (chart points): above the row, its tip on the ⚡, inside the chart (left of the price scale).
    pub fn place(&self, text: &Text) -> Rect {
        let h = TETE + self.lignes(text, CARTE_W, 15.0).iter().map(|l| l.0).sum::<f64>() + PIED;
        let x = (self.ancre.0 - CARTE_W + 60.0).clamp(8.0, (self.plot_w - CARTE_W - 8.0).max(8.0));
        let y = (self.haut_rangee - 14.0 - h).max(8.0);
        Rect::new(x, y, CARTE_W, h)
    }

    /// The sheet's place in a `w` × `h` window.
    pub fn place_feuille(w: f64, h: f64) -> Rect {
        let top = (h * 0.28).max(60.0);
        Rect::new(0.0, top, w, h - top)
    }

    fn zones(&self, text: &Text, r: Rect) -> Vec<(Rect, Zone)> {
        let mut z = vec![(Rect::new(r.x + r.w - PAD - 32.0, r.y + 12.0, 32.0, 32.0), Zone::Fermer)];
        let (taille, liste): (f32, Vec<&News>) = if self.feuille {
            (17.5, self.news.iter().collect())
        } else {
            (15.0, self.carte_news())
        };
        let mut y = r.y + TETE - if self.feuille { self.scroll } else { 0.0 };
        for ((h, _), n) in self.lignes(text, r.w, taille).iter().zip(liste) {
            z.push((Rect::new(r.x + 6.0, y, r.w - 12.0, *h), Zone::Ligne(n.link.clone())));
            y += h;
        }
        if !self.feuille {
            z.push((Rect::new(r.x + r.w - 120.0, r.y + r.h - PIED, 112.0, PIED), Zone::VoirTout));
        }
        z
    }

    /// A click at (`x`, `y`): chart points for the card, window points for the sheet (`w` × `h`).
    pub fn click(&self, text: &Text, x: f64, y: f64, w: f64, h: f64) -> EclairClick {
        let r = if self.feuille { Self::place_feuille(w, h) } else { self.place(text) };
        if !r.contains(x, y) {
            return EclairClick::Fermer;
        }
        let tete = r.y + TETE;
        match self
            .zones(text, r)
            .into_iter()
            .find(|(z, k)| z.contains(x, y) && (*k == Zone::Fermer || !self.feuille || y >= tete))
        {
            Some((_, Zone::Fermer)) => EclairClick::Fermer,
            Some((_, Zone::Ligne(l))) => EclairClick::Ouvrir(l),
            Some((_, Zone::VoirTout)) => EclairClick::VoirTout(self.coin.clone()),
            None => EclairClick::None,
        }
    }

    /// Scrolls the sheet.
    pub fn wheel(&mut self, text: &Text, dy: f64, w: f64, h: f64) {
        let r = Self::place_feuille(w, h);
        let total: f64 = self.lignes(text, r.w, 17.5).iter().map(|l| l.0).sum();
        self.scroll = (self.scroll + dy).clamp(0.0, (total - (r.h - TETE - 20.0)).max(0.0));
    }

    /// Draws the card (PC: its own image and its place, chart points) or the sheet (window `w` × `h`).
    pub fn draw(&self, text: &Text, w: f64, h: f64, mouse: Option<(f64, f64)>) -> Option<(Pixmap, Rect)> {
        use crate::jetons as j;
        let r = if self.feuille { Self::place_feuille(w, h) } else { self.place(text) };
        let pointe = if self.feuille { 0.0 } else { 10.0 };
        let mut pm = crate::px::new(r.w.ceil() as u32, (r.h + pointe).ceil() as u32)?;
        let local = Rect::new(0.5, 0.5, r.w - 1.0, r.h - 1.0);
        let rayon = if self.feuille { j::R_VOLET } else { j::R_CARTE * 0.8 };
        round_fill(&mut pm, local, rayon, Rgb(0x12, 0x15, 0x22), 1.0);
        round_stroke(&mut pm, local, rayon, j::sur_fond(j::LISERE), 1.0);
        if !self.feuille {
            // the small tip towards the ⚡
            let px = (self.ancre.0 - r.x).clamp(24.0, r.w - 24.0);
            let mut pb = PathBuilder::new();
            pb.move_to((px - 9.0) as f32, (r.h - 1.5) as f32);
            pb.line_to(px as f32, (r.h + pointe - 1.0) as f32);
            pb.line_to((px + 9.0) as f32, (r.h - 1.5) as f32);
            pb.close();
            if let Some(p) = pb.finish() {
                pm.fill_path(&p, &paint(Rgb(0x12, 0x15, 0x22), 1.0), FillRule::Winding, crate::px::t(), None);
            }
        }
        let gras = Text::graisse(Graisse::Bold);
        let survol = |z: &Rect| mouse.is_some_and(|(x, y)| z.contains(x, y));
        let (taille, liste): (f32, Vec<&News>) = if self.feuille {
            (17.5, self.news.iter().collect())
        } else {
            (15.0, self.carte_news())
        };
        let lignes = self.lignes(text, r.w, taille);
        let zones = self.zones(text, r);
        // the list (under the header on the sheet)
        let mut k = 0;
        for (z, zone) in &zones {
            let Zone::Ligne(_) = zone else { continue };
            let (Some(n), Some((_, l))) = (liste.get(k), lignes.get(k)) else { break };
            k += 1;
            let y = z.y - r.y;
            if y + z.h < TETE - 4.0 || y > r.h {
                continue;
            }
            let loc = Rect::new(z.x - r.x, y, z.w, z.h - 4.0);
            if survol(z) || (!self.feuille && k == 1) {
                round_fill(&mut pm, loc, j::R_LIGNE, j::BLANC, if survol(z) { 0.06 } else { 0.035 });
            }
            let coins = argos_data::actu::coins_of(&n.title);
            let pw = pastilles(&mut pm, text, &coins, PAD, y + 18.0, 9.5);
            text.draw(
                &mut pm,
                &text.fit(&quand(n, self.tz), 12.5, r.w - 2.0 * PAD - pw - 10.0),
                ((PAD + pw + 8.0) as f32, (y + 22.5) as f32),
                12.5,
                j::DIM,
                Align::Left,
            );
            for (i, s) in l.iter().enumerate() {
                gras.draw(
                    &mut pm,
                    s,
                    (PAD as f32, (y + 30.0 + (i as f64 + 1.0) * taille as f64 * 1.35 - 4.0) as f32),
                    taille,
                    j::TEXTE,
                    Align::Left,
                );
            }
        }
        // header (blurred band that stays on top of the sheet's list)
        if self.feuille {
            crate::fill(&mut pm, 0.0, 0.0, r.w, TETE, Rgb(0x12, 0x15, 0x22), 1.0);
            crate::fill(&mut pm, 0.0, TETE - 1.0, r.w, 1.0, j::BLANC, 0.08);
        }
        let titre = if self.feuille {
            format!("Gros titres pour {}", self.symbole)
        } else {
            "Dernières mises à jour".to_string()
        };
        rond(&mut pm, PAD + 14.0, 28.0, 14.0, false);
        gras.draw(
            &mut pm,
            &gras.fit(&titre, 17.0, r.w - 2.0 * PAD - 80.0),
            ((PAD + 38.0) as f32, 34.0),
            17.0,
            j::TEXTE,
            Align::Left,
        );
        let f = Rect::new(r.w - PAD - 32.0, 12.0, 32.0, 32.0);
        round_fill(
            &mut pm,
            f,
            16.0,
            j::BLANC,
            if survol(&Rect::new(f.x + r.x, f.y + r.y, f.w, f.h)) { 0.14 } else { 0.07 },
        );
        text.draw(&mut pm, "✕", (f.cx() as f32, (f.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
        if !self.feuille {
            crate::fill(&mut pm, 0.0, r.h - PIED, r.w, 1.0, j::BLANC, 0.08);
            let n = self.aujourd_hui();
            let bas = format!("{} · {n} nouvelle{} aujourd'hui", self.coin, if n > 1 { "s" } else { "" });
            text.draw(&mut pm, &bas, (PAD as f32, (r.h - PIED / 2.0 + 5.0) as f32), 13.0, j::DIM, Align::Left);
            text.draw(
                &mut pm,
                "Voir tout ›",
                ((r.w - PAD) as f32, (r.h - PIED / 2.0 + 5.0) as f32),
                13.5,
                j::ACCENT,
                Align::Right,
            );
        }
        Some((pm, r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(t: &str, ts: i64) -> News {
        News {
            title: t.into(),
            link: format!("https://x.fr/{ts}"),
            ts,
            source: "Cointribune".into(),
            lang: "fr".into(),
        }
    }

    fn eclair() -> Eclair {
        Eclair {
            coin: "BTC".into(),
            symbole: "BTCUSDT.P".into(),
            news: vec![
                n("Bitcoin monte", 3_000),
                n("Le BTC recule", 2_000),
                n("Les ETF bitcoin", 1_000),
                n("Bitcoin hier", 100),
            ],
            lien: "https://x.fr/2000".into(),
            ancre: (900.0, 800.0),
            haut_rangee: 780.0,
            plot_w: 1300.0,
            now: 3_600,
            ..Default::default()
        }
    }

    /// ⚡ The card: the clicked news first, 3 at most, above the row, inside the chart; a click on a news opens it,
    /// « Voir tout › » the module, outside or ✕ closes.
    #[test]
    fn carte_derniere_mises_a_jour() {
        let t = Text::default();
        let e = eclair();
        let v = e.carte_news();
        assert_eq!(v.iter().map(|n| n.ts).collect::<Vec<_>>(), [2_000, 3_000, 1_000]);
        assert_eq!(e.aujourd_hui(), 4);
        let r = e.place(&t);
        assert!(r.y + r.h <= e.haut_rangee && r.x >= 8.0 && r.x + r.w <= e.plot_w - 8.0, "{r:?}");
        let (pm, rr) = e.draw(&t, 1440.0, 900.0, None).unwrap();
        assert_eq!(rr, r);
        assert!(pm.height() as f64 > r.h, "with its tip");
        // 🫧 the glass takes the card's most frequent opaque colour for its see-through background: the background must
        // be opaque, or the white of the bold titles became the « background » (hollow letters, capture 7)
        let fond = pm.pixel(crate::px::dev(r.w / 2.0) as u32, crate::px::dev(r.h - 6.0) as u32).unwrap();
        assert_eq!(fond.alpha(), 255, "opaque background");
        let z = e.zones(&t, r);
        let (l, _) = z.iter().find(|(_, k)| matches!(k, Zone::Ligne(_))).unwrap();
        assert_eq!(e.click(&t, l.cx(), l.cy(), 0.0, 0.0), EclairClick::Ouvrir("https://x.fr/2000".into()));
        assert_eq!(e.click(&t, r.x + r.w - 40.0, r.y + r.h - 20.0, 0.0, 0.0), EclairClick::VoirTout("BTC".into()));
        assert_eq!(e.click(&t, 10.0, 10.0, 0.0, 0.0), EclairClick::Fermer, "outside");
        assert_eq!(e.click(&t, r.x + r.w - PAD - 16.0, r.y + 28.0, 0.0, 0.0), EclairClick::Fermer, "✕");
        // near the right edge: still inside the chart
        let bord = Eclair {
            ancre: (1290.0, 800.0),
            ..eclair()
        };
        let r = bord.place(&t);
        assert!(r.x + r.w <= bord.plot_w - 8.0);
    }

    /// 📱 The sheet: every news, the list scrolls under its header, a touch on a news opens it.
    #[test]
    fn feuille_gros_titres() {
        let t = Text::default();
        let mut e = Eclair { feuille: true, ..eclair() };
        let (_, r) = e.draw(&t, 402.0, 874.0, None).unwrap();
        assert_eq!(r, Eclair::place_feuille(402.0, 874.0));
        assert_eq!(e.zones(&t, r).iter().filter(|(_, k)| matches!(k, Zone::Ligne(_))).count(), 4, "all of them");
        assert_eq!(
            e.click(&t, 200.0, r.y + TETE + 20.0, 402.0, 874.0),
            EclairClick::Ouvrir("https://x.fr/3000".into())
        );
        assert_eq!(e.click(&t, 200.0, 20.0, 402.0, 874.0), EclairClick::Fermer, "above the sheet");
        e.wheel(&t, 10_000.0, 402.0, 874.0);
        assert!(e.scroll >= 0.0);
    }
}
