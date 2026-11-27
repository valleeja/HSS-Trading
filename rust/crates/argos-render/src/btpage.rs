//! 📊 The full report of a backtest (lot 6b): a full-screen page of the ARGOS menu, like the mock-up's Backtest page
//! and like 💼 Portfolio. Its numbers, the curve of the account with its drop from the highest point, then the tabs
//! 📋 Trades (the list as TradingView's « List of trades ») and 📅 Par mois; ⬇️ Export CSV in TradingView's columns.
//! ⬆ Comparer avec TradingView: TradingView's own list (its CSV export) beside the backtest's, trade by trade, in a
//! ⚖️ TradingView tab (each ✗ says what differs). A simulation, not financial advice.

use crate::dock::BtView;
use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::portfolio::TEAL;
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Rgb, Text};
use argos_core::{Money, Side};
use argos_script::lang::comparer::{Comparaison, Ecart, Rangee};
use argos_script::lang::rapport::{baisse_max_pct, baisses, par_mois, Ligne};
use tiny_skia::Pixmap;

/// What a click asks of the page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BtClick {
    None,
    /// Redraw (a tab).
    Changed,
    Close,
    /// ⬇️ Export CSV.
    Csv,
    /// ⬆ Comparer avec TradingView: the app asks for TradingView's file.
    Tv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hit {
    Close,
    Csv,
    Tv,
    Tab(usize),
}

const TABS: [&str; 3] = ["📋 Trades", "📅 Par mois", "⚖️ TradingView"];
/// The ⚖️ TradingView tab (there once a file is read).
const TAB_TV: usize = 2;
const BOUTON_TV: &str = "⬆ Comparer avec TradingView";
/// 📱 The same button on a narrow screen.
const BOUTON_TV_COURT: &str = "⬆ TradingView";
/// The ⚖️ tab's line of pills above its table, and its note under it.
const PILULES: f64 = 44.0;
const NOTE: f64 = 26.0;
/// Below this useful width, the blocks are stacked and the table has fewer columns (phone, small window).
const NARROW: f64 = 900.0;
/// Height of a table row.
const ROW: f64 = 28.0;
/// 📱 Room left under the table on a phone, for the Argos AI round that stays over every page (bottom right).
const BAS_ROND: f64 = 76.0;
pub(crate) const MONTHS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

/// ⚖️ TradingView's list read and compared with the backtest.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VueTv {
    /// The file's name, as chosen.
    pub fichier: String,
    pub c: Comparaison,
}

/// The page's state.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct BtPage {
    pub tab: usize,
    /// 📱 Scroll of the page (points): a narrow screen stacks the blocks.
    pub scroll: f64,
    /// First row of the table shown (the newest first).
    pub rang: usize,
    /// ⚖️ The comparison with TradingView, once its file is read (its tab comes with it).
    pub tv: Option<VueTv>,
    /// The chart's time zone by name (« America/New_York »): each date with its own summer time, like TradingView's
    /// list. Empty: the `tz` given to [`BtPage::draw`] for every date.
    pub fuseau: String,
}

/// Where the blocks are.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Geo {
    col1: Rect,
    col2: Rect,
    col3: Rect,
    table: Rect,
}

fn usd(v: f64) -> String {
    crate::viewport::format_money(v, 2)
}

fn signe(v: f64) -> Rgb {
    if v >= 0.0 {
        pal::up()
    } else {
        pal::down()
    }
}

/// « 21 nov. 2026 14:30 » in the chart's time zone (`tz`: seconds from UTC); `court`: without the year.
fn quand(t: i64, tz: i64, court: bool) -> String {
    use chrono::{Datelike, Timelike};
    let d = chrono::DateTime::from_timestamp(t + tz, 0).unwrap_or_default();
    let an = if court { String::new() } else { format!(" {}", d.year()) };
    format!("{} {}{an} {:02}:{:02}", d.day(), MONTHS[d.month0() as usize], d.hour(), d.minute())
}

/// A quantity as written in the list: whole, or up to 4 decimals without the trailing zeros.
fn quantite(q: f64) -> String {
    if q.fract() == 0.0 {
        return crate::viewport::format_number(q, 0);
    }
    let s = crate::viewport::format_number(q, 4);
    s.trim_end_matches('0').trim_end_matches([',', '.']).to_string()
}

/// `s` cut with « … » to fit `w` points at `size`.
pub(crate) fn couper(text: &Text, s: &str, size: f32, w: f64) -> String {
    let mut s = s.to_string();
    while text.width(&s, size) as f64 > w && s.chars().count() > 2 {
        s.pop();
        s.pop();
        s.push('…');
    }
    s
}

