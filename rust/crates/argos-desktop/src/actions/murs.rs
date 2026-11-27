//! 🧱 Argos Murs in the app: the books of the chart's coin read every 10 s while the indicator is on (crypto only),
//! the walls followed (perp and spot apart), their events recorded (desktop: `murs.redb`, 30 days; browser: this
//! session only) and, in replay, the walls rebuilt from the recording at the replay clock (never past it).

use crate::*;
use argos_core::murs::{self, Cfg, Depth, Event, Kind, Tracker, Vue};
use argos_data::murs::Carnet;
use argos_render::indicators::Ind;
use argos_render::murs as rm;

/// The depth bars use a grid this many times finer than the walls' (BTC: 5 $ against 100 $), so the 4 px rows near
/// the price are filled by real levels even when the chart is zoomed in.
const FINE: f64 = 20.0;

/// Room between the phone's legend and the first pill (pixels).
const GAP_LEGENDE: f64 = 6.0;

/// REMPLI: the trades counted are those of the last 5 minutes before the wall went.
const REMPLI_FENETRE_MS: i64 = 5 * 60_000;

/// Books read every 10 s.
const PERIOD_S: u64 = 10;
/// A recording heartbeat every minute: replay knows which periods were recorded.
const ALIVE_MS: i64 = 60_000;

/// What Argos Murs keeps while it is on.
pub(crate) struct MursState {
    pub(crate) coin: String,
    /// 🫧 The book's pressure (sells' share), smoothed over 30 s, and its time (ms): the bubble (F1g). A new coin
    /// starts a new state, so it starts again.
    pub(crate) pression: std::cell::Cell<Option<(f64, i64)>>,
    pub(crate) perp: Tracker,
    pub(crate) spot: Tracker,
    pub(crate) at: Option<Instant>,
    pub(crate) busy: bool,
    /// Books asked and the ones that answered last time.
    pub(crate) asked: Vec<Carnet>,
    pub(crate) ok: Vec<Carnet>,
    pub(crate) depth_perp: Depth,
    pub(crate) depth_spot: Depth,
    /// The same books on a grid `FINE` times finer: the depth bars (one 3 px bar per step, mockup v5).
    pub(crate) bars_perp: Depth,
    pub(crate) bars_spot: Depth,
    pub(crate) step: f64,
    pub(crate) last_px: f64,
    /// Last bar seen (time, high, low): a new time closes it (SWEEP), a new extreme is part of the price path.
    pub(crate) last_bar: Option<argos_core::Bar>,
    pub(crate) alive_ms: i64,
    pub(crate) purged_ms: i64,
    /// Events of this session (the browser's only record; on the desktop also in `murs.redb`).
    pub(crate) mem: Vec<Event>,
    /// Replay: the recorded events of the coin, read once per replay.
    pub(crate) replay: Option<Vec<Event>>,
    /// Grey words recorded before this session (desktop, `murs.redb`): the chart does not start empty each time.
    pub(crate) anciens: Vec<murs::Word>,
    /// REMPLI marks: walls really filled by trades (this session, and the recorded ones still young).
    pub(crate) remplis: Vec<murs::Rempli>,
    /// Walls still shown when the recording stopped (desktop, `murs.redb`): a wall seen again starts its line where it
    /// was first seen then, not at this session's first photo.
    pub(crate) debuts: Vec<murs::Pill>,
    /// REMPLI: the chart exchange's own book in the photos of the last `REMPLI_FENETRE_MS` (its part of a wall before
    /// the hit), and its latest share of the summed books.
    pub(crate) photos_chart: Vec<(i64, murs::VenueBook)>,
    pub(crate) part_chart: Option<f64>,
    /// The books answered at least once (« Lecture des carnets… » until then).
    pub(crate) recu: bool,
    /// Each book's last good snapshot and when it was read (ms): a book whose request fails (HTTP error, 429, network,
    /// parse) is replaced by it while it is younger than `CARNET_GARDE_MS`; its walls do not vanish for one bad photo.
    pub(crate) derniers: Vec<(Carnet, i64, murs::VenueBook)>,
    /// Books that answered HTTP 429 (too many requests): photos still to skip (`PAUSE_429`).
    pub(crate) pause: Vec<(Carnet, u8)>,
    /// 🧱 Far walls (lot loin-1): photos taken (the far book is asked on photos 0, 3, 6…), the far walls shown, far
    /// photos still to skip after a 429, and far photos failed in a row (`LOIN_ECHECS` clear the far walls).
    pub(crate) photos: u64,
    pub(crate) loin: Vec<murs::Lointain>,
    pub(crate) loin_pause: u8,
    pub(crate) loin_echecs: u8,
    /// 📖 Binance perp's book merged from its photos and its diff depth stream (lot loin-1 B); `None`: no stream
    /// (Binance = its photo alone).
    pub(crate) flux: Option<FluxBinance>,
}

/// 📖 Binance perp's merged book and whether its stream is connected.
#[derive(Default)]
pub(crate) struct FluxBinance {
    pub(crate) carnet: murs::CarnetFusionne,
    pub(crate) connecte: bool,
}

/// A book that did not answer is replaced by its last good snapshot while it is younger than this (ms).
pub(crate) const CARNET_GARDE_MS: i64 = 60_000;
/// After an HTTP 429, an exchange is not asked for this many photos (its last snapshot fills the gap).
pub(crate) const PAUSE_429: u8 = 2;
/// 🧱 The far book is asked every this many photos (30 s).
pub(crate) const LOIN_TOUS: u64 = 3;
/// Far photos failed in a row that clear the far walls (90 s).
pub(crate) const LOIN_ECHECS: u8 = 3;

impl MursState {
    fn new(coin: &str, cfg_perp: Cfg, cfg_spot: Cfg) -> Self {
        Self {
            coin: coin.into(),
            pression: std::cell::Cell::new(None),
            perp: Tracker::new(cfg_perp, false),
            spot: Tracker::new(cfg_spot, true),
            at: None,
            busy: false,
            asked: vec![],
            ok: vec![],
            depth_perp: Depth::default(),
            depth_spot: Depth::default(),
            bars_perp: Depth::default(),
            bars_spot: Depth::default(),
            step: 0.0,
            last_px: 0.0,
            last_bar: None,
            alive_ms: 0,
            purged_ms: 0,
            mem: vec![],
            replay: None,
            anciens: vec![],
            debuts: vec![],
            remplis: vec![],
            photos_chart: vec![],
            part_chart: None,
            recu: false,
            derniers: vec![],
            pause: vec![],
            photos: 0,
            loin: vec![],
            loin_pause: 0,
            loin_echecs: 0,
            flux: None,
        }
    }
}

