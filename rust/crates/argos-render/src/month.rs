//! 📅 The « Rapport du mois » page (lot 8 of the Simulator, mock-up v1 approved by Jee on 21 Nov 2026): a glass panel
//! over the chart (🖥 beside the Simulator panel; 📱 the whole screen, scrolling), opened by « 📅 Rapport du mois »
//! under the Simulator's statistics. The month with ‹ ›, « 📄 Imprimer en PDF », four tiles (net result, R, win rate,
//! plans followed), the account curve day by day, « Argos AI · ton mois », the best and the worst trade, then the rest
//! folded under « Tous les détails ». The window computes the figures (`argos_core::month`, `argos_ai::month_review`)
//! into a [`MonthView`]; here: the layout, the drawing, the clicks and the printable page. Never an order.

use crate::avis::{couper, Partie, Ton};
use crate::palette::WHITE;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::simulateur::argent;
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::Money;
use tiny_skia::Pixmap;

/// 🖥 The panel's width on PC (mock-up: 960).
pub const PANEL_W: f64 = 960.0;
/// Below this width: the phone's layout (one column; the month and « Imprimer en PDF » under the title).
const NARROW: f64 = 700.0;
const RADIUS: f64 = 26.0;
/// « Meilleur trade » / « Pire trade »: its label (14, 6 above the row) and its row (two lines, 9 above and under).
const EXTREME: f64 = 70.0;
/// ⌃ is drawn higher than ⌄ in its line: lowered by this much to the place of ⌄.
const CHEVRON_UP_DROP: f64 = 6.0;

// — the mock-up's colours (rapport.css, carte.css) —
const PANEL: (Rgb, f32) = (Rgb(16, 22, 46), 0.74);
const SHEET: (Rgb, f32) = (Rgb(12, 17, 36), 0.8);
const EDGE: (Rgb, f32) = (WHITE, 0.15);
const TITLE: Rgb = Rgb(0xe8, 0xec, 0xf7);
const SOFT: Rgb = Rgb(0x93, 0xa0, 0xc2);
const DIM: Rgb = Rgb(0x8a, 0x93, 0xad);
const BODY: Rgb = Rgb(0xc9, 0xd3, 0xee);
const ROW: Rgb = Rgb(0xdf, 0xe5, 0xf5);
const UP: Rgb = Rgb(0x3c, 0xcf, 0x8e);
const DOWN: Rgb = Rgb(0xff, 0x5c, 0x66);
const TEAL: Rgb = Rgb(0x4f, 0xd1, 0xc5);
const INK: Rgb = Rgb(0x06, 0x20, 0x1d);
/// ‹ › without a month to go to.
const OFF: Rgb = Rgb(0x4a, 0x53, 0x70);
const AXIS: Rgb = Rgb(0x7d, 0x87, 0xa5);

const MONTHS: [&str; 12] = [
    "janvier",
    "février",
    "mars",
    "avril",
    "mai",
    "juin",
    "juillet",
    "août",
    "septembre",
    "octobre",
    "novembre",
    "décembre",
];
const MONTHS_SHORT: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];
const WEEKDAYS: [&str; 7] = ["dim.", "lun.", "mar.", "mer.", "jeu.", "ven.", "sam."];

/// « novembre 2026 » (`short`: « nov. 2026 »).
pub fn month_label(year: i32, month: u32, short: bool) -> String {
    let k = (month.clamp(1, 12) - 1) as usize;
    format!("{} {year}", if short { MONTHS_SHORT[k] } else { MONTHS[k] })
}

/// « mer. 18 nov. » for a time already in the chart's time zone (`local`: UTC seconds plus the offset).
pub fn day_label(local: i64) -> String {
    let (_, m, d) = argos_core::month::date_of(local, 0);
    let wd = (local.div_euclid(86_400) + 4).rem_euclid(7) as usize;
    format!("{} {d} {}", WEEKDAYS[wd], MONTHS_SHORT[(m - 1) as usize])
}

/// « 14 mars 2025 » for a time already in the chart's time zone.
pub fn date_label(local: i64) -> String {
    let (y, m, d) = argos_core::month::date_of(local, 0);
    format!("{d} {} {y}", MONTHS_SHORT[(m - 1) as usize])
}

/// « +4,18 R », « −1,62 R », « 0 R » (in the chosen number format).
pub fn r_text(r: f64) -> String {
    if r.abs() < 0.005 {
        return "0 R".into();
    }
    format!("{}{} R", if r < 0.0 { "−" } else { "+" }, argos_core::nombres::format_number(r.abs(), 2))
}

/// « +3,54 % », « −2,43 % ».
fn pct_text(p: f64) -> String {
    let sign = if p >= 0.005 {
        "+"
    } else if p <= -0.005 {
        "−"
    } else {
        ""
    };
    format!("{sign}{} %", argos_core::nombres::format_number(p.abs(), 2))
}

fn colour(x: f64) -> Rgb {
    if x > 0.0049 {
        UP
    } else if x < -0.0049 {
        DOWN
    } else {
        TITLE
    }
}

/// `c` at `a` opacity over the page's rows.
fn faded(c: Rgb, a: f32) -> Rgb {
    let m = |x: u8, f: u8| (f as f32 + (x as f32 - f as f32) * a).round() as u8;
    Rgb(m(c.0, 24), m(c.1, 30), m(c.2, 52))
}

/// The page's state (kept with the Simulator panel's state: closing the panel closes it).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MonthPage {
    /// The month chosen with ‹ › (`None`: the window's choice, the current month or the latest one with trades).
    pub month: Option<(i32, u32)>,
    /// « Tous les détails » unfolded.
    pub details: bool,
    /// Scroll of the content (points).
    pub scroll: f64,
}

/// How a trade followed its plan (its lot 7 verdict).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanMark {
    Followed,
    Partly,
    NotFollowed,
    /// Ended by the PropFirm Guardian, the end of the replay…: not judged.
    NotJudged,
}

impl PlanMark {
    /// ✓ ~ ✗ — and their colour.
    fn sign(self) -> (&'static str, Rgb) {
        match self {
            PlanMark::Followed => ("✓", UP),
            PlanMark::Partly => ("~", Rgb(0xff, 0xb7, 0x4d)),
            PlanMark::NotFollowed => ("✗", Rgb(0xff, 0x6b, 0x73)),
            PlanMark::NotJudged => ("—", DIM),
        }
    }
}

/// A finished trade of the month.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRow {
    pub long: bool,
    pub base: String,
    /// How it ended (« objectif », « stop », « fermé à la main »…).
    pub exit: String,
    /// When it was made (real time, chart's time zone): « mer. 18 nov. », « 09:44 ».
    pub day: String,
    pub time: String,
    /// ⏪ A replay trade: the market's date then (« 14 mars 2025 »).
    pub market: Option<String>,
    /// Its R (`None`: without a stop).
    pub r: Option<f64>,
    /// Its net result and its costs (fees, spread, slippage, funding), all its exits included.
    pub net: Money,
    pub costs: Money,
    pub plan: PlanMark,
    /// The index of its last exit in the account's history (its lot 7 card).
    pub last: usize,
}

/// « Argos AI · ton mois »: what worked, what to review (their chips), a lesson and the Academy lesson that goes with
/// it, the reminder.
#[derive(Debug, Clone, PartialEq)]
pub struct MonthAi {
    pub worked: Partie,
    pub to_review: Partie,
    pub lesson: String,
    pub academy: Option<&'static str>,
    pub reminder: String,
}

/// A line of the detail by symbol.
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolRow {
    pub base: String,
    pub trades: usize,
    pub r: f64,
    pub net: Money,
}

/// What the page shows (computed by the window).
#[derive(Debug, Clone, PartialEq)]
pub struct MonthView {
    /// « Démo Argos 100K ».
    pub account: String,
    /// « heure de New York ».
    pub zone: String,
    pub year: i32,
    pub month: u32,
    /// The months before and after it that can be shown (‹ ›).
    pub prev: Option<(i32, u32)>,
    pub next: Option<(i32, u32)>,
    /// The balance on the 1st, the month's net result and its share of that balance (%).
    pub start: Money,
    pub net: Money,
    pub pct: f64,
    /// The sum of the R of the trades with a stop, and their average (`None`: no trade with a stop).
    pub r: f64,
    pub r_avg: Option<f64>,
    pub winners: usize,
    /// The plans judged, followed, partly followed and not followed.
    pub judged: usize,
    pub followed: usize,
    pub partly: usize,
    pub not_followed: usize,
    /// The balance at the end of each day shown (the 1st to today, or to the month's end) and the month's days.
    pub days: Vec<Money>,
    pub month_days: u32,
    /// The month's highest balance, its largest drop (in $, in %, and its days) and its longest losing streak.
    pub high: Money,
    pub drawdown: Money,
    pub drawdown_pct: f64,
    pub drawdown_days: Option<(u32, u32)>,
    pub losing_streak: usize,
    pub streak_days: Option<(u32, u32)>,
    /// The costs paid: the fees, the spread and the slippage, the funding.
    pub fees: Money,
    pub slippage: Money,
    pub funding: Money,
    /// The month's finished trades, in order; the best (if it won) and the worst (if it lost) among them.
    pub trades: Vec<TradeRow>,
    pub best: Option<usize>,
    pub worst: Option<usize>,
    pub symbols: Vec<SymbolRow>,
    /// `None`: no trade to comment on.
    pub ai: Option<MonthAi>,
}

impl MonthView {
    fn replays(&self) -> usize {
        self.trades.iter().filter(|t| t.market.is_some()).count()
    }

