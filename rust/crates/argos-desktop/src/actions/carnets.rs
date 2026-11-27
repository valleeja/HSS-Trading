//! 📖 🔥 The books of the order-book heatmap and the order-book panel: by default the chart's own exchange and pair,
//! or the exchanges ticked in the heatmap's ⚙, in USDT, USDC or both summed (Jee, 21 Nov). Hyperliquid's USDC book
//! alone keeps its live stream (`book::sub`); any other choice reads snapshots every 5 s (the readers of Argos Murs)
//! and sums them into one book. The books actually read are named in the heatmap's legend and the panel's band.

use crate::*;
use argos_data::murs::{self as dm, Carnet, Choix, Monnaie};
use argos_render::indicators::Ind;

/// Snapshots read every 5 s.
pub(crate) const PERIOD_S: u64 = 5;

/// What the chosen books keep.
#[derive(Default)]
pub(crate) struct CarnetsState {
    /// Coin and choice read (another one: start again).
    pub(crate) key: String,
    pub(crate) at: Option<Instant>,
    pub(crate) busy: bool,
    /// Books that answered last time.
    pub(crate) ok: usize,
    /// 🧱 The books are Argos Murs' photos (it reads them already, every 10 s): the time of the last one taken
    /// (`None`: read here).
    pub(crate) murs_vu: Option<i64>,
}

/// 🧱 A photo of Argos Murs is still the book of now this long after it was taken (its photos come every 10 s).
const MURS_FRAIS_MS: i64 = 15_000;

/// ⚡ A Bitfinex perpetual's mark, funding and open interest (`status/deriv`), read every 10 s while its chart is shown.
#[derive(Default)]
pub(crate) struct StatutBitfinex {
    /// The chart base read (another one: start again).
    pub(crate) base: String,
    pub(crate) at: Option<Instant>,
    pub(crate) busy: bool,
    pub(crate) statut: Option<argos_data::exchanges::bitfinex::Statut>,
}

/// The perpetual's figures read again every 10 s.
const STATUT_S: u64 = 10;