impl BtPage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Seconds from UTC at `t` in the chart's time zone (`tz`: its offset now, when the page has no zone's name).
    fn decalage(&self, t: i64, tz: i64) -> i64 {
        if self.fuseau.is_empty() {
            return tz;
        }
        crate::settings::tz_offset(&self.fuseau, t).unwrap_or(tz)
    }

    /// ⚖️ TradingView's list compared: its tab opens at its newest row.
    pub fn montrer_tv(&mut self, v: VueTv) {
        (self.tv, self.tab, self.rang) = (Some(v), TAB_TV, 0);
    }

    /// The tabs there are: ⚖️ TradingView only once a file is read.
    fn onglets(&self) -> &'static [&'static str] {
        if self.tv.is_some() {
            &TABS
        } else {
            &TABS[..2]
        }
    }

    /// Room the open tab takes above and under its rows (its pills and its note on ⚖️).
    fn extra(&self) -> f64 {
        if self.tab == TAB_TV {
            PILULES + NOTE
        } else {
            0.0
        }
    }

    /// Where everything is, in a page `w` × `h`.
    fn layout(&self, text: &Text, w: f64, h: f64) -> (Vec<(Rect, Hit)>, Geo) {
        let inner = (w - 40.0).min(1400.0);
        let x0 = (w - inner) / 2.0;
        let narrow = inner < NARROW;
        let csv = Rect::new(x0, 64.0, text.width("⬇️ Export CSV", 13.5) as f64 + 28.0, 34.0);
        let tv = if narrow { BOUTON_TV_COURT } else { BOUTON_TV };
        let mut z = vec![
            (Rect::new(x0 + inner - 36.0, 16.0, 32.0, 32.0), Hit::Close),
            (csv, Hit::Csv),
            (Rect::new(csv.x + csv.w + 12.0, 64.0, text.width(tv, 13.5) as f64 + 32.0, 34.0), Hit::Tv),
        ];
        let top = 116.0;
        let (col1, col2, col3) = if narrow {
            (
                Rect::new(x0, top, inner, 300.0),
                Rect::new(x0, top + 312.0, inner, 300.0),
                Rect::new(x0, top + 624.0, inner, 300.0),
            )
        } else {
            (
                Rect::new(x0, top, 240.0, 300.0),
                Rect::new(x0 + 252.0, top, 320.0, 300.0),
                Rect::new(x0 + 584.0, top, inner - 584.0, 300.0),
            )
        };
        let top = if narrow { top + 624.0 } else { top };
        let table = if narrow {
            Rect::new(x0, top + 312.0, inner, 460.0)
        } else {
            Rect::new(x0, top + 312.0, inner, (h - top - 312.0 - 20.0).max(220.0))
        };
        let mut tx = table.x + 16.0;
        for (k, name) in self.onglets().iter().enumerate() {
            let tw = text.width(name, 14.0) as f64;
            z.push((Rect::new(tx, table.y + 6.0, tw, 34.0), Hit::Tab(k)));
            tx += tw + 22.0;
        }
        (z, Geo { col1, col2, col3, table })
    }

    /// Height of the whole content (more than the page when the blocks are stacked).
    fn content_h(&self, text: &Text, w: f64, h: f64) -> f64 {
        let (_, g) = self.layout(text, w, h);
        let rond = if w - 40.0 < NARROW { BAS_ROND } else { 0.0 };
        (g.table.y + g.table.h + 20.0 + rond).max(h)
    }

    /// Rows of the table that fit.
    fn visibles(&self, text: &Text, w: f64, h: f64) -> usize {
        let (_, g) = self.layout(text, w, self.content_h(text, w, h));
        ((g.table.h - 90.0 - self.extra()) / ROW).max(0.0) as usize
    }

    /// Wheel / finger (`dy` > 0: down): the page scrolls to its bottom first, then the table's `n` rows; back up, the
    /// rows first.
    pub fn wheel(&mut self, dy: f64, text: &Text, w: f64, h: f64, n: usize) {
        let max = (self.content_h(text, w, h) - h).max(0.0);
        let pas = ((dy.abs() / ROW).round() as usize).max(1);
        let fin = n.saturating_sub(self.visibles(text, w, h));
        if dy > 0.0 {
            if self.scroll < max {
                self.scroll = (self.scroll + dy).min(max);
            } else {
                self.rang = (self.rang + pas).min(fin);
            }
        } else if dy < 0.0 {
            if self.rang > 0 {
                self.rang = self.rang.saturating_sub(pas);
            } else {
                self.scroll = (self.scroll + dy).max(0.0);
            }
        }
    }

    /// Rows of the open tab (for the wheel).
    pub fn lignes(&self, v: &BtView, tz: i64) -> usize {
        match self.tab {
            0 => v.detail.lignes.len(),
            TAB_TV => self.tv.as_ref().map_or(0, |t| t.c.rangees.len()),
            _ => par_mois(&v.detail.lignes, v.start, tz).len(),
        }
    }

    pub fn click(&mut self, text: &Text, w: f64, h: f64, x: f64, y: f64) -> BtClick {
        let ch = self.content_h(text, w, h);
        let (z, _) = self.layout(text, w, ch);
        match z.into_iter().find(|(r, _)| r.contains(x, y + self.scroll)).map(|(_, h)| h) {
            Some(Hit::Close) => BtClick::Close,
            Some(Hit::Csv) => BtClick::Csv,
            Some(Hit::Tv) => BtClick::Tv,
            Some(Hit::Tab(k)) => {
                (self.tab, self.rang) = (k, 0);
                BtClick::Changed
            }
            None => BtClick::None,
        }
    }

    /// Draws the page (the whole window `w` × `h`); `tz`: the chart's time zone (seconds from UTC).
    pub fn draw(&self, text: &Text, w: u32, h: u32, v: &BtView, tz: i64, mouse: (f64, f64)) -> Option<Pixmap> {
        let ch = self.content_h(text, w as f64, h as f64).ceil() as u32;
        let full = self.draw_content(text, w, ch, v, tz, (mouse.0, mouse.1 + self.scroll))?;
        if ch == h {
            return Some(full);
        }
        let mut pm = crate::px::new(w, h)?;
        let row = full.width() as usize;
        let top = ((self.scroll * crate::px::scale() as f64).round() as usize).min((full.height() as usize).saturating_sub(pm.height() as usize));
        let n = pm.height() as usize * row;
        pm.pixels_mut().copy_from_slice(&full.pixels()[top * row..top * row + n]);
        Some(pm)
    }

    fn draw_content(&self, text: &Text, w: u32, h: u32, v: &BtView, tz: i64, mouse: (f64, f64)) -> Option<Pixmap> {
        let mut pm = crate::px::new(w, h)?;
        let (wf, hf) = (w as f64, h as f64);
        crate::fill(&mut pm, 0.0, 0.0, wf, hf, pal::backdrop(), 1.0);
        let (z, geo) = self.layout(text, wf, hf);
        let x0 = geo.col1.x;
        text.draw(&mut pm, "Backtest", (x0 as f32, 44.0), 26.0, WHITE, Align::Left);
        let pill = format!("{} · {} · {} bougies", v.name, v.symbol, crate::viewport::format_number(v.bars as f64, 0));
        let px = x0 + text.width("Backtest", 26.0) as f64 + 14.0;
        // (up to the ✕)
        let pill = couper(text, &pill, 12.5, geo.table.x + geo.table.w - 48.0 - px - 18.0);
        let pw = text.width(&pill, 12.5) as f64 + 18.0;
        round_fill(&mut pm, Rect::new(px, 24.0, pw, 24.0), 6.0, TEAL, 0.14);
        text.draw(&mut pm, &pill, ((px + 9.0) as f32, 41.0), 12.5, TEAL, Align::Left);
        let card = |pm: &mut Pixmap, r: Rect| {
            round_fill(pm, r, 10.0, pal::panel(), 1.0);
            round_stroke(pm, r, 10.0, pal::edge(), 1.0);
        };
        let c1a = Rect::new(geo.col1.x, geo.col1.y, geo.col1.w, 140.0);
        let c1b = Rect::new(geo.col1.x, geo.col1.y + 152.0, geo.col1.w, 148.0);
        for r in [c1a, c1b, geo.col2, geo.col3, geo.table] {
            card(&mut pm, r);
        }
        let r = &v.report;
        let net = r.net_pnl.to_f64();
        let small = |pm: &mut Pixmap, s: &str, x: f64, y: f64| text.draw(pm, s, (x as f32, y as f32), 13.0, GREY, Align::Left);
        small(&mut pm, "Résultat net", c1a.x + 16.0, c1a.y + 28.0);
        text.draw(
            &mut pm,
            &usd(net),
            ((c1a.x + 16.0) as f32, (c1a.y + 66.0) as f32),
            24.0,
            signe(net),
            Align::Left,
        );
        small(&mut pm, &format!("{:+.2} % du capital", net / v.start * 100.0), c1a.x + 16.0, c1a.y + 92.0);
        small(&mut pm, "Capital de départ", c1b.x + 16.0, c1b.y + 28.0);
        text.draw(&mut pm, &usd(v.start), ((c1b.x + 16.0) as f32, (c1b.y + 66.0) as f32), 24.0, WHITE, Align::Left);
        small(&mut pm, &format!("à la fin : {}", usd(r.end_balance.to_f64())), c1b.x + 16.0, c1b.y + 92.0);
        // the summary
        let l = &v.detail.lignes;
        let gains: Vec<f64> = l.iter().map(|x| x.net.to_f64()).filter(|x| *x > 0.0).collect();
        let pertes: Vec<f64> = l.iter().map(|x| x.net.to_f64()).filter(|x| *x < 0.0).collect();
        let moyenne = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
        let longs = l.iter().filter(|x| x.side == Side::Long).count();
        let baisse = if v.detail.equite.is_empty() {
            r.max_drawdown_pct
        } else {
            baisse_max_pct(&v.detail.equite)
        };
        let frais: Money = r.trades.iter().map(|t| t.fees).sum();
        let mut rows: Vec<(&str, String, Rgb)> = vec![
            ("Trades", format!("{} ({longs} L · {} S)", l.len(), l.len() - longs), WHITE),
            (
                "% gagnants",
                format!("{:.1} %", r.win_rate),
                if r.win_rate >= 50.0 { pal::up() } else { pal::down() },
            ),
            (
                "Profit factor",
                if r.profit_factor.is_finite() {
                    format!("{:.2}", r.profit_factor)
                } else {
                    "∞".into()
                },
                if r.profit_factor >= 1.0 { pal::up() } else { pal::down() },
            ),
            ("Baisse max", format!("−{baisse:.2} %"), pal::down()),
            ("Gain moyen", usd(moyenne(&gains)), pal::up()),
            ("Perte moyenne", usd(moyenne(&pertes)), pal::down()),
            ("Frais payés", usd(frais.to_f64()), WHITE),
        ];
        if let Some(bh) = v.buy_hold_pct {
            rows.push(("Buy & hold", format!("{bh:+.2} %"), signe(bh)));
        }
        if let Some(hz) = &v.hasard {
            rows.push((
                "Contre le hasard",
                format!("mieux que {:.0} %", hz.battus_pct),
                if hz.battus_pct >= 50.0 { pal::up() } else { pal::down() },
            ));
        }
        for (k, (name, val, c)) in rows.iter().enumerate() {
            let y = geo.col2.y + 34.0 + k as f64 * 29.0;
            text.draw(&mut pm, name, ((geo.col2.x + 16.0) as f32, y as f32), 13.5, GREY, Align::Left);
            text.draw(&mut pm, val, ((geo.col2.x + geo.col2.w - 16.0) as f32, y as f32), 13.5, *c, Align::Right);
        }
        self.draw_courbe(&mut pm, text, geo.col3, v);
        self.draw_table(&mut pm, text, geo.table, v, tz, wf - 40.0 < NARROW);
        // buttons and tabs
        for (r, hit) in &z {
            match *hit {
                Hit::Close => lines(&mut pm, r.cx(), r.cy(), &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]], GREY, 1.8),
                Hit::Csv => {
                    let on = r.contains(mouse.0, mouse.1);
                    round_fill(&mut pm, *r, 8.0, TEAL, if on { 0.85 } else { 1.0 });
                    text.draw(
                        &mut pm,
                        "⬇️ Export CSV",
                        (r.cx() as f32, (r.cy() + 5.0) as f32),
                        13.5,
                        Rgb(0x04, 0x20, 0x1c),
                        Align::Center,
                    );
                }
                Hit::Tv => {
                    // (glass, beside the teal ⬇️ Export CSV: the second action)
                    let on = r.contains(mouse.0, mouse.1);
                    round_fill(&mut pm, *r, 17.0, pal::panel(), if on { 1.0 } else { 0.92 });
                    round_stroke(&mut pm, *r, 17.0, if on { TEAL } else { pal::edge() }, 1.0);
                    let s = if wf - 40.0 < NARROW { BOUTON_TV_COURT } else { BOUTON_TV };
                    text.draw(&mut pm, s, (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, WHITE, Align::Center);
                }
                Hit::Tab(k) => {
                    let on = k == self.tab;
                    text.draw(
                        &mut pm,
                        TABS[k],
                        (r.x as f32, (r.cy() + 5.0) as f32),
                        14.0,
                        if on { WHITE } else { GREY },
                        Align::Left,
                    );
                    if on {
                        crate::fill(&mut pm, r.x, r.y + r.h - 1.0, r.w, 2.0, TEAL, 1.0);
                    }
                }
            }
        }
        Some(pm)
    }

    /// The account at each bar (line) and, under it, its drop from the highest point (red).
    fn draw_courbe(&self, pm: &mut Pixmap, text: &Text, c: Rect, v: &BtView) {
        text.draw(pm, "Courbe du compte", ((c.x + 16.0) as f32, (c.y + 28.0) as f32), 13.5, WHITE, Align::Left);
        let legende = "baisse depuis le plus haut";
        text.draw(pm, legende, ((c.x + c.w - 16.0) as f32, (c.y + 28.0) as f32), 12.0, pal::down(), Align::Right);
        crate::fill(pm, c.x + 16.0, c.y + 40.0, c.w - 32.0, 1.0, pal::edge(), 1.0);
        let e = &v.detail.equite;
        if e.len() < 2 {
            text.draw(
                pm,
                "Pas de courbe : lance le backtest.",
                ((c.x + 16.0) as f32, (c.y + 80.0) as f32),
                13.0,
                GREY,
                Align::Left,
            );
            return;
        }
        let haut = Rect::new(c.x + 16.0, c.y + 52.0, c.w - 80.0, (c.h - 72.0) * 0.72);
        let bas = Rect::new(haut.x, haut.y + haut.h + 8.0, haut.w, c.h - 72.0 - haut.h - 8.0);
        // (at most one point a pixel: the highest and lowest of each column are kept by the area)
        let n = e.len();
        let x = |i: usize| (haut.x + i as f64 / (n - 1) as f64 * haut.w) as f32;
        let (lo, hi) = e.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(*p), b.max(*p)));
        let (lo, hi) = if hi > lo {
            (lo - (hi - lo) * 0.08, hi + (hi - lo) * 0.08)
        } else {
            (lo - 1.0, hi + 1.0)
        };
        let y = |p: f64| (haut.y + (hi - p) / (hi - lo) * haut.h) as f32;
        let pas = (n / (haut.w.max(1.0) as usize * 2)).max(1);
        let ligne: Vec<(f32, f32)> = (0..n).step_by(pas).chain([n - 1]).map(|i| (x(i), y(e[i]))).collect();
        let couleur = signe(e[n - 1] - e[0]);
        let mut aire = ligne.clone();
        aire.push((x(n - 1), (haut.y + haut.h) as f32));
        aire.push((x(0), (haut.y + haut.h) as f32));
        poly_fill(pm, 0.0, 0.0, &aire, couleur, 0.16);
        lines(pm, 0.0, 0.0, &[&ligne], couleur, 1.8);
        // the start: a dotted line at the capital
        let yc = y(v.start) as f64;
        if (haut.y..=haut.y + haut.h).contains(&yc) {
            let mut xx = haut.x;
            while xx < haut.x + haut.w {
                crate::fill(pm, xx, yc, 4.0, 1.0, GREY, 0.6);
                xx += 8.0;
            }
        }
        for (k, val) in [hi, (hi + lo) / 2.0, lo].iter().enumerate() {
            text.draw(
                pm,
                &crate::viewport::format_number(*val, 0),
                ((haut.x + haut.w + 56.0) as f32, (haut.y + k as f64 * haut.h / 2.0 + 4.0) as f32),
                11.0,
                GREY,
                Align::Right,
            );
        }
        // the drop, hanging from the top of its band
        let b = baisses(e);
        let pire = b.iter().fold(0.0f64, |a, x| a.max(*x));
        if pire > 0.0 {
            let yb = |d: f64| (bas.y + d / pire * bas.h) as f32;
            let mut aire: Vec<(f32, f32)> = vec![(x(0), bas.y as f32)];
            aire.extend((0..n).step_by(pas).chain([n - 1]).map(|i| (x(i), yb(b[i]))));
            aire.push((x(n - 1), bas.y as f32));
            poly_fill(pm, 0.0, 0.0, &aire, pal::down(), 0.35);
            text.draw(
                pm,
                &format!("−{}", crate::viewport::format_number(pire, 0)),
                ((bas.x + bas.w + 56.0) as f32, (bas.y + bas.h) as f32),
                11.0,
                pal::down(),
                Align::Right,
            );
        }
        crate::fill(pm, bas.x, bas.y, bas.w, 1.0, pal::edge(), 1.0);
    }

    /// The open tab's table, the newest first from row `rang`.
    fn draw_table(&self, pm: &mut Pixmap, text: &Text, t: Rect, v: &BtView, tz: i64, etroit: bool) {
        crate::fill(pm, t.x + 16.0, t.y + 41.0, t.w - 32.0, 1.0, pal::edge(), 1.0);
        let (tetes, rangs, vide) = self.rangs(v, tz, etroit);
        // ⚖️ its pills (how many the same, how many not), its file, its note: the rows start lower
        let haut = if self.tab == TAB_TV {
            self.draw_pilules(pm, text, t, etroit);
            PILULES
        } else {
            0.0
        };
        // each column as wide as its weight says (dates and prices wider)
        let poids: Vec<f64> = tetes
            .iter()
            .map(|h| match *h {
                "#" => 0.45,
                // (📱 ⚖️: room for the result and the side, the rest for what differs)
                "Sens" if etroit && self.tab == TAB_TV => 0.9,
                "Résultat" if etroit && self.tab == TAB_TV => 1.7,
                "%" if etroit => 1.0,
                "Sens" | "%" | "Trades" | "Gagnants" => 0.75,
                "Signaux" | "Entrée" | "Sortie" if !etroit => 1.45,
                "Sortie" => 1.6,
                "Prix" => 1.6,
                // (⚖️ the ✓ or what differs)
                "" if etroit => 3.4,
                "" => 2.3,
                _ => 1.0,
            })
            .collect();
        let somme: f64 = poids.iter().sum::<f64>().max(1.0);
        let mut xs = vec![t.x + 16.0];
        for p in &poids {
            xs.push(xs[xs.len() - 1] + p / somme * (t.w - 32.0));
        }
        for (k, h) in tetes.iter().enumerate() {
            text.draw(pm, h, (xs[k] as f32, (t.y + 70.0 + haut) as f32), 12.5, GREY, Align::Left);
        }
        if rangs.is_empty() {
            text.draw(pm, vide, ((t.x + 16.0) as f32, (t.y + 104.0 + haut) as f32), 13.5, GREY, Align::Left);
        }
        let place = ((t.h - 90.0 - self.extra()) / ROW).max(0.0) as usize;
        let debut = self.rang.min(rangs.len().saturating_sub(place));
        let rouges = self.rouges();
        for (r, row) in rangs.iter().skip(debut).take(place).enumerate() {
            let y = t.y + 100.0 + haut + r as f64 * ROW;
            if rouges.get(debut + r).copied().unwrap_or(false) {
                crate::fill(pm, t.x + 8.0, y - 18.0, t.w - 16.0, ROW - 2.0, pal::down(), 0.08);
            }
            for (k, (cell, c)) in row.iter().enumerate() {
                // (a cell never runs over the next one)
                let s = couper(text, cell, 13.0, xs[k + 1] - xs[k] - 8.0);
                text.draw(pm, &s, (xs[k] as f32, y as f32), 13.0, *c, Align::Left);
            }
        }
        if rangs.len() > place && place > 0 {
            let fin = (debut + place).min(rangs.len());
            // (📋 Trades and ⚖️: the numbers of their # column, newest first, not the rows' places: « #23 à #11 sur 218 »)
            let numero = |k: usize| rangs[k][0].0.parse::<usize>().ok();
            let vus = if self.tab != 1 && numero(debut).is_some() && numero(fin - 1).is_some() {
                format!("#{} à #{}", rangs[debut][0].0, rangs[fin - 1][0].0)
            } else {
                format!("{}–{}", debut + 1, fin)
            };
            let s = if etroit {
                format!("{vus} sur {}", rangs.len())
            } else {
                format!("{vus} sur {} · molette ou doigt pour défiler", rangs.len())
            };
            // (⚖️: on its pills' line, its file's name is on the tabs' line; 📱 under the table, the pills fill their line)
            let y = match (self.tab, etroit) {
                (TAB_TV, true) => t.y + t.h - 12.0,
                (TAB_TV, false) => t.y + 41.0 + 27.0,
                _ => t.y + 26.0,
            };
            text.draw(pm, &s, ((t.x + t.w - 16.0) as f32, y as f32), 11.5, GREY, Align::Right);
        }
    }

    /// ⚖️ The rows that differ (in the table's order, newest first): drawn on a red background.
    fn rouges(&self) -> Vec<bool> {
        match (&self.tv, self.tab) {
            (Some(tv), TAB_TV) => tv.c.rangees.iter().rev().map(|r| !r.pareil()).collect(),
            _ => vec![],
        }
    }

    /// ⚖️ The line above the table: « ✓ n trades pareils », « ✗ n différents », on how many and to what precision; the
    /// file's name on the tabs' line; under the table, what a ✗ means.
    fn draw_pilules(&self, pm: &mut Pixmap, text: &Text, t: Rect, etroit: bool) {
        let Some(tv) = &self.tv else { return };
        let c = &tv.c;
        let fichier = couper(text, &format!("fichier : {}", tv.fichier), 12.5, (t.w * 0.4).max(120.0));
        if !etroit {
            text.draw(pm, &fichier, ((t.x + t.w - 16.0) as f32, (t.y + 26.0) as f32), 12.5, GREY, Align::Right);
        }
        let y = t.y + 41.0 + 10.0;
        let mut x = t.x + 16.0;
        let pluriel = |n: usize, un: &str, plusieurs: &str| if n > 1 { plusieurs.to_string() } else { un.to_string() };
        let pilules = [
            (format!("✓ {} {}", c.pareils, pluriel(c.pareils, "trade pareil", "trades pareils")), pal::up()),
            (format!("✗ {} {}", c.differents, pluriel(c.differents, "différent", "différents")), pal::down()),
        ];
        for (s, coul) in pilules.iter().filter(|(s, _)| !s.starts_with("✗ 0 ")) {
            let w = text.width(s, 13.5) as f64 + 24.0;
            let r = Rect::new(x, y, w, 26.0);
            round_fill(pm, r, 13.0, *coul, 0.13);
            round_stroke(pm, r, 13.0, *coul, 0.45);
            text.draw(pm, s, (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, *coul, Align::Center);
            x += w + 10.0;
        }
        let total = c.rangees.len();
        let mut reste = format!("sur {total}");
        if !etroit {
            reste.push_str(&format!(
                " · même entrée, même sortie, même résultat à {} près",
                usd(argos_script::lang::comparer::RESULTAT_PRES)
            ));
            if c.hors_periode > 0 {
                reste.push_str(&format!(" · {} hors de la période commune", c.hors_periode));
            }
            if c.ouverts_tv > 0 {
                reste.push_str(&format!(" · {} encore ouvert chez TV", c.ouverts_tv));
            }
        }
        let reste = couper(text, &reste, 13.5, t.x + t.w - 16.0 - x - if etroit { 0.0 } else { 260.0 });
        text.draw(pm, &reste, ((x + 4.0) as f32, (y + 18.0) as f32), 13.5, GREY, Align::Left);
        if c.differents > 0 {
            let note = if etroit {
                "Le ✗ dit quoi regarder."
            } else {
                "Le ✗ dit quoi regarder : souvent le stop et l'objectif dans la même bougie, ou une bougie qui manque d'un côté."
            };
            text.draw(pm, note, ((t.x + 16.0) as f32, (t.y + t.h - 12.0) as f32), 13.0, GREY, Align::Left);
        }
    }

    /// The open tab's table: (headers, rows of (text, colour), the message when empty).
    fn rangs(&self, v: &BtView, tz: i64, etroit: bool) -> (Vec<&'static str>, Vec<Vec<(String, Rgb)>>, &'static str) {
        // (the symbol's own decimals: « 63 705,6 », not « 63 705,6000 »)
        let prix = |p: f64| crate::viewport::format_price(p, v.detail.dec);
        match self.tab {
            TAB_TV => self.rangs_tv(v, tz, etroit),
            0 => {
                let ligne = |l: &Ligne| -> Vec<(String, Rgb)> {
                    let long = l.side == Side::Long;
                    let sens = (if long { "Long" } else { "Short" }.to_string(), if long { pal::up() } else { pal::down() });
                    let net = l.net.to_f64();
                    if etroit {
                        return vec![
                            (l.numero.to_string(), GREY),
                            sens,
                            (quand(l.t_out, self.decalage(l.t_out, tz), true), WHITE),
                            (usd(net), signe(net)),
                            (format!("{:+.2} %", l.net_pct), signe(net)),
                        ];
                    }
                    vec![
                        (l.numero.to_string(), GREY),
                        sens,
                        (format!("{} → {}", l.entree, l.sortie), WHITE),
                        (quand(l.t_in, self.decalage(l.t_in, tz), false), WHITE),
                        (quand(l.t_out, self.decalage(l.t_out, tz), false), WHITE),
                        (format!("{} → {}", prix(l.prix_in), prix(l.prix_out)), WHITE),
                        (quantite(l.qty), WHITE),
                        (usd(net), signe(net)),
                        (format!("{:+.2} %", l.net_pct), signe(net)),
                        (usd(l.run_up), pal::up()),
                        (usd(-l.drawdown), pal::down()),
                    ]
                };
                (
                    if etroit {
                        vec!["#", "Sens", "Sortie", "Résultat", "%"]
                    } else {
                        vec![
                            "#",
                            "Sens",
                            "Signaux",
                            "Entrée",
                            "Sortie",
                            "Prix",
                            "Quantité",
                            "Résultat",
                            "%",
                            "Run-up",
                            "Drawdown",
                        ]
                    },
                    v.detail.lignes.iter().rev().map(ligne).collect(),
                    "Aucun trade fermé.",
                )
            }
            _ => (
                vec!["Mois", "Trades", "Gagnants", "Résultat", "% du compte"],
                par_mois(&v.detail.lignes, v.start, tz)
                    .iter()
                    .rev()
                    .map(|m| {
                        let net = m.net.to_f64();
                        vec![
                            (format!("{} {}", MONTHS[(m.mois as usize).clamp(1, 12) - 1], m.annee), WHITE),
                            (m.trades.to_string(), WHITE),
                            (m.gagnants.to_string(), WHITE),
                            (usd(net), signe(net)),
                            (format!("{:+.2} %", m.pct), signe(net)),
                        ]
                    })
                    .collect(),
                "Aucun mois avec un trade fermé.",
            ),
        }
    }
}

