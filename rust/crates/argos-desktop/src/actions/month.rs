//! 📅 Lot 8 of the Simulator, « Rapport du mois » (mock-up v1, Jee's « ok maquette lot 8 » of 21 Nov): the month of
//! the demo account on one glass page, opened by « 📅 Rapport du mois » in the Simulator panel (🖥 over the chart, left
//! of the panel; 📱 the whole screen). Its figures come from `argos_core::month` (each trade in the month of its real
//! last exit, in the chart's time zone), « Argos AI · ton mois » from `argos_ai::month_review`. « 📄 Imprimer en PDF »
//! prints its light A4 page: the browser's print window (« Enregistrer en PDF »); 🖥 the page goes to Downloads and
//! opens in the browser, which prints it.
//!
//! Never an order: the page only reads the account's history.

use crate::*;
use argos_core::portfolio::ClosedTrade;
use argos_render::month::{Frame, MonthAi, MonthClick, MonthPage, MonthView, PlanMark, SymbolRow, TradeRow};
use argos_render::text::Text;

/// 📱 Room kept under the page's content: the Argos AI round over the bottom right (the mock-up's).
const PHONE_ROOM: f64 = 76.0;
/// 🖥 The page's margins: to the chart's left side and to the Simulator panel (narrowed down to `MIN_SIDE` before
/// the page gets narrower than the mock-up's: its tiles keep their lines), under the top bar (narrowed down to
/// `MIN_TOP` before the page has to scroll: a short window), above the window's bottom.
const SIDE: f64 = 30.0;
const MIN_SIDE: f64 = 16.0;
const TOP: f64 = 40.0;
const MIN_TOP: f64 = 16.0;
const BOTTOM: f64 = 16.0;
/// 🖥 With less room than this left of the panel, the page goes over the panel.
const MIN_ROOM: f64 = 760.0;
/// 🖥 The narrowest page.
const MIN_W: f64 = 320.0;

/// « heure de New York », « heure d'Athènes », « heure des Açores », « heure UTC »: the chart's time zone `tz` (IANA
/// name), named as in the list of time zones.
pub(crate) fn zone_words(tz: &str) -> String {
    let city = argos_render::settings::TIMEZONES
        .iter()
        .find(|(_, z)| *z == tz)
        .map_or_else(|| tz.rsplit('/').next().unwrap_or(tz).replace('_', " "), |(c, _)| c.to_string());
    match city.as_str() {
        "UTC" | "Marché" => "heure UTC".into(),
        "Açores" => "heure des Açores".into(),
        "Ville de Mexico" => "heure de Mexico".into(),
        c if c.starts_with(['A', 'E', 'I', 'O', 'U', 'Â', 'À', 'É', 'È', 'Ê', 'Î', 'Ô', 'Û']) => format!("heure d'{c}"),
        c => format!("heure de {c}"),
    }
}

/// A finished trade of the month as the page lists it: `p`, its exits in `history`; `off(t)`, the time zone's offset.
fn trade_row(history: &[ClosedTrade], p: &[usize], off: &impl Fn(i64) -> i64) -> Option<TradeRow> {
    let last = *p.last()?;
    let c = &history[last];
    let parts: Vec<&ClosedTrade> = p.iter().map(|&i| &history[i]).collect();
    let t = c.real_close();
    let local = t + off(t);
    let s = local.rem_euclid(86_400);
    Some(TradeRow {
        long: c.side == argos_core::Side::Long,
        base: c.base.clone(),
        exit: c.reason.clone(),
        day: argos_render::month::day_label(local),
        time: format!("{:02}:{:02}", s / 3600, s % 3600 / 60),
        // ⏪ a replay trade: the market's date then
        market: c.replay().then(|| argos_render::month::date_label(c.closed_at + off(c.closed_at))),
        r: argos_core::month::trade_r(history, p),
        net: argos_core::month::trade_net(history, p),
        costs: parts.iter().map(|c| c.fees + c.funding).sum(),
        plan: match argos_ai::review(&parts).map(|r| r.verdict) {
            Some(argos_ai::Verdict::Suivi) => PlanMark::Followed,
            Some(argos_ai::Verdict::EnPartie) => PlanMark::Partly,
            Some(argos_ai::Verdict::PasSuivi) => PlanMark::NotFollowed,
            _ => PlanMark::NotJudged,
        },
        last,
    })
}