    /// The last day shown (today for the current month).
    fn last_day(&self) -> u32 {
        if self.days.is_empty() {
            self.month_days
        } else {
            self.days.len() as u32
        }
    }

    /// The balance at the end of the last day shown.
    fn end(&self) -> Money {
        self.start + self.net
    }

    /// « Plus forte baisse du mois : −2 548,20 $ (−2,43 %) ».
    fn drawdown_text(&self) -> String {
        if self.drawdown <= Money::ZERO {
            return "Pas de baisse ce mois-ci.".into();
        }
        format!(
            "Plus forte baisse du mois : {} ({})",
            argent(Money::ZERO - self.drawdown, true),
            pct_text(-self.drawdown_pct)
        )
    }

    /// « du 10 au 16 nov. » (empty without days).
    fn days_text(&self, days: Option<(u32, u32)>) -> String {
        let m = MONTHS_SHORT[(self.month.clamp(1, 12) - 1) as usize];
        match days {
            Some((a, b)) if a == b => format!("le {a} {m}"),
            Some((a, b)) => format!("du {a} au {b} {m}"),
            None => String::new(),
        }
    }

    /// The four tiles: label, figure, its colour, the line under it.
    fn tiles(&self) -> [(&'static str, String, Rgb, String); 4] {
        let n = self.trades.len();
        let m = MONTHS_SHORT[(self.month.clamp(1, 12) - 1) as usize];
        let net = (
            "Résultat net",
            argent(self.net, true),
            colour(self.net.to_f64()),
            format!("{} du compte au 1ᵉʳ\u{a0}{m}", pct_text(self.pct)),
        );
        let r = match self.r_avg {
            Some(avg) => ("En R", r_text(self.r), colour(self.r), format!("moyenne {} par trade", r_text(avg))),
            None => ("En R", "—".into(), TITLE, "aucun trade avec un stop".into()),
        };
        let win = if n == 0 {
            ("Réussite", "—".into(), TITLE, "aucun trade fermé".into())
        } else {
            let pct = self.winners as f64 * 100.0 / n as f64;
            let g = if self.winners > 1 { "gagnants" } else { "gagnant" };
            (
                "Réussite",
                format!("{} %", argos_core::nombres::format_number(pct, 0)),
                TITLE,
                format!("{} {g} sur {n}", self.winners),
            )
        };
        let plans = if self.judged == 0 {
            ("Plans suivis", "—".into(), TITLE, "aucun plan jugé".into())
        } else {
            let p = if self.not_followed > 1 { "pas suivis" } else { "pas suivi" };
            (
                "Plans suivis",
                format!("{} sur {}", self.followed, self.judged),
                TITLE,
                format!("{} en partie · {} {p}", self.partly, self.not_followed),
            )
        };
        [net, r, win, plans]
    }
}

/// Where the page is drawn: its size; 📱 a full-screen sheet (no edge, square corners); the room kept under the
/// content (📱 the Argos AI round over the bottom right).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame {
    pub w: f64,
    pub h: f64,
    pub sheet: bool,
    pub room: f64,
}

