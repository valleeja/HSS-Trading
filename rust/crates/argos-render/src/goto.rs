//! 📅 Fenêtre « Aller à » (comme TradingView), ouverte par le 📅 de la barre du bas ou Alt + G.
//! Deux onglets : **Date** (une date + heure → la bougie au milieu de la chart) et **Plage personnalisée** (début et fin
//! → la plage sur toute la largeur). Calendrier du mois : lundi en premier, jours après la dernière bougie en gris,
//! jour choisi en blanc, aujourd'hui souligné. On peut aussi taper la date (AAAA-MM-JJ) et l'heure (HH:MM).
//! Partie pure (état, clics, clavier : testée) + dessin.

use crate::fill;
use crate::palette as pal;
use crate::palette::{ICON, INK as DARK, MUTED as DIM, SILVER as GREY, TEXT, WHITE};
use crate::shapes::{arc, lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Text};
use crate::viewport::Viewport;
use argos_core::Bar;
use chrono::{DateTime, Datelike, Months, NaiveDate, NaiveDateTime, NaiveTime};
use tiny_skia::Pixmap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Date,
    Range,
}

/// Champ où va ce qu'on tape (et le jour cliqué).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    FromDate,
    FromTime,
    ToDate,
    ToTime,
}

/// Où aller (secondes UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    At(i64),
    Range(i64, i64),
}

/// Résultat d'un clic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Stay,
    Close,
    Go(Target),
}

/// État de la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub struct GoTo {
    pub tab: Tab,
    pub from_date: String,
    pub from_time: String,
    pub to_date: String,
    pub to_time: String,
    pub focus: Field,
    /// Le prochain caractère tapé remplace le champ (comme un champ tout sélectionné).
    fresh: bool,
    /// Premier jour du mois affiché.
    pub month: NaiveDate,
    /// Jour de la dernière bougie : les jours après sont en gris.
    pub max: NaiveDate,
    /// Aujourd'hui (souligné).
    pub today: NaiveDate,
    /// Champ illisible au dernier « Aller à ».
    pub error: Option<Field>,
}

const MONTHS: [&str; 12] = [
    "Janvier",
    "Février",
    "Mars",
    "Avril",
    "Mai",
    "Juin",
    "Juillet",
    "Août",
    "Septembre",
    "Octobre",
    "Novembre",
    "Décembre",
];
const WEEKDAYS: [&str; 7] = ["Lu", "Ma", "Me", "Jeu", "Ven", "Sam", "Dim"];

fn local(t: i64, tz: i64) -> NaiveDateTime {
    DateTime::from_timestamp(t + tz, 0).map_or_else(NaiveDateTime::default, |d| d.naive_utc())
}

/// « 2026-10-25 ».
fn ymd(d: impl Datelike) -> String {
    format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day())
}

/// « 18:30 ».
fn hm(t: NaiveDateTime) -> String {
    use chrono::Timelike;
    format!("{:02}:{:02}", t.hour(), t.minute())
}

fn first_of(d: NaiveDate) -> NaiveDate {
    d.with_day(1).unwrap_or(d)
}

/// Lit une date (`AAAA-MM-JJ`) et une heure (`HH:MM`, `HHMM` ou vide = minuit) du fuseau affiché → secondes UTC.
pub fn parse(date: &str, time: &str, tz: i64) -> Option<i64> {
    let d = NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()?;
    let time = time.trim();
    let t = if time.is_empty() {
        NaiveTime::MIN
    } else {
        ["%H:%M", "%H%M"].iter().find_map(|f| NaiveTime::parse_from_str(time, f).ok())?
    };
    Some(d.and_time(t).and_utc().timestamp() - tz)
}

/// Bougie qui contient ce moment (la plus proche si avant ou après l'historique).
pub fn index_at_time(bars: &[Bar], t: i64) -> Option<usize> {
    (!bars.is_empty()).then(|| bars.partition_point(|b| b.time <= t).saturating_sub(1))
}

