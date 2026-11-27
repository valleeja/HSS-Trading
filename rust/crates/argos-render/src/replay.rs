//! ⏪ Replay des barres (comme Argos v1) : rejouer le marché bougie par bougie. La chart s'arrête à l'heure `t` ;
//! ✂️ choisir la bougie de départ, ⏮ ⏭ une bougie, ▶ / ⏸ jouer, 1× / 3× / 10× la vitesse, ✕ retour au direct.
//! Ici : l'état et la barre (dessin et clic, une table de zones) ; la fenêtre coupe les bougies et fait avancer `t`.

use crate::palette as pal;
use crate::palette::{accent, GREY, WHITE};
use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use tiny_skia::Pixmap;

/// État du replay.
#[derive(Debug, Clone, PartialEq)]
pub struct Replay {
    /// Heure de la dernière bougie montrée.
    pub t: i64,
    pub play: bool,
    /// 1, 3 ou 10 bougies par seconde.
    pub speed: u32,
    /// Prochain clic sur la chart = bougie de départ.
    pub pick: bool,
    /// Bougies encore à venir après `t` (remplies par la fenêtre).
    pub left: usize,
}

impl Replay {
    pub fn new(t: i64) -> Self {
        Self {
            t,
            play: false,
            speed: 1,
            pick: true,
            left: 0,
        }
    }

    /// Vitesse suivante : 1× → 2× → 5× → 10× → 1× (les mêmes que le Simulateur, `argos_core::simu::Replay::SPEEDS`).
    pub fn next_speed(&mut self) {
        self.speed = argos_core::simu::Replay::next_speed(self.speed);
    }

    /// Temps entre deux bougies quand ça joue.
    pub fn period_ms(&self) -> u64 {
        1000 / self.speed.max(1) as u64
    }

    /// Heure après `d` bougies (−1 = précédente) parmi `times` (toutes les bougies, triées).
    pub fn step(&self, times: &[i64], d: isize) -> Option<i64> {
        let i = times.iter().position(|&x| x > self.t).unwrap_or(times.len()) as isize; // bougies montrées
        let j = (i + d).clamp(1, times.len() as isize);
        times.get(j as usize - 1).copied()
    }
}

/// Ce qu'un clic dans la barre demande.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayClick {
    Exit,
    Pick,
    Prev,
    Play,
    Next,
    Speed,
}

const BW: f64 = 38.0;
const H: f64 = 46.0;
const INFO: f64 = 150.0;
/// La barre sur une rangée : les boutons, puis la date et ce qui reste à venir.
const LARGE: f64 = 6.0 * BW + INFO + 16.0;
/// 📱 La barre étroite : les boutons seuls, la date et ce qui reste à venir sur une 2e rangée.
pub const ETROITE: f64 = 6.0 * BW + 12.0;
const RANG2: f64 = 22.0;
/// Hauteur entre le bas de la barre et le bas de la zone : la place des boutons de zoom (− + ‹ › ↻).
pub const AU_DESSUS: f64 = 64.0;

/// Barre centrée en bas de la zone de prix (`zone_w` × `plot_h`, coordonnées de la chart), au-dessus des boutons de
/// zoom (− + ‹ › ↻) : sur une rangée quand elle tient ; 📱 sinon étroite, la date sous les boutons, jamais coupée au
/// bord (Suivi 7 : sur le téléphone, son ✕ et son ▶ sortaient de la chart).
pub fn rect(zone_w: f64, plot_h: f64) -> Rect {
    if zone_w >= LARGE + 8.0 {
        return Rect::new((zone_w - LARGE) / 2.0, plot_h - H - AU_DESSUS, LARGE, H);
    }
    let h = H + RANG2;
    Rect::new(((zone_w - ETROITE) / 2.0).max(4.0), plot_h - h - AU_DESSUS, ETROITE, h)
}