/// « 14:30 » in the chart's time zone.
fn heure(t: i64, tz: i64) -> String {
    use chrono::Timelike;
    let d = chrono::DateTime::from_timestamp(t + tz, 0).unwrap_or_default();
    format!("{:02}:{:02}", d.hour(), d.minute())
}

fn meme_jour(a: i64, b: i64, tz: i64) -> bool {
    (a + tz).div_euclid(86_400) == (b + tz).div_euclid(86_400)
}

/// ⚖️ What differs, in words (« sortie 1 bougie plus tôt chez TV »); `dec`: the symbol's price decimals.
pub(crate) fn raison(e: &Ecart, dec: usize, etroit: bool) -> String {
    // (📱 shorter: the table says it is TradingView's list)
    let tv = if etroit { "" } else { " chez TV" };
    let bougies = |k: i64| format!("{} bougie{}", k.abs(), if k.abs() > 1 { "s" } else { "" });
    let sens = |k: i64| if k < 0 { "plus tôt" } else { "plus tard" };
    let ecart_prix = |d: f64| format!("{}{}", if d < 0.0 { "−" } else { "+" }, crate::viewport::format_price(d.abs(), dec));
    match *e {
        Ecart::PasChezTv if etroit => "pas chez TV".into(),
        Ecart::PasChezArgos if etroit => "pas chez Argos".into(),
        Ecart::PasChezTv => "pas dans la liste de TradingView".into(),
        Ecart::PasChezArgos => "pas dans le backtest d'Argos".into(),
        Ecart::Entree(k) => format!("entrée {} {}{tv}", bougies(k), sens(k)),
        Ecart::Sortie(k) => format!("sortie {} {}{tv}", bougies(k), sens(k)),
        Ecart::PrixEntree(d) => format!("prix d'entrée {}{tv}", ecart_prix(d)),
        Ecart::PrixSortie(d) => format!("prix de sortie {}{tv}", ecart_prix(d)),
        Ecart::Resultat(d) => format!("résultat {}{}{tv}", if d < 0.0 { "" } else { "+" }, usd(d)),
    }
}

