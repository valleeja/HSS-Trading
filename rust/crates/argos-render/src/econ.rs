//! 📅 Annonces économiques sur la chart (comme TradingView) : un petit rond avec le drapeau de la devise au bas de la
//! zone des bougies, à l'heure de l'annonce (passée ou à venir) ; anneau rouge = impact fort, orange = moyen. Souris
//! dessus : l'heure, le titre, réel / prévu / précédent. Les annonces viennent de l'Agenda (Forex Factory).
//! ⚡ Les nouvelles de la crypto de la chart : un rond violet en verre par bougie, dans la même rangée (les drapeaux
//! gardent la priorité : un ⚡ qui toucherait un drapeau ou un autre ⚡ s'y regroupe ou s'efface) ; un clic ouvre
//! sa carte (`eclair`), la bulle au survol reste pour les drapeaux.

use crate::palette as pal;
use crate::palette::{GOLD, GREY, WHITE};
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use tiny_skia::Pixmap;

/// Une annonce à poser : heure (s), devise, impact (2 = fort, 1 = moyen, 0 = nouvelle du marché), titre et chiffres
/// (pour une nouvelle : sa source). `window_s` : fenêtre PropFirm autour de l'annonce (secondes de chaque côté, 0 =
/// aucune), dessinée en bande rouge pâle sur la hauteur des bougies.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Mark {
    pub ts: i64,
    pub currency: String,
    pub impact: u8,
    pub title: String,
    pub numbers: String,
    pub window_s: i64,
    /// ⚡ The news' article (news only).
    pub link: String,
    /// ⚡ The one whose card is open.
    pub choisi: bool,
}

/// Devise d'une nouvelle du marché (pas une annonce économique).
pub const NEWS: &str = "NEWS";

const R: f64 = 11.0;

/// Drapeau d'une devise (émoji), ou `None`.
pub fn flag(currency: &str) -> Option<&'static str> {
    Some(match currency {
        "USD" => "🇺🇸",
        "EUR" => "🇪🇺",
        "GBP" => "🇬🇧",
        "JPY" => "🇯🇵",
        "CAD" => "🇨🇦",
        "AUD" => "🇦🇺",
        "NZD" => "🇳🇿",
        "CHF" => "🇨🇭",
        "CNY" => "🇨🇳",
        _ => return None,
    })
}

/// Ronds à dessiner : (centre x, devise, impact le plus fort, annonces) — une par devise et par heure ; deux devises à
/// la même heure se décalent.
fn spots<'a>(marks: &'a [Mark], x_of: &dyn Fn(i64) -> f64, w: f64) -> Vec<(f64, &'a str, u8, Vec<&'a Mark>)> {
    let mut out: Vec<(i64, f64, &str, u8, Vec<&Mark>)> = vec![];
    // les annonces (drapeaux) d'abord, les 📰 ensuite : une nouvelle ne pousse ni ne fait sortir un drapeau
    let (annonces, nouvelles): (Vec<&Mark>, Vec<&Mark>) = marks.iter().partition(|m| m.currency != NEWS);
    for m in annonces.into_iter().chain(nouvelles) {
        match out.iter_mut().find(|s| s.0 == m.ts && s.2 == m.currency) {
            Some(s) => {
                s.3 = s.3.max(m.impact);
                s.4.push(m);
            }
            None => out.push((m.ts, x_of(m.ts), &m.currency, m.impact, vec![m])),
        }
    }
    // même heure, autre devise : décalé à droite ; ⚡ : regroupé avec le ⚡ qu'il toucherait, effacé s'il touche un
    // drapeau (rien ne se chevauche)
    let mut placed: Vec<(f64, &str, u8, Vec<&Mark>)> = vec![];
    for (_, x, c, i, l) in out {
        let mut x = x;
        if c == NEWS {
            if let Some(p) = placed.iter_mut().find(|p| (p.0 - x).abs() < 2.0 * R + 2.0) {
                if p.1 == NEWS {
                    p.3.extend(l);
                }
                continue;
            }
        }
        while placed.iter().any(|p| (p.0 - x).abs() < 2.0 * R - 1.0) {
            x += 2.0 * R + 2.0;
        }
        if (-R..w + R).contains(&x) {
            placed.push((x, c, i, l));
        }
    }
    placed
}