impl App {
    /// The chart's time zone offset (seconds) at the time `t`, daylight saving time included (a zone unknown to the
    /// list: the offset used now).
    fn month_offset(&self) -> impl Fn(i64) -> i64 {
        let (tz, now) = (self.fuseau().to_string(), self.tz_offset_s());
        move |t| argos_render::settings::tz_offset(&tz, t).unwrap_or(now)
    }

    /// 📅 What the page shows: the month chosen with ‹ ›, otherwise the current month (when it has trades, or when no
    /// month has any), otherwise the last month with trades.
    pub(crate) fn month_view(&self, page: &MonthPage) -> MonthView {
        use argos_core::month as m;
        let off = self.month_offset();
        let h = &self.demo.history;
        let now = now_s();
        let months = m::months(h, &off);
        let (y, mo, _) = m::date_of(now, off(now));
        let shown = page.month.unwrap_or_else(|| match months.last() {
            Some(&last) if !months.contains(&(y, mo)) => last,
            _ => (y, mo),
        });
        let rep = m::month_report(h, self.demo.start, shown, now, &off);
        let parts: Vec<Vec<&ClosedTrade>> = rep.trades.iter().map(|p| p.iter().map(|&i| &h[i]).collect()).collect();
        let review = argos_ai::month_review(&parts);
        let n = |f: fn(&argos_ai::MonthReview) -> usize| review.as_ref().map_or(0, f);
        MonthView {
            account: self.demo.name.clone(),
            zone: zone_words(self.fuseau()),
            year: shown.0,
            month: shown.1,
            prev: months.iter().rev().find(|x| **x < shown).copied(),
            next: months.iter().find(|x| **x > shown).copied(),
            start: rep.start,
            net: rep.net,
            pct: rep.pct,
            r: rep.r,
            r_avg: rep.r_avg(),
            winners: rep.winners,
            judged: n(|r| r.judged),
            followed: n(|r| r.followed),
            partly: n(|r| r.partly),
            not_followed: n(|r| r.not_followed),
            days: rep.days.clone(),
            month_days: m::days_in_month(shown.0, shown.1),
            high: rep.high,
            drawdown: rep.drawdown,
            drawdown_pct: rep.drawdown_pct,
            drawdown_days: rep.drawdown_days,
            losing_streak: rep.losing_streak,
            streak_days: rep.streak_days,
            fees: rep.fees,
            slippage: rep.slippage,
            funding: rep.funding,
            trades: rep.trades.iter().filter_map(|p| trade_row(h, p, &off)).collect(),
            best: rep.best,
            worst: rep.worst,
            symbols: rep
                .symbols
                .iter()
                .map(|s| SymbolRow {
                    base: s.base.clone(),
                    trades: s.trades,
                    r: s.r,
                    net: s.net,
                })
                .collect(),
            ai: review.map(|r| MonthAi {
                worked: super::avis::partie(r.worked),
                to_review: super::avis::partie(r.to_review),
                lesson: r.lesson,
                academy: r.academy,
                reminder: r.reminder.to_string(),
            }),
        }
    }

    /// The window's size (points); 🧪 without a window, the test's image size, otherwise the screen of the layout.
    fn month_window(&self) -> (f64, f64) {
        match (&self.window, self.ui.taille_test) {
            (Some(w), _) => {
                let s = ui_size(w);
                (s.width as f64, s.height as f64)
            }
            (None, Some(s)) => (s.width as f64, s.height as f64),
            (None, None) => (self.win_w(), self.cell_h()),
        }
    }

