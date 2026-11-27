//! Barre du bas (comme TradingView), sous l'échelle de temps : périodes 1D 5D 1M 3M 6M YTD 1Y Tout, bouton 📅 « Aller à »
//! (une date) et l'horloge à droite. Les actions passent par la table unique (`actions`) : `ShowRange` (afficher
//! une plage de bougies) et `GoTo` (ouvrir la fenêtre « Aller à » du module `goto`, Alt + G).

use crate::palette as pal;
use crate::palette::{TEXT, WHITE};
use crate::text::{Align, Rgb, Text};
use crate::viewport::Viewport;
use crate::{fill, Theme};
use argos_core::Bar;
use tiny_skia::Pixmap;

/// Hauteur de la barre.
pub const HEIGHT: u32 = 34;

/// Les périodes, de gauche à droite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    D1,
    D5,
    M1,
    M3,
    M6,
    Ytd,
    Y1,
    All,
}

impl Range {
    pub const ALL: [Range; 8] = [Range::D1, Range::D5, Range::M1, Range::M3, Range::M6, Range::Ytd, Range::Y1, Range::All];

    pub fn label(self) -> &'static str {
        match self {
            Range::D1 => "1D",
            Range::D5 => "5D",
            Range::M1 => "1M",
            Range::M3 => "3M",
            Range::M6 => "6M",
            Range::Ytd => "YTD",
            Range::Y1 => "1Y",
            Range::All => "Tout",
        }
    }

    /// Bulle au survol.
    pub fn tip(self) -> &'static str {
        match self {
            Range::D1 => "1 jour",
            Range::D5 => "5 jours",
            Range::M1 => "1 mois",
            Range::M3 => "3 mois",
            Range::M6 => "6 mois",
            Range::Ytd => "Depuis le début de l'année",
            Range::Y1 => "1 an",
            Range::All => "Tout l'historique",
        }
    }

    /// Début de la période (secondes UTC), en partant de la dernière bougie ; `None` = tout.
    pub fn from(self, last_time: i64, tz_offset_s: i64) -> Option<i64> {
        const DAY: i64 = 86_400;
        Some(match self {
            Range::D1 => last_time - DAY,
            Range::D5 => last_time - 5 * DAY,
            Range::M1 => last_time - 30 * DAY,
            Range::M3 => last_time - 91 * DAY,
            Range::M6 => last_time - 182 * DAY,
            Range::Y1 => last_time - 365 * DAY,
            Range::Ytd => {
                use chrono::{DateTime, Datelike, NaiveDate};
                let local = DateTime::from_timestamp(last_time + tz_offset_s, 0)?.date_naive();
                let jan1 = NaiveDate::from_ymd_opt(local.year(), 1, 1)?.and_hms_opt(0, 0, 0)?;
                jan1.and_utc().timestamp() - tz_offset_s
            }
            Range::All => return None,
        })
    }

    /// Première bougie à afficher pour cette période (0 si l'historique est plus court).
    pub fn start_index(self, bars: &[Bar], tz_offset_s: i64) -> usize {
        match (bars.last(), self) {
            (Some(last), r) => r.from(last.time, tz_offset_s).map_or(0, |t| bars.partition_point(|b| b.time < t)),
            (None, _) => 0,
        }
    }
}

/// Ce qu'il y a sous la souris dans la barre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    Range(Range),
    GoTo,
    /// Sessions et horloge (en bas à droite) : menu Fuseau horaire.
    Sessions,
    /// 🟠 Pastille DÉMO (tablette) : le Simulateur.
    Demo,
}

/// 🟠 Place de la pastille DÉMO (tablette) : juste après 📅, si elle tient avant les sessions.
pub fn demo_rect(vp: &Viewport, text: &Text, label: &str) -> Option<crate::shapes::Rect> {
    let (l, w, _) = buttons(vp, text).into_iter().find(|(_, _, h)| *h == Hit::GoTo)?;
    let x = l + w + 14.0;
    let dw = text.width(label, 12.5) as f64 + 26.0;
    (vp.footer > 0 && x + dw <= sessions_x(vp) - 6.0).then(|| crate::shapes::Rect::new(x, top(vp) + (vp.footer as f64 - 24.0) / 2.0, dw, 24.0))
}

/// 🖥 Pied flottant du cadre PC (maquette : pilule en verre de 1050 × 32, à 8 du bas, centrée, au survol) : hauteur de
/// la pilule, place de sa bulle au-dessus, largeur.
pub const PIED_H: f64 = 32.0;
pub const PIED_BULLE: f64 = 34.0;
pub const PIED_W: f64 = 1050.0;