impl GoTo {
    /// Ouvre sur le jour de la dernière bougie ; la plage proposée = les dernières 24 h.
    pub fn open(last_time: i64, now_s: i64, tz: i64) -> Self {
        let last = local(last_time, tz);
        Self {
            tab: Tab::Date,
            from_date: ymd(last),
            from_time: "00:00".into(),
            to_date: ymd(last),
            to_time: hm(last),
            focus: Field::FromDate,
            fresh: true,
            month: first_of(last.date()),
            max: last.date(),
            today: local(now_s, tz).date(),
            error: None,
        }
    }

    fn field_mut(&mut self, f: Field) -> &mut String {
        match f {
            Field::FromDate => &mut self.from_date,
            Field::FromTime => &mut self.from_time,
            Field::ToDate => &mut self.to_date,
            Field::ToTime => &mut self.to_time,
        }
    }

    fn focus_on(&mut self, f: Field) {
        self.focus = f;
        self.fresh = true;
    }

    fn switch_tab(&mut self, t: Tab) {
        if self.tab != t {
            self.tab = t;
            if t == Tab::Range {
                // plage : début = 24 h avant la fin proposée
                if let Some(end) = parse(&self.to_date, &self.to_time, 0) {
                    let start = local(end - 86_400, 0);
                    self.from_date = ymd(start);
                    self.from_time = hm(start);
                }
            }
        }
        self.focus_on(Field::FromDate);
        self.error = None;
    }

    /// Un caractère tapé dans le champ actif (chiffres, « - » pour la date, « : » pour l'heure).
    pub fn type_char(&mut self, c: char) {
        let date = matches!(self.focus, Field::FromDate | Field::ToDate);
        let ok = c.is_ascii_digit() || (date && c == '-') || (!date && c == ':');
        if !ok {
            return;
        }
        let fresh = std::mem::replace(&mut self.fresh, false);
        let max = if date { 10 } else { 5 };
        let f = self.focus;
        let s = self.field_mut(f);
        if fresh {
            s.clear();
        }
        if s.len() < max {
            s.push(c);
        }
        self.error = None;
        // date complète : le calendrier va à ce mois
        if date {
            if let Ok(d) = NaiveDate::parse_from_str(self.field_mut(f), "%Y-%m-%d") {
                self.month = first_of(d);
            }
        }
    }

    pub fn backspace(&mut self) {
        self.fresh = false;
        let f = self.focus;
        self.field_mut(f).pop();
        self.error = None;
    }

    /// « Aller à » : `None` si un champ est illisible (il passe en rouge).
    pub fn submit(&mut self, tz: i64) -> Option<Target> {
        let from = parse(&self.from_date, &self.from_time, tz);
        let Some(from) = from else {
            self.error = Some(if parse(&self.from_date, "", tz).is_none() {
                Field::FromDate
            } else {
                Field::FromTime
            });
            return None;
        };
        if self.tab == Tab::Date {
            return Some(Target::At(from));
        }
        let Some(to) = parse(&self.to_date, &self.to_time, tz) else {
            self.error = Some(if parse(&self.to_date, "", tz).is_none() {
                Field::ToDate
            } else {
                Field::ToTime
            });
            return None;
        };
        Some(Target::Range(from.min(to), from.max(to)))
    }

    /// Le mois suivant est-il permis ? (pas après la dernière bougie)
    fn can_next(&self) -> bool {
        self.month < first_of(self.max)
    }

    /// Jour à une case du calendrier (`None` : case vide).
    fn day_at(&self, cell: usize) -> Option<NaiveDate> {
        let offset = self.month.weekday().num_days_from_monday() as usize;
        let d = (cell + 1).checked_sub(offset)?;
        self.month.with_day(d as u32)
    }

    fn selected(&self, d: NaiveDate) -> bool {
        let is = |s: &str| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok() == Some(d);
        is(&self.from_date) || (self.tab == Tab::Range && is(&self.to_date))
    }