/// Dessine les annonces au bas de la zone des bougies (`bottom` = bas de cette zone, `w` = sa largeur) ; `mouse` :
/// souris sur la chart (bulle de l'annonce survolée) ; `tz_offset_s` : fuseau affiché.
pub fn draw(pm: &mut Pixmap, text: &Text, marks: &[Mark], x_of: &dyn Fn(i64) -> f64, w: f64, bottom: f64, mouse: Option<(f64, f64)>, tz_offset_s: i64) {
    let spots = draw_marks(pm, text, marks, x_of, w, bottom, &[]);
    bubble(pm, text, &spots, w, bottom, mouse, tz_offset_s);
}

/// One round drawn on the chart, kept (owned) for the hover bubble, so the mouse layer can be drawn later on its own.
#[derive(Debug, Clone, PartialEq)]
pub struct Spot {
    pub x: f64,
    pub currency: String,
    pub marks: Vec<Mark>,
}

/// Draws the PropFirm windows and the rounds (no bubble); returns the rounds for [`bubble`].
/// 📅 Top of the row of rounds [`draw_marks`] draws above `bottom` (chart points): what sits at the bottom of the price
/// pane (Argos Murs' bottom pills) stays above it.
pub fn haut_rangee(bottom: f64) -> f64 {
    bottom - R - 6.0 - R
}

/// 📅 The row of rounds above `bottom`, `w` wide (chart points): kept clear of the high / low pills.
pub fn rangee_zone(bottom: f64, w: f64) -> Option<Rect> {
    let top = haut_rangee(bottom) - 3.0;
    (w > 0.0).then(|| Rect::new(0.0, top, w, bottom - 6.0 + 3.0 - top))
}

thread_local! {
    /// 📅 The rounds of the last [`draw_marks`] (chart points).
    static RONDS: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    /// ⚡ The ⚡ of the last [`draw_marks`]: (centre x, link of its newest news).
    static ECLAIRS: std::cell::RefCell<Vec<(f64, String)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// 📅 The bottom of the row of the last [`draw_marks`] (chart points).
    static BAS: std::cell::Cell<f64> = const { std::cell::Cell::new(0.0) };
}

/// ⚡ The ⚡ under (`x`, `y`) (chart points) at the last drawing: (its centre x, the link of its news).
pub fn eclair_sous(x: f64, y: f64) -> Option<(f64, String)> {
    let cy = rangee_cy();
    ECLAIRS.with(|e| e.borrow().iter().find(|(cx, _)| (x - cx).hypot(y - cy) <= R + 3.0).cloned())
}

/// 📅 The bottom of the row at the last drawing (chart points).
pub fn rangee_bas() -> f64 {
    BAS.with(|b| b.get())
}

/// 📅 The centre line of the row at the last drawing (chart points).
pub fn rangee_cy() -> f64 {
    rangee_bas() - R - 6.0
}

/// 📅 The rounds drawn by the last [`draw_marks`] (chart points; empty: none).
pub fn ronds_dessines() -> Vec<Rect> {
    RONDS.with(|r| r.borrow().clone())
}