/// The recordings file (desktop), opened the first time it is needed.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) type MursDb = argos_store::murs::MursDb;

impl App {
    /// 🧱 Is Argos Murs on?
    pub(crate) fn murs_on(&self) -> bool {
        self.ind.on.contains(&Ind::ArgosMurs)
    }

    pub(crate) fn murs_vals(&self) -> Vec<f64> {
        argos_render::indicators::values(&self.ind.set, Ind::ArgosMurs)
    }

    fn murs_inputs(vals: &[f64]) -> argos_script::inputs::Inputs<'_> {
        argos_script::inputs::Inputs { params: &rm::PARAMS, vals }
    }

    /// Tracker settings from the ⚙ (perp, spot).
    pub(crate) fn murs_cfg(vals: &[f64]) -> (Cfg, Cfg) {
        let p = Self::murs_inputs(vals);
        let base = Cfg {
            per_side: p.get("perN").max(1.0) as usize,
            mult: p.get("mult"),
            min_venues: p.get("minEx").max(1.0) as u32,
            hold_ms: (p.get("holdMin") * 60_000.0) as i64,
            ghost_ms: (p.get("ghostH") * 3_600_000.0) as i64,
            range_pct: p.get("range"),
            gate: !p.on("weak"),
            tag: p.on("binance"),
            ..Cfg::default()
        };
        let spot_n = if p.pick("carnets") == 2 { p.get("spotN") } else { p.get("perN") };
        (
            base,
            Cfg {
                per_side: spot_n.max(0.0) as usize,
                ..base
            },
        )
    }

    /// The books to read (mode and exchange boxes of the ⚙).
    pub(crate) fn murs_carnets(vals: &[f64]) -> Vec<Carnet> {
        let p = Self::murs_inputs(vals);
        let mode = p.pick("carnets");
        let key = |c: Carnet| match c {
            Carnet::BinancePerp => "exBinance",
            Carnet::BybitPerp => "exBybit",
            Carnet::OkxPerp => "exOkx",
            Carnet::BitunixPerp => "exBitunix",
            Carnet::HyperliquidPerp => "exHyperliquid",
            Carnet::BinanceSpot => "spBinance",
            Carnet::BybitSpot => "spBybit",
            Carnet::OkxSpot => "spOkx",
            Carnet::CoinbaseSpot => "spCoinbase",
        };
        let mut l = vec![];
        if mode != 1 {
            l.extend(PERP_MURS);
        }
        if mode != 0 {
            l.extend(Carnet::SPOT);
        }
        l.retain(|c| p.on(key(*c)));
        l
    }

    /// 🧱 Hidden above 4 h (setting « Cacher au-dessus de 4 h »): nothing drawn, the books still read.
    pub(crate) fn murs_cache(&self) -> bool {
        rm::cache(&Self::murs_inputs(&self.murs_vals()), self.chart.interval.secs())
    }

    /// The replay clock (ms): the end of the last bar shown; `None`: live.
    pub(crate) fn murs_horloge(&self) -> Option<i64> {
        self.replay.as_ref().map(|r| (r.t + self.chart.interval.secs()) * 1000 - 1)
    }

    /// 🧱 The room Argos Murs' pills and depth bars take at the right of the chart (`Ecran::marge`: where they are
    /// placed, over the candles since H7): about the widest pill, PC 330 px, phone 190 px (short wording); 0 when it is
    /// off or hidden. No band is kept for it in the viewport any more (H7, 24 Nov).
    pub(crate) fn murs_marge(&self) -> f64 {
        if !self.murs_on() || self.ind.out.hidden.contains(&Ind::ArgosMurs) || self.book_coin().is_none() || self.murs_cache() {
            return 0.0;
        }
        let pw = self.chart.viewport.plot_w();
        if pw < 560.0 {
            190.0f64.min(pw * 0.5)
        } else {
            330.0f64.min(pw * 0.4)
        }
    }

    /// 🧱 « Cadrer sur les murs » (legend button, or the ⚙ box): the price scale set once on the walls drawn and the
    /// last price, never wider than the depth read (±3 % by default) around it. Never done on its own. A short notice
    /// (the chart's banner) says the range framed, or in one sentence why nothing was framed ([`App::murs_cadre`]).
    pub(crate) fn murs_cadrer(&mut self) {
        let (lo, hi) = match self.murs_cadre() {
            Ok(r) => r,
            Err(raison) => {
                self.banner = Some(raison);
                return self.redraw();
            }
        };
        let sc = &mut self.chart.settings.scale;
        sc.manual = Some(match sc.mode {
            argos_render::settings::ScaleMode::Log => (lo.ln(), hi.ln()),
            _ => (lo, hi),
        });
        if sc.mode != argos_render::settings::ScaleMode::Log {
            sc.mode = argos_render::settings::ScaleMode::Normal;
        }
        sc.lock_ratio = None;
        let dec = argos_render::viewport::data_decimals(&self.chart.bars);
        let fp = |p: f64| argos_render::viewport::format_price(p, dec);
        self.banner = Some(format!("Murs cadrés : {} à {}", fp(lo), fp(hi)));
        self.redraw();
    }

    /// 🧱 The price range « Cadrer sur les murs » sets, or why there is none (one sentence for the notice).
    pub(crate) fn murs_cadre(&self) -> Result<(f64, f64), String> {
        if !self.murs_on() {
            return Err("Argos Murs n'est pas sur la chart".into());
        }
        if self.ind.out.hidden.contains(&Ind::ArgosMurs) {
            return Err("Argos Murs est masqué".into());
        }
        if self.murs_cache() {
            return Err("Argos Murs se cache au-dessus de 4 h : passe en 4 h ou moins".into());
        }
        if self.book_coin().is_none() {
            return Err("Pas de carnet public pour ce symbole".into());
        }
        let Some(e) = self.murs_ecran() else {
            return Err("Carnets pas encore lus : réessaie dans un instant".into());
        };
        let last = self.chart.bars.last().map_or(0.0, |b| b.close);
        if last <= 0.0 {
            return Err("Pas encore de prix : réessaie dans un instant".into());
        }
        let r = e.range.clamp(0.5, 5.0) / 100.0;
        let proche = |p: f64| (last * (1.0 - r)..=last * (1.0 + r)).contains(&p);
        let murs = e.vue.pills.iter().map(|p| p.price).chain(e.loin.iter().map(|l| l.price));
        if !murs.into_iter().any(proche) {
            let pct = argos_render::viewport::format_number(e.range.clamp(0.5, 5.0), 0);
            return Err(if e.vue.pills.is_empty() && e.etat.is_some() {
                "Carnets pas encore lus : réessaie dans un instant".into()
            } else {
                format!("Aucun mur à ±{pct} % du prix")
            });
        }
        rm::cadre(&e, last).ok_or_else(|| "Rien à cadrer : réessaie dans un instant".into())
    }

    /// Reads the books if it is time (every 10 s, live only); drops everything when the indicator is off.
    pub(crate) fn murs_refresh(&mut self) {
        // 🧱 H7 (Jee, 24 Nov: « un bloc noir » right of the candles, in every pane): no band is kept for Argos Murs any
        // more; the candles and the panes go up to the price scale as with Argos Murs off, and the pills, their lines
        // and the depth bars are drawn over them at the same place as before (`Ecran::marge` still places them)
        self.chart.viewport.marge = 0.0;
        // the ⚙ box « Cadrer sur les murs » is an action: done once, then unticked
        if self.murs_on() {
            let mut v = self.murs_vals();
            if let Some(k) = rm::PARAMS
                .iter()
                .position(|p| p.key == "cadrer")
                .filter(|k| v.get(*k).is_some_and(|x| *x != 0.0))
            {
                v[k] = 0.0;
                self.ind.set.insert(Ind::ArgosMurs, v);
                self.murs_cadrer();
            }
        }
        let coin = self.book_coin();
        let (Some(coin), true) = (coin, self.murs_on()) else {
            self.feeds.murs = None;
            self.murs_flux_suivre(None);
            return;
        };
        let vals = self.murs_vals();
        let (cp, cs) = Self::murs_cfg(&vals);
        if self.feeds.murs.as_ref().is_none_or(|m| m.coin != coin) {
            self.feeds.murs = Some(Box::new(MursState::new(&coin, cp, cs)));
            self.murs_anciens(&coin, cp.ghost_ms);
        }
        self.murs_charger_relecture();
        let carnets = Self::murs_carnets(&vals);
        // 📖 Binance perp's diff depth stream: live, perp books shown, Binance perp asked, setting on
        let flux = (self.replay.is_none() && flux_voulu(&vals, &carnets)).then(|| coin.clone());
        self.murs_flux_suivre(flux);
        let price = self.chart.bars.last().map_or(0.0, |b| b.close);
        let Some(m) = self.feeds.murs.as_mut() else { return };
        (m.perp.cfg, m.spot.cfg) = (cp, cs);
        if self.replay.is_some() || m.busy || price <= 0.0 || m.at.is_some_and(|t| t.elapsed().as_secs() < PERIOD_S) {
            return;
        }
        // one grid per coin: its step changes only if the price moved a lot (no reset at every round number)
        let s = murs::step_for(price);
        if !(m.step > 0.0 && (0.4..=2.5).contains(&(s / m.step))) {
            m.step = s;
        }
        let Some(proxy) = self.proxy.clone() else { return };
        // 🧱 far walls: only with the perp books shown, Hyperliquid perp among the books asked and « Murs lointains » on
        let loin_voulu = loin_voulu(&vals, &carnets);
        if !loin_voulu {
            (m.loin, m.loin_echecs) = (vec![], 0);
        }
        (m.busy, m.at, m.asked) = (true, Some(Instant::now()), carnets.clone());
        let step = m.step;
        // the far book on photos 0, 3, 6… (not while Hyperliquid is paused after a 429, nor on the far photos to skip)
        let hl_pause = m.pause.iter().any(|(c, n)| *c == Carnet::HyperliquidPerp && *n > 0);
        let loin = loin_voulu && tour_loin(&mut m.loin_pause, m.photos, hl_pause);
        m.photos += 1;
        // an exchange that answered 429 is left out of its next photos (its last snapshot is used meanwhile)
        let carnets = sauter_pauses(&mut m.pause, carnets);
        argos_data::rt::spawn(move || async move {
            let http = argos_data::rt::Client::default();
            let mut quoi: Vec<Option<Carnet>> = carnets.iter().copied().map(Some).collect();
            if loin {
                quoi.push(None);
            }
            let futs: Vec<_> = quoi.iter().map(|c| argos_data::murs::fetch_photo(&http, *c, &coin, price, step)).collect();
            let mut res: Vec<Result<(murs::VenueBook, Option<u64>), String>> =
                argos_data::http::ensemble(futs).await.into_iter().map(|r| r.map_err(String::from)).collect();
            let loin = if loin { res.pop().map(|r| r.map(|x| x.0)) } else { None };
            // (Binance perp's photo id: the merged book)
            let id = carnets
                .iter()
                .zip(&res)
                .find(|(c, _)| **c == Carnet::BinancePerp)
                .and_then(|(_, r)| r.as_ref().ok()?.1);
            let out: Vec<(Carnet, Result<murs::VenueBook, String>)> = carnets.into_iter().zip(res).map(|(c, r)| (c, r.map(|x| x.0))).collect();
            let _ = proxy.send_event(UserEvent::Murs(coin, out, loin, id));
        });
    }

    /// The books answered: walls followed, events recorded, alerts.
    pub(crate) fn murs_recu(
        &mut self,
        coin: String,
        res: Vec<(Carnet, Result<murs::VenueBook, String>)>,
        loin: Option<Result<murs::VenueBook, String>>,
        photo_id: Option<u64>,
    ) {
        let now = now_s() * 1000;
        let vals = self.murs_vals();
        let p = Self::murs_inputs(&vals);
        let range = p.get("range");
        let Some(last) = self.chart.bars.last().copied() else { return };
        // the real trades of the chart's market (1-minute orderflow): a wall eaten is REMPLI only if they hit it
        let minutes = self.murs_minutes();
        // the chart's own book: its trades are weighed against its own part of a wall
        let chart_carnet = match &self.chart.source {
            Source::Live { exchange, .. } => argos_data::murs::carnet_du_chart(*exchange),
            _ => None,
        };
        let Some(m) = self.feeds.murs.as_mut().filter(|m| m.coin == coin) else {
            return;
        };
        (m.busy, m.recu) = (false, true);
        // a 429 pauses that exchange; a book not asked (paused) or failed: its last good snapshot while it is young
        for (c, r) in &res {
            if r.as_ref().err().is_some_and(|e| e.contains("HTTP 429")) {
                m.pause.retain(|(k, _)| k != c);
                m.pause.push((*c, PAUSE_429));
            }
        }
        let mut res = res;
        let absents: Vec<Carnet> = m.asked.iter().copied().filter(|c| !res.iter().any(|(k, _)| k == c)).collect();
        res.extend(absents.into_iter().map(|c| (c, Err("pause après HTTP 429".to_string()))));
        // 📖 Binance perp's photo goes into its merged book (stream on), checked against it
        let px_photo = last.close;
        if let (Some(f), Some(l)) = (m.flux.as_mut(), photo_id) {
            if let Some((_, Ok(b))) = res.iter().find(|(c, _)| *c == Carnet::BinancePerp) {
                f.carnet.photo(l, b, px_photo, range + 1.0);
            }
        }
        let mut res = garder(&mut m.derniers, res, now);
        // the merged book replaces Binance perp's photo in everything Argos Murs computes
        let span_binance = m.flux.as_ref().and_then(|f| f.carnet.span);
        if let (Some(f), Some(_)) = (m.flux.as_ref(), span_binance) {
            for (c, r) in res.iter_mut() {
                if *c == Carnet::BinancePerp && r.is_ok() {
                    *r = Ok(f.carnet.livre(c.name(), px_photo));
                }
            }
        }
        // (a book replaced by its last snapshot counts as answered)
        m.ok = res.iter().filter(|(_, r)| r.is_ok()).map(|(c, _)| *c).collect();
        // (each book named after its exchange: the base books and Binance's tag are found by name)
        let books = |spot: bool| -> Vec<murs::VenueBook> {
            res.iter()
                .filter(|(c, _)| c.spot() == spot)
                .filter_map(|(c, r)| {
                    r.as_ref().ok().map(|b| murs::VenueBook {
                        venue: c.name().into(),
                        ..b.clone()
                    })
                })
                .collect()
        };
        let px = last.close;
        // the price path since the last photo: last and new price, and a new extreme of the bar
        let (mut lo, mut hi) = if m.last_px > 0.0 { (m.last_px.min(px), m.last_px.max(px)) } else { (px, px) };
        match m.last_bar {
            Some(b) if b.time == last.time => {
                if last.high > b.high {
                    hi = hi.max(last.high);
                }
                if last.low < b.low {
                    lo = lo.min(last.low);
                }
            }
            Some(b) => {
                // a new bar: the previous one closed (sweeps), read from the chart (its final values)
                let closed = self.chart.bars.iter().rev().find(|x| x.time == b.time).copied().unwrap_or(b);
                m.perp.bar_closed(now, closed.time, closed.high, closed.low, closed.close);
                m.spot.bar_closed(now, closed.time, closed.high, closed.low, closed.close);
            }
            None => {}
        }
        (m.last_px, m.last_bar) = (px, Some(last));
        // 🧱 the far walls: a far photo renews them; a failed one keeps them (three in a row clear them); a 429 skips
        // the next two far photos
        if let Some(r) = loin {
            let n = p.get("loinN").max(0.0) as usize;
            recevoir_loin(m, r, px, range, p.get("loinPct"), n, now);
        }
        m.depth_perp = murs::aggregate(&books(false), px, m.step, range);
        m.depth_spot = murs::aggregate(&books(true), px, m.step, range);
        m.bars_perp = murs::profile(&books(false), px, m.step / FINE, range);
        m.bars_spot = murs::profile(&books(true), px, m.step / FINE, range);
        // (Binance's coverage stays its photo's span: beyond it, what the stream told is partial)
        if let Some(span) = span_binance {
            for d in [&mut m.depth_perp, &mut m.bars_perp] {
                if let Some(i) = d.noms.iter().position(|n| n == Carnet::BinancePerp.name()) {
                    if let Some(r) = d.reach.get_mut(i) {
                        *r = span;
                    }
                }
            }
        }
        // REMPLI: the chart exchange's book in this photo (kept for the window), and its share of its market's books
        if let Some(c) = chart_carnet {
            let marche = books(c.spot());
            let idx = res.iter().filter(|(k, r)| k.spot() == c.spot() && r.is_ok()).position(|(k, _)| *k == c);
            if let Some(i) = idx {
                m.part_chart = murs::usual_share(&marche, i, px, range).or(m.part_chart);
                m.photos_chart.push((now, marche[i].clone()));
            }
        }
        m.photos_chart.retain(|(t, _)| now - *t <= REMPLI_FENETRE_MS);
        let (dp, ds) = (m.depth_perp.clone(), m.depth_spot.clone());
        m.perp.update(now, &dp, px, lo, hi);
        m.spot.update(now, &ds, px, lo, hi);
        let mut ev = m.perp.take_events();
        ev.extend(m.spot.take_events());
        // REMPLI (walls of the chart's market: its trades are that exchange's), recorded as its own event; the trades
        // are weighed against the chart exchange's own part of the wall (its biggest size there in the photos of the
        // window, before the hit), or the summed wall × its usual share when it held nothing there
        let spot_chart = chart_carnet.is_some_and(|c| c.spot());
        let filled: Vec<Event> = ev
            .iter()
            .filter(|e| e.spot == spot_chart)
            .filter_map(|e| {
                let part = murs::PartBourse {
                    own_usd: m
                        .photos_chart
                        .iter()
                        .filter(|(t, _)| *t <= e.t_ms)
                        .map(|(_, b)| murs::own_usd(b, e.price(), e.step, e.sell))
                        .fold(0.0, f64::max),
                    share: m.part_chart,
                };
                murs::rempli(e, &minutes, e.t_ms - REMPLI_FENETRE_MS, part).map(|_| Event { kind: Kind::Filled, ..*e })
            })
            .collect();
        for e in &filled {
            m.remplis.push(murs::Rempli {
                price: e.price(),
                sell: e.sell,
                spot: e.spot,
                usd: e.usd,
                at_ms: e.t_ms,
            });
        }
        let vieux = now - 2 * rm::REMPLI_BARS * self.chart.interval.secs().max(60) * 1000;
        m.remplis.retain(|r| r.at_ms > vieux);
        ev.extend(filled);
        if now - m.alive_ms >= ALIVE_MS {
            m.alive_ms = now;
            ev.push(Event {
                t_ms: now,
                key: 0,
                step: m.step,
                sell: false,
                spot: false,
                usd: 0.0,
                kind: Kind::Alive,
                venues: 0,
            });
        }
        m.mem.extend(ev.iter().copied());
        if m.mem.len() > 50_000 {
            m.mem.drain(..10_000);
        }
        let purge = now - m.purged_ms > 3_600_000;
        if purge {
            m.purged_ms = now;
        }
        let keep_ms = (p.get("keepDays").max(1.0) * 86_400_000.0) as i64;
        self.murs_enregistrer(&coin, &ev, purge.then_some(now - keep_ms));
        self.murs_alertes(&coin, &ev, &vals);
        self.ind.out.murs = self.murs_titre();
        self.redraw();
    }

    /// 📖 Starts, keeps or stops Binance perp's diff depth stream: `flux` is the coin it must follow (`None`: none).
    /// One stream at most; a new coin or a stop changes its generation (the old one ends at its next message).
    pub(crate) fn murs_flux_suivre(&mut self, flux: Option<String>) {
        if self.feeds.murs_flux == flux {
            if let (Some(_), Some(m)) = (&flux, self.feeds.murs.as_mut()) {
                m.flux.get_or_insert_with(FluxBinance::default);
            }
            return;
        }
        let gen = self.feeds.murs_flux_gen.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
        self.feeds.murs_flux = flux.clone();
        if let Some(m) = self.feeds.murs.as_mut() {
            m.flux = flux.as_ref().map(|_| FluxBinance::default());
        }
        let (Some(coin), Some(proxy)) = (flux, self.proxy.clone()) else { return };
        let courant = self.feeds.murs_flux_gen.clone();
        argos_data::live::spawn(
            argos_data::murs::flux_binance(&coin),
            |t| argos_data::murs::parse_maj(t).map(|m| argos_data::live::LiveEvent::Profondeur(Box::new(m))),
            move |ev| courant.load(std::sync::atomic::Ordering::SeqCst) == gen && proxy.send_event(UserEvent::MursFlux(gen, ev)).is_ok(),
        );
    }

    /// 📖 An event of Binance perp's diff depth stream: applied to the merged book (a gap or a reconnection forgets
    /// what lies outside the last photo's span).
    pub(crate) fn murs_flux_recu(&mut self, gen: u64, ev: argos_data::live::LiveEvent) {
        use argos_data::live::LiveEvent;
        if gen != self.feeds.murs_flux_gen.load(std::sync::atomic::Ordering::SeqCst) {
            return;
        }
        let Some(f) = self.feeds.murs.as_mut().and_then(|m| m.flux.as_mut()) else {
            return;
        };
        match ev {
            LiveEvent::Profondeur(u) => {
                f.carnet.evenement(u.pu, u.u, &u.bids, &u.asks);
                // 📖 the order-book panel reads this merged book when its choice is Binance perp USDT
                self.carnet_flux_bouge();
            }
            LiveEvent::Connected => {
                f.connecte = true;
                f.carnet.trou();
            }
            LiveEvent::Disconnected(_) => {
                f.carnet.trou();
            }
            _ => {}
        }
    }

    /// Records the events (desktop: `murs.redb`; browser: nothing more than this session's memory).
    #[cfg(not(target_arch = "wasm32"))]
    fn murs_enregistrer(&mut self, coin: &str, ev: &[Event], purge_before: Option<i64>) {
        if self.feeds.murs_db.is_none() {
            #[cfg(test)]
            let db = argos_store::murs::MursDb::memory().ok();
            #[cfg(not(test))]
            let db = argos_store::murs::default_path().and_then(|p| argos_store::murs::MursDb::open(&p).ok());
            self.feeds.murs_db = db;
        }
        let Some(db) = self.feeds.murs_db.as_ref() else { return };
        let _ = db.add(&format!("{coin}|murs"), ev);
        if let Some(t) = purge_before {
            let _ = db.purge(t);
        }
    }

    /// Desktop: the grey words recorded earlier for this coin (still within their fading time), read once when the
    /// indicator starts on it.
    #[cfg(not(target_arch = "wasm32"))]
    fn murs_anciens(&mut self, coin: &str, ghost_ms: i64) {
        if self.feeds.murs_db.is_none() {
            #[cfg(not(test))]
            {
                self.feeds.murs_db = argos_store::murs::default_path().and_then(|p| argos_store::murs::MursDb::open(&p).ok());
            }
        }
        let now = now_s() * 1000;
        let Some(ev) = self
            .feeds
            .murs_db
            .as_ref()
            .and_then(|db| db.read(&format!("{coin}|murs"), now - ghost_ms - 86_400_000, now).ok())
        else {
            return;
        };
        if let Some(m) = self.feeds.murs.as_mut() {
            let v = murs::replay(&ev, now, ghost_ms);
            (m.anciens, m.debuts, m.remplis) = (v.words, v.pills, v.remplis);
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn murs_anciens(&mut self, _coin: &str, _ghost_ms: i64) {}

    #[cfg(target_arch = "wasm32")]
    fn murs_enregistrer(&mut self, _coin: &str, _ev: &[Event], _purge_before: Option<i64>) {}

    /// 🔔 Alerts (each one off by default): a wall pulled, tested, swept, eaten, or a spoof.
    fn murs_alertes(&mut self, coin: &str, ev: &[Event], vals: &[f64]) {
        let p = Self::murs_inputs(vals);
        for e in ev {
            let (cle, quoi) = match e.kind {
                Kind::Filled if e.usd >= p.get("rempliMin") * 1e6 => ("alRempli", format!("REMPLI {}", rm::dollars(e.usd))),
                Kind::Filled => continue,
                Kind::End(f) => (rm::alerte(f), f.mot()),
                Kind::Tested => ("alTeste", "TESTÉ".to_string()),
                Kind::Sweep => ("alSweep", "SWEEP".to_string()),
                _ => continue,
            };
            if !p.on(cle) {
                continue;
            }
            let prix = argos_render::viewport::format_price(e.price(), 2);
            let cote = if e.sell { "vente" } else { "achat" };
            let sorte = if e.spot { " (comptant)" } else { "" };
            let msg = format!("🧱 Argos Murs {coin} : mur {cote}{sorte} {prix} · {quoi}");
            self.murs_notifier(msg);
        }
    }

    /// 🧮 The real trades of the chart's market, 1-minute bars (closed and current); empty when not followed.
    pub(crate) fn murs_minutes(&self) -> Vec<argos_core::orderflow::FlowBar> {
        let Some(cle) = self.orderflow_cle() else { return vec![] };
        self.feeds
            .orderflow
            .iter()
            .find(|f| f.cle == cle)
            .map(|f| {
                let mut l = f.minutes.clone();
                l.extend(f.agg.en_cours());
                l
            })
            .unwrap_or_default()
    }

    /// The legend after the name: exchanges and count, the books, or the replay's note.
    pub(crate) fn murs_titre(&self) -> Option<rm::Legende> {
        if !self.murs_on() {
            return None;
        }
        self.book_coin()?;
        let vals = self.murs_vals();
        let mode = Self::murs_inputs(&vals).pick("carnets");
        let carnets = Self::murs_carnets(&vals);
        let perp: Vec<&str> = carnets.iter().filter(|c| !c.spot()).map(|c| c.name()).collect();
        let noms: Vec<&str> = if perp.is_empty() {
            carnets.iter().map(|c| c.name()).collect()
        } else {
            perp.clone()
        };
        let m = self.feeds.murs.as_deref();
        let ok = m.map_or(0, |m| m.ok.iter().filter(|c| noms.contains(&c.name()) && c.spot() == (mode == 1)).count());
        let note = self.murs_horloge().and_then(|t| (!self.murs_enregistre_a(t)).then_some(PAS_DE_CARNET));
        // perp: the base books first, then « + » the others (mockup v6); spot: as they are
        let (base, autres): (Vec<&str>, Vec<&str>) = if perp.is_empty() {
            (vec![], noms.clone())
        } else {
            noms.iter().partition(|n| murs::BASE_VENUES.contains(n))
        };
        let l = rm::titre(&base, &autres, ok, noms.len(), mode, note);
        Some(if self.murs_cache() { rm::titre_cache(l) } else { l })
    }

    /// Replay: were the books recorded during the bar that ends at this clock (a heartbeat in it, or in the 2 minutes
    /// before it)?
    fn murs_enregistre_a(&self, t: i64) -> bool {
        let fenetre = self.chart.interval.secs() * 1000 + 2 * ALIVE_MS;
        self.murs_evenements()
            .iter()
            .any(|e| e.kind == Kind::Alive && e.t_ms <= t && t - e.t_ms <= fenetre)
    }

    /// Every recorded event of the chart's coin (replay); this session's when nothing is on disk.
    fn murs_evenements(&self) -> Vec<Event> {
        let Some(m) = self.feeds.murs.as_deref() else { return vec![] };
        if let Some(r) = &m.replay {
            return r.clone();
        }
        m.mem.clone()
    }

    /// Replay starts or stops: the coin's recorded events are read once.
    pub(crate) fn murs_charger_relecture(&mut self) {
        let replay = self.replay.is_some();
        let Some(m) = self.feeds.murs.as_mut() else { return };
        if !replay {
            m.replay = None;
            return;
        }
        if m.replay.is_some() {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let coin = m.coin.clone();
            if self.feeds.murs_db.is_none() {
                #[cfg(not(test))]
                {
                    self.feeds.murs_db = argos_store::murs::default_path().and_then(|p| argos_store::murs::MursDb::open(&p).ok());
                }
            }
            let disk = self.feeds.murs_db.as_ref().and_then(|db| db.read(&format!("{coin}|murs"), 0, i64::MAX).ok());
            let Some(m) = self.feeds.murs.as_mut() else { return };
            let mut l = disk.unwrap_or_default();
            if l.is_empty() {
                l = m.mem.clone();
            }
            m.replay = Some(l);
        }
        #[cfg(target_arch = "wasm32")]
        {
            m.replay = Some(m.mem.clone());
        }
    }

    /// 🧱 What the chart draws (live or replayed); `None`: off or not a crypto chart.
    pub(crate) fn murs_ecran(&self) -> Option<rm::Ecran> {
        if !self.murs_on() || self.ind.out.hidden.contains(&Ind::ArgosMurs) || self.murs_cache() {
            return None;
        }
        let m = self.feeds.murs.as_deref()?;
        let vals = self.murs_vals();
        let p = Self::murs_inputs(&vals);
        let mode = p.pick("carnets");
        let (perp, spot) = (mode != 1, mode != 0);
        let carnets = Self::murs_carnets(&vals);
        let spot_names = carnets.iter().filter(|c| c.spot()).map(|c| c.name()).collect::<Vec<_>>().join(" · ");
        let keep = |v: Vue| Vue {
            pills: v.pills.into_iter().filter(|x| if x.spot { spot } else { perp }).collect(),
            words: v.words.into_iter().filter(|x| if x.spot { spot } else { perp }).collect(),
            remplis: v.remplis.into_iter().filter(|x| if x.spot { spot } else { perp }).collect(),
        };
        let base = rm::Ecran {
            coin: m.coin.clone(),
            range: p.get("range"),
            spot_names,
            perp,
            spot,
            step: m.step,
            last: self.chart.bars.last().map_or(0.0, |b| b.close),
            bar_ms: self.chart.interval.secs() * 1000,
            // 📱 the legend spans the phone's width: the pills stay under it
            haut: if self.top.phone {
                argos_render::indleg::bas_legende(&self.ind.out, self.ind.leg.0, true) + GAP_LEGENDE
            } else {
                0.0
            },
            marge: self.murs_marge(),
            ..Default::default()
        };
        if let Some(t) = self.murs_horloge() {
            // replay: the recorded walls at the replay clock, nothing from after it; no live book
            let ev = self.murs_evenements();
            let step = ev.iter().rev().find(|e| e.t_ms <= t).map_or(m.step, |e| e.step);
            return Some(rm::Ecran {
                vue: keep(murs::replay(&ev, t, (p.get("ghostH") * 3_600_000.0) as i64)),
                step,
                now_ms: t,
                books: carnets.iter().filter(|c| c.spot() != perp).count() as u32,
                ..base
            });
        }
        let d = if perp { &m.depth_perp } else { &m.depth_spot };
        let f = if perp { &m.bars_perp } else { &m.bars_spot };
        let mut depth: Vec<(f64, f64, bool)> = f.asks.iter().map(|(k, c)| (f.price(*k), c.usd, true)).collect();
        depth.extend(f.bids.iter().map(|(k, c)| (f.price(*k), c.usd, false)));
        // (each bucket split by exchange, in the same order: the tooltip under the mouse)
        let detail: Vec<_> = f.asks.values().chain(f.bids.values()).map(|c| c.par).collect();
        // this session's grey words, then the ones recorded before it (not where a word or a wall already is)
        let now = now_s() * 1000;
        let ghost_ms = (p.get("ghostH") * 3_600_000.0) as i64;
        let mut vue = m.perp.vue().with(m.spot.vue());
        vue.remplis = m.remplis.clone();
        let pris: Vec<(bool, bool, i64)> = vue
            .words
            .iter()
            .map(|w| (w.spot, w.sell, (w.price / m.step).round() as i64))
            .chain(vue.pills.iter().map(|q| (q.spot, q.sell, (q.price / m.step).round() as i64)))
            .collect();
        // a wall already shown before this session: its line starts where it was first seen then
        for q in vue.pills.iter_mut() {
            if let Some(d) = m
                .debuts
                .iter()
                .find(|d| d.spot == q.spot && d.sell == q.sell && (d.price - q.price).abs() < m.step.max(f64::EPSILON) / 2.0)
            {
                q.since_ms = q.since_ms.min(d.since_ms);
            }
        }
        vue.words.extend(
            m.anciens
                .iter()
                .filter(|w| now - w.at_ms < ghost_ms && m.step > 0.0 && !pris.contains(&(w.spot, w.sell, (w.price / m.step).round() as i64)))
                .copied(),
        );
        // nothing read yet, or no exchange has a public book for this coin: said against the price column
        let etat = if m.ok.is_empty() && d.venues == 0 {
            Some(if m.recu { PAS_DE_CARNET_PUBLIC } else { LECTURE }.to_string())
        } else {
            None
        };
        // 🫧 the book's pressure for the bubble: the sells' share of the totals, smoothed over 30 s (F1g)
        let totals = (d.venues != 0).then(|| (d.total_coins(true), d.total_coins(false)));
        let pression = totals.filter(|(v, a)| v + a > 0.0).map(|(v, a)| {
            let (p, t) = rm::lisser_pression(m.pression.get(), v / (v + a), now);
            m.pression.set(Some((p, t)));
            p
        });
        // far walls: live only, perp books shown, the setting on
        let loin = if perp && loin_voulu(&vals, &carnets) { m.loin.clone() } else { vec![] };
        Some(rm::Ecran {
            vue: keep(vue),
            loin,
            now_ms: now,
            books: m.ok.iter().filter(|c| c.spot() != perp).count() as u32,
            depth,
            detail,
            noms: f.noms.clone(),
            noms_perp: m.depth_perp.noms.clone(),
            noms_spot: m.depth_spot.noms.clone(),
            reach: f.reach.clone(),
            etat,
            depth_step: f.step,
            totals,
            pression,
            sweeps: m.perp.sweeps.iter().chain(&m.spot.sweeps).copied().collect(),
            ..base
        })
    }
}

/// Each book's answer, or for a book that failed, its last good snapshot younger than `CARNET_GARDE_MS`; `derniers`
/// keeps the latest good snapshot of each book.
pub(crate) fn garder(
    derniers: &mut Vec<(Carnet, i64, murs::VenueBook)>,
    res: Vec<(Carnet, Result<murs::VenueBook, String>)>,
    now: i64,
) -> Vec<(Carnet, Result<murs::VenueBook, String>)> {
    res.into_iter()
        .map(|(c, r)| match r {
            Ok(b) => {
                derniers.retain(|(k, _, _)| *k != c);
                derniers.push((c, now, b.clone()));
                (c, Ok(b))
            }
            Err(e) => match derniers.iter().find(|(k, t, _)| *k == c && now - *t <= CARNET_GARDE_MS) {
                Some((_, _, b)) => (c, Ok(b.clone())),
                None => (c, Err(e)),
            },
        })
        .collect()
}

/// The books to ask this photo: the paused ones (after a 429) are left out, and their count goes down.
pub(crate) fn sauter_pauses(pause: &mut Vec<(Carnet, u8)>, carnets: Vec<Carnet>) -> Vec<Carnet> {
    let gardes: Vec<Carnet> = carnets.into_iter().filter(|c| !pause.iter().any(|(k, n)| k == c && *n > 0)).collect();
    for (_, n) in pause.iter_mut() {
        *n = n.saturating_sub(1);
    }
    pause.retain(|(_, n)| *n > 0);
    gardes
}

/// 🧱 Are far walls asked (lot loin-1)? The perp books are shown, Hyperliquid perp is among the books asked and
/// « Murs lointains par côté » is above 0.
pub(crate) fn loin_voulu(vals: &[f64], carnets: &[Carnet]) -> bool {
    let p = argos_script::inputs::Inputs { params: &rm::PARAMS, vals };
    p.pick("carnets") != 1 && carnets.contains(&Carnet::HyperliquidPerp) && p.get("loinN") >= 1.0
}

/// 📖 Is Binance perp's diff depth stream wanted? The perp books are shown, Binance perp is among the books asked and
/// « Binance : carnet complet en direct » is on.
pub(crate) fn flux_voulu(vals: &[f64], carnets: &[Carnet]) -> bool {
    let p = argos_script::inputs::Inputs { params: &rm::PARAMS, vals };
    p.pick("carnets") != 1 && carnets.contains(&Carnet::BinancePerp) && p.on("binFlux")
}

/// Is photo number `photo` a far photo? Every `LOIN_TOUS` photos, unless Hyperliquid is paused (`hl_pause`) or far
/// photos are still to skip after a 429 (`pause`, counted down here).
pub(crate) fn tour_loin(pause: &mut u8, photo: u64, hl_pause: bool) -> bool {
    if !photo.is_multiple_of(LOIN_TOUS) || hl_pause {
        return false;
    }
    if *pause > 0 {
        *pause -= 1;
        return false;
    }
    true
}

/// A far photo arrived: new far walls (ranked against the ones shown), or on an error the shown ones kept, cleared
/// after `LOIN_ECHECS` failures in a row; a 429 skips the next `PAUSE_429` far photos.
pub(crate) fn recevoir_loin(m: &mut MursState, r: Result<murs::VenueBook, String>, mid: f64, near: f64, far: f64, n: usize, now: i64) {
    match r {
        Ok(b) => {
            m.loin = murs::lointains(&b, mid, near, far, n, &m.loin, now);
            m.loin_echecs = 0;
        }
        Err(e) => {
            if e.contains("HTTP 429") {
                m.loin_pause = PAUSE_429;
            }
            m.loin_echecs = m.loin_echecs.saturating_add(1);
            if m.loin_echecs >= LOIN_ECHECS {
                m.loin.clear();
            }
        }
    }
}

/// Replay where nothing was recorded.
pub(crate) const PAS_DE_CARNET: &str = "pas de carnet enregistré pour cette période";
/// The first books are on their way.
pub(crate) const LECTURE: &str = "Lecture des carnets…";
/// Every book asked answered without a book for this coin.
pub(crate) const PAS_DE_CARNET_PUBLIC: &str = "Pas de carnet public pour ce symbole";

/// The perp books Argos Murs reads, in the order of its legend (Jee, 21 Nov): the base books (Binance, OKX,
/// Hyperliquid), then the others (Bybit, Bitunix).
pub(crate) const PERP_MURS: [Carnet; 5] = [
    Carnet::BinancePerp,
    Carnet::OkxPerp,
    Carnet::HyperliquidPerp,
    Carnet::BybitPerp,
    Carnet::BitunixPerp,
];

impl App {
    /// 🔔 An Argos Murs alert goes where the price alerts go: the history (🔔 sheet), the banner, the system notice.
    fn murs_notifier(&mut self, msg: String) {
        self.alerts.history.push(argos_core::alerts::Fired {
            id: 0,
            ts: now_s(),
            text: msg.clone(),
        });
        self.save_alerts();
        self.banner = Some(format!("🔔 {msg}"));
        // 🔊 the sound chosen in the settings (Argos Murs alerts have no sound of their own)
        let son = argos_core::alerts::son_a_jouer(std::iter::empty::<Option<&str>>(), &self.top.prefs.son_alertes);
        platform::alert_notify("🔔 Alerte Argos", &msg, son, self.top.prefs.volume_alertes);
    }

    /// 📈 Default indicators: on a fresh install Argos Murs, Argos Vague and TVD (Jee's favourites); on a profile that
    /// already exists, Argos Murs and Argos Vague once each (if never offered). Each can be removed, and that choice
    /// stays (the offered ones are written in `indicators_defaults.json`).
    pub(crate) fn indicateurs_par_defaut(&mut self, fresh: bool) {
        let mut offerts: Vec<Ind> = Self::read_prefs(DEFAUTS_FICHIER);
        let a_offrir: &[Ind] = if fresh { &DEFAUTS } else { &NOUVEAUX };
        let mut change = false;
        for i in a_offrir {
            if offerts.contains(i) {
                continue;
            }
            offerts.push(*i);
            change = true;
            if !self.ind.on.contains(i) {
                self.ind.on.push(*i);
            }
        }
        if change {
            Self::write_prefs("indicators.json", &self.ind.on);
            Self::write_prefs(DEFAUTS_FICHIER, &offerts);
        }
    }
}

/// The default set on a fresh install.
pub(crate) const DEFAUTS: [Ind; 3] = [Ind::ArgosMurs, Ind::ArgosVague, Ind::Tvd];
/// Offered once to a profile that already exists (TVD is not forced on it).
pub(crate) const NOUVEAUX: [Ind; 2] = [Ind::ArgosMurs, Ind::ArgosVague];
/// Defaults already offered once (a removed one never comes back).
pub(crate) const DEFAUTS_FICHIER: &str = "indicators_defaults.json";

#[cfg(test)]
mod tests_3e {
    use super::*;

    fn livre(v: &str) -> murs::VenueBook {
        murs::VenueBook {
            venue: v.into(),
            bids: vec![(100.0, 1.0)],
            asks: vec![(101.0, 1.0)],
        }
    }

    /// Lot 3e S1: a failed request is replaced by the book's last good snapshot while it is younger than 60 s; older,
    /// the book is missing.
    #[test]
    fn carnet_garde_60_s() {
        let mut d = vec![];
        let ok = garder(&mut d, vec![(Carnet::HyperliquidPerp, Ok(livre("Hyperliquid")))], 0);
        assert!(ok[0].1.is_ok());
        let r = garder(&mut d, vec![(Carnet::HyperliquidPerp, Err("HTTP 429".into()))], CARNET_GARDE_MS);
        assert_eq!(r[0].1.as_ref().ok().map(|b| b.venue.as_str()), Some("Hyperliquid"), "kept");
        let r = garder(&mut d, vec![(Carnet::HyperliquidPerp, Err("HTTP 429".into()))], CARNET_GARDE_MS + 1);
        assert!(r[0].1.is_err(), "too old");
        assert!(garder(&mut d, vec![(Carnet::OkxPerp, Err("x".into()))], 0)[0].1.is_err(), "never read");
    }

    /// Lot 3e S2: after a 429 the exchange is left out of its next two photos, then asked again.
    #[test]
    fn pause_de_deux_photos_apres_429() {
        let tous = vec![Carnet::BinancePerp, Carnet::HyperliquidPerp];
        let mut p = vec![(Carnet::HyperliquidPerp, PAUSE_429)];
        assert_eq!(sauter_pauses(&mut p, tous.clone()), [Carnet::BinancePerp]);
        assert_eq!(sauter_pauses(&mut p, tous.clone()), [Carnet::BinancePerp]);
        assert_eq!(sauter_pauses(&mut p, tous.clone()), tous);
    }
}

#[cfg(test)]
mod tests_loin {
    use super::*;

    /// Lot loin-1: the far book goes out on photos 0, 3, 6…, never while Hyperliquid is paused; after a 429 the next
    /// two far photos are skipped.
    #[test]
    fn photos_0_3_6_et_pause() {
        let mut pause = 0;
        let tours: Vec<u64> = (0..9).filter(|n| tour_loin(&mut pause, *n, false)).collect();
        assert_eq!(tours, [0, 3, 6]);
        assert!(!tour_loin(&mut pause, 9, true), "Hyperliquid paused");
        pause = PAUSE_429;
        let tours: Vec<u64> = (0..12).filter(|n| tour_loin(&mut pause, *n, false)).collect();
        assert_eq!(tours, [6, 9], "0 and 3 skipped");
    }

    /// Lot loin-1: asked only with the perp books shown, Hyperliquid among the books and « Murs lointains » above 0.
    #[test]
    fn quand_demander() {
        let mut v: Vec<f64> = rm::PARAMS.iter().map(|p| p.def).collect();
        let k = |key: &str| rm::PARAMS.iter().position(|p| p.key == key).unwrap();
        let tous = PERP_MURS.to_vec();
        assert!(loin_voulu(&v, &tous));
        assert!(!loin_voulu(&v, &[Carnet::BinancePerp]), "no Hyperliquid");
        v[k("carnets")] = 1.0;
        assert!(!loin_voulu(&v, &tous), "spot only");
        v[k("carnets")] = 2.0;
        assert!(loin_voulu(&v, &tous), "both");
        v[k("loinN")] = 0.0;
        assert!(!loin_voulu(&v, &tous), "0");
    }
}