impl BtPage {
    /// ⚖️ The comparison's table, newest first: the backtest's entry, exit and result, TradingView's beside each (grey,
    /// red where it differs), then ✓ or what differs.
    fn rangs_tv(&self, v: &BtView, tz: i64, etroit: bool) -> (Vec<&'static str>, Vec<Vec<(String, Rgb)>>, &'static str) {
        let vide = "Aucun trade à comparer : le fichier et la chart ne couvrent pas la même période.";
        let tetes = if etroit {
            vec!["#", "Sens", "Résultat", ""]
        } else {
            vec!["#", "Sens", "Entrée", "TradingView", "Sortie", "TradingView", "Résultat", "TradingView", ""]
        };
        let Some(tv) = &self.tv else { return (tetes, vec![], vide) };
        let dec = v.detail.dec;
        let prix = |p: f64| crate::viewport::format_price(p, dec);
        let ligne = |r: &Rangee| -> Vec<(String, Rgb)> {
            let a = r.argos.and_then(|k| v.detail.lignes.get(k));
            let t = r.tv.as_ref();
            let long = a.map(|a| a.side).or(t.map(|t| t.side)) == Some(Side::Long);
            let sens = (if long { "Long" } else { "Short" }.to_string(), if long { pal::up() } else { pal::down() });
            let numero = match (a, t) {
                (Some(a), _) => a.numero.to_string(),
                (None, Some(t)) if t.numero > 0 => format!("TV {}", t.numero),
                _ => "—".into(),
            };
            let a_ecart = |f: &dyn Fn(&Ecart) -> bool| r.ecarts.iter().any(f);
            let e_in = a_ecart(&|e| matches!(e, Ecart::Entree(_) | Ecart::PrixEntree(_)));
            let e_out = a_ecart(&|e| matches!(e, Ecart::Sortie(_) | Ecart::PrixSortie(_)));
            let e_net = a_ecart(&|e| matches!(e, Ecart::Resultat(_)));
            let verdict = if r.pareil() {
                ("✓".to_string(), pal::up())
            } else {
                // (the first one: the others follow from it, and TradingView's cells say them in red)
                (
                    format!("✗ {}", r.ecarts.first().map(|e| raison(e, dec, etroit)).unwrap_or_default()),
                    pal::down(),
                )
            };
            let net_a = a.map(|a| a.net.to_f64());
            let resultat = net_a.map_or(("—".to_string(), GREY), |n| (usd(n), signe(n)));
            if etroit {
                // (📱 what differs needs the room more than the dates)
                return vec![(numero, GREY), sens, resultat, verdict];
            }
            let ici = |q: i64, p: f64| format!("{} · {}", quand(q, self.decalage(q, tz), true), prix(p));
            // (TradingView's time without its day when it is the backtest's day)
            let la_bas = |q: i64, p: f64, a_q: Option<i64>| {
                let z = self.decalage(q, tz);
                if a_q.is_some_and(|x| meme_jour(x, q, z)) {
                    format!("{} · {}", heure(q, z), prix(p))
                } else {
                    ici(q, p)
                }
            };
            let gris = |rouge: bool| if rouge { pal::down() } else { GREY };
            vec![
                (numero, GREY),
                sens,
                (a.map_or("—".into(), |a| ici(a.t_in, a.prix_in)), WHITE),
                (t.map_or("—".into(), |t| la_bas(t.t_in, t.prix_in, a.map(|a| a.t_in))), gris(e_in)),
                (a.map_or("—".into(), |a| ici(a.t_out, a.prix_out)), WHITE),
                (t.map_or("—".into(), |t| la_bas(t.t_out, t.prix_out, a.map(|a| a.t_out))), gris(e_out)),
                resultat,
                (t.and_then(|t| t.net).map_or("—".into(), usd), gris(e_net)),
                verdict,
            ]
        };
        (tetes, tv.c.rangees.iter().rev().map(ligne).collect(), vide)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock::BtDetail;
    use argos_core::backtest::rapport;
    use argos_core::portfolio::ClosedTrade;

    /// A report of `n` trades (one an hour from 1 Nov 2026), alternately won and lost.
    fn vue(n: usize) -> BtView {
        let t0 = 1_793_491_200; // 2026-11-01 00:00 UTC
        let trades: Vec<ClosedTrade> = (0..n)
            .map(|k| ClosedTrade {
                base: "BTC".into(),
                side: if k % 2 == 0 { Side::Long } else { Side::Short },
                qty: 1.0,
                entry: 100.0,
                exit: if k % 2 == 0 { 110.0 } else { 105.0 },
                pnl: Money::from_f64(if k % 2 == 0 { 10.0 } else { -5.0 }),
                fees: Money::ZERO,
                opened_at: t0 + k as i64 * 3600,
                closed_at: t0 + k as i64 * 3600 + 1800,
                reason: "X".into(),
                risk: Money::ZERO,
                partiel: false,
                glissement: Money::ZERO,
                funding: Money::ZERO,
                levier: None,
                plan: None,
                real_closed_at: None,
            })
            .collect();
        let net: Money = trades.iter().map(|t| t.net()).sum();
        let mut cumul = Money::ZERO;
        let lignes = trades
            .iter()
            .enumerate()
            .map(|(k, t)| {
                cumul += t.net();
                Ligne {
                    numero: k + 1,
                    side: t.side,
                    entree: "Achat".into(),
                    sortie: "X".into(),
                    bar_in: k,
                    bar_out: k,
                    t_in: t.opened_at,
                    t_out: t.closed_at,
                    prix_in: t.entry,
                    prix_out: t.exit,
                    qty: t.qty,
                    net: t.net(),
                    net_pct: t.net().to_f64(),
                    cumul,
                    run_up: 12.0,
                    drawdown: 3.0,
                }
            })
            .collect();
        let equite: Vec<f64> = (0..n).map(|k| 1000.0 + if k % 2 == 0 { 10.0 } else { 5.0 } * k as f64).collect();
        BtView {
            name: "📜 Test".into(),
            settings: String::new(),
            frais: String::new(),
            symbol: "BTC".into(),
            bars: n,
            start: 1000.0,
            report: rapport(1000.0, Money::from_f64(1000.0) + net, trades, &[]),
            buy_hold_pct: Some(3.0),
            hasard: None,
            script: true,
            detail: BtDetail {
                lignes,
                temps: (0..n as i64).map(|k| t0 + k * 3600).collect(),
                equite,
                dec: 2,
                ..Default::default()
            },
            ouverte: None,
        }
    }

    /// The page: its tabs, ⬇️ Export CSV and ✕; 100 trades scroll in the table (newest first), the page draws on a
    /// big screen and on a phone (the blocks stacked, fewer columns, the page scrolls first).
    #[test]
    fn page_du_rapport() {
        let t = Text::default();
        let v = vue(100);
        let mut p = BtPage::new();
        let (z, _) = p.layout(&t, 1600.0, 900.0);
        let at = |h: Hit| z.iter().find(|(_, x)| *x == h).map(|(r, _)| (r.cx(), r.cy())).unwrap();
        let (x, y) = at(Hit::Csv);
        assert_eq!(p.click(&t, 1600.0, 900.0, x, y), BtClick::Csv);
        let (x, y) = at(Hit::Close);
        assert_eq!(p.click(&t, 1600.0, 900.0, x, y), BtClick::Close);
        // the trades, newest first
        let (tetes, rangs, _) = p.rangs(&v, 0, false);
        assert_eq!(tetes.len(), 11);
        assert_eq!(rangs[0][0].0, "100");
        assert_eq!(rangs[99][3].0, "1 nov. 2026 00:00");
        // prices with the symbol's decimals, not 4 for every symbol
        let l = &v.detail.lignes[99];
        let px = |p: f64| crate::viewport::format_price(p, 2);
        assert_eq!(rangs[0][5].0, format!("{} → {}", px(l.prix_in), px(l.prix_out)));
        // its counter says the numbers of the # column (newest first), not the rows' places
        let n = p.visibles(&t, 1600.0, 900.0);
        let _ = crate::text::ecrits();
        assert!(p.draw(&t, 1600, 900, &v, 0, (0.0, 0.0)).is_some());
        let compteur = format!("#100 à #{} sur 100 · molette ou doigt pour défiler", 101 - n);
        assert!(crate::text::ecrits().contains(&compteur), "{compteur}");
        // the wheel: on a big screen the page does not scroll, the rows do
        p.wheel(10_000.0, &t, 1600.0, 900.0, p.lignes(&v, 0));
        assert_eq!(p.scroll, 0.0);
        assert_eq!(p.rang, 100 - p.visibles(&t, 1600.0, 900.0));
        p.wheel(-56.0, &t, 1600.0, 900.0, 100);
        assert_eq!(p.rang, 100 - p.visibles(&t, 1600.0, 900.0) - 2);
        assert!(p.draw(&t, 1600, 900, &v, 0, (0.0, 0.0)).is_some());
        // 📅 by month: one month, 50 won of 100
        let (x, y) = at(Hit::Tab(1));
        assert_eq!(p.click(&t, 1600.0, 900.0, x, y), BtClick::Changed);
        assert_eq!((p.tab, p.rang), (1, 0));
        let (_, mois, _) = p.rangs(&v, 0, false);
        assert_eq!(mois.len(), 1);
        assert_eq!(
            (mois[0][0].0.as_str(), mois[0][1].0.as_str(), mois[0][2].0.as_str()),
            ("nov. 2026", "100", "50")
        );
        // 📱 phone: stacked, 5 columns, the page scrolls before the rows
        let mut c = BtPage::new();
        assert_eq!(c.rangs(&v, 0, true).0.len(), 5);
        c.wheel(400.0, &t, 375.0, 700.0, 100);
        assert!(c.scroll > 0.0 && c.rang == 0);
        // at the bottom, the table ends above the Argos AI round (bottom right of every page)
        c.wheel(10_000.0, &t, 375.0, 700.0, 100);
        let (_, g) = c.layout(&t, 375.0, c.content_h(&t, 375.0, 700.0));
        assert!(g.table.y + g.table.h - c.scroll <= 700.0 - BAS_ROND, "{} {}", g.table.y + g.table.h, c.scroll);
        assert!(c.draw(&t, 375, 700, &v, 0, (0.0, 0.0)).is_some());
        // scrolled with the finger: « #23 à #11 », as the rows it shows (not « 78–90 »)
        c.wheel(10_000.0, &t, 375.0, 700.0, 100);
        let m = c.visibles(&t, 375.0, 700.0);
        let (_, lignes, _) = c.rangs(&v, 0, true);
        let haut = lignes[c.rang][0].0.clone();
        assert_eq!(haut, (100 - c.rang).to_string());
        let _ = crate::text::ecrits();
        assert!(c.draw(&t, 375, 700, &v, 0, (0.0, 0.0)).is_some());
        let compteur = format!("#{haut} à #{} sur 100", 101 - c.rang - m);
        assert!(crate::text::ecrits().contains(&compteur), "{compteur}");
        // no trade: the page still draws, with its message
        let mut vide = vue(0);
        vide.detail.equite.clear();
        assert_eq!(BtPage::new().rangs(&vide, 0, false).2, "Aucun trade fermé.");
        assert!(BtPage::new().draw(&t, 1600, 900, &vide, 0, (0.0, 0.0)).is_some());
    }

    /// ⚖️ TradingView: the page opens on its tab with the file's name, the ✓ / ✗ pills and each ✗ in words (here an
    /// exit a bar earlier at TradingView), on a big screen and on a phone; without a comparison, no third tab.
    #[test]
    fn onglet_tradingview() {
        use argos_script::lang::{comparer, rapport};
        let t = Text::default();
        let v = vue(20);
        let mut p = BtPage::new();
        assert_eq!(p.onglets().len(), 2, "no ⚖️ tab before a file");
        let mut l = v.detail.lignes.clone();
        l[15].t_out -= 3600;
        let liste = comparer::lire_csv(&rapport::csv(&l, v.start, "USD", 0)).unwrap();
        let c = comparer::comparer(&v.detail.lignes, &liste, &v.detail.temps, 3600, &|_| 0, 0.01);
        assert_eq!((c.pareils, c.differents), (19, 1));
        p.montrer_tv(VueTv {
            fichier: "Liste_des_trades.csv".into(),
            c,
        });
        assert_eq!((p.tab, p.onglets().len()), (TAB_TV, 3));
        for (w, h) in [(1600, 900), (402, 874)] {
            let _ = crate::text::ecrits();
            assert!(p.draw(&t, w, h, &v, 0, (0.0, 0.0)).is_some());
            let ecrits = crate::text::ecrits();
            let vu = |s: &str| ecrits.iter().any(|e| e.contains(s));
            // (📱 without « chez TV »: the column is TradingView's)
            let raison = if w > 900 {
                "✗ sortie 1 bougie plus tôt chez TV"
            } else {
                "✗ sortie 1 bougie plus tôt"
            };
            for s in ["⚖️ TradingView", "✓ 19 trades pareils", "✗ 1 différent", raison, "#20 à #"] {
                assert!(vu(s), "« {s} » not drawn at {w} × {h}: {ecrits:?}");
            }
            assert_eq!(vu("Liste_des_trades.csv"), w > 900, "the file's name, on a big screen");
            assert_eq!(vu(BOUTON_TV), w > 900);
        }
    }

    /// The dates in the chart's zone with their own summer time (TradingView's list): 1 Nov 2026, New York leaves
    /// summer time at 06:00 UTC; without the zone's name, today's offset for every date.
    #[test]
    fn dates_a_l_heure_d_ete() {
        let v = vue(10);
        let mut p = BtPage::new();
        let (_, rangs, _) = p.rangs(&v, -5 * 3600, false);
        assert_eq!(rangs[9][3].0, "31 oct. 2026 19:00");
        p.fuseau = "America/New_York".into();
        let (_, rangs, _) = p.rangs(&v, -5 * 3600, false);
        assert_eq!((rangs[9][3].0.as_str(), rangs[0][3].0.as_str()), ("31 oct. 2026 20:00", "1 nov. 2026 04:00"));
    }
}