    pub fn click(&mut self, vp: &Viewport, x: f64, y: f64, tz: i64) -> Outcome {
        let l = layout(vp, self.tab);
        if !l.panel.contains(x, y) {
            return Outcome::Close;
        }
        if l.close.contains(x, y) || l.cancel.contains(x, y) {
            return Outcome::Close;
        }
        if l.go.contains(x, y) {
            return self.submit(tz).map_or(Outcome::Stay, Outcome::Go);
        }
        if l.tabs[0].contains(x, y) {
            self.switch_tab(Tab::Date);
        } else if l.tabs[1].contains(x, y) {
            self.switch_tab(Tab::Range);
        } else if l.prev.contains(x, y) {
            self.month = self.month.checked_sub_months(Months::new(1)).unwrap_or(self.month);
        } else if l.next.contains(x, y) && self.can_next() {
            self.month = self.month.checked_add_months(Months::new(1)).unwrap_or(self.month);
        } else if let Some(f) = l.fields(self.tab).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, f)| f) {
            self.focus_on(f);
        } else if let Some(cell) = (0..42).find(|&c| l.cell(c).contains(x, y)) {
            if let Some(d) = self.day_at(cell).filter(|d| *d <= self.max) {
                let s = ymd(d);
                self.error = None;
                if matches!(self.focus, Field::ToDate | Field::ToTime) && self.tab == Tab::Range {
                    self.to_date = s;
                    self.focus_on(Field::ToDate);
                } else {
                    self.from_date = s;
                    // plage : après le début, on choisit la fin
                    self.focus_on(if self.tab == Tab::Range { Field::ToDate } else { Field::FromDate });
                }
            }
        }
        Outcome::Stay
    }
}

/// Où est chaque chose dans la fenêtre.
struct Layout {
    panel: Rect,
    close: Rect,
    tabs: [Rect; 2],
    from_date: Rect,
    from_time: Rect,
    to_date: Rect,
    to_time: Rect,
    prev: Rect,
    next: Rect,
    band: Rect,
    grid: Rect,
    cancel: Rect,
    go: Rect,
}

const W: f64 = 360.0;
const PAD: f64 = 20.0;
const CELL_H: f64 = 38.0;

impl Layout {
    fn cell(&self, c: usize) -> Rect {
        let cw = self.grid.w / 7.0;
        Rect::new(self.grid.x + (c % 7) as f64 * cw, self.grid.y + (c / 7) as f64 * CELL_H, cw, CELL_H)
    }
    fn fields(&self, tab: Tab) -> Vec<(Rect, Field)> {
        let mut v = vec![(self.from_date, Field::FromDate), (self.from_time, Field::FromTime)];
        if tab == Tab::Range {
            v.extend([(self.to_date, Field::ToDate), (self.to_time, Field::ToTime)]);
        }
        v
    }
}

fn layout(vp: &Viewport, tab: Tab) -> Layout {
    let rows = if tab == Tab::Range { 2.0 } else { 1.0 };
    let fields_end = 106.0 + 38.0 + (rows - 1.0) * 48.0;
    let nav = fields_end + 14.0;
    let band = nav + 42.0;
    let grid = band + 32.0;
    let sep = grid + 6.0 * CELL_H + 10.0;
    let h = sep + 14.0 + 36.0 + 16.0;
    // collée coin à coin au-dessus du 📅 de la barre du bas (sinon au milieu de la chart)
    let (x, y) = match crate::bottombar::goto_rect(vp) {
        Some(icon) => (icon.x.min(vp.width as f64 - W - 4.0).max(4.0), (icon.y - h).max(4.0)),
        None => (((vp.plot_w() - W) / 2.0).max(4.0), ((vp.plot_h() - h) / 2.0).max(4.0)),
    };
    let r = |dx: f64, dy: f64, w: f64, hh: f64| Rect::new(x + dx, y + dy, w, hh);
    let time_w = W - 2.0 * PAD - 202.0;
    Layout {
        panel: r(0.0, 0.0, W, h),
        close: r(W - 46.0, 14.0, 30.0, 30.0),
        tabs: [r(PAD, 58.0, 54.0, 28.0), r(PAD + 66.0, 58.0, 180.0, 28.0)],
        from_date: r(PAD, 106.0, 190.0, 38.0),
        from_time: r(PAD + 202.0, 106.0, time_w, 38.0),
        to_date: r(PAD, 154.0, 190.0, 38.0),
        to_time: r(PAD + 202.0, 154.0, time_w, 38.0),
        prev: r(PAD, nav, 32.0, 32.0),
        next: r(W - PAD - 32.0, nav, 32.0, 32.0),
        band: r(PAD, band, W - 2.0 * PAD, 26.0),
        grid: r(PAD, grid, W - 2.0 * PAD, 6.0 * CELL_H),
        cancel: r(W - PAD - 100.0 - 10.0 - 92.0, sep + 14.0, 92.0, 36.0),
        go: r(W - PAD - 100.0, sep + 14.0, 100.0, 36.0),
    }
}