/// What a click asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonthClick {
    /// On the page, nothing to do (the click does not reach what is under it).
    Nothing,
    /// To redraw (another month, « Tous les détails »).
    Changed,
    Close,
    /// 📄 Imprimer en PDF.
    Print,
    /// 🤖 The lot 7 card of this trade (the index of its last exit in the history).
    Card(usize),
    /// 🎓 Open this Academy lesson.
    Lesson(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Close,
    Month((i32, u32)),
    Print,
    Details,
    Card(usize),
    Lesson(&'static str),
}

/// A piece of the page, placed (content coordinates; the `y` of a text is its baseline).
#[derive(Debug, Clone, PartialEq)]
enum Piece {
    /// A rounded box: its radius, its fill and its edge (colour, opacity).
    Fill(Rect, f64, (Rgb, f32), Option<(Rgb, f32)>),
    Text(f64, f64, String, f32, Rgb, Graisse, Align),
    /// A title or a big figure: its letters a hundredth of its size closer (the mock-up's `letter-spacing: -.01em`).
    Tight(f64, f64, String, f32, Rgb, Graisse),
    /// A small capitals label (11 points, letters spaced).
    Label(f64, f64, String),
    Emoji(Rect, &'static str),
    /// The Argos AI round.
    Round(Rect),
    Chip(Rect, String, Ton),
    /// 🎓 The link to an Academy lesson (as on the lot 7 card).
    Link(Rect, String),
    Curve(Rect),
    /// A thin line: its `y`, from `x0` to `x1`.
    Rule(f64, f64, f64),
}

/// The page laid out for a width: its pieces, its zones (with their radius, for the hover), its height.
#[derive(Default)]
struct Layout {
    pieces: Vec<Piece>,
    zones: Vec<(Rect, f64, Zone)>,
    h: f64,
}

/// The ascent of Inter at `s` points, rounded: where the baseline falls under the top of a `line-height: normal` line
/// (as the mock-up's browser lays its lines out).
fn asc(s: f64) -> f64 {
    (s * 0.96875).round()
}

/// The height of a `line-height: normal` line of Inter at `s` points (its ascent and its descent rounded apart).
fn lh(s: f64) -> f64 {
    asc(s) + (s * 0.2412).round()
}

/// The baseline under the top of a line `l` high at `s` points (the room left halved, rounded down).
fn base_in(s: f64, l: f64) -> f64 {
    ((l - lh(s)) / 2.0).floor().max(0.0) + asc(s)
}

/// The baseline of a `line-height: normal` line of `s` points centred on `cy`.
fn base_at(s: f64, cy: f64) -> f64 {
    cy - lh(s) / 2.0 + asc(s)
}

/// The mock-up's `letter-spacing: -.01em` at `size` points.
fn tight(size: f32) -> f32 {
    -0.01 * size
}

/// A `line-height` as the browser keeps it (in 64ths of a point, rounded down: 16.2 → 16.1875).
fn snap(l: f64) -> f64 {
    (l * 64.0).floor() / 64.0
}

/// The lines of 12 points under the tiles and the boxes (`line-height: 1.35`).
const SMALL: f64 = 16.1875;

/// The superscript « er » of « 1ᵉʳ » (the mock-up's `<sup>`): 10 points, 5 higher; its line 3 higher.
const SUP: &str = "ᵉʳ";
const SUP_ROOM: f64 = 3.0;

/// Lays the page out.
struct Builder<'a> {
    text: &'a Text,
    medium: Text,
    bold: Text,
    l: Layout,
}

impl Builder<'_> {
    fn width(&self, s: &str, size: f64, g: Graisse) -> f64 {
        let size = size as f32;
        f64::from(match g {
            Graisse::Regular => self.text.width(s, size),
            Graisse::Medium => self.medium.width(s, size),
            Graisse::Bold => self.bold.width(s, size),
        })
    }

    fn fit(&self, s: &str, size: f64, w: f64) -> String {
        self.text.fit(s, size as f32, w)
    }

    fn lines(&self, s: &str, size: f64, w: f64) -> Vec<String> {
        couper(self.text, s, size as f32, w)
    }

    fn fill(&mut self, r: Rect, radius: f64, bg: (Rgb, f32), edge: Option<(Rgb, f32)>) -> usize {
        self.l.pieces.push(Piece::Fill(r, radius, bg, edge));
        self.l.pieces.len() - 1
    }

    /// Sets the height of the box pushed at `k` (known once its content is laid out).
    fn close_fill(&mut self, k: usize, h: f64) {
        if let Some(Piece::Fill(r, ..)) = self.l.pieces.get_mut(k) {
            r.h = h;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn text_at(&mut self, x: f64, y: f64, s: impl Into<String>, size: f64, c: Rgb, g: Graisse, a: Align) {
        self.l.pieces.push(Piece::Text(x, y, s.into(), size as f32, c, g, a));
    }

    /// A title or a big figure (left-aligned), its letters closer.
    fn tight(&mut self, x: f64, y: f64, s: impl Into<String>, size: f64, c: Rgb, g: Graisse) {
        self.l.pieces.push(Piece::Tight(x, y, s.into(), size as f32, c, g));
    }

    /// The width of a [`Piece::Tight`] text (each letter followed by its spacing, as the browser counts it).
    fn tight_width(&self, s: &str, size: f64, g: Graisse) -> f64 {
        self.width(s, size, g) + crate::i18n::tr(s).chars().count() as f64 * tight(size as f32) as f64
    }

    /// A line of 12 points that may hold « 1ᵉʳ »: its « er » drawn as the mock-up's superscript.
    fn sup_line(&mut self, x: f64, y: f64, line: &str, c: Rgb) {
        let Some((before, after)) = line.split_once(SUP) else {
            self.text_at(x, y, line, 12.0, c, Graisse::Regular, Align::Left);
            return;
        };
        let w = self.width(before, 12.0, Graisse::Regular);
        self.text_at(x, y, before, 12.0, c, Graisse::Regular, Align::Left);
        self.text_at(x + w, y - 5.0, "er", 10.0, c, Graisse::Regular, Align::Left);
        self.text_at(
            x + w + self.width("er", 10.0, Graisse::Regular),
            y,
            after,
            12.0,
            c,
            Graisse::Regular,
            Align::Left,
        );
    }

    fn label(&mut self, x: f64, y: f64, s: &str) {
        self.l.pieces.push(Piece::Label(x, y, crate::i18n::tr(s).to_uppercase()));
    }

    fn zone(&mut self, r: Rect, radius: f64, z: Zone) {
        self.l.zones.push((r, radius, z));
    }

    /// The month between ‹ and › (a pill of 38 at `x`, centred on `cy`).
    fn month_pill(&mut self, v: &MonthView, x: f64, cy: f64, short: bool) {
        let label = month_label(v.year, v.month, short);
        let tw = self.width(&label, 14.0, Graisse::Medium);
        let w = 1.0 + 8.0 + 26.0 + 14.0 + tw + 14.0 + 26.0 + 8.0 + 1.0;
        self.fill(Rect::new(x, cy - 19.0, w, 38.0), 19.0, (WHITE, 0.07), Some((WHITE, 0.1)));
        for (cx, arrow, to) in [(x + 22.0, "‹", v.prev), (x + w - 22.0, "›", v.next)] {
            let c = Rect::new(cx - 13.0, cy - 13.0, 26.0, 26.0);
            self.fill(c, 13.0, (WHITE, 0.06), None);
            let colour = if to.is_some() { BODY } else { OFF };
            self.text_at(cx, base_at(18.0, cy), arrow, 18.0, colour, Graisse::Regular, Align::Center);
            if let Some(m) = to {
                self.zone(c, 13.0, Zone::Month(m));
            }
        }
        self.text_at(x + 49.0, base_at(14.0, cy), label, 14.0, TITLE, Graisse::Medium, Align::Left);
    }

    /// 📄 Imprimer en PDF (a teal pill of 33 ending at `right`, centred on `cy`).
    fn print_button(&mut self, right: f64, cy: f64) {
        let t = "Imprimer en PDF";
        let w = 14.0 + 17.0 + 7.0 + self.width(t, 13.5, Graisse::Medium) + 14.0;
        let r = Rect::new(right - w, cy - 16.5, w, 33.0);
        self.fill(r, 16.5, (TEAL, 1.0), None);
        self.l.pieces.push(Piece::Emoji(Rect::new(r.x + 14.0, cy - 8.5, 17.0, 17.0), "📄"));
        self.text_at(r.x + 38.0, base_at(13.5, cy), t, 13.5, INK, Graisse::Medium, Align::Left);
        self.zone(r, 16.5, Zone::Print);
    }

    /// ✕ (20 points, 2 of room on each side) ending at `right`, centred on `cy`; its left edge.
    fn close(&mut self, right: f64, cy: f64) -> f64 {
        let w = self.width("✕", 20.0, Graisse::Regular) + 4.0;
        let cx = right - w / 2.0;
        self.text_at(cx, base_at(20.0, cy), "✕", 20.0, SOFT, Graisse::Regular, Align::Center);
        self.zone(Rect::new(cx - 17.0, cy - 17.0, 34.0, 34.0), 17.0, Zone::Close);
        right - w
    }

    /// The header; returns the `y` under it.
    fn header(&mut self, v: &MonthView, x0: f64, cw: f64, y: f64, narrow: bool) -> f64 {
        let title = "Rapport du mois";
        if narrow {
            // 📱 the title and ✕, then the month and « Imprimer en PDF »
            let cy = y + lh(20.0) / 2.0;
            self.l.pieces.push(Piece::Emoji(Rect::new(x0, cy - 12.0, 24.0, 24.0), "📅"));
            self.tight(x0 + 36.0, base_at(20.0, cy), title, 20.0, TITLE, Graisse::Bold);
            self.close(x0 + cw, cy);
            let cy = y + lh(20.0) + 14.0 + 19.0;
            self.month_pill(v, x0, cy, true);
            self.print_button(x0 + cw, cy);
            return cy + 19.0;
        }
        let cy = y + 19.0;
        self.l.pieces.push(Piece::Emoji(Rect::new(x0, cy - 12.0, 24.0, 24.0), "📅"));
        self.tight(x0 + 36.0, base_at(21.0, cy), title, 21.0, TITLE, Graisse::Bold);
        self.month_pill(v, x0 + 36.0 + self.tight_width(title, 21.0, Graisse::Bold) + 26.0, cy, false);
        let left = self.close(x0 + cw, cy);
        self.print_button(left - 12.0, cy);
        y + 38.0
    }

    /// The line under the header: the account, the trades, the time zone, the replay trades.
    fn subline(&mut self, v: &MonthView, x0: f64, cw: f64, mut y: f64) -> f64 {
        let trades = match v.trades.len() {
            0 => "aucun trade fermé".to_string(),
            1 => "1 trade fermé".to_string(),
            n => format!("{n} trades fermés"),
        };
        let mut s = format!("{} · {trades} · {}", v.account, v.zone);
        let replays = v.replays();
        if replays > 0 {
            s = format!("{s} · dont {replays} en replay ⏪");
        }
        for l in self.lines(&s, 12.5, cw - 2.0) {
            self.text_at(x0 + 2.0, y + base_in(12.5, 17.5), l, 12.5, DIM, Graisse::Regular, Align::Left);
            y += 17.5;
        }
        y
    }

    /// The four tiles (🖥 one row, 📱 two by two); returns the `y` under them.
    fn tiles(&mut self, v: &MonthView, x0: f64, cw: f64, mut y: f64, narrow: bool) -> f64 {
        let (cols, gap) = if narrow { (2, 10.0) } else { (4, 14.0) };
        let tw = (cw - gap * (cols - 1) as f64) / cols as f64;
        let (pt, px, pb, big) = if narrow { (12.0, 13.0, 11.0, 22.0) } else { (13.0, 15.0, 12.0, 25.0) };
        let tiles = v.tiles();
        for (k, row) in tiles.chunks(cols).enumerate() {
            if k > 0 {
                y += gap;
            }
            let subs: Vec<Vec<String>> = row.iter().map(|t| self.lines(&t.3, 12.0, tw - 2.0 * (px + 1.0))).collect();
            let height = |sub: &Vec<String>| sub.len() as f64 * SMALL + if sub.iter().any(|l| l.contains(SUP)) { SUP_ROOM } else { 0.0 };
            let under = subs.iter().map(height).fold(0.0, f64::max);
            let h = 1.0 + pt + lh(11.0) + 7.0 + lh(big) + 5.0 + under + pb + 1.0;
            for (i, (t, sub)) in row.iter().zip(&subs).enumerate() {
                let r = Rect::new(x0 + i as f64 * (tw + gap), y, tw, h);
                self.fill(r, 18.0, (WHITE, 0.045), Some((WHITE, 0.08)));
                let x = r.x + 1.0 + px;
                let top = r.y + 1.0 + pt;
                self.label(x, top + asc(11.0), t.0);
                let big_top = top + lh(11.0) + 7.0;
                self.tight(x, big_top + asc(big), t.1.clone(), big, t.2, Graisse::Medium);
                let mut ly = big_top + lh(big) + 5.0;
                for l in sub {
                    if l.contains(SUP) {
                        ly += SUP_ROOM;
                    }
                    self.sup_line(x, ly + base_in(12.0, SMALL), l, DIM);
                    ly += SMALL;
                }
            }
            y += h;
        }
        y
    }

    /// « Courbe du compte » in a box at (`x`, `y`) of width `w`; its height.
    fn curve_box(&mut self, v: &MonthView, x: f64, y: f64, w: f64, curve_h: f64) -> f64 {
        let k = self.fill(Rect::new(x, y, w, 0.0), 18.0, (WHITE, 0.035), Some((WHITE, 0.08)));
        // its title row (18.1875 high: two sizes on one baseline), the curve 8 under it (2 wider than the text)
        let base = y + 15.0 + asc(14.0);
        self.text_at(x + 17.0, base, "Courbe du compte", 14.0, TITLE, Graisse::Medium, Align::Left);
        let high = format!("plus haut {}", argent(v.high, false));
        self.text_at(x + w - 17.0, base, high, 12.0, DIM, Graisse::Regular, Align::Right);
        let c = Rect::new(x + 17.0, y + 15.0 + 18.1875 + 8.0, w - 32.0, curve_h);
        self.l.pieces.push(Piece::Curve(c));
        let top = c.y + curve_h + 8.0;
        let fit = self.fit(&v.drawdown_text(), 12.0, w - 34.0);
        self.text_at(x + 17.0, top + base_in(12.0, SMALL), fit, 12.0, DIM, Graisse::Regular, Align::Left);
        let h = top + SMALL + 11.0 - y;
        self.close_fill(k, h);
        h
    }

    /// « Argos AI · ton mois », a glass card at (`x`, `y`) of width `w` (as the lot 7 card); its height.
    fn ai_card(&mut self, v: &MonthView, x: f64, y: f64, w: f64) -> f64 {
        let k = self.fill(Rect::new(x, y, w, 0.0), 22.0, (Rgb(22, 30, 58), 0.6), Some((WHITE, 0.16)));
        self.l.pieces.push(Piece::Round(Rect::new(x + 17.0, y + 15.0, 32.0, 32.0)));
        self.text_at(
            x + 59.0,
            base_at(16.0, y + 31.0),
            "Argos AI · ton mois",
            16.0,
            TITLE,
            Graisse::Bold,
            Align::Left,
        );
        let large = w - 34.0;
        let mut yy = y + 47.0;
        let para = |b: &mut Self, yy: &mut f64, s: &str, size: f64, c: Rgb| {
            let step = snap(size * 1.4);
            for l in b.lines(s, size, large) {
                b.text_at(x + 17.0, *yy + base_in(size, step), l, size, c, Graisse::Regular, Align::Left);
                *yy += step;
            }
        };
        // a part: its small title and its chip (a row of 22, 13 above it and 5 under it)
        let part = |b: &mut Self, yy: &mut f64, title: &str, chip: Option<(&str, Ton)>| {
            *yy += 13.0;
            b.label(x + 19.0, *yy + base_at(11.0, 11.0), title);
            if let Some((t, ton)) = chip {
                let pw = b.width(t, 12.0, Graisse::Bold) + 20.0;
                b.l.pieces
                    .push(Piece::Chip(Rect::new(x + w - 17.0 - pw, *yy + 0.5, pw, 21.0), t.to_string(), ton));
            }
            *yy += 27.0;
        };
        let reminder = match &v.ai {
            Some(a) => {
                for (title, p) in [("Ce qui a marché", &a.worked), ("À revoir", &a.to_review)] {
                    part(self, &mut yy, title, Some((p.pastille.as_str(), p.ton)));
                    para(self, &mut yy, &p.texte, 13.0, BODY);
                }
                part(self, &mut yy, "Une leçon", None);
                para(self, &mut yy, &a.lesson, 13.5, TITLE);
                if let Some(id) = a.academy {
                    yy += 9.0;
                    let r = Rect::new(x + 17.0, yy, large, 32.0);
                    self.l.pieces.push(Piece::Link(r, format!("Leçon : {}", crate::avis::titre_lecon(id))));
                    self.zone(r, 16.0, Zone::Lesson(id));
                    yy += 32.0;
                }
                a.reminder.clone()
            }
            None => {
                yy += 13.0;
                para(
                    self,
                    &mut yy,
                    "Pas encore de trade fini ce mois-ci : l'analyse viendra avec le premier.",
                    13.0,
                    BODY,
                );
                "Analyse affichée, pas un ordre.".to_string()
            }
        };
        yy += 11.0;
        for l in self.lines(&reminder, 11.5, large) {
            self.text_at(x + 17.0, yy + asc(11.5), l, 11.5, SOFT, Graisse::Regular, Align::Left);
            yy += lh(11.5);
        }
        let h = yy + 14.0 - y;
        self.close_fill(k, h);
        h
    }

    /// « Meilleur trade » or « Pire trade » at (`x`, `y`), width `w` ([`EXTREME`] high).
    fn extreme(&mut self, v: &MonthView, x: f64, y: f64, w: f64, (title, k, none): (&str, Option<usize>, &str)) {
        self.label(x + 4.0, y + asc(11.0), title);
        let row = Rect::new(x, y + lh(11.0) + 6.0, w, 50.0);
        self.fill(row, 14.0, (WHITE, 0.04), Some((WHITE, 0.07)));
        let Some(t) = k.and_then(|k| v.trades.get(k)) else {
            self.text_at(row.x + 16.0, base_at(13.0, row.cy()), none, 13.0, DIM, Graisse::Regular, Align::Left);
            return;
        };
        self.l.pieces.push(Piece::Round(Rect::new(row.x + 9.0, row.cy() - 14.0, 28.0, 28.0)));
        let (r1, r2) = match t.r {
            Some(r) => (r_text(r), argent(t.net, true)),
            None => (argent(t.net, true), String::new()),
        };
        let c = colour(t.net.to_f64());
        let right = row.x + row.w - 13.0;
        let rw = self.width(&r1, 13.5, Graisse::Medium).max(self.width(&r2, 11.5, Graisse::Medium));
        let tx = row.x + 47.0;
        let room = right - rw - 12.0 - tx;
        // two lines (13.5 and 11.5, 2 apart) 9 under the row's top
        let (b1, b2) = (row.y + 9.0 + asc(13.5), row.y + 9.0 + lh(13.5) + 2.0 + asc(11.5));
        let first = self.fit(&format!("{} {} · {}", if t.long { "▲" } else { "▼" }, t.base, t.exit), 13.5, room);
        self.text_at(tx, b1, first, 13.5, TITLE, Graisse::Regular, Align::Left);
        let when = if t.market.is_some() {
            format!("{} · ⏪ replay", t.day)
        } else {
            t.day.clone()
        };
        let when = self.fit(&when, 11.5, room);
        self.text_at(tx, b2, when, 11.5, DIM, Graisse::Regular, Align::Left);
        self.text_at(right, b1, r1, 13.5, c, Graisse::Medium, Align::Right);
        self.text_at(right, b2, r2, 11.5, faded(c, 0.85), Graisse::Medium, Align::Right);
        self.zone(row, 14.0, Zone::Card(t.last));
    }

    /// « Tous les détails »: its row, then (unfolded) by symbol, the costs and the risk, the month's trades.
    fn details(&mut self, v: &MonthView, page: &MonthPage, x0: f64, cw: f64, y: f64, narrow: bool) -> f64 {
        let row = Rect::new(x0, y, cw, 44.0);
        self.fill(row, 16.0, (WHITE, 0.05), None);
        let cy = row.cy();
        let t = "Tous les détails";
        self.text_at(x0 + 16.0, base_at(14.0, cy), t, 14.0, ROW, Graisse::Regular, Align::Left);
        let chevron = if page.details { "⌃" } else { "⌄" };
        let cwid = self.width(chevron, 16.0, Graisse::Regular);
        if !narrow {
            let x = x0 + 16.0 + self.width(t, 14.0, Graisse::Regular) + 14.0;
            let sub = self.fit(
                "par symbole, coûts payés, série de pertes, liste des trades",
                12.0,
                x0 + cw - 16.0 - cwid - 10.0 - x,
            );
            self.text_at(x, cy - SMALL / 2.0 + base_in(12.0, SMALL), sub, 12.0, DIM, Graisse::Regular, Align::Left);
        }
        // (⌃ sits high in its line: lowered to the place of ⌄)
        let lower = if page.details { CHEVRON_UP_DROP } else { 0.0 };
        self.text_at(x0 + cw - 16.0, base_at(16.0, cy) + lower, chevron, 16.0, SOFT, Graisse::Regular, Align::Right);
        self.zone(row, 22.0, Zone::Details);
        let mut y = y + row.h;
        if !page.details {
            return y;
        }
        y += 14.0;
        let col = if narrow { cw } else { (cw - 14.0) / 2.0 };
        // — by symbol —
        self.label(x0 + 4.0, y + asc(11.0), "Par symbole");
        let mut ys = y + lh(11.0) + 6.0;
        let (c_n, c_r) = (x0 + col * 0.5, x0 + col * 0.74);
        for (s, x, a) in [
            ("Symbole", x0 + 4.0, Align::Left),
            ("Trades", c_n, Align::Right),
            ("R", c_r, Align::Right),
            ("Net", x0 + col - 4.0, Align::Right),
        ] {
            self.text_at(x, ys + 17.0, s, 11.5, DIM, Graisse::Regular, a);
        }
        ys += 26.0;
        for s in &v.symbols {
            self.l.pieces.push(Piece::Rule(ys, x0, x0 + col));
            self.text_at(x0 + 4.0, ys + 17.5, s.base.clone(), 13.0, TITLE, Graisse::Regular, Align::Left);
            self.text_at(c_n, ys + 17.5, s.trades.to_string(), 13.0, BODY, Graisse::Regular, Align::Right);
            self.text_at(c_r, ys + 17.5, r_text(s.r), 13.0, colour(s.r), Graisse::Regular, Align::Right);
            self.text_at(
                x0 + col - 4.0,
                ys + 17.5,
                argent(s.net, true),
                13.0,
                colour(s.net.to_f64()),
                Graisse::Medium,
                Align::Right,
            );
            ys += 26.0;
        }
        // — the costs and the risk (🖥 beside, 📱 under) —
        let (kx, mut yk) = if narrow { (x0, ys + 14.0) } else { (x0 + col + 14.0, y) };
        self.label(kx + 4.0, yk + asc(11.0), "Coûts et risque");
        yk += lh(11.0) + 6.0;
        let mut rows = vec![
            ("Coûts payés", argent(v.fees + v.slippage + v.funding, false), String::new()),
            ("dont frais", argent(v.fees, false), String::new()),
            ("dont écart et glissement", argent(v.slippage, false), String::new()),
        ];
        if v.funding != Money::ZERO {
            rows.push(("dont funding", argent(v.funding, false), String::new()));
        }
        let dd = if v.drawdown > Money::ZERO {
            format!("{} ({})", argent(Money::ZERO - v.drawdown, true), pct_text(-v.drawdown_pct))
        } else {
            "aucune".to_string()
        };
        rows.push(("Plus forte baisse", dd, v.days_text(v.drawdown_days)));
        let streak = match v.losing_streak {
            0 => "aucune".to_string(),
            1 => "1 trade".to_string(),
            n => format!("{n} trades"),
        };
        rows.push(("Plus longue série de pertes", streak, v.days_text(v.streak_days)));
        for (k, val, days) in rows {
            let indent = if k.starts_with("dont") { 14.0 } else { 0.0 };
            self.text_at(kx + 4.0 + indent, yk + 17.5, k, 13.0, DIM, Graisse::Regular, Align::Left);
            self.text_at(kx + col - 4.0, yk + 17.5, val, 13.0, TITLE, Graisse::Regular, Align::Right);
            // their days under the drop and the streak (« du 10 au 16 nov. »)
            if !days.is_empty() {
                self.text_at(kx + col - 4.0, yk + 32.5, days, 11.5, DIM, Graisse::Regular, Align::Right);
                yk += 14.0;
            }
            yk += 26.0;
        }
        y = if narrow { yk } else { ys.max(yk) };
        // — the month's trades: 🖥 two columns (the first half on the left), 📱 one —
        y += 14.0;
        self.label(x0 + 4.0, y + asc(11.0), "Les trades du mois");
        y += lh(11.0) + 6.0;
        let cols = if narrow { 1 } else { 2 };
        let cwid = (cw - 14.0 * (cols - 1) as f64) / cols as f64;
        let per = v.trades.len().div_ceil(cols).max(1);
        let mut bottom = y;
        for (c, chunk) in v.trades.chunks(per).enumerate() {
            let x = x0 + c as f64 * (cwid + 14.0);
            let mut yy = y;
            for (i, t) in chunk.iter().enumerate() {
                if i > 0 {
                    self.l.pieces.push(Piece::Rule(yy, x, x + cwid));
                }
                yy += self.trade_row(t, x, yy, cwid);
            }
            bottom = bottom.max(yy);
        }
        bottom
    }

    /// A trade of the list at (`x`, `y`), width `w`: ✓ ~ ✗ —, the trade and its R, when (and its market date in
    /// replay) with its costs, its net result. Its height.
    fn trade_row(&mut self, t: &TradeRow, x: f64, y: f64, w: f64) -> f64 {
        let (mark, mc) = t.plan.sign();
        self.text_at(x + 4.0, y + 18.0, mark, 13.0, mc, Graisse::Medium, Align::Left);
        let tx = x + 24.0;
        let right = x + w - 4.0;
        let c = colour(t.net.to_f64());
        let r = t.r.map_or(String::new(), r_text);
        let rc = t.r.map_or(c, colour);
        let rw = self.width(&r, 13.0, Graisse::Medium);
        let first = self.fit(
            &format!("{} {} · {}", if t.long { "▲" } else { "▼" }, t.base, t.exit),
            13.0,
            right - rw - 10.0 - tx,
        );
        self.text_at(tx, y + 18.0, first, 13.0, TITLE, Graisse::Regular, Align::Left);
        self.text_at(right, y + 18.0, r, 13.0, rc, Graisse::Medium, Align::Right);
        let net = argent(t.net, true);
        let nw = self.width(&net, 11.5, Graisse::Medium);
        self.text_at(right, y + 34.0, net, 11.5, faded(c, 0.85), Graisse::Medium, Align::Right);
        // (a date, the market's date, the costs: never cut at the end of a line)
        let keep = |s: &str| s.replace(' ', "\u{a0}");
        let mut detail = match &t.market {
            Some(m) => format!("⏪ {} · {} · {}", keep(&t.day), t.time, keep(&format!("marché {m}"))),
            None => format!("{} · {}", keep(&t.day), t.time),
        };
        if t.costs != Money::ZERO {
            detail = format!("{detail} · {}", keep(&format!("coûts {}", argent(t.costs, false))));
        }
        let lines = self.lines(&detail, 11.5, right - nw - 10.0 - tx);
        let n = lines.len().max(1);
        for (j, l) in lines.into_iter().enumerate() {
            self.text_at(tx, y + 34.0 + 15.0 * j as f64, l, 11.5, DIM, Graisse::Regular, Align::Left);
        }
        44.0 + 15.0 * (n - 1) as f64
    }
}

/// The page laid out on the frame `f`.
fn layout(v: &MonthView, page: &MonthPage, text: &Text, f: Frame) -> Layout {
    let narrow = f.w < NARROW;
    let edge = if f.sheet { 0.0 } else { 1.0 };
    let (pad_top, pad_x, pad_bottom) = if narrow { (18.0, 16.0, 28.0) } else { (22.0, 24.0, 20.0) };
    let x0 = edge + pad_x;
    let cw = (f.w - 2.0 * x0).max(100.0);
    let mut b = Builder {
        text,
        medium: Text::graisse(Graisse::Medium),
        bold: Text::graisse(Graisse::Bold),
        l: Layout::default(),
    };
    let mut y = b.header(v, x0, cw, edge + pad_top, narrow);
    y = b.subline(v, x0, cw, y + 10.0);
    y = b.tiles(v, x0, cw, y + 16.0, narrow);
    // the curve and Argos AI (🖥 side by side, as the mock-up's 488 and the rest; 📱 one under the other)
    if narrow {
        y += 12.0;
        y += b.curve_box(v, x0, y, cw, 170.0) + 12.0;
        y += b.ai_card(v, x0, y, cw);
    } else {
        y += 14.0;
        let bw = ((cw - 14.0) * 488.0 / 896.0).round();
        let h1 = b.curve_box(v, x0, y, bw, 262.0);
        let h2 = b.ai_card(v, x0 + bw + 14.0, y, cw - bw - 14.0);
        y += h1.max(h2);
    }
    if !v.trades.is_empty() {
        let best = ("Meilleur trade", v.best, "Aucun trade gagnant ce mois-ci.");
        let worst = ("Pire trade", v.worst, "Aucun trade perdant ce mois-ci.");
        if narrow {
            y += 12.0;
            b.extreme(v, x0, y, cw, best);
            y += EXTREME + 12.0;
            b.extreme(v, x0, y, cw, worst);
        } else {
            y += 14.0;
            let w = (cw - 14.0) / 2.0;
            b.extreme(v, x0, y, w, best);
            b.extreme(v, x0 + w + 14.0, y, w, worst);
        }
        y += EXTREME;
    }
    y = b.details(v, page, x0, cw, y + 14.0, narrow);
    b.l.h = y + pad_bottom + edge + f.room;
    b.l
}

/// Space between the round values of the curve: 1, 2 or 5 times a power of ten, the nearest to `x` (the mock-up:
/// 1 000 $ for a quarter of its axis of 4 635 $).
fn round_step(x: f64) -> f64 {
    if !(x.is_finite() && x > 0.0) {
        return 1.0;
    }
    let p = 10f64.powf(x.log10().floor());
    let m = x / p;
    p * if m < 1.5 {
        1.0
    } else if m < 3.0 {
        2.0
    } else if m < 7.0 {
        5.0
    } else {
        10.0
    }
}

/// « 105 k », « 10,5 k », « 950 »: a round value of the curve.
fn axis_text(v: f64, step: f64) -> String {
    if v.abs() >= 1000.0 {
        let dec = (3.0 - step.log10().floor()).clamp(0.0, 3.0) as usize;
        format!("{} k", argos_core::nombres::format_number(v / 1000.0, dec))
    } else {
        argos_core::nombres::format_number(v, 0)
    }
}

/// The curve's scale: its axis (low, high, with a margin) and the round values written on it.
struct CurveGeo {
    lo: f64,
    hi: f64,
    step: f64,
    marks: Vec<f64>,
}

fn curve_geo(v: &MonthView) -> CurveGeo {
    let start = v.start.to_f64();
    let (min, max) = v.days.iter().map(|m| m.to_f64()).fold((start, start), |(a, b), x| (a.min(x), b.max(x)));
    // a tenth of the balances' range above and under, rounded up on its first digit (the mock-up: 400 $ for 3 835 $)
    let raw = ((max - min) * 0.1).max(start.abs() * 0.002).max(1.0);
    let unit = 10f64.powf(raw.log10().floor());
    let pad = (raw / unit).ceil() * unit;
    let (lo, hi) = (min - pad, max + pad);
    let step = round_step((hi - lo) / 4.0);
    // the round values among the balances (the mock-up: 102 k to 105 k), on the whole axis when fewer than two
    let between = |a: f64, b: f64| {
        ((a / step).ceil() as i64..=(b / step).floor() as i64)
            .map(|k| k as f64 * step)
            .collect::<Vec<_>>()
    };
    let mut marks = between(min, max);
    if marks.len() < 2 {
        marks = between(lo, hi).into_iter().filter(|m| *m > lo && *m < hi).collect();
    }
    CurveGeo { lo, hi, step, marks }
}

/// The days written under the curve: the 1st, the 8th, the 15th, the 22nd and the last.
fn curve_days(n: u32) -> [u32; 5] {
    [1, 8, 15, 22, n]
}

fn paint(c: Rgb, a: f32) -> tiny_skia::Paint<'static> {
    let mut p = tiny_skia::Paint::default();
    p.set_color_rgba8(c.0, c.1, c.2, (a.clamp(0.0, 1.0) * 255.0).round() as u8);
    p.anti_alias = true;
    p
}

/// 📈 The account curve in `r` (the mock-up's svg): round values, the dashed line of the balance on the 1st, the area
/// and the line of the balance day by day, the last point and the highest one.
fn draw_curve(pm: &mut Pixmap, text: &Text, r: Rect, v: &MonthView) {
    use tiny_skia::{FillRule, LineCap, LineJoin, PathBuilder, Stroke, StrokeDash};
    const MARGIN: f64 = 46.0;
    let g = curve_geo(v);
    let n = f64::from(v.month_days.max(2));
    let xs = |d: f64| r.x + 8.0 + (r.w - MARGIN - 16.0) * (d - 1.0) / (n - 1.0);
    let ys = |val: f64| r.y + 10.0 + (r.h - 34.0) * (g.hi - val) / (g.hi - g.lo);
    let right = r.x + r.w - MARGIN;
    let t = crate::px::t();
    for val in &g.marks {
        let y = ys(*val);
        crate::fill(pm, r.x, y - 0.5, right - r.x, 1.0, WHITE, 0.07);
        text.draw(pm, &axis_text(*val, g.step), ((right + 6.0) as f32, (y + 4.0) as f32), 11.0, AXIS, Align::Left);
    }
    // the balance on the 1st, dashed
    let mut pb = PathBuilder::new();
    let y0 = ys(v.start.to_f64());
    pb.move_to(r.x as f32, y0 as f32);
    pb.line_to(right as f32, y0 as f32);
    if let (Some(p), Some(dash)) = (pb.finish(), StrokeDash::new(vec![4.0, 4.0], 0.0)) {
        let s = Stroke {
            width: 1.0,
            dash: Some(dash),
            ..Stroke::default()
        };
        pm.stroke_path(&p, &paint(AXIS, 0.7), &s, t, None);
    }
    let c = if v.net >= Money::ZERO { UP } else { DOWN };
    let pts: Vec<(f32, f32)> = v
        .days
        .iter()
        .enumerate()
        .map(|(d, m)| (xs(d as f64 + 1.0) as f32, ys(m.to_f64()) as f32))
        .collect();
    if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
        // the area under the line, then the line
        let mut pb = PathBuilder::new();
        pb.move_to(first.0, first.1);
        for p in &pts[1..] {
            pb.line_to(p.0, p.1);
        }
        let line = pb.clone().finish();
        let bottom = ys(g.lo) as f32;
        pb.line_to(last.0, bottom);
        pb.line_to(first.0, bottom);
        pb.close();
        if let Some(area) = pb.finish().filter(|_| pts.len() > 1) {
            pm.fill_path(&area, &paint(c, 0.16), FillRule::Winding, t, None);
        }
        if let Some(line) = line.filter(|_| pts.len() > 1) {
            let s = Stroke {
                width: 2.0,
                line_cap: LineCap::Round,
                line_join: LineJoin::Round,
                ..Stroke::default()
            };
            pm.stroke_path(&line, &paint(c, 1.0), &s, t, None);
        }
        // the last point, full; the highest one, hollow
        if let Some(dot) = PathBuilder::from_circle(last.0, last.1, 3.5) {
            pm.fill_path(&dot, &paint(c, 1.0), FillRule::Winding, t, None);
        }
        let top = pts.iter().enumerate().fold(0, |m, (i, p)| if p.1 < pts[m].1 { i } else { m });
        if let Some(ring) = PathBuilder::from_circle(pts[top].0, pts[top].1, 3.0) {
            pm.stroke_path(
                &ring,
                &paint(c, 1.0),
                &Stroke {
                    width: 1.0,
                    ..Stroke::default()
                },
                t,
                None,
            );
        }
    }
    let m = MONTHS_SHORT[(v.month.clamp(1, 12) - 1) as usize];
    let days = curve_days(v.month_days.max(1));
    for (i, d) in days.iter().enumerate() {
        let a = match i {
            0 => Align::Left,
            4 => Align::Right,
            _ => Align::Center,
        };
        text.draw(pm, &format!("{d} {m}"), (xs(f64::from(*d)) as f32, (r.y + r.h - 6.0) as f32), 11.0, AXIS, a);
    }
}