    /// 📅 Where the page can be, when it shows (its content not measured: 🖥 from `MIN_TOP` under the top bar down to
    /// the window's bottom): 🖥 over the chart, left of the Simulator panel (centred, 960 wide at most; little room:
    /// over the panel); 📱 the whole screen. A page over the chart (Accueil, Académie…) hides it.
    fn month_box(&self) -> Option<(Rect, Frame)> {
        self.ui.sim.as_ref()?.month.as_ref()?;
        if self.page_ouverte() {
            return None;
        }
        let (ww, wh) = self.month_window();
        if self.top.phone {
            let f = Frame {
                w: ww,
                h: wh,
                sheet: true,
                room: PHONE_ROOM,
            };
            return Some((Rect::new(0.0, 0.0, ww, wh), f));
        }
        let left = self.rw();
        let mut right = self.sim_rect().x;
        if right - left < MIN_ROOM {
            right = ww;
        }
        let side = ((right - left - argos_render::month::PANEL_W) / 2.0).clamp(MIN_SIDE, SIDE);
        let w = (right - left - 2.0 * side).clamp(MIN_W, argos_render::month::PANEL_W);
        let (x, y) = (left + ((right - left - w) / 2.0).max(0.0), self.top_h() + MIN_TOP);
        let f = Frame {
            w,
            h: (wh - y - BOTTOM).max(120.0),
            sheet: false,
            room: 0.0,
        };
        Some((Rect::new(x, y, w, f.h), f))
    }

    /// 📅 The page (its state and what it shows), its place in the window and its frame, when it shows (🖥 as high as
    /// its content, `TOP` under the top bar; a short window: higher, down to `MIN_TOP`, before it scrolls).
    pub(crate) fn month_place(&self) -> Option<(MonthPage, MonthView, Rect, Frame)> {
        let (mut r, mut f) = self.month_box()?;
        let page = self.ui.sim.as_ref()?.month.clone()?;
        let v = self.month_view(&page);
        if !f.sheet {
            let h = page.height(&v, &Text::default(), f);
            let dy = (f.h - h).clamp(0.0, TOP - MIN_TOP);
            (r.y, f.h) = (r.y + dy, h.min(f.h - dy));
            r.h = f.h;
        }
        Some((page, v, r, f))
    }

    /// 📅 Draws the page (and gives its place).
    pub(crate) fn month_draw(&self, text: &Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let (page, v, r, f) = self.month_place()?;
        let mouse = self.survol(Some(self.pointer.win_mouse)).map(|(x, y)| (x - r.x, y - r.y));
        Some((page.draw(&v, text, f, mouse)?, r))
    }

    /// 📅 Is the page under this point (window), with nothing over it? The menus, the search and the Argos AI button
    /// are drawn over it: they take the click, the wheel and the cursor before it.
    pub(crate) fn month_under(&self, x: f64, y: f64) -> bool {
        self.ui.sim.as_ref().is_some_and(|s| s.month.is_some())
            && !self.ui.context.is_open()
            && self.ui.palette.is_none()
            && !self.ui.list.as_ref().is_some_and(|(_, m)| m.rect.contains(x, y))
            && !self.ai_bouton_rect().contains(x, y)
            // (the content measured only when the point may be on it: 🖥 under its top, between its sides)
            && self.month_box().is_some_and(|(r, f)| r.contains(x, y) && (f.sheet || self.month_place().is_some_and(|(.., p, _)| p.contains(x, y))))
    }

    /// 📅 On the page (nothing over it): `Some(true)` on its buttons (the hand), `Some(false)` elsewhere (the arrow).
    pub(crate) fn month_hand(&self, x: f64, y: f64) -> Option<bool> {
        if !self.month_under(x, y) {
            return None;
        }
        let (page, v, r, f) = self.month_place()?;
        Some(page.over_button(&v, &Text::default(), f, x - r.x, y - r.y))
    }