/// 🖥 Viewport du pied flottant de largeur `w` : la pilule en bas (`PIED_H`), la place de la bulle au-dessus ; les mêmes
/// boutons et le même `hit` que la barre du bas.
pub fn pied_vp(w: f64) -> Viewport {
    Viewport {
        width: w.max(1.0) as u32,
        height: (PIED_H + PIED_BULLE) as u32,
        time_axis: 0,
        footer: PIED_H as u32,
        ..Viewport::default()
    }
}

/// Le pied flottant (sans échelle de temps sous la chart) ?
fn flottant(vp: &Viewport) -> bool {
    vp.time_axis == 0 && vp.footer > 0
}

/// 🖥 Image du pied flottant (largeur `w`, souris dans ses coordonnées) : horloge et sessions de `now_s`.
pub fn draw_pied(text: &Text, w: f64, mouse: Option<(f64, f64)>, now_s: i64, tz_offset_s: i64, th: &Theme) -> Option<Pixmap> {
    let vp = pied_vp(w);
    let mut pm = crate::px::new(vp.width, vp.height)?;
    // (l'horloge reste dans l'échelle de temps, toujours visible : pas une 2e fois dans le pied)
    let _ = tz_offset_s;
    let chips = if now_s > 0 { crate::sessions::chips(now_s) } else { vec![] };
    draw(&mut pm, text, &vp, mouse, None, &chips, th, None);
    Some(pm)
}

/// Largeur de la zone sessions + horloge, au bout à droite.
const SESSIONS_W: f64 = 380.0;

/// Haut de la barre.
fn top(vp: &Viewport) -> f64 {
    vp.plot_h() + vp.time_axis as f64
}

/// Boutons : (gauche, largeur, ce que c'est).
fn buttons(vp: &Viewport, text: &Text) -> Vec<(f64, f64, Hit)> {
    let mut x = 8.0;
    let mut out = vec![];
    for r in Range::ALL {
        let w = text.width(r.label(), 13.0) as f64 + 16.0;
        out.push((x, w, Hit::Range(r)));
        x += w + 2.0;
    }
    out.push((x + 14.0, 32.0, Hit::GoTo)); // après le séparateur
    let _ = vp;
    out
}

/// Début de la zone sessions + horloge : au plus `SESSIONS_W` du bout, jamais sur les boutons (fenêtre étroite).
fn sessions_x(vp: &Viewport) -> f64 {
    let end = buttons(vp, &Text::default()).last().map_or(0.0, |(l, w, _)| l + w + 8.0);
    (vp.width as f64 - SESSIONS_W).max(end)
}

/// Case du bouton 📅 (la fenêtre « Aller à » s'ouvre collée à son coin) ; `None` sans barre du bas.
pub fn goto_rect(vp: &Viewport) -> Option<crate::shapes::Rect> {
    if vp.footer == 0 {
        return None;
    }
    let (l, w, _) = buttons(vp, &Text::default()).into_iter().find(|(_, _, h)| *h == Hit::GoTo)?;
    Some(crate::shapes::Rect::new(l, top(vp) + 5.0, w, vp.footer as f64 - 10.0))
}

/// Bouton sous la souris.
pub fn hit(vp: &Viewport, mouse: Option<(f64, f64)>, demo: Option<&str>) -> Option<Hit> {
    let (x, y) = mouse?;
    if vp.footer == 0 || y < top(vp) || y > top(vp) + vp.footer as f64 {
        return None;
    }
    if demo.and_then(|d| demo_rect(vp, &Text::default(), d)).is_some_and(|r| r.contains(x, y)) {
        return Some(Hit::Demo);
    }
    if x >= sessions_x(vp) {
        return Some(Hit::Sessions);
    }
    buttons(vp, &Text::default())
        .into_iter()
        .find(|(l, w, _)| x >= *l && x <= l + w)
        .map(|(_, _, h)| h)
}

/// Zone sessions + horloge (le menu Fuseau horaire s'ouvre au-dessus) ; `None` sans barre du bas.
pub fn sessions_rect(vp: &Viewport) -> Option<crate::shapes::Rect> {
    (vp.footer > 0).then(|| {
        let x = sessions_x(vp);
        crate::shapes::Rect::new(x, top(vp), (vp.width as f64 - x).max(0.0), vp.footer as f64)
    })
}