fn calendar_icon(pm: &mut Pixmap, cx: f64, cy: f64) {
    let box_ = [(-7.0, -6.0), (7.0, -6.0), (7.0, 7.0), (-7.0, 7.0), (-7.0, -6.0)];
    lines(pm, cx, cy, &[&box_, &[(-4.0, -9.0), (-4.0, -5.0)], &[(4.0, -9.0), (4.0, -5.0)]], GREY, 1.3);
    for (dx, dy) in [(-3.5, -1.0), (0.0, -1.0), (3.5, -1.0), (-3.5, 3.0), (0.0, 3.0), (3.5, 3.0)] {
        fill(pm, cx + dx - 0.75, cy + dy - 0.75, 1.5, 1.5, GREY, 1.0);
    }
}

fn clock_icon(pm: &mut Pixmap, cx: f64, cy: f64) {
    let circle = arc(8.0, 0.0, 360.0, 32);
    lines(pm, cx, cy, &[&circle, &[(0.0, -4.5), (0.0, 0.0), (3.0, 2.0)]], GREY, 1.3);
}

/// Dessine la fenêtre.
pub fn draw(pm: &mut Pixmap, text: &Text, vp: &Viewport, g: &GoTo) {
    let l = layout(vp, g.tab);
    let p = l.panel;
    round_fill(pm, p, 8.0, pal::panel(), 1.0);
    round_stroke(pm, p, 8.0, pal::edge_soft(), 1.0);
    text.draw(pm, "Aller à", ((p.x + PAD) as f32, (p.y + 36.0) as f32), 19.0, WHITE, Align::Left);
    let x = 6.0f32;
    lines(pm, l.close.cx(), l.close.cy(), &[&[(-x, -x), (x, x)], &[(-x, x), (x, -x)]], GREY, 1.6);

    // onglets + barre de soulignement
    let names = ["Date", "Plage personnalisée"];
    let active = if g.tab == Tab::Date { 0 } else { 1 };
    for (i, (r, name)) in l.tabs.iter().zip(names).enumerate() {
        let col = if i == active { WHITE } else { GREY };
        text.draw(pm, name, (r.x as f32, (r.y + 19.0) as f32), 15.0, col, Align::Left);
    }
    let track = Rect::new(p.x + PAD, p.y + 92.0, W - 2.0 * PAD, 4.0);
    round_fill(pm, track, 2.0, pal::track(), 1.0);
    let at = l.tabs[active];
    let tw = text.width(names[active], 15.0) as f64;
    round_fill(pm, Rect::new(at.x, track.y, tw, 4.0), 2.0, WHITE, 1.0);

    // champs date / heure
    for (r, f) in l.fields(g.tab) {
        let (border, width) = if g.error == Some(f) {
            (crate::palette::down(), 2.0)
        } else if g.focus == f {
            (crate::palette::accent(), 2.0)
        } else {
            (pal::thumb(), 1.0)
        };
        round_stroke(pm, r, 7.0, border, width);
        let value = match f {
            Field::FromDate => &g.from_date,
            Field::FromTime => &g.from_time,
            Field::ToDate => &g.to_date,
            Field::ToTime => &g.to_time,
        };
        text.draw(pm, value, ((r.x + 11.0) as f32, (r.y + 24.0) as f32), 14.5, WHITE, Align::Left);
        if matches!(f, Field::FromDate | Field::ToDate) {
            calendar_icon(pm, r.x + r.w - 20.0, r.cy());
        } else {
            clock_icon(pm, r.x + r.w - 20.0, r.cy());
        }
    }

    // mois : ‹ Octobre 2026 ›
    lines(pm, l.prev.cx(), l.prev.cy(), &[&[(3.0, -6.0), (-3.0, 0.0), (3.0, 6.0)]], WHITE, 1.6);
    let next_col = if g.can_next() { WHITE } else { DIM };
    lines(pm, l.next.cx(), l.next.cy(), &[&[(-3.0, -6.0), (3.0, 0.0), (-3.0, 6.0)]], next_col, 1.6);
    let title = format!("{} {}", MONTHS[g.month.month0() as usize], g.month.year());
    text.draw(pm, &title, (p.cx() as f32, (l.prev.y + 22.0) as f32), 16.0, GREY, Align::Center);

    // Lu Ma Me Jeu Ven Sam Dim
    round_fill(pm, l.band, 4.0, pal::hover(), 0.7);
    let cw = l.band.w / 7.0;
    for (i, d) in WEEKDAYS.iter().enumerate() {
        let cx = l.band.x + (i as f64 + 0.5) * cw;
        text.draw(pm, d, (cx as f32, (l.band.y + 18.0) as f32), 13.0, GREY, Align::Center);
    }

    // jours
    for c in 0..42 {
        let Some(d) = g.day_at(c) else { continue };
        let r = l.cell(c);
        let sel = g.selected(d);
        let col = if sel {
            DARK
        } else if d > g.max {
            DIM
        } else {
            ICON
        };
        if sel {
            round_fill(pm, Rect::new(r.x + 4.0, r.y + 3.0, r.w - 8.0, r.h - 6.0), 7.0, WHITE, 1.0);
        }
        let label = d.day().to_string();
        text.draw(pm, &label, (r.cx() as f32, (r.y + 25.0) as f32), 16.0, col, Align::Center);
        if d == g.today {
            let w = text.width(&label, 16.0) as f64 + 2.0;
            fill(pm, r.cx() - w / 2.0, r.y + 30.0, w, 2.0, col, 1.0);
        }
    }

    // pied : Annuler · Aller à
    fill(pm, p.x, l.cancel.y - 14.0, W, 1.0, pal::edge_soft(), 1.0);
    round_stroke(pm, l.cancel, 8.0, TEXT, 1.0);
    text.draw(pm, "Annuler", (l.cancel.cx() as f32, (l.cancel.y + 23.0) as f32), 14.5, WHITE, Align::Center);
    round_fill(pm, l.go, 8.0, WHITE, 1.0);
    text.draw(pm, "Aller à", (l.go.cx() as f32, (l.go.y + 23.0) as f32), 14.5, DARK, Align::Center);
}