fn emoji(pm: &mut Pixmap, e: &str, r: Rect) {
    if let Some(img) = crate::emoji::image(e, crate::px::dev(r.w).round() as u32) {
        crate::px::put(pm, r.x, r.y, &img);
    }
}

/// 📱 Is `y` (page coordinates) in the room at the page's bottom, under the Argos AI round? Nothing of the content is
/// there.
fn in_room(f: Frame, y: f64) -> bool {
    f.room > 0.0 && y >= f.h - f.room
}

impl MonthPage {
    /// The content's height on the frame `f` (scroll not counted).
    pub fn height(&self, v: &MonthView, text: &Text, f: Frame) -> f64 {
        layout(v, self, text, f).h
    }

    /// The scroll kept within the content.
    fn scroll_in(&self, l: &Layout, f: Frame) -> f64 {
        self.scroll.clamp(0.0, (l.h - f.h).max(0.0))
    }

    /// Wheel (or finger) on the page: `dy` points down.
    pub fn wheel(&mut self, dy: f64, v: &MonthView, text: &Text, f: Frame) {
        let l = layout(v, self, text, f);
        self.scroll = (self.scroll + dy).clamp(0.0, (l.h - f.h).max(0.0));
    }

    /// Is there something to click at (`x`, `y`) (page coordinates)? (🖥 the hand cursor.)
    pub fn over_button(&self, v: &MonthView, text: &Text, f: Frame, x: f64, y: f64) -> bool {
        if in_room(f, y) {
            return false;
        }
        let l = layout(v, self, text, f);
        let s = self.scroll_in(&l, f);
        l.zones.iter().any(|(r, ..)| r.contains(x, y + s))
    }

