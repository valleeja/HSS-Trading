//! 🧭 Explorer Hyperliquid and 📶 Stats Hyperliquid: when to read what, the readings received, the clicks that need
//! the app (read an address, ⭐ follow it, its lines on the chart, a market's chart), and the alert of a followed
//! address that opens or closes a position.
//!
//! 🚦 Hyperliquid weights added (1,200 points a minute per IP for every screen): Explorer open on an address,
//! `clearinghouseState` every 10 s (2 × 6 = 12) + `portfolio` and `frontendOpenOrders` every minute (20 + 20) + the
//! open tab every minute (Trades: `userFills` up to 120; Funding: `userFunding` up to 45): 52 a minute on the
//! Positions tab, 172 at most on Trades; nothing while closed. Each followed address: `clearinghouseState` once a
//! minute (2). Stats: nothing (the price list already read).

use crate::*;
use argos_data::explorer as hx;
use argos_render::hlexplorer::{ExClick, LignesAdresse};
use serde::{Deserialize, Serialize};

/// A reading of the Explorer (its address first in the event).
#[derive(Debug)]
pub(crate) enum HlLu {
    Compte(Result<hx::Compte, String>),
    Ordres(Result<Vec<hx::Order>, String>),
    Fills(Result<Vec<hx::Fill>, String>),
    Funding(Result<Vec<hx::Funding>, String>),
    Portfolio(Result<(Option<f64>, Option<f64>), String>),
    /// ⭐ A followed address read in the background (alerts).
    Suivi(Result<hx::Compte, String>),
}

/// When each reading was asked (the address they were for).
#[derive(Debug, Default)]
pub(crate) struct HlTemps {
    pub(crate) adresse: String,
    pub(crate) compte: Option<Instant>,
    pub(crate) portfolio: Option<Instant>,
    pub(crate) ordres: Option<Instant>,
    pub(crate) fills: Option<Instant>,
    pub(crate) funding: Option<Instant>,
    /// ⭐ followed addresses: last reading asked, last account seen.
    pub(crate) suivis: std::collections::HashMap<String, (Option<Instant>, Option<hx::Compte>)>,
}

/// 🧭 Kept between two launches: the followed addresses and the reading interval.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct ExplorerPrefs {
    pub(crate) suivies: Vec<String>,
    pub(crate) intervalle: usize,
}

/// Reading every minute for the tabs, the orders and the PnL.
const MINUTE: u64 = 60;
/// ⭐ A followed address is read once a minute.
const SUIVI_S: u64 = 60;

impl App {
    fn hl_ouvert(&self, t: wl::Tool, m: argos_render::dock::Module) -> bool {
        (self.watch.prefs.open && self.watch.prefs.panel == t) || (self.dock.rect.is_some() && self.watch.dock.shown().iter().any(|i| i.module == m))
    }