#[cfg(test)]
mod tests {
    use super::*;

    const TZ: i64 = -4 * 3600;
    // dernière bougie : 25 oct. 2026 18:30 (UTC-4) = 22:30 UTC
    const LAST: i64 = 1_792_886_400 + 22 * 3600 + 1800;

    fn vp() -> Viewport {
        Viewport {
            width: 1400,
            height: 900,
            ..Viewport::default()
        }
    }

    #[test]
    fn ouverture_et_calendrier() {
        let g = GoTo::open(LAST, LAST + 600, TZ);
        assert_eq!((g.from_date.as_str(), g.from_time.as_str()), ("2026-10-25", "00:00"));
        assert_eq!(g.month, NaiveDate::from_ymd_opt(2026, 10, 1).unwrap());
        // octobre 2026 commence un jeudi : 3 cases vides (Lu Ma Me)
        assert_eq!(g.day_at(2), None);
        assert_eq!(g.day_at(3).unwrap().day(), 1);
        assert!(!g.can_next(), "pas de mois après la dernière bougie");
    }

    #[test]
    fn choisir_un_jour_et_aller() {
        let v = vp();
        let mut g = GoTo::open(LAST, LAST, TZ);
        let l = layout(&v, Tab::Date);
        // le 20 oct. = case 3 + 19
        let c = l.cell(22);
        assert_eq!(g.click(&v, c.cx(), c.cy(), TZ), Outcome::Stay);
        assert_eq!(g.from_date, "2026-10-20");
        // un jour après la dernière bougie : rien
        let c = l.cell(3 + 27);
        g.click(&v, c.cx(), c.cy(), TZ);
        assert_eq!(g.from_date, "2026-10-20");
        // heure tapée puis « Aller à »
        g.click(&v, l.from_time.cx(), l.from_time.cy(), TZ);
        for ch in "14:30".chars() {
            g.type_char(ch);
        }
        let go = g.click(&v, l.go.cx(), l.go.cy(), TZ);
        assert_eq!(go, Outcome::Go(Target::At(1_792_454_400 + 18 * 3600 + 1800)), "14 h 30 UTC-4 = 18 h 30 UTC");
        // fermer : X, Annuler, clic dehors
        assert_eq!(g.click(&v, l.close.cx(), l.close.cy(), TZ), Outcome::Close);
        assert_eq!(g.click(&v, l.cancel.cx(), l.cancel.cy(), TZ), Outcome::Close);
        assert_eq!(g.click(&v, 2.0, 2.0, TZ), Outcome::Close);
    }