    /// A click at (`x`, `y`) (page coordinates).
    pub fn click(&mut self, v: &MonthView, text: &Text, f: Frame, x: f64, y: f64) -> MonthClick {
        if in_room(f, y) {
            return MonthClick::Nothing;
        }
        let l = layout(v, self, text, f);
        let s = self.scroll_in(&l, f);
        let Some(z) = l.zones.iter().rev().find(|(r, ..)| r.contains(x, y + s)).map(|z| z.2) else {
            return MonthClick::Nothing;
        };
        match z {
            Zone::Close => MonthClick::Close,
            Zone::Month(m) => {
                (self.month, self.scroll) = (Some(m), 0.0);
                MonthClick::Changed
            }
            Zone::Print => MonthClick::Print,
            Zone::Details => {
                self.details = !self.details;
                self.scroll = s;
                MonthClick::Changed
            }
            Zone::Card(k) => MonthClick::Card(k),
            Zone::Lesson(id) => MonthClick::Lesson(id),
        }
    }

    /// The page on the frame `f`; `mouse`: the pointer (page coordinates), for the hover.
    pub fn draw(&self, v: &MonthView, text: &Text, f: Frame, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let l = layout(v, self, text, f);
        let mut pm = crate::px::new(f.w.ceil() as u32, f.h.ceil() as u32)?;
        let all = Rect::new(0.0, 0.0, f.w, f.h);
        let (radius, bg) = if f.sheet { (0.0, SHEET) } else { (RADIUS, PANEL) };
        // 🫧 the mock-up's glass (without the glass style: a nearly full background under it, readable on the chart)
        if !crate::glass::on() {
            round_fill(&mut pm, all, radius, crate::jetons::FOND, 0.94);
        }
        round_fill(&mut pm, all, radius, bg.0, bg.1);
        let s = self.scroll_in(&l, f);
        // 📱 the content is drawn above the room at the bottom (the Argos AI round lies over it): nothing of it passes
        // under the round while it scrolls (Suivi 5)
        let vue_h = (f.h - f.room).max(1.0);
        let mut body = crate::px::new(f.w.ceil() as u32, vue_h.ceil() as u32)?;
        let at = |r: Rect| Rect::new(r.x, r.y - s, r.w, r.h);
        let seen = |y0: f64, y1: f64| y1 - s >= -40.0 && y0 - s <= vue_h + 40.0;
        let medium = Text::graisse(Graisse::Medium);
        let bold = Text::graisse(Graisse::Bold);
        for p in &l.pieces {
            match p {
                Piece::Fill(r, rad, (c, a), edge) if seen(r.y, r.y + r.h) => {
                    round_fill(&mut body, at(*r), *rad, *c, *a);
                    if let Some(e) = edge {
                        round_stroke(&mut body, at(*r), *rad, crate::jetons::sur_fond(*e), 1.0);
                    }
                }
                Piece::Text(x, y, t, size, c, g, a) if seen(*y - 30.0, *y + 8.0) => {
                    let face = match g {
                        Graisse::Regular => text,
                        Graisse::Medium => &medium,
                        Graisse::Bold => &bold,
                    };
                    face.draw(&mut body, t, (*x as f32, (*y - s) as f32), *size, *c, *a);
                }
                Piece::Tight(x, y, t, size, c, g) if seen(*y - 30.0, *y + 8.0) => {
                    let face = match g {
                        Graisse::Regular => text,
                        Graisse::Medium => &medium,
                        Graisse::Bold => &bold,
                    };
                    face.draw_espace(&mut body, t, (*x as f32, (*y - s) as f32), *size, *c, tight(*size));
                }
                Piece::Label(x, y, t) if seen(*y - 12.0, *y + 4.0) => {
                    text.draw_espace(&mut body, t, (*x as f32, (*y - s) as f32), 11.0, SOFT, 1.1);
                }
                Piece::Emoji(r, e) if seen(r.y, r.y + r.h) => emoji(&mut body, e, at(*r)),
                Piece::Round(r) if seen(r.y, r.y + r.h) => crate::aibtn::bouton_verre(&mut body, at(*r), false),
                Piece::Chip(r, t, ton) if seen(r.y, r.y + r.h) => crate::avis::pastille(&mut body, &bold, at(*r), t, *ton),
                Piece::Link(r, t) if seen(r.y, r.y + r.h) => crate::avis::lien(&mut body, text, at(*r), t, false),
                Piece::Curve(r) if seen(r.y, r.y + r.h) => draw_curve(&mut body, text, at(*r), v),
                Piece::Rule(y, x0, x1) if seen(*y, *y + 1.0) => crate::fill(&mut body, *x0, *y - s, x1 - x0, 1.0, WHITE, 0.06),
                _ => {}
            }
        }
        // the zone under the pointer, lit
        if let Some((r, rad, z)) = mouse.and_then(|(x, y)| l.zones.iter().rev().find(|(r, ..)| r.contains(x, y + s))) {
            round_fill(&mut body, at(*r), *rad, WHITE, if *z == Zone::Print { 0.14 } else { 0.06 });
        }
        crate::px::put(&mut pm, 0.0, 0.0, &body);
        if !f.sheet {
            // the content stays inside the rounded corners, then the edge
            if let (Some(mut mask), Some(path)) = (tiny_skia::Mask::new(pm.width(), pm.height()), crate::shapes::round_path(all, RADIUS)) {
                mask.fill_path(&path, tiny_skia::FillRule::Winding, true, crate::px::t());
                pm.apply_mask(&mask);
            }
            round_stroke(&mut pm, Rect::new(0.5, 0.5, f.w - 1.0, f.h - 1.0), RADIUS, crate::jetons::sur_fond(EDGE), 1.0);
        }
        // 🫧 the scroll bar: there is more, and where we are
        if l.h > f.h + 1.0 {
            let pouce = (vue_h * vue_h / (l.h - f.room) - 8.0).max(24.0);
            let y = 4.0 + (vue_h - 8.0 - pouce) * (s / (l.h - f.h)).clamp(0.0, 1.0);
            crate::jetons::barre_defilement(&mut pm, f.w - 3.0, y, pouce, false);
        }
        Some(pm)
    }
}