/// `eviter`: what is drawn over the row afterwards (Argos Murs' wall pills, chart points): a ⚡ or a flag under one of
/// them is left out (it would be hidden).
#[allow(clippy::too_many_arguments)]
pub fn draw_marks(pm: &mut Pixmap, text: &Text, marks: &[Mark], x_of: &dyn Fn(i64) -> f64, w: f64, bottom: f64, eviter: &[Rect]) -> Vec<Spot> {
    let cy = bottom - R - 6.0;
    // 🛡️ fenêtres PropFirm d'abord (sous les ronds) : une bande rouge pâle sur toute la hauteur des bougies
    for m in marks.iter().filter(|m| m.window_s > 0) {
        let (x0, x1) = (x_of(m.ts - m.window_s), x_of(m.ts + m.window_s));
        let (a, b) = (x0.min(x1).max(0.0), x0.max(x1).min(w));
        if b > 0.0 && a < w {
            crate::fill(pm, a.min(b - 2.0).max(0.0), 0.0, (b - a).max(2.0), bottom, Rgb(0xef, 0x44, 0x44), 0.10);
        }
    }
    let mut out = vec![];
    RONDS.with(|r| r.borrow_mut().clear());
    ECLAIRS.with(|r| r.borrow_mut().clear());
    BAS.with(|b| b.set(bottom));
    for (x, cur, impact, list) in spots(marks, x_of, w) {
        let ring = match impact {
            2.. => Rgb(0xef, 0x44, 0x44),
            1 => Rgb(0xf5, 0x9e, 0x0b),
            _ => GREY,
        };
        let r = Rect::new(x - R, cy - R, 2.0 * R, 2.0 * R);
        if eviter.iter().any(|e| r.x < e.x + e.w && e.x < r.x + r.w && r.y < e.y + e.h && e.y < r.y + r.h) {
            continue;
        }
        RONDS.with(|v| v.borrow_mut().push(r));
        if cur == NEWS {
            // ⚡ violet glass round (its newest news first)
            let newest = list.iter().max_by_key(|m| m.ts).map_or(String::new(), |m| m.link.clone());
            crate::eclair::rond(pm, x, cy, R, list.iter().any(|m| m.choisi));
            ECLAIRS.with(|v| v.borrow_mut().push((x, newest)));
            out.push(Spot {
                x,
                currency: cur.to_string(),
                marks: list.into_iter().cloned().collect(),
            });
            continue;
        }
        round_fill(pm, r, R, pal::field(), 1.0);
        match flag(cur) {
            Some(f) => crate::icons::draw_emoji(pm, f, x, cy, 20),
            None => text.draw(
                pm,
                &cur.chars().take(1).collect::<String>(),
                (x as f32, (cy + 4.0) as f32),
                10.0,
                Rgb(255, 255, 255),
                Align::Center,
            ),
        }
        round_stroke(pm, r, R, ring, 1.6);
        out.push(Spot {
            x,
            currency: cur.to_string(),
            marks: list.into_iter().cloned().collect(),
        });
    }
    out
}