fn zones(r: Rect) -> Vec<(Rect, ReplayClick)> {
    [
        ReplayClick::Exit,
        ReplayClick::Pick,
        ReplayClick::Prev,
        ReplayClick::Play,
        ReplayClick::Next,
        ReplayClick::Speed,
    ]
    .into_iter()
    .enumerate()
    .map(|(k, c)| (Rect::new(r.x + 6.0 + k as f64 * BW, r.y + 4.0, BW - 2.0, H - 8.0), c))
    .collect()
}

/// Clic dans la barre (coordonnées de la chart) ; `None` : à côté.
pub fn click(zone_w: f64, plot_h: f64, x: f64, y: f64) -> Option<ReplayClick> {
    let r = rect(zone_w, plot_h);
    if !r.contains(x, y) {
        return None;
    }
    zones(r).into_iter().find(|(q, _)| q.contains(x, y)).map(|(_, c)| c)
}

/// Dessine la barre dans la chart ; `date` : heure de `t` déjà écrite.
pub fn draw(pm: &mut Pixmap, text: &Text, rp: &Replay, zone_w: f64, plot_h: f64, date: &str, mouse: Option<(f64, f64)>) {
    let r = rect(zone_w, plot_h);
    round_fill(pm, r, 12.0, pal::panel(), 0.96);
    round_stroke(pm, r, 12.0, pal::edge(), 1.0);
    for (q, c) in zones(r) {
        let on = matches!(c, ReplayClick::Pick) && rp.pick;
        if on || mouse.is_some_and(|(x, y)| q.contains(x, y)) {
            round_fill(pm, q, 8.0, if on { accent() } else { pal::hover() }, if on { 0.35 } else { 1.0 });
        }
        let (cx, cy) = (q.cx(), q.cy());
        match c {
            ReplayClick::Exit => lines(pm, cx, cy, &[&[(-5.0, -5.0), (5.0, 5.0)], &[(5.0, -5.0), (-5.0, 5.0)]], WHITE, 1.6),
            ReplayClick::Pick => crate::icons::draw(pm, "✂️", cx, cy, 18, WHITE),
            ReplayClick::Prev => lines(
                pm,
                cx,
                cy,
                &[&[(-6.0, -6.0), (-6.0, 6.0)], &[(6.0, -6.0), (-3.0, 0.0), (6.0, 6.0), (6.0, -6.0)]],
                WHITE,
                1.6,
            ),
            ReplayClick::Next => lines(
                pm,
                cx,
                cy,
                &[&[(6.0, -6.0), (6.0, 6.0)], &[(-6.0, -6.0), (3.0, 0.0), (-6.0, 6.0), (-6.0, -6.0)]],
                WHITE,
                1.6,
            ),
            ReplayClick::Play if rp.play => {
                round_fill(pm, Rect::new(cx - 6.0, cy - 7.0, 4.0, 14.0), 1.0, WHITE, 1.0);
                round_fill(pm, Rect::new(cx + 2.0, cy - 7.0, 4.0, 14.0), 1.0, WHITE, 1.0);
            }
            ReplayClick::Play => crate::shapes::poly_fill(pm, cx, cy, &[(-5.0, -7.5), (7.0, 0.0), (-5.0, 7.5)], WHITE, 1.0),
            ReplayClick::Speed => text.draw(pm, &format!("{}×", rp.speed), (cx as f32, (cy + 5.0) as f32), 13.5, WHITE, Align::Center),
        }
    }
    let sub = if rp.pick {
        // (👆 by the finger, phone and tablet: nothing is clicked)
        crate::i18n::tr(if crate::actions::tactile() {
            "Touche la bougie de départ"
        } else {
            "Clique la bougie de départ"
        })
        .into_owned()
    } else {
        format!("{} {}", rp.left, crate::i18n::tr("à venir"))
    };
    if r.h > H {
        // 📱 étroite : sous les boutons, la date et ce qui reste à venir (au choix du départ : quoi faire)
        let y = (r.y + H + 13.0) as f32;
        if rp.pick {
            text.draw(pm, &sub, ((r.x + r.w / 2.0) as f32, y), 11.0, GREY, Align::Center);
        } else {
            text.draw(pm, date, ((r.x + 12.0) as f32, y), 12.0, WHITE, Align::Left);
            text.draw(pm, &sub, ((r.x + r.w - 12.0) as f32, y), 11.0, GREY, Align::Right);
        }
        return;
    }
    let ix = r.x + 6.0 + 6.0 * BW + 8.0;
    text.draw(pm, date, (ix as f32, (r.y + 20.0) as f32), 12.5, WHITE, Align::Left);
    text.draw(pm, &sub, (ix as f32, (r.y + 36.0) as f32), 11.0, GREY, Align::Left);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replay() {
        let times: Vec<i64> = (0..10).map(|i| i * 60).collect();
        let mut rp = Replay::new(300); // 6 bougies montrées (0..=300)
        assert_eq!(rp.step(&times, 1), Some(360));
        assert_eq!(rp.step(&times, -1), Some(240));
        rp.t = 540;
        assert_eq!(rp.step(&times, 1), Some(540), "au bout : on reste");
        rp.t = 0;
        assert_eq!(rp.step(&times, -1), Some(0), "au début : au moins une bougie");
        rp.next_speed();
        assert_eq!((rp.speed, rp.period_ms()), (2, 500));
        rp.next_speed();
        rp.next_speed();
        assert_eq!((rp.speed, rp.period_ms()), (10, 100), "×10 : une bougie tous les dixièmes de seconde");
        rp.next_speed();
        assert_eq!(rp.speed, 1);
        let r = rect(1000.0, 600.0);
        let (q, _) = zones(r)[3];
        assert_eq!(click(1000.0, 600.0, q.cx(), q.cy()), Some(ReplayClick::Play));
        assert_eq!(click(1000.0, 600.0, 5.0, 5.0), None);
        let mut pm = crate::px::new(1000, 600).unwrap();
        draw(&mut pm, &Text::default(), &rp, 1000.0, 600.0, "23 oct. 10:30", None);
    }

    /// 📱 Suivi 7 (Jee, 21 Nov): on the phone the bar came out of the chart (its ✕ cut off; in 3 columns, its ▶ too).
    /// Where the whole bar does not fit: the buttons alone, the date and what is left under them, all inside.
    #[test]
    fn etroite_jamais_coupee() {
        assert_eq!(rect(1000.0, 600.0).h, H, "wide: one row");
        let t = Text::default();
        let rp = Replay::new(300);
        for zone in [347.0, 250.0] {
            let r = rect(zone, 600.0);
            assert!(r.x >= 4.0 && r.x + r.w <= zone - 4.0 && r.h > H, "{zone}: {r:?}");
            for (q, c) in zones(r) {
                assert!(q.x >= r.x && q.x + q.w <= r.x + r.w);
                assert_eq!(click(zone, 600.0, q.cx(), q.cy()), Some(c), "{zone}: {c:?}");
            }
            // the date under the buttons, inside the bar
            let mut pm = crate::px::new(zone as u32, 600).unwrap();
            draw(&mut pm, &t, &rp, zone, 600.0, "23 oct. 10:30", None);
            let fond = pm.pixel((r.x + r.w / 2.0) as u32, (r.y + H + 2.0) as u32).unwrap();
            let encre = (r.y as u32 + H as u32..(r.y + r.h) as u32 - 3)
                .flat_map(|y| (r.x as u32 + 8..(r.x + r.w) as u32 - 8).map(move |x| (x, y)))
                .filter(|&(x, y)| pm.pixel(x, y) != Some(fond))
                .count();
            assert!(encre > 50, "{zone}: the date drawn under the buttons ({encre})");
        }
    }
}