/// Dessine la barre : périodes, 📅, sessions ouvertes (`chips` : nom, couleur) et horloge (`clock` déjà formatée).
#[allow(clippy::too_many_arguments)]
pub fn draw(
    pm: &mut Pixmap,
    text: &Text,
    vp: &Viewport,
    mouse: Option<(f64, f64)>,
    clock: Option<&str>,
    chips: &[(String, Rgb)],
    th: &Theme,
    demo: Option<&(String, Rgb)>,
) {
    if vp.footer == 0 {
        return;
    }
    let (y0, h, w) = (top(vp), vp.footer as f64, vp.width as f64);
    // thème clair : la barre est dessinée en sombre puis retournée (`light_ui`) ; ses couleurs doivent donc être
    // celles de l'interface sombre, sinon le texte (déjà foncé) ressort pâle une fois retourné
    let light = crate::palette::is_light(crate::theme_index());
    let (ink, line) = if light { (crate::palette::WHITE, pal::track()) } else { (th.text, th.grid) };
    if flottant(vp) {
        // 🖥 pilule en verre (fond presque plein sans le style verre)
        let r = crate::shapes::Rect::new(0.5, y0 + 0.5, w - 1.0, h - 1.0);
        if !crate::glass::on() {
            crate::shapes::round_fill(pm, r, 14.0, pal::night(), 0.96);
        }
        crate::jetons::verre(pm, r, crate::jetons::R_PIED);
    } else {
        fill(pm, 0.0, y0, w, h, pal::night(), 1.0);
        fill(pm, 0.0, y0, w, 1.0, line, 1.0);
    }
    let hovered = hit(vp, mouse, demo.map(|d| d.0.as_str()));
    let base = (y0 + h / 2.0 + 5.0) as f32;
    for (l, bw, b) in buttons(vp, text) {
        if hovered == Some(b) {
            fill(pm, l, y0 + 5.0, bw, h - 10.0, pal::hover(), 1.0);
        }
        match b {
            Hit::Range(r) => text.draw(pm, r.label(), ((l + bw / 2.0) as f32, base), 13.0, ink, Align::Center),
            Hit::GoTo => {
                // séparateur, puis un petit calendrier dessiné
                fill(pm, l - 9.0, y0 + 8.0, 1.0, h - 16.0, pal::track(), 1.0);
                let (cx, cy) = (l + bw / 2.0, y0 + h / 2.0);
                let c = TEXT;
                fill(pm, cx - 8.0, cy - 6.0, 16.0, 1.5, c, 1.0);
                fill(pm, cx - 8.0, cy + 6.0, 16.0, 1.5, c, 1.0);
                fill(pm, cx - 8.0, cy - 6.0, 1.5, 13.5, c, 1.0);
                fill(pm, cx + 6.5, cy - 6.0, 1.5, 13.5, c, 1.0);
                fill(pm, cx - 8.0, cy - 3.0, 16.0, 1.0, c, 1.0);
                fill(pm, cx - 4.0, cy - 9.0, 1.5, 4.0, c, 1.0);
                fill(pm, cx + 2.5, cy - 9.0, 1.5, 4.0, c, 1.0);
                fill(pm, cx - 3.0, cy + 1.0, 6.0, 1.5, c, 1.0); // flèche « aller »
                fill(pm, cx + 1.5, cy - 0.5, 1.5, 4.5, c, 1.0);
            }
            Hit::Sessions | Hit::Demo => {} // dessinées à part
        }
    }
    // 🟠 pastille DÉMO (tablette) : « DÉMO » en or, le montant en vert / rouge
    if let Some((label, c)) = demo {
        if let Some(r) = demo_rect(vp, text, label) {
            let orange = crate::palette::ORANGE;
            crate::shapes::round_fill(pm, r, 12.0, Rgb(0x2a, 0x1f, 0x10), 0.95);
            crate::shapes::round_stroke(pm, r, 12.0, orange, 1.0);
            crate::shapes::round_fill(pm, crate::shapes::Rect::new(r.x + 8.0, r.cy() - 3.0, 6.0, 6.0), 3.0, orange, 1.0);
            let (mot, montant) = label.split_once(' ').unwrap_or(("", label));
            let yb = (r.cy() + 4.5) as f32;
            text.draw(pm, mot, ((r.x + 18.0) as f32, yb), 12.5, crate::palette::GOLD, Align::Left);
            let mx = (r.x + 18.0) as f32 + text.width(mot, 12.5) + 4.0;
            text.draw(pm, montant, (mx, yb), 12.5, *c, Align::Left);
        }
    }
    // horloge puis pastilles des sessions ouvertes (de droite à gauche, la dernière collée à l'horloge)
    let clock_w = clock.map_or(0.0, |c| text.width(c, 12.5) as f64 + 12.0);
    let mut right = w - 12.0 - clock_w;
    let zone = sessions_x(vp);
    let mut placed = vec![];
    for (name, c) in chips.iter().rev() {
        let cw = text.width(name, 12.0) as f64 + 18.0;
        let r = crate::shapes::Rect::new(right - cw, y0 + (h - 22.0) / 2.0, cw, 22.0);
        if r.x < zone {
            break;
        }
        placed.push((name, *c, r));
        right = r.x - 6.0;
    }
    // survol : fond seulement derrière ce qui est écrit
    if hovered == Some(Hit::Sessions) {
        let left = (right - 2.0).max(zone);
        crate::shapes::round_fill(pm, crate::shapes::Rect::new(left, y0 + 5.0, w - 4.0 - left, h - 10.0), 6.0, pal::card(), 1.0);
    }
    // horloge seulement si elle tient à droite du 📅 (téléphone étroit : on la cache plutôt que de chevaucher)
    if let Some(clock) = clock.filter(|_| w - 12.0 - clock_w >= zone) {
        text.draw(pm, clock, ((w - 12.0) as f32, base), 12.5, ink, Align::Right);
    }
    for (name, c, r) in placed {
        crate::shapes::round_fill(pm, r, 11.0, c, 0.12);
        crate::shapes::round_stroke(pm, r, 11.0, c, 1.2);
        text.draw(pm, name, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.0, c, Align::Center);
    }
    // bulle au-dessus du bouton survolé
    if let Some((l, bw, b)) = buttons(vp, text).into_iter().find(|(_, _, b)| hovered == Some(*b)) {
        let tip = bulle(b);
        let tw = text.width(&tip, 12.0) as f64 + 16.0;
        let bx = (l + bw / 2.0 - tw / 2.0).clamp(2.0, w - tw - 2.0);
        fill(pm, bx, y0 - 30.0, tw, 24.0, pal::hover(), 1.0);
        text.draw(pm, &tip, ((bx + 8.0) as f32, (y0 - 13.0) as f32), 12.0, WHITE, Align::Left);
    }
}