/// Bubble of the round under the mouse (the last one drawn wins): time, then each release and its numbers.
pub fn bubble(pm: &mut Pixmap, text: &Text, spots: &[Spot], w: f64, bottom: f64, mouse: Option<(f64, f64)>, tz_offset_s: i64) {
    let cy = bottom - R - 6.0;
    // (the bubble is for the flags only: a ⚡ opens its card on a click)
    let hover = spots
        .iter()
        .filter(|s| s.currency != NEWS)
        .rev()
        .find(|s| mouse.is_some_and(|(mx, my)| (mx - s.x).hypot(my - cy) <= R + 2.0))
        .map(|s| (s.x, s.currency.as_str(), &s.marks));
    // bulle : heure, puis chaque annonce et ses chiffres
    if let Some((x, cur, list)) = hover {
        let t = (list[0].ts + tz_offset_s).rem_euclid(86_400);
        // le drapeau se pose en image devant la 1re ligne (le texte n'a pas les drapeaux)
        let mut lines = vec![format!(
            "{}{} · {:02}:{:02}",
            if flag(cur).is_some() { "      " } else { "" },
            if cur == NEWS { "📰 Nouvelle" } else { cur },
            t / 3600,
            t % 3600 / 60
        )];
        if let Some(m) = list.iter().find(|m| m.window_s > 0) {
            lines.push(format!("   🛡️ PropFirm : pas d'ordre ±{} min", m.window_s / 60));
        }
        for m in list {
            lines.push(m.title.clone());
            if !m.numbers.is_empty() {
                lines.push(format!("   {}", m.numbers));
            }
        }
        let bw = lines.iter().map(|l| text.width(l, 12.0) as f64).fold(120.0, f64::max) + 20.0;
        let bh = lines.len() as f64 * 17.0 + 12.0;
        let bx = (x - bw / 2.0).clamp(4.0, (w - bw - 4.0).max(4.0));
        let by = cy - R - 8.0 - bh;
        let b = Rect::new(bx, by, bw, bh);
        round_fill(pm, b, 6.0, pal::popup(), 0.97);
        round_stroke(pm, b, 6.0, pal::edge(), 1.0);
        if let Some(f) = flag(cur) {
            crate::icons::draw_emoji(pm, f, bx + 19.0, by + 14.0, 20);
        }
        for (k, l) in lines.iter().enumerate() {
            let c = if k == 0 {
                GOLD
            } else if l.starts_with("   ") {
                GREY
            } else {
                WHITE
            };
            text.draw(
                pm,
                &text.fit(l, 12.0, bw - 16.0),
                ((bx + 10.0) as f32, (by + 18.0 + k as f64 * 17.0) as f32),
                12.0,
                c,
                Align::Left,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(ts: i64, cur: &str, impact: u8) -> Mark {
        Mark {
            ts,
            currency: cur.into(),
            impact,
            title: format!("{cur} {ts}"),
            numbers: "Prévu 0.3%".into(),
            window_s: if impact >= 2 { 120 } else { 0 },
            ..Default::default()
        }
    }

    #[test]
    fn annonces_sur_la_chart() {
        let x_of = |t: i64| t as f64 / 10.0;
        // USD fort + USD moyen à la même heure = un rond (impact fort) ; EUR à la même heure : décalé
        let marks = vec![m(1000, "USD", 2), m(1000, "USD", 1), m(1000, "EUR", 1), m(5000, "CAD", 2), m(99_999, "JPY", 2)];
        let mut marks = marks;
        marks.push(Mark {
            ts: 3000,
            currency: NEWS.into(),
            title: "Nouvelle".into(),
            numbers: "source".into(),
            ..Default::default()
        });
        let s = spots(&marks, &x_of, 800.0);
        assert_eq!(s.len(), 4, "JPY hors de la chart ; la nouvelle a son ⚡");
        assert!(s.iter().any(|x| x.1 == NEWS && x.2 == 0));
        assert_eq!((s[0].1, s[0].2, s[0].3.len()), ("USD", 2, 2));
        assert!(s[1].0 > s[0].0 + R, "EUR décalé");
        assert_eq!(flag("EUR"), Some("🇪🇺"));
        for c in ["USD", "EUR", "GBP", "JPY", "CAD", "AUD", "NZD", "CHF", "CNY"] {
            assert!(crate::emoji::image(flag(c).unwrap(), 13).is_some(), "drapeau {c} dans Twemoji");
        }
        let mut pm = crate::px::new(800, 300).unwrap();
        draw(
            &mut pm,
            &Text::default(),
            &marks,
            &x_of,
            800.0,
            300.0,
            Some((100.0, 300.0 - R - 6.0)),
            -4 * 3600,
        );
    }

    /// ⚡ News rounds: two that would touch make one (both news in it), one that would touch a flag is dropped, a
    /// click finds the ⚡ and its newest news; the hover bubble is for the flags only.
    #[test]
    fn eclairs_dans_la_rangee() {
        let x_of = |t: i64| t as f64 / 10.0;
        let news = |ts: i64, l: &str| Mark {
            ts,
            currency: NEWS.into(),
            link: l.into(),
            ..Default::default()
        };
        let marks = vec![news(1000, "a"), news(1100, "b"), news(3000, "c"), m(3050, "USD", 2)];
        let s = spots(&marks, &x_of, 800.0);
        assert_eq!(s.iter().filter(|p| p.1 == NEWS).count(), 1, "a + b together, c drops before the USD flag");
        assert_eq!(s.iter().find(|p| p.1 == NEWS).unwrap().3.len(), 2);
        assert!(s.iter().any(|p| p.1 == "USD" && (p.0 - 305.0).abs() < 0.01), "the flag keeps its place");
        let mut pm = crate::px::new(800, 300).unwrap();
        let spots = draw_marks(&mut pm, &Text::default(), &marks, &x_of, 800.0, 300.0, &[]);
        let cy = rangee_cy();
        assert_eq!(eclair_sous(100.0, cy), Some((100.0, "b".into())), "the newest news of that ⚡");
        assert_eq!(eclair_sous(100.0, cy - 40.0), None);
        assert_eq!(eclair_sous(305.0, cy), None, "a flag is not a ⚡");
        // the bubble: nothing over a ⚡
        let mut a = crate::px::new(800, 300).unwrap();
        bubble(&mut a, &Text::default(), &spots, 800.0, 300.0, Some((100.0, cy)), 0);
        assert!(a.data().iter().all(|&v| v == 0), "no bubble over a ⚡");
        // a wall pill of Argos Murs over the USD flag (drawn later): the flag is left out, the ⚡ stays
        let pilule = Rect::new(290.0, cy - 8.0, 120.0, 22.0);
        let spots = draw_marks(&mut pm, &Text::default(), &marks, &x_of, 800.0, 300.0, &[pilule]);
        assert!(spots.iter().all(|s| s.currency != "USD"), "no flag under the wall pill");
        assert_eq!(spots.iter().filter(|s| s.currency == NEWS).count(), 1);
        assert!(ronds_dessines().iter().all(|r| !(r.x < pilule.x + pilule.w && pilule.x < r.x + r.w)));
        // the row is kept clear of the high / low pills: a low pill on the row goes above it
        let row = rangee_zone(300.0, 800.0).unwrap();
        assert!(row.y <= cy - R && row.y + row.h >= cy + R, "{row:?}");
        let bars: Vec<argos_core::Bar> = (0..3)
            .map(|i| argos_core::Bar {
                time: i,
                open: 105.0,
                high: 110.0,
                low: if i == 1 { 90.0 } else { 100.0 },
                close: 105.0,
                volume: 1.0,
            })
            .collect();
        let y_of = |p: f64| if p < 95.0 { cy } else { 50.0 };
        let hl = crate::hilo::marks(&bars, 0, 2, &|i| 200.0 + i as f64 * 10.0, &y_of, &|_| 60.0, 800.0, &[row]);
        let bas = hl.iter().find(|m| !m.high).unwrap();
        assert!(bas.pill.y + bas.pill.h <= row.y, "the low pill above the row: {:?}", bas.pill);
    }

    /// 📅 Drapeaux verrouillés (bug du 16 nov. : des dizaines de 📰 avaient remplacé les drapeaux) : agenda + 60
    /// nouvelles → chaque drapeau garde sa place exacte (celle qu'il a sans nouvelles), et au pixel le rond d'un drapeau
    /// n'est pas l'icône 📰 (il a la couleur du drapeau, pas le gris du journal).
    #[test]
    fn drapeaux_jamais_chasses_par_les_nouvelles() {
        let x_of = |ts: i64| ts as f64 / 10.0;
        let m = |ts: i64, c: &str| Mark {
            ts,
            currency: c.into(),
            impact: 2,
            title: "x".into(),
            ..Default::default()
        };
        let agenda = vec![m(1000, "CAD"), m(2000, "USD"), m(3000, "AUD"), m(4000, "GBP"), m(5000, "EUR")];
        let seul = spots(&agenda, &x_of, 800.0);
        let mut tout: Vec<Mark> = (0..60)
            .map(|k| Mark {
                ts: 500 + k * 120,
                currency: NEWS.into(),
                ..Default::default()
            })
            .collect();
        tout.extend(agenda.clone()); // (même placées après les nouvelles dans la liste)
        let s = spots(&tout, &x_of, 800.0);
        for (x, c, _, _) in &seul {
            assert!(s.iter().any(|p| p.1 == *c && (p.0 - x).abs() < 0.01), "{c} à sa place {x}");
        }
        // au pixel : sous le rond CAD, du rouge du drapeau canadien (le 📰 est gris et bleu)
        let mut pm = crate::px::new(800, 100).unwrap();
        draw(&mut pm, &Text::default(), &tout, &x_of, 800.0, 100.0, None, 0);
        let (cx, cy) = (100.0, 100.0 - R - 6.0);
        let rouge = (-6..=6).flat_map(|dx| (-6..=6).map(move |dy| (dx, dy))).any(|(dx, dy)| {
            let p = pm.pixel((cx + dx as f64) as u32, (cy + dy as f64) as u32).unwrap();
            p.red() > 180 && p.green() < 90 && p.blue() < 90
        });
        assert!(rouge, "le drapeau 🇨🇦 est dessiné, pas l'icône 📰");
    }
}