impl App {
    /// ⚡ A Bitfinex perpetual chart: its mark, funding and open interest, read every [`STATUT_S`] (in the browser
    /// through the PC server's relay).
    pub(crate) fn bitfinex_statut(&mut self) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        if *exchange != Exchange::Bitfinex {
            return;
        }
        let s = self.feeds.bitfinex_statut.get_or_insert_with(Default::default);
        if s.base != *base {
            *s = StatutBitfinex {
                base: base.clone(),
                ..Default::default()
            };
        }
        if s.busy || s.at.is_some_and(|t| t.elapsed().as_secs() < STATUT_S) {
            return;
        }
        let Some(proxy) = self.proxy.clone() else { return };
        (s.busy, s.at) = (true, Some(Instant::now()));
        let (base, req) = (base.clone(), argos_data::exchanges::bitfinex::status_request(base));
        argos_data::rt::spawn(move || async move {
            use argos_data::Http as _;
            let argos_data::exchanges::Request::Get(url) = req else { return };
            let res = argos_data::rt::Client::default()
                .get(&url)
                .await
                .and_then(|b| argos_data::exchanges::bitfinex::parse_status(&b))
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::StatutBitfinex(base, res));
        });
    }

    /// ⚡ The perpetual's figures arrived: into the header.
    pub(crate) fn bitfinex_statut_recu(&mut self, base: String, res: Result<Option<argos_data::exchanges::bitfinex::Statut>, String>) {
        let Some(s) = self.feeds.bitfinex_statut.as_mut().filter(|s| s.base == base) else {
            return;
        };
        s.busy = false;
        match res {
            Ok(Some(st)) => s.statut = Some(st),
            Ok(None) => {}
            Err(e) => eprintln!("⚠️ Statut Bitfinex : {e}"),
        }
        self.refresh_top();
        self.redraw();
    }

    /// ⚡ The figures of the Bitfinex perpetual of the chart, when read.
    pub(crate) fn bitfinex_statut_de(&self, base: &str) -> Option<argos_data::exchanges::bitfinex::Statut> {
        self.feeds.bitfinex_statut.as_ref().filter(|s| s.base == base).and_then(|s| s.statut)
    }

    /// The books chosen (heatmap ⚙, shared with the order-book panel); `None`: not a crypto chart.
    pub(crate) fn carnets_choix(&self) -> Option<Choix> {
        let Source::Live { exchange, base } = &self.chart.source else { return None };
        // Bitfinex: its own book (`bitfinex_carnet`), never the books of Argos Murs
        if exchange.bitfinex().is_some() {
            return None;
        }
        self.book_coin()?;
        let v = argos_render::indicators::values(&self.ind.set, Ind::BookHeat);
        let p = argos_script::inputs::Inputs {
            params: &argos_render::bookheat::PARAMS,
            vals: &v,
        };
        let coches = ["cBinance", "cBybit", "cOkx", "cBitunix", "cHyperliquid"].map(|k| p.on(k));
        let usdc = base.to_uppercase().contains("USDC");
        Some(dm::carnets_choisis(
            Some((*exchange, usdc)),
            p.pick("source") == 0,
            coches,
            p.pick("monnaie"),
            cfg!(target_arch = "wasm32"),
        ))
    }

    /// Is the order-book panel shown (its volet 📖, or its module in the column)?
    pub(crate) fn carnet_montre(&self) -> bool {
        self.carnet_volet_ouvert() || (self.dock.rect.is_some() && self.watch.dock.shown().iter().any(|i| i.module == argos_render::dock::Module::Book))
    }

    /// Snapshots instead of Hyperliquid's live stream: the heatmap or the panel is shown and the choice is not
    /// Hyperliquid USDC alone.
    pub(crate) fn carnets_instantanes(&self) -> bool {
        (self.heat_on() || self.carnet_montre()) && self.carnets_choix().is_some_and(|c| !c.hyperliquid_seul())
    }

    /// What the legend and the panel say: the books read and how many answered (`None`: Hyperliquid's live book).
    pub(crate) fn carnets_titre(&self) -> Option<String> {
        let c = self.carnets_choix()?;
        if c.hyperliquid_seul() && c.note.is_none() {
            return None;
        }
        let ok = if c.hyperliquid_seul() {
            1
        } else {
            self.feeds.carnets.as_ref().map_or(0, |s| s.ok)
        };
        Some(dm::titre_carnets(&c, ok))
    }

    /// 📖 A Bitfinex chart: its own order book, read every 5 s while the heatmap or the order-book panel is shown
    /// (one request, Bitfinex's public limits are low). `true`: the chart is on Bitfinex (nothing else to read).
    fn bitfinex_carnet(&mut self) -> bool {
        let Source::Live { exchange, base } = &self.chart.source else { return false };
        let Some(perp) = exchange.bitfinex() else { return false };
        if !(self.heat_on() || self.carnet_montre()) {
            self.feeds.carnets = None;
            return true;
        }
        let key = format!("bitfinex|{perp}|{base}");
        let s = self.feeds.carnets.get_or_insert_with(Default::default);
        if s.key != key {
            *s = CarnetsState {
                key: key.clone(),
                ..Default::default()
            };
            (self.feeds.book, self.feeds.heat) = (None, None);
        }
        let Some(s) = self.feeds.carnets.as_mut() else { return true };
        if s.busy || s.at.is_some_and(|t| t.elapsed().as_secs() < PERIOD_S) {
            return true;
        }
        let Some(proxy) = self.proxy.clone() else { return true };
        (s.busy, s.at) = (true, Some(Instant::now()));
        let (base, req) = (base.clone(), argos_data::exchanges::bitfinex::book_request(base, perp));
        argos_data::rt::spawn(move || async move {
            use argos_data::Http as _;
            let http = argos_data::rt::Client::default();
            let argos_data::exchanges::Request::Get(url) = req else { return };
            let res = http
                .get(&url)
                .await
                .and_then(|b| argos_data::exchanges::bitfinex::parse_book(&b, &base, now_s() * 1000))
                .map_err(|e| e.to_string());
            let _ = proxy.send_event(UserEvent::CarnetBitfinex(key, res));
        });
        true
    }

    /// 📖 Bitfinex's book arrived: into the heatmap and the panel.
    pub(crate) fn bitfinex_carnet_recu(&mut self, key: String, res: Result<argos_data::book::Book, String>) {
        let Some(s) = self.feeds.carnets.as_mut().filter(|s| s.key == key) else {
            return;
        };
        // (an answer: a reading was asked, even one not timed here)
        s.at.get_or_insert_with(Instant::now);
        s.busy = false;
        s.ok = usize::from(res.is_ok());
        match res {
            Ok(b) => {
                self.heat_add(&b);
                self.feeds.book = Some(b);
                self.carnet_livre();
            }
            // the book could not be read: the old one goes (the panel says so, as for the other exchanges)
            Err(e) => {
                eprintln!("⚠️ Carnet Bitfinex : {e}");
                self.feeds.book = None;
                self.carnet_livre();
            }
        }
        self.redraw();
    }

    /// Reads the chosen books if it is time (every 5 s while the heatmap or the panel is shown).
    pub(crate) fn carnets_refresh(&mut self) {
        self.bitfinex_statut();
        if self.bitfinex_carnet() {
            return;
        }
        if !self.carnets_instantanes() {
            self.feeds.carnets = None;
            return;
        }
        let (Some(c), Some(coin)) = (self.carnets_choix(), self.book_coin()) else {
            return;
        };
        let key = format!("{coin}|{:?}", c.carnets);
        let s = self.feeds.carnets.get_or_insert_with(Default::default);
        if s.key != key {
            *s = CarnetsState {
                key: key.clone(),
                ..Default::default()
            };
            self.feeds.book = None;
            self.feeds.heat = None;
        }
        // 🧱 Argos Murs already reads these books: its photos, nothing read twice (« même flux qu'Argos Murs »)
        if let Some(books) = self.carnets_de_murs(&c.carnets, &coin) {
            let vu = books.iter().map(|b| b.0).max();
            let Some(s) = self.feeds.carnets.as_mut() else { return };
            s.busy = false;
            if s.murs_vu != vu {
                (s.murs_vu, s.ok, s.at) = (vu, books.len(), Some(Instant::now()));
                self.carnets_poser(books.into_iter().map(|b| b.1).collect());
            }
            return;
        }
        let Some(s) = self.feeds.carnets.as_mut() else { return };
        s.murs_vu = None;
        let price = self.chart.bars.last().map_or(0.0, |b| b.close);
        if s.busy || price <= 0.0 || s.at.is_some_and(|t| t.elapsed().as_secs() < PERIOD_S) {
            return;
        }
        let Some(proxy) = self.proxy.clone() else { return };
        (s.busy, s.at) = (true, Some(Instant::now()));
        let step = argos_core::murs::step_for(price) / 10.0;
        let list = c.carnets;
        argos_data::rt::spawn(move || async move {
            let http = argos_data::rt::Client::default();
            let futs: Vec<_> = list.iter().map(|(c, m)| dm::fetch_q(&http, *c, &coin, price, step, *m)).collect();
            let res = argos_data::http::ensemble(futs).await;
            let out: Vec<(Carnet, Monnaie, Result<argos_core::murs::VenueBook, String>)> =
                list.into_iter().zip(res).map(|((c, m), r)| (c, m, r.map_err(String::from))).collect();
            let _ = proxy.send_event(UserEvent::Carnets(key, out));
        });
    }

    /// The books answered: summed into one, into the heatmap and the panel.
    pub(crate) fn carnets_recus(&mut self, key: String, res: Vec<(Carnet, Monnaie, Result<argos_core::murs::VenueBook, String>)>) {
        let Some(s) = self.feeds.carnets.as_mut().filter(|s| s.key == key) else {
            return;
        };
        s.at.get_or_insert_with(Instant::now);
        s.busy = false;
        s.ok = res.iter().filter(|r| r.2.is_ok()).count();
        let books: Vec<argos_core::murs::VenueBook> = res.into_iter().filter_map(|r| r.2.ok()).collect();
        self.carnets_poser(books);
    }

    /// 🧱 The chosen books from Argos Murs' last photos (each one younger than [`MURS_FRAIS_MS`]), with their time;
    /// `None`: Argos Murs does not read them all (another coin, a currency it does not read, not on).
    pub(crate) fn carnets_de_murs(&self, choix: &[(Carnet, Monnaie)], coin: &str) -> Option<Vec<(i64, argos_core::murs::VenueBook)>> {
        let m = self.feeds.murs.as_deref().filter(|m| m.coin == coin && self.replay.is_none())?;
        let now = now_s() * 1000;
        choix
            .iter()
            .map(|(c, mo)| {
                // (its photos: perps in USDT, Hyperliquid in USDC)
                let sienne = if *c == Carnet::HyperliquidPerp { Monnaie::Usdc } else { Monnaie::Usdt };
                (*mo == sienne && m.asked.contains(c))
                    .then(|| m.derniers.iter().find(|(k, t, _)| k == c && now - *t <= MURS_FRAIS_MS))
                    .flatten()
                    .map(|(_, t, b)| (*t, b.clone()))
            })
            .collect()
    }

    /// The books read (here or by Argos Murs) summed into one: into the heatmap and the panel.
    fn carnets_poser(&mut self, books: Vec<argos_core::murs::VenueBook>) {
        let Some(px) = self.chart.bars.last().map(|b| b.close) else { return };
        if books.is_empty() {
            // (none could be read: 📖 the panel says so too)
            self.ind.out.carnets = self.carnets_titre();
            self.carnet_livre();
            self.redraw();
            return;
        }
        // the same grouping as the heatmap (HD); 📖 the panel shown: its finest step (the heatmap groups it again)
        let tick = match argos_core::carnet::pas_possibles(px).first() {
            Some(p) if self.carnet_montre() => *p,
            _ => argos_render::bookheat::tick_for(px, true),
        };
        let coin = self.feeds.flow_coin.clone().or_else(|| self.book_coin()).unwrap_or_default();
        let b = dm::somme(&coin, &books, tick, now_s() * 1000);
        self.heat_add(&b);
        self.feeds.book = Some(b);
        self.ind.out.carnets = self.carnets_titre();
        self.carnet_livre();
        self.redraw();
    }
}