    /// 📅 Closes the page (the Simulator panel stays).
    pub(crate) fn month_close(&mut self) {
        if let Some(s) = self.ui.sim.as_mut() {
            s.month = None;
        }
    }

    /// 📅 « Rapport du mois » in the panel: the page opens, or closes; the lot 7 card it would cover closes.
    pub(crate) fn month_toggle(&mut self) {
        if let Some(s) = self.ui.sim.as_mut() {
            s.month = match s.month.take() {
                Some(_) => None,
                None => Some(MonthPage::default()),
            };
        }
        self.ui.avis = None;
    }

    /// 📅 A click on the page: ✕ closes it; ‹ › another month; « Tous les détails » unfolds; a trade's Argos AI round
    /// opens its lot 7 card (the page closes to show it); 🎓 the Academy lesson; 📄 the printing.
    pub(crate) fn month_click(&mut self, x: f64, y: f64) {
        let Some((mut page, v, r, f)) = self.month_place() else { return };
        let c = page.click(&v, &Text::default(), f, x - r.x, y - r.y);
        if let Some(s) = self.ui.sim.as_mut() {
            s.month = Some(page);
        }
        match c {
            MonthClick::Nothing | MonthClick::Changed => {}
            MonthClick::Close => self.month_close(),
            MonthClick::Print => self.month_print(&v),
            MonthClick::Card(k) => {
                self.month_close();
                if let Some(c) = self.demo.history.get(k).filter(|h| !h.partiel) {
                    self.ui.avis = Some(super::avis::AvisDe::de(c));
                }
                // 📱 the panel covers the screen: it closes, the card shows on the chart
                if self.top.phone {
                    self.ui.sim = None;
                }
            }
            MonthClick::Lesson(id) => {
                self.month_close();
                return self.palette_cible(argos_render::recherche::Cible::Lecon(id.to_string()));
            }
        }
        self.redraw();
    }

    /// 📅 Wheel (or finger) on the page: it scrolls (and the chart under it does not move).
    pub(crate) fn month_wheel(&mut self, dy: f64) -> bool {
        let (x, y) = self.pointer.win_mouse;
        if !self.month_under(x, y) {
            return false;
        }
        let Some((mut page, v, _, f)) = self.month_place() else { return false };
        page.wheel(dy, &v, &Text::default(), f);
        if let Some(s) = self.ui.sim.as_mut() {
            s.month = Some(page);
        }
        self.redraw();
        true
    }

    /// 📄 « Imprimer en PDF »: the light A4 page of the month, printed on `printed` (now, in the chart's time zone).
    fn month_print(&mut self, v: &MonthView) {
        let off = self.month_offset();
        let now = now_s();
        let local = now + off(now);
        let (year, ..) = argos_core::month::date_of(now, off(now));
        let s = local.rem_euclid(86_400);
        let printed = format!("{} {year} à {:02}:{:02}", argos_render::month::day_label(local), s / 3600, s % 3600 / 60);
        let name = format!("argos-rapport-du-mois-{}-{:02}.html", v.year, v.month);
        #[cfg(target_arch = "wasm32")]
        let page = argos_render::month::printable(v, &printed);
        #[cfg(not(target_arch = "wasm32"))]
        let page = argos_render::month::printable_document(v, &printed);
        self.banner = Some(match platform::print_page(&name, &page) {
            Ok(_) if cfg!(target_arch = "wasm32") => "📄 Choisis « Enregistrer en PDF » dans la fenêtre d'impression".into(),
            Ok(dir) => format!("📄 Rapport du mois dans {dir} : ton navigateur l'ouvre, choisis « Enregistrer en PDF »"),
            Err(e) => format!("📄 Impression impossible : {e}"),
        });
    }
}