    #[test]
    fn plage_personnalisee() {
        let v = vp();
        let mut g = GoTo::open(LAST, LAST, TZ);
        let l = layout(&v, Tab::Date);
        g.click(&v, l.tabs[1].cx(), l.tabs[1].cy(), TZ);
        assert_eq!(g.tab, Tab::Range);
        assert_eq!((g.from_date.as_str(), g.from_time.as_str()), ("2026-10-24", "18:30"), "24 h avant la fin");
        let l = layout(&v, Tab::Range);
        // clic sur le 22 : début, puis le focus passe à la fin ; clic sur le 23 : fin
        let c = l.cell(3 + 21);
        g.click(&v, c.cx(), c.cy(), TZ);
        assert_eq!((g.from_date.as_str(), g.focus), ("2026-10-22", Field::ToDate));
        let c = l.cell(3 + 22);
        g.click(&v, c.cx(), c.cy(), TZ);
        assert_eq!(g.to_date, "2026-10-23");
        match g.submit(TZ) {
            Some(Target::Range(a, b)) => assert!(a < b),
            other => panic!("plage attendue : {other:?}"),
        }
    }

    #[test]
    fn taper_et_erreurs() {
        let mut g = GoTo::open(LAST, LAST, TZ);
        // le premier caractère remplace le champ
        for ch in "2026-09-0x5".chars() {
            g.type_char(ch);
        }
        assert_eq!(g.from_date, "2026-09-05");
        assert_eq!(g.month.month(), 9, "le calendrier suit la date tapée");
        g.backspace();
        assert_eq!(g.submit(TZ), None);
        assert_eq!(g.error, Some(Field::FromDate));
        assert_eq!(parse("2026-10-20", "1430", TZ), parse("2026-10-20", "14:30", TZ));
        assert_eq!(parse("2026-10-20", "", 0), Some(1_792_454_400));
        assert_eq!(index_at_time(&[], 0), None);
    }

    #[test]
    fn collee_au_bouton_calendrier() {
        let v = Viewport {
            footer: crate::bottombar::HEIGHT,
            ..vp()
        };
        let icon = crate::bottombar::goto_rect(&v).unwrap();
        let p = layout(&v, Tab::Date).panel;
        assert_eq!((p.x, p.y + p.h), (icon.x, icon.y), "coin bas-gauche de la fenêtre = coin haut-gauche du 📅");
        let p = layout(&v, Tab::Range).panel;
        assert_eq!(p.y + p.h, icon.y, "plage : grandit vers le haut, toujours collée");
    }

    #[test]
    fn dessin() {
        let v = vp();
        let mut g = GoTo::open(LAST, LAST, TZ);
        let mut pm = crate::px::new(1400, 900).unwrap();
        draw(&mut pm, &Text::default(), &v, &g);
        let l = layout(&v, Tab::Date);
        let sel = l.cell(3 + 24); // le 25, choisi : fond blanc (a round patch: looked at beside its number)
        let px = pm.pixel((sel.cx() - sel.w.min(sel.h) * 0.3) as u32, sel.cy() as u32).unwrap();
        assert_eq!((px.red(), px.green(), px.blue()), (WHITE.0, WHITE.1, WHITE.2));
        g.tab = Tab::Range;
        let mut pm2 = crate::px::new(1400, 900).unwrap();
        draw(&mut pm2, &Text::default(), &v, &g);
        assert_ne!(pm.data(), pm2.data(), "deux lignes de champs en plage");
    }
}