/// Tooltip of a button of the bar (📱 phone view: « Aller à… » without « Alt + G »).
fn bulle(b: Hit) -> String {
    match b {
        Hit::Range(r) => r.tip().to_string(),
        Hit::GoTo => match crate::actions::shortcut_bulle(crate::actions::ChartAction::GoTo) {
            Some(sc) => format!("Aller à…   {sc}"),
            None => "Aller à…".into(),
        },
        Hit::Sessions => "Fuseau horaire et sessions".into(),
        Hit::Demo => "Compte démo : ouvrir le Simulateur".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📱 Jee (20 Nov): « Alt + G » only on the PC; the phone and the touch tablet keep « Aller à… ».
    #[test]
    fn aller_a_sans_alt_g_sur_cell() {
        crate::actions::set_tactile(false);
        assert_eq!(bulle(Hit::GoTo), "Aller à…   Alt + G");
        crate::actions::set_tactile(true);
        assert_eq!(bulle(Hit::GoTo), "Aller à…");
        crate::actions::set_tactile(false);
    }

    fn bars() -> Vec<Bar> {
        // 400 jours de bougies quotidiennes jusqu'au 25 oct. 2026
        let end = 1_792_886_400; // 2026-10-25 00:00 UTC
        (0..400)
            .map(|i| Bar {
                time: end - (399 - i) * 86_400,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 1.0,
            })
            .collect()
    }

    #[test]
    fn periodes() {
        let b = bars();
        assert_eq!(Range::D5.start_index(&b, 0), 394, "5 jours avant la dernière");
        assert_eq!(Range::Y1.start_index(&b, 0), 34);
        assert_eq!(Range::All.start_index(&b, 0), 0);
        // YTD : 1er janvier 2026 = jour 297 avant le 25 oct.
        assert_eq!(b[Range::Ytd.start_index(&b, 0)].time, 1_767_225_600);
        assert_eq!(Range::Ytd.start_index(&[], 0), 0);
    }

    #[test]
    fn boutons_cliquables() {
        let vp = Viewport {
            width: 1200,
            height: 800,
            footer: HEIGHT,
            ..Viewport::default()
        };
        let y = vp.plot_h() + vp.time_axis as f64 + 15.0;
        assert_eq!(hit(&vp, Some((20.0, y)), None), Some(Hit::Range(Range::D1)));
        assert_eq!(hit(&vp, Some((20.0, y - 30.0)), None), None, "pas dans l'échelle de temps");
        let goto = buttons(&vp, &Text::default()).last().copied().unwrap();
        assert_eq!(hit(&vp, Some((goto.0 + 5.0, y)), None), Some(Hit::GoTo));
        let none = Viewport { footer: 0, ..vp };
        assert_eq!(hit(&none, Some((20.0, y)), None), None);
        // 🟠 tablette : la pastille DÉMO après 📅, qu'on touche pour ouvrir le Simulateur
        let d = "DÉMO −580,22 $";
        let r = demo_rect(&vp, &Text::default(), d).expect("place pour la pastille");
        assert!(r.x > goto.0 + goto.1 && r.x + r.w < sessions_x(&vp));
        assert_eq!(hit(&vp, Some((r.cx(), r.cy())), Some(d)), Some(Hit::Demo));
        assert_eq!(hit(&vp, Some((r.cx(), r.cy())), None), None, "sans pastille : rien");
    }
}