// — 📄 the printable page (Imprimer en PDF): a light A4 page, the mock-up's « imprimé » —

/// The characters that would break the page's HTML.
fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A sentence inside another (the printed paragraph): its first letter small, « : » said « , », without its final
/// period (« Garde le stop… : c'est lui… » → « garde le stop…, c'est lui… »).
fn inside(s: &str) -> String {
    let s = s.trim().trim_end_matches('.').replace(" : ", ", ");
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_lowercase().chain(c).collect(),
        None => String::new(),
    }
}

/// A part of the card inside the printed paragraph, its chip « N fois » said in it when the sentence does not say it
/// (« Stop éloigné à la main : −3,02 R… » → « stop éloigné à la main 2 fois, −3,02 R… »).
fn inside_part(p: &Partie) -> String {
    let mut s = p.texte.clone();
    if p.pastille.ends_with("fois") && !s.contains(&p.pastille) {
        if let Some(k) = s.find(" : ") {
            s.insert_str(k, &format!(" {}", p.pastille));
        }
    }
    inside(&s)
}

/// The print's class of a value: green, red or none.
fn tone(x: f64) -> &'static str {
    if x > 0.0049 {
        "up"
    } else if x < -0.0049 {
        "down"
    } else {
        ""
    }
}

/// 📈 The curve of the printed page (svg, light colours: the mock-up's `courbe(692, 170, { clair: true })`).
fn print_curve(v: &MonthView, w: f64, h: f64) -> String {
    const MARGIN: f64 = 46.0;
    let g = curve_geo(v);
    let n = f64::from(v.month_days.max(2));
    let xs = |d: f64| 8.0 + (w - MARGIN - 16.0) * (d - 1.0) / (n - 1.0);
    let ys = |val: f64| 10.0 + (h - 34.0) * (g.hi - val) / (g.hi - g.lo);
    let c = if v.net >= Money::ZERO { "#13955f" } else { "#c62a35" };
    let mut s =
        format!(r#"<svg width="{w}" height="{h}" viewBox="0 0 {w} {h}" xmlns="http://www.w3.org/2000/svg" font-family="Inter,DejaVu Sans,sans-serif">"#);
    for val in &g.marks {
        let y = ys(*val);
        s += &format!(
            r##"<line x1="0" x2="{:.1}" y1="{y:.1}" y2="{y:.1}" stroke="#d9dde6"/><text x="{:.1}" y="{:.1}" fill="#6b7385" font-size="11">{}</text>"##,
            w - MARGIN,
            w - MARGIN + 6.0,
            y + 4.0,
            esc(&axis_text(*val, g.step))
        );
    }
    let y0 = ys(v.start.to_f64());
    s += &format!(
        r##"<line x1="0" x2="{:.1}" y1="{y0:.1}" y2="{y0:.1}" stroke="#6b7385" stroke-dasharray="4 4" opacity=".7"/>"##,
        w - MARGIN
    );
    let pts: Vec<(f64, f64)> = v.days.iter().enumerate().map(|(d, m)| (xs(d as f64 + 1.0), ys(m.to_f64()))).collect();
    if let (Some(first), Some(last)) = (pts.first(), pts.last()) {
        let d: Vec<String> = pts
            .iter()
            .enumerate()
            .map(|(i, p)| format!("{}{:.1},{:.1}", if i == 0 { "M" } else { "L" }, p.0, p.1))
            .collect();
        let d = d.join(" ");
        let bottom = ys(g.lo);
        s += &format!(
            r#"<path d="{d} L{:.1},{bottom:.1} L{:.1},{bottom:.1} Z" fill="{c}" opacity=".12"/>"#,
            last.0, first.0
        );
        s += &format!(r#"<path d="{d}" fill="none" stroke="{c}" stroke-width="2"/>"#);
        s += &format!(r#"<circle cx="{:.1}" cy="{:.1}" r="3.5" fill="{c}"/>"#, last.0, last.1);
        let top = pts.iter().enumerate().fold(0, |m, (i, p)| if p.1 < pts[m].1 { i } else { m });
        s += &format!(r#"<circle cx="{:.1}" cy="{:.1}" r="3" fill="none" stroke="{c}"/>"#, pts[top].0, pts[top].1);
    }
    let m = MONTHS_SHORT[(v.month.clamp(1, 12) - 1) as usize];
    for (i, d) in curve_days(v.month_days.max(1)).iter().enumerate() {
        let anchor = match i {
            0 => "start",
            4 => "end",
            _ => "middle",
        };
        s += &format!(
            r##"<text x="{:.1}" y="{:.1}" fill="#6b7385" font-size="11" text-anchor="{anchor}">{d} {m}</text>"##,
            xs(f64::from(*d)),
            h - 6.0
        );
    }
    s + "</svg>"
}

/// The style of the printed page (the mock-up's imprime.html), inside `#argos-imprime` (the /app/ page keeps its own).
const PRINT_STYLE: &str = r#"@page{size:A4;margin:0}
#argos-imprime .page{box-sizing:border-box;width:794px;min-height:1122px;padding:38px 40px 22px;display:flex;flex-direction:column;background:#fff;color:#1c2333;font-family:Inter,"DejaVu Sans",sans-serif;-webkit-print-color-adjust:exact;print-color-adjust:exact}
#argos-imprime *{box-sizing:border-box}
#argos-imprime .hd{display:flex;justify-content:space-between;align-items:flex-end;padding-bottom:14px;border-bottom:2px solid #1c2333}
#argos-imprime .marque{font-size:10.5px;letter-spacing:.22em;color:#6b7385;font-weight:700}
#argos-imprime .titre{font-size:26px;font-weight:700;margin-top:4px;letter-spacing:-.01em}
#argos-imprime .mois{font-size:17px;font-weight:600;color:#3a4357;margin-top:2px}
#argos-imprime .droite{text-align:right;font-size:11.5px;color:#6b7385;line-height:1.55}
#argos-imprime .droite b{color:#1c2333;font-weight:600}
#argos-imprime .chiffres{display:grid;grid-template-columns:repeat(4,1fr);gap:10px;margin-top:16px}
#argos-imprime .ch{border:1px solid #dfe3ec;border-radius:10px;padding:10px 12px 9px}
#argos-imprime .lb{font-size:9.5px;letter-spacing:.12em;color:#6b7385;text-transform:uppercase;font-weight:600}
#argos-imprime .big{font-size:21px;font-weight:700;margin:5px 0 3px;white-space:nowrap}
#argos-imprime .pt{font-size:10.5px;color:#6b7385}
#argos-imprime .up{color:#13955f}
#argos-imprime .down{color:#c62a35}
#argos-imprime .sec{font-size:12.5px;font-weight:700;margin:16px 0 6px;display:flex;justify-content:space-between;align-items:baseline}
#argos-imprime .sec span{font-size:10.5px;font-weight:400;color:#6b7385}
#argos-imprime .courbe{border:1px solid #dfe3ec;border-radius:10px;padding:8px 10px 2px}
#argos-imprime .ai{display:flex;gap:12px;margin-top:14px;padding:11px 14px;border-radius:10px;background:#f4f6fb;border:1px solid #e3e7f0}
#argos-imprime .ai img{width:26px;height:26px;border-radius:50%;background:#1c2333;padding:4px;flex:none}
#argos-imprime .ai .t{font-size:11.5px;line-height:1.5}
#argos-imprime .ai .t b{font-weight:700}
#argos-imprime .ai .rap{font-size:10px;color:#6b7385;margin-top:3px}
#argos-imprime table{width:100%;border-collapse:collapse;font-size:11px}
#argos-imprime thead{display:table-header-group}
#argos-imprime tr{break-inside:avoid}
#argos-imprime th{font-size:9.5px;letter-spacing:.08em;text-transform:uppercase;color:#6b7385;font-weight:600;text-align:left;padding:5px 6px;border-bottom:1px solid #1c2333}
#argos-imprime td{padding:5px 6px;border-bottom:1px solid #eceff4;white-space:nowrap}
#argos-imprime td.n,#argos-imprime th.n{text-align:right}
#argos-imprime td.c,#argos-imprime th.c{text-align:center}
#argos-imprime .rep{color:#6b7385;font-size:10px}
#argos-imprime .ok{color:#13955f;font-weight:700}
#argos-imprime .mid{color:#b7791f;font-weight:700}
#argos-imprime .ko{color:#c62a35;font-weight:700}
#argos-imprime .nn{color:#9aa1b1}
#argos-imprime .det{display:grid;grid-template-columns:1fr 1fr;gap:16px;margin-top:4px}
#argos-imprime .det table td{padding:3px 6px}
#argos-imprime .kv{font-size:11px;line-height:1.75}
#argos-imprime .kv span{color:#6b7385}
#argos-imprime .pied{margin-top:auto;display:flex;justify-content:space-between;font-size:9.5px;color:#6b7385;border-top:1px solid #dfe3ec;padding-top:8px}"#;

/// A month fits on one A4 page up to this many trades (beyond, the list goes on to the next page).
const ONE_PAGE: usize = 20;

/// 📄 The printable page of the month (HTML, light, A4): a `<style>` and the page, to put in `#argos-imprime`
/// (the /app/ page prints it; see [`printable_document`] for a file). `printed`: « sam. 21 nov. 2026 à 18:20 ».
pub fn printable(v: &MonthView, printed: &str) -> String {
    let m = MONTHS_SHORT[(v.month.clamp(1, 12) - 1) as usize];
    let month = month_label(v.year, v.month, false);
    let mut cap = month.chars();
    let month = cap.next().map(|c| c.to_uppercase().chain(cap).collect::<String>()).unwrap_or_default();
    let mut h = format!("<style>{PRINT_STYLE}</style><div class=\"page\">");
    h += &format!(
        "<div class=\"hd\"><div><div class=\"marque\">ARGOS HSS TRADING</div><div class=\"titre\">Rapport du mois</div><div class=\"mois\">{}</div></div>\
         <div class=\"droite\"><b>Compte démo · {}</b><br>Du 1<sup>er</sup> au {} {m} {} · {}<br>Imprimé le {}</div></div>",
        esc(&month),
        esc(&v.account),
        v.last_day(),
        v.year,
        esc(&v.zone),
        esc(printed)
    );
    // the four figures
    h += "<div class=\"chiffres\">";
    for (k, (label, big, _, sub)) in v.tiles().into_iter().enumerate() {
        let class = match k {
            0 => tone(v.net.to_f64()),
            1 if v.r_avg.is_some() => tone(v.r),
            _ => "",
        };
        let sub = esc(&sub).replace("1ᵉʳ\u{a0}", "1<sup>er</sup> ");
        h += &format!(
            "<div class=\"ch\"><div class=\"lb\">{}</div><div class=\"big {class}\">{}</div><div class=\"pt\">{sub}</div></div>",
            esc(label),
            esc(&big)
        );
    }
    h += "</div>";
    // the curve
    let today = if v.days.len() as u32 >= v.month_days { "fin du mois" } else { "aujourd'hui" };
    h += &format!(
        "<div class=\"sec\">Courbe du compte <span>solde le 1er : {} · {today} : {} · plus haut : {}</span></div><div class=\"courbe\">{}</div>",
        esc(&argent(v.start, false)),
        esc(&argent(v.end(), false)),
        esc(&argent(v.high, false)),
        print_curve(v, 692.0, 170.0)
    );
    // Argos AI · ton mois
    let logo = argos_core::retours::base64_ecrire(crate::aibtn::LOGO_COULEUR);
    let (said, reminder) = match &v.ai {
        Some(a) => {
            let mut t = format!(
                "<b>Argos AI · ton mois.</b> Ce qui a marché : {}. À revoir : {}. Une leçon : {}",
                esc(&inside_part(&a.worked)),
                esc(&inside_part(&a.to_review)),
                esc(&inside(&a.lesson))
            );
            if let Some(id) = a.academy {
                t += &format!(" (Académie : « {} »)", esc(crate::avis::titre_lecon(id)));
            }
            (t + ".", a.reminder.clone())
        }
        None => (
            "<b>Argos AI · ton mois.</b> Pas encore de trade fini ce mois-ci.".to_string(),
            "Analyse affichée, pas un ordre.".to_string(),
        ),
    };
    h += &format!(
        "<div class=\"ai\"><img alt=\"\" src=\"data:image/png;base64,{logo}\"><div><div class=\"t\">{said}</div><div class=\"rap\">{}</div></div></div>",
        esc(&reminder)
    );
    // the month's trades
    let hint = if v.replays() > 0 {
        "<span>⏪ : trade de replay, compté le jour où tu l'as fait (date du marché à côté)</span>"
    } else {
        ""
    };
    h += &format!(
        "<div class=\"sec\">Les trades du mois {hint}</div><table><thead><tr><th>Date</th><th>Heure</th><th>Marché</th><th>Sens</th><th>Sortie</th>\
         <th class=\"n\">R</th><th class=\"n\">Coûts</th><th class=\"n\">Net</th><th class=\"c\">Plan</th></tr></thead><tbody>"
    );
    for t in &v.trades {
        let date = match &t.market {
            Some(mk) => format!("⏪ {} <span class=\"rep\">(marché {})</span>", esc(&t.day), esc(mk)),
            None => esc(&t.day),
        };
        let (r, rc) = t.r.map_or(("—".to_string(), ""), |r| (r_text(r), tone(r)));
        let (sign, class) = match t.plan {
            PlanMark::Followed => ("✓", "ok"),
            PlanMark::Partly => ("~", "mid"),
            PlanMark::NotFollowed => ("✗", "ko"),
            PlanMark::NotJudged => ("—", "nn"),
        };
        h += &format!(
            "<tr><td>{date}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"n {rc}\">{}</td><td class=\"n\">{}</td>\
             <td class=\"n {}\">{}</td><td class=\"c {class}\">{sign}</td></tr>",
            esc(&t.time),
            esc(&t.base),
            if t.long { "▲ achat" } else { "▼ vente" },
            esc(&t.exit),
            esc(&r),
            esc(&argent(t.costs, false)),
            tone(t.net.to_f64()),
            esc(&argent(t.net, true))
        );
    }
    if v.trades.is_empty() {
        h += "<tr><td colspan=\"9\">Aucun trade fermé ce mois-ci.</td></tr>";
    }
    h += "</tbody></table>";
    // the details
    h += "<div class=\"sec\">Détails</div><div class=\"det\"><table><thead><tr><th>Par symbole</th><th class=\"n\">Trades</th><th class=\"n\">R</th><th class=\"n\">Net</th></tr></thead><tbody>";
    for s in &v.symbols {
        h += &format!(
            "<tr><td>{}</td><td class=\"n\">{}</td><td class=\"n {}\">{}</td><td class=\"n {}\">{}</td></tr>",
            esc(&s.base),
            s.trades,
            tone(s.r),
            esc(&r_text(s.r)),
            tone(s.net.to_f64()),
            esc(&argent(s.net, true))
        );
    }
    h += "</tbody></table><div class=\"kv\">";
    let mut costs = format!("frais {} · écart et glissement {}", argent(v.fees, false), argent(v.slippage, false));
    if v.funding != Money::ZERO {
        costs += &format!(" · funding {}", argent(v.funding, false));
    }
    h += &format!(
        "<span>Coûts payés :</span> {} ({})<br>",
        esc(&argent(v.fees + v.slippage + v.funding, false)),
        esc(&costs)
    );
    let dd = if v.drawdown > Money::ZERO {
        format!(
            "{} ({}), {}",
            argent(Money::ZERO - v.drawdown, true),
            pct_text(-v.drawdown_pct),
            v.days_text(v.drawdown_days)
        )
    } else {
        "aucune".to_string()
    };
    h += &format!("<span>Plus forte baisse :</span> {}<br>", esc(&dd));
    let streak = match v.losing_streak {
        0 => "aucune".to_string(),
        n => format!(
            "{n} trade{} ({})",
            if n > 1 { "s" } else { "" },
            v.days_text(v.streak_days).trim_start_matches("du ")
        ),
    };
    h += &format!("<span>Plus longue série de pertes :</span> {}", esc(&streak));
    for (label, k) in [("Meilleur trade", v.best), ("Pire trade", v.worst)] {
        if let Some(t) = k.and_then(|k| v.trades.get(k)) {
            let r = t.r.map_or(String::new(), |r| format!(", {}", r_text(r)));
            h += &format!(
                "<br><span>{label} :</span> {} {}, {}{} {}",
                if t.long { "▲" } else { "▼" },
                esc(&t.base),
                esc(&t.day),
                esc(&r),
                esc(&format!("({})", argent(t.net, true)))
            );
        }
    }
    h += "</div></div>";
    let pages = if v.trades.len() <= ONE_PAGE { " · page 1 / 1" } else { "" };
    h += &format!(
        "<div class=\"pied\"><span>Simulation sur compte démo : pas un conseil financier. Aucun ordre réel n'a été passé.</span><span>Argos HSS Trading{pages}</span></div></div>"
    );
    h
}

/// 📄 The printable page as a whole HTML file (🖥 the desktop app writes it to Downloads and opens it: the browser's
/// print window opens with it).
pub fn printable_document(v: &MonthView, printed: &str) -> String {
    format!(
        "<!doctype html><html lang=\"fr\"><head><meta charset=\"utf-8\"><title>Argos · Rapport du mois · {}</title>\
         <style>body{{margin:0;background:#fff}}</style></head><body><div id=\"argos-imprime\">{}</div>\
         <script>addEventListener(\"load\",()=>setTimeout(()=>print(),200))</script></body></html>",
        esc(&month_label(v.year, v.month, false)),
        printable(v, printed)
    )
}

#[cfg(test)]
mod tests;