    /// 🧭 📶 Called with the other periodic readings: the Explorer's readings that are due (panel or module open),
    /// the followed addresses, and the Stats taken again from the price list every 30 s.
    pub(crate) fn hl_refresh(&mut self) {
        use argos_render::dock::Module;
        let now_ms = now_s() * 1000;
        self.watch.explorer.now_ms = now_ms;
        self.watch.explorer.tz_s = self.tz_offset_s();
        self.watch.stats.now_ms = now_ms;
        // 📶 Stats: the price list in memory, no request
        if self.hl_ouvert(wl::Tool::Stats, Module::Stats) && self.watch.stats.du(now_ms) && !self.feeds.quotes.is_empty() {
            self.watch.stats.prendre(self.feeds.quotes.values(), now_ms);
        }
        let Some(proxy) = self.proxy.clone() else { return };
        let du = |t: Option<Instant>, s: u64| t.is_none_or(|t| t.elapsed().as_secs() >= s);
        let ouvert = self.hl_ouvert(wl::Tool::Explorer, Module::Explorer);
        if let Some(a) = self.watch.explorer.adresse.clone().filter(|_| ouvert) {
            // the market decimals, from the price list
            if self.watch.explorer.decimales.len() != self.feeds.quotes.len() {
                self.watch.explorer.decimales = self
                    .feeds
                    .quotes
                    .values()
                    .map(|q| (q.coin.clone(), argos_render::hlexplorer::decimales_de(&q.mark)))
                    .collect();
            }
            if self.feeds.hlx.adresse != a {
                self.feeds.hlx = HlTemps {
                    adresse: a.clone(),
                    suivis: std::mem::take(&mut self.feeds.hlx.suivis),
                    ..Default::default()
                };
            }
            let t = &mut self.feeds.hlx;
            let mut lire: Vec<&'static str> = vec![];
            if du(t.compte, self.watch.explorer.intervalle_s()) {
                t.compte = Some(Instant::now());
                lire.push("compte");
            }
            if du(t.portfolio, MINUTE) {
                t.portfolio = Some(Instant::now());
                lire.push("portfolio");
            }
            if du(t.ordres, MINUTE) {
                t.ordres = Some(Instant::now());
                lire.push("ordres");
            }
            match self.watch.explorer.onglet {
                2 if du(t.fills, MINUTE) => {
                    t.fills = Some(Instant::now());
                    lire.push("fills");
                }
                3 if du(t.funding, MINUTE) => {
                    t.funding = Some(Instant::now());
                    lire.push("funding");
                }
                _ => {}
            }
            for k in lire {
                let (a, proxy) = (a.clone(), proxy.clone());
                argos_data::rt::spawn(move || async move {
                    let lu = match k {
                        "compte" => HlLu::Compte(hx::compte(&a).await.map_err(String::from)),
                        "portfolio" => HlLu::Portfolio(hx::portfolio(&a).await.map_err(String::from)),
                        "ordres" => HlLu::Ordres(hx::ordres(&a).await.map_err(String::from)),
                        "fills" => HlLu::Fills(hx::fills(&a).await.map_err(String::from)),
                        _ => HlLu::Funding(hx::funding(&a, now_s() * 1000).await.map_err(String::from)),
                    };
                    let _ = proxy.send_event(UserEvent::Hl(a, lu));
                });
            }
        }
        // ⭐ followed addresses (not the one the open panel already reads): once a minute, for the alerts
        let lue = self.watch.explorer.adresse.clone().filter(|_| ouvert);
        for a in self.watch.explorer.suivies.clone() {
            if Some(&a) == lue.as_ref() {
                continue;
            }
            let e = self.feeds.hlx.suivis.entry(a.clone()).or_insert((None, None));
            if du(e.0, SUIVI_S) {
                e.0 = Some(Instant::now());
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let lu = HlLu::Suivi(hx::compte(&a).await.map_err(String::from));
                    let _ = proxy.send_event(UserEvent::Hl(a, lu));
                });
            }
        }
    }

    /// 🧭 A reading received.
    pub(crate) fn hl_lu(&mut self, a: String, lu: HlLu) {
        if let HlLu::Suivi(r) = lu {
            if let Ok(c) = r {
                self.hl_suivi(&a, c);
            }
            return self.redraw();
        }
        if self.watch.explorer.adresse.as_deref() != Some(a.as_str()) {
            return;
        }
        if let HlLu::Compte(Ok(c)) = &lu {
            if self.watch.explorer.suivies.contains(&a) {
                self.hl_suivi(&a, c.clone());
            }
        }
        let e = &mut self.watch.explorer;
        match lu {
            HlLu::Suivi(_) => {}
            HlLu::Compte(Ok(c)) => {
                (e.compte, e.compte_ms) = (Some(c), Some(now_s() * 1000));
                e.note.clear();
                // the lines on the chart follow the position (entry and liquidation move)
                if let Some(l) = self.ui.hl_lignes.as_mut().filter(|l| l.adresse == a) {
                    match e.compte.as_ref().and_then(|c| c.positions.iter().find(|p| p.coin == l.coin)) {
                        Some(p) => (l.long, l.taille, l.entree, l.liq) = (p.long(), p.size, p.entry, p.liq),
                        None => self.ui.hl_lignes = None,
                    }
                }
            }
            HlLu::Compte(Err(m)) => e.note = format!("⚠️ Compte illisible : {m}"),
            HlLu::Ordres(r) => e.ordres = r.ok().or(e.ordres.take()),
            HlLu::Fills(r) => {
                if let Ok(f) = r {
                    // PnL from the fills when `portfolio` could not be read
                    if e.pnl_fills {
                        let now = now_s() * 1000;
                        e.pnl = (Some(hx::pnl_des_fills(&f, now, 86_400_000)), Some(hx::pnl_des_fills(&f, now, 30 * 86_400_000)));
                    }
                    e.fills = Some(f);
                }
            }
            HlLu::Funding(r) => e.funding = r.ok().or(e.funding.take()),
            HlLu::Portfolio(Ok(p)) => (e.pnl, e.pnl_fills) = (p, false),
            HlLu::Portfolio(Err(_)) => {
                // no `portfolio`: the PnL tiles from the fills (read now)
                e.pnl_fills = true;
                self.feeds.hlx.fills = None;
            }
        }
        self.redraw();
    }

    /// ⭐ A followed address read: an alert (banner, system notification, the 🔔 list) when it opened or closed a
    /// position since the last reading.
    fn hl_suivi(&mut self, a: &str, c: hx::Compte) {
        let avant = self.feeds.hlx.suivis.get(a).and_then(|e| e.1.clone());
        if let Some(av) = avant {
            let (ouvertes, fermees) = hx::changements(&av, &c);
            if !ouvertes.is_empty() || !fermees.is_empty() {
                let mut v = vec![];
                for coin in &ouvertes {
                    let p = c.positions.iter().find(|p| p.coin == *coin);
                    let sens = p.map_or("", |p| if p.long() { " long" } else { " short" });
                    v.push(format!("ouvre {coin}{sens}"));
                }
                v.extend(fermees.iter().map(|coin| format!("ferme {coin}")));
                let texte = format!("🧭 {} {}", hx::court(a), v.join(" · "));
                self.feeds.hl_alertes.insert(0, (now_s(), texte.clone()));
                self.feeds.hl_alertes.truncate(20);
                self.banner = Some(texte.clone());
                let son = self.top.prefs.son_alertes.clone();
                platform::alert_notify("🧭 Adresse suivie", &texte, &son, self.top.prefs.volume_alertes);
            }
        }
        self.feeds.hlx.suivis.entry(a.to_string()).or_insert((Some(Instant::now()), None)).1 = Some(c);
    }

    fn hl_garder(&self) {
        let e = &self.watch.explorer;
        Self::write_prefs(
            "explorer.json",
            &ExplorerPrefs {
                suivies: e.suivies.clone(),
                intervalle: e.intervalle,
            },
        );
    }

    /// 🧭 A click of the Explorer that needs the app.
    pub(crate) fn explorer_click(&mut self, c: ExClick) {
        match c {
            ExClick::None => return,
            ExClick::Changed => self.hl_garder(),
            ExClick::Lire(_) => {
                self.feeds.hlx.compte = None;
                self.feeds.hlx.adresse.clear();
                self.hl_refresh();
            }
            ExClick::Suivre(a) => {
                let e = &mut self.watch.explorer;
                if e.suivies.contains(&a) {
                    e.suivies.retain(|s| *s != a);
                    self.feeds.hlx.suivis.remove(&a);
                    self.banner = Some(format!("⭐ {} n'est plus suivie", hx::court(&a)));
                } else {
                    e.suivies.push(a.clone());
                    // (the same as « 👁 Suivre une baleine »: its positions in the Baleines panel too)
                    self.watch.whales.follow = Some(a.clone());
                    self.watch.whales.account = None;
                    if let Some(c) = e.compte.clone() {
                        self.feeds.hlx.suivis.insert(a.clone(), (Some(Instant::now()), Some(c)));
                    }
                    self.banner = Some(format!("⭐ {} suivie : une alerte quand elle ouvre ou ferme une position", hx::court(&a)));
                    platform::ask_notifications();
                }
                self.hl_garder();
                self.save_whales(true);
            }
            ExClick::Voir(a, coin) => {
                let e = &self.watch.explorer;
                let Some(p) = e.compte.as_ref().and_then(|c| c.positions.iter().find(|p| p.coin == coin)).cloned() else {
                    return;
                };
                let dec = e.decimales.get(&coin).copied().unwrap_or(1);
                self.ui.hl_lignes = Some(LignesAdresse {
                    adresse: a,
                    coin: coin.clone(),
                    long: p.long(),
                    taille: p.size,
                    entree: p.entry,
                    liq: p.liq,
                    dec,
                });
                // 📱 the sheet goes down: the chart and its lines are in view
                if self.top.phone {
                    self.watch.prefs.open = false;
                }
                if self.chart_hl_coin().as_deref() != Some(coin.as_str()) {
                    return self.open_coin(coin);
                }
            }
            ExClick::Chart(coin) => return self.open_coin(coin),
            ExClick::Onglet(_) => self.hl_refresh(),
        }
        self.redraw();
    }

    /// The chart's Hyperliquid market name (« BTC », « xyz:NVDA »), `None` for another source.
    pub(crate) fn chart_hl_coin(&self) -> Option<String> {
        match &self.chart.source {
            Source::Live { base, .. } if base.contains(':') => Some(base.clone()),
            // ₿ a pair against BTC is not a Hyperliquid market (its prices are in BTC)
            Source::Live { base, .. } if argos_data::paire::contre_btc(base) => None,
            Source::Live { base, .. } => Some(argos_data::whales::coin_of(base)),
            _ => None,
        }
    }

    /// 📈 The lines of « Voir sur la chart » for the chart drawn now (`None`: none, or another market).
    pub(crate) fn hl_lignes_vue(&self) -> Option<LignesAdresse> {
        let l = self.ui.hl_lignes.as_ref()?;
        (self.chart_hl_coin().as_deref() == Some(l.coin.as_str())).then(|| l.clone())
    }

    /// 📈 A press on the chart (chart points): the ✕ of a line's pill removes the lines.
    pub(crate) fn hl_lignes_press(&mut self, x: f64, y: f64) -> bool {
        if self.ui.hl_lignes.is_some() && argos_render::hlexplorer::fermer_sous(x, y) {
            self.ui.hl_lignes = None;
            self.redraw();
            return true;
        }
        false
    }
}
