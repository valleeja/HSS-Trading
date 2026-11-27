//! 🔎 Market panel of the symbol pill (Jee, 21 Nov, mock-up v2): the hover timer (PC), its rows (récents,
//! favoris, the shared market search engine), opening a market, the keyboard. The panel itself is
//! `argos_render::marches`.

use crate::*;
use argos_render::marches::{Ligne, MarcheClick, Panneau};

/// 🌐 In the browser, these exchanges stay hidden from the BOURSE chips (the browser blocks them: no CORS header).
/// Bitfinex and KuCoin come back when the page found the PC server's relay (`argos_data::relais`, `/api/bourse`).
/// (Bitstamp: catalogue, prices, candles and book answer with CORS open, checked on 23 Nov 2026: no longer hidden.)
const BLOQUEES_WEB: [Exchange; 5] = [Exchange::Bitunix, Exchange::Mexc, Exchange::Kucoin, Exchange::Bitfinex, Exchange::BitfinexSpot];

/// 📊 The biggest coins, in a rough order of size (a fixed list, not live data), for [`App::rang_volume`] when
/// Hyperliquid's 24 h volumes are not there yet.
const GROSSES: [&str; 30] = [
    "ETH", "XRP", "BNB", "SOL", "DOGE", "ADA", "TRX", "LINK", "AVAX", "XLM", "TON", "SUI", "HBAR", "BCH", "DOT", "LTC", "SHIB", "UNI", "NEAR", "APT", "ICP",
    "ETC", "POL", "ATOM", "FIL", "ARB", "OP", "AAVE", "XMR", "ALGO",
];

/// 🌐 `true` when `e` stays hidden in this page: in the browser, blocked by CORS and not carried by the relay.
pub(crate) fn bloquee_web(e: &Exchange) -> bool {
    bloquee_si(e, cfg!(target_arch = "wasm32"), argos_data::relais::actif())
}

/// [`bloquee_web`] for a page (`web`) with or without the relay.
pub(crate) fn bloquee_si(e: &Exchange, web: bool, relais: bool) -> bool {
    web && BLOQUEES_WEB.contains(e) && !(relais && argos_data::relais::BOURSES.contains(&e.id()))
}

impl App {
    /// 🔁 Browser: asks the PC's server whether its exchanges relay is there (`/api/bourse/etat`).
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn relais_bourse_chercher(&mut self) {
        let Some(proxy) = self.proxy.clone() else { return };
        argos_data::rt::spawn(move || async move {
            use argos_data::http::Http;
            let oui = argos_data::rt::Client::default()
                .get(argos_data::relais::ETAT)
                .await
                .is_ok_and(|b| argos_data::relais::lire_etat(&b));
            let _ = proxy.send_event(UserEvent::RelaisBourse(oui));
        });
    }

    /// 🔁 The relay is there: Bitfinex and KuCoin go through it and come back in the BOURSE chips, the BTC catalogues
    /// are asked again (once) and an open market panel fills again.
    pub(crate) fn relais_bourse_recu(&mut self, oui: bool) {
        argos_data::relais::activer(oui);
        if oui {
            self.ui.catalogues_btc = false;
            if self.ui.marches.is_some() {
                self.marches_remplir();
            }
        }
    }

    /// The symbol pill of the top bar (window), PC.
    pub(crate) fn pastille_symbole(&self) -> Option<Rect> {
        let t = argos_render::text::Text::default();
        self.top.symbole_rect(self.win_w(), &t)
    }

    /// 🖱 PC, on each loop wait: the mouse resting 0.3 s on the symbol pill opens the panel, away 0.3 s from the
    /// pill and the panel closes it (not while its field has the keyboard). `true`: still waiting (wake up soon).
    pub(crate) fn marches_tick(&mut self) -> bool {
        if self.top.phone || argos_render::actions::tactile() {
            return false;
        }
        let Some(p) = self.pastille_symbole() else { return false };
        let (x, y) = self.pointer.win_mouse;
        let panneau = self.ui.marches.as_ref();
        let sur_panneau = panneau.is_some_and(|m| m.rect.contains(x, y));
        // (opened by a click: it stays, like when its field has the keyboard)
        let focus = panneau.is_some_and(|m| m.focus || m.epingle);
        let t = self.ui.marches_horloge.elapsed().as_secs_f64();
        match self.ui.marches_survol.pas(t, p.contains(x, y), sur_panneau, panneau.is_some(), focus) {
            Some(true) => self.marches_ouvrir(p, false),
            Some(false) => {
                self.ui.marches = None;
                self.redraw();
            }
            None => {}
        }
        self.ui.marches_survol.en_cours()
    }

    /// Opens the panel under the pill `p` (PC, tablet), or as the phone's bottom sheet (`feuille`).
    pub(crate) fn marches_ouvrir(&mut self, p: Rect, feuille: bool) {
        let (ww, wh) = self.taille_points();
        let mut m = if feuille {
            Panneau::feuille(ww, wh)
        } else {
            Panneau::sous(p, ww, wh, argos_render::actions::tactile())
        };
        m.ouvert = Some(self.base_of(&self.watch.selected)).filter(|b| !b.is_empty());
        // 📱 the Argos AI round sits over the bottom of the sheet: « Tous les filtres » stops before it
        let ai = self.ai_bouton_rect();
        if feuille && ai.w > 0.0 && ai.y + ai.h > m.rect.y + m.rect.h - 70.0 {
            m.place_ai = (m.rect.x + m.rect.w - ai.x + 4.0).max(0.0);
        }
        self.ui.marches = Some(m);
        self.marches_remplir();
        // ⭐ the FAVORIS follow the chart's exchange: its catalogue first (an exchange not enabled has none yet); the
        // panel fills again when it arrives
        if let (Some(ex), Some(st)) = (self.bourse_du_graphique().filter(|e| *e != Exchange::Hyperliquid), self.store.as_ref()) {
            let frais = st.catalogs().unwrap_or_default().iter().any(|c| c.exchange == ex.id());
            if !frais {
                self.download_catalogs(vec![ex]);
            }
        }
        self.redraw();
    }

    /// A market row from its id (« binance:BTCUSDT », « hl:BTC »…), with its 24 h change when the watchlist knows it.
    pub(crate) fn marche_ligne(&self, id: &str) -> Ligne {
        // 🧮 the computed pair (« ratio:binance:XRP:BTC »): XRP, BTC, in the CALCULÉE section
        if let Some((ex, base, quote)) = argos_data::tv::parse_ratio_id(id) {
            return Ligne {
                id: id.to_string(),
                base,
                quote,
                bourse: ex.name().to_string(),
                perp: ex.market_kind() != "Spot",
                groupe: "Crypto".into(),
                variation: None,
                calculee: true,
            };
        }
        let ins = self.store.as_ref().and_then(|s| s.instrument(id).ok().flatten());
        let ex = id.split_once(':').and_then(|(e, _)| Exchange::parse(e));
        let bourse = ex.map_or(String::new(), |e| e.name().trim_end_matches(" Spot").to_string());
        let (base, quote, perp, groupe) = match &ins {
            Some(i) => (i.base.clone(), i.quote.clone(), i.kind == argos_core::MarketKind::Perp, i.group.clone()),
            None => (
                self.base_of(id),
                if ex == Some(Exchange::Hyperliquid) { "USDC".into() } else { String::new() },
                true,
                "Crypto".into(),
            ),
        };
        // (24 h change: the watchlist's row, else the Hyperliquid price the app already has)
        let cle = cle_marche(id);
        let pct = |mark: &str, prev: f64| {
            let m = mark.replace([' ', '\u{202f}'], "").replace(',', ".").parse::<f64>().ok()?;
            (prev > 0.0).then(|| (m / prev - 1.0) * 100.0)
        };
        let variation = self
            .watch
            .sections
            .iter()
            .flat_map(|s| s.rows.iter())
            .find(|r| cle_marche(&r.coin) == cle)
            .and_then(|r| pct(&r.mark, r.prev))
            .or_else(|| self.feeds.quotes.get(&cle).and_then(|q| pct(&q.mark, q.prev)))
            .or_else(|| self.variation_des_tickers(ex, &base, &quote, perp));
        Ligne {
            id: id.to_string(),
            base,
            quote,
            bourse,
            perp,
            groupe,
            variation,
            calculee: false,
        }
    }

    /// 📈 The 24 h change of a USDT perpetual of Binance or OKX from the tickers the app already read
    /// (`all_tickers`: one request for every market, no new call); `None` for anything else.
    fn variation_des_tickers(&self, ex: Option<Exchange>, base: &str, quote: &str, perp: bool) -> Option<f64> {
        let ex = ex.filter(|e| matches!(e, Exchange::Binance | Exchange::Okx))?;
        if !perp || !(quote.is_empty() || quote == "USDT") {
            return None;
        }
        let (_, list) = self.feeds.tickers.get(&ex)?;
        list.iter().find(|t| t.base.eq_ignore_ascii_case(base)).and_then(|t| t.pct())
    }

    /// ⭐ FAVORIS ids: the flagged rows, then the rows of Jee's own watchlist (the list shown, else « Mes favoris », else
    /// the first one; never « 🌐 Tous les marchés »), without the ones already in `vus` (RÉCENTS) nor twice, 10 at most.
    fn marches_favoris(&self, vus: &mut std::collections::HashSet<String>) -> Vec<String> {
        let liste = self
            .user_list()
            .map(|l| l.id)
            .or_else(|| self.favoris_id())
            .or_else(|| self.lists.first().map(|l| l.id));
        let items = liste.and_then(|id| self.store.as_ref()?.items(id).ok()).unwrap_or_default();
        let ids = self.watch.prefs.flags.iter().map(|(c, _)| c.clone()).chain(items.into_iter().map(|(_, c)| c));
        ids.map(|c| self.sur_bourse_du_graphique(&c))
            .filter(|c| vus.insert(cle_marche(c)))
            .take(10)
            .collect()
    }

    /// The exchange of the chart shown (a live market; else the selected row's), `None` for other sources.
    fn bourse_du_graphique(&self) -> Option<Exchange> {
        match &self.chart.source {
            Source::Live { exchange, .. } => Some(*exchange),
            _ => match self.watch.selected.split_once(':') {
                None => Some(Exchange::Hyperliquid),
                Some((e, _)) => Exchange::parse(e),
            },
        }
    }

    /// ⭐ A favourite brought to the chart's exchange (Jee, 22 Nov: « on a Binance chart, BTC, ETH… show and open on
    /// Binance; if that exchange doesn't have it, keep the favourite's »): same base, same kind (perp / spot) as the
    /// chart, quote of the chart first, then USDT, USDC, USD. Not found (or not a crypto exchange id): `id` as is.
    fn sur_bourse_du_graphique(&self, id: &str) -> String {
        let Some(st) = self.store.as_ref() else { return id.to_string() };
        let bourse = |c: &str| match c.split_once(':') {
            None => Some(Exchange::Hyperliquid),
            Some((e, _)) => Exchange::parse(e),
        };
        let (Some(ex), Some(fav)) = (self.bourse_du_graphique(), bourse(id)) else {
            return id.to_string();
        };
        if ex == fav {
            return id.to_string();
        }
        // the chart's market (its kind and quote) when the selected row is on that exchange
        let graphique = Some(self.watch.selected.as_str())
            .filter(|s| bourse(s) == Some(ex))
            .and_then(|s| st.instrument(s).ok().flatten());
        let perp = graphique
            .as_ref()
            .map_or(!ex.name().ends_with(" Spot"), |i| i.kind == argos_core::MarketKind::Perp);
        let base = self.base_of(id);
        let q = Search {
            query: &base,
            exchange: Some(ex.id()),
            group: None,
            limit: 400,
        };
        let cands = st.search(&q).unwrap_or_default();
        equivalent(&base, perp, graphique.as_ref().map(|i| i.quote.as_str()), &cands).unwrap_or_else(|| id.to_string())
    }

    /// 🔥 POPULAIRES (no favourite at all): the 8 biggest 24 h volumes of the chart's exchange among what the app
    /// already has in memory (Hyperliquid prices, the last « Tous les marchés » reading), no new request.
    fn marches_populaires(&self, vus: &std::collections::HashSet<String>) -> Vec<String> {
        let ex = self
            .watch
            .selected
            .split_once(':')
            .and_then(|(e, _)| Exchange::parse(e))
            .unwrap_or(Exchange::Hyperliquid);
        let mut v: Vec<(f64, String)> = match &self.feeds.tous {
            Some((e, Ok(l))) if *e == ex => l
                .iter()
                .map(|l| {
                    let id = if ex == Exchange::Hyperliquid {
                        l.symbol.clone()
                    } else {
                        format!("{}:{}", ex.id(), l.symbol)
                    };
                    (l.vol_usd.unwrap_or(0.0), id)
                })
                .collect(),
            _ if ex == Exchange::Hyperliquid => self.feeds.quotes.values().map(|q| (q.volume, q.coin.clone())).collect(),
            _ => vec![],
        };
        v.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
        v.into_iter()
            .filter(|(vol, id)| *vol > 0.0 && !vus.contains(&cle_marche(id)))
            .take(8)
            .map(|(_, id)| id)
            .collect()
    }

    /// Fills the panel: récents, favoris (flagged rows then the watchlist's) or else populaires, the chips, and the
    /// results of its search.
    pub(crate) fn marches_remplir(&mut self) {
        let Some(m) = self.ui.marches.as_ref() else { return };
        let mut requete = m.requete();
        // ₿ « xrp btc », « xrp/btc »: the whole text asks for XRP against BTC (real pairs, then the computed one)
        let paire_btc = matches!(argos_data::tv::parse(&m.texte), argos_data::tv::Tv::Ticker(t) if t.is_btc_pair() && t.exchange.is_none());
        if paire_btc {
            requete = m.texte.clone();
        }
        // ₿ the « btc » chip with nothing typed: every market against BTC
        let contre_btc = requete.trim().is_empty() && m.jetons.iter().any(|(f, v)| *f == argos_render::marches::Filtre::Quote && v == "btc");
        let cherche = m.cherche();
        // ₿ the catalogues that have pairs against BTC, once (the panel fills again when they arrive); « xrpbtc » typed
        // whole asks them too (it was the same text as the words of the search: nothing was asked on the web)
        if (contre_btc || paire_btc) && !std::mem::replace(&mut self.ui.catalogues_btc, true) {
            let cats = self.store.as_ref().and_then(|s| s.catalogs().ok()).unwrap_or_default();
            let manque: Vec<Exchange> = crate::actions::watchlist::BOURSES_BTC
                .into_iter()
                .filter(|e| !bloquee_web(e))
                .filter(|e| !cats.iter().any(|c| c.exchange == e.id()))
                .collect();
            self.download_catalogs(manque);
        }
        let recents: Vec<Ligne> = self.ui.sym_recents.iter().map(|c| self.marche_ligne(c)).collect();
        let mut vus: std::collections::HashSet<String> = self.ui.sym_recents.iter().map(|c| cle_marche(c)).collect();
        let favoris: Vec<Ligne> = self.marches_favoris(&mut vus).iter().map(|c| self.marche_ligne(c)).collect();
        let populaires: Vec<Ligne> = if favoris.is_empty() {
            self.marches_populaires(&vus).iter().map(|c| self.marche_ligne(c)).collect()
        } else {
            vec![]
        };
        let resultats: Vec<Ligne> = if contre_btc {
            self.marches_contre_btc().iter().map(|id| self.marche_ligne(id)).collect()
        } else if cherche {
            let mut found = self.chercher_marches(&requete, None, None).unwrap_or_default();
            // ₿ a search about BTC (« btc », « xrpbtc »…): the most traded first, not the catalogue's alphabet
            if requete.to_uppercase().contains("BTC") {
                self.trier_par_volume(&mut found);
            }
            found
                .iter()
                .filter(|f| !f.id.starts_with(crate::actions::DEX_ASK))
                .take(200)
                .map(|f| self.marche_ligne(&f.id))
                .collect()
        } else {
            vec![]
        };
        let bourses: Vec<String> = Exchange::ALL
            .into_iter()
            .filter(|e| !e.name().ends_with(" Spot"))
            .filter(|e| !bloquee_web(e))
            .map(|e| e.name().to_string())
            .collect();
        let classes: Vec<String> = self
            .store
            .as_ref()
            .and_then(|s| s.groups().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|(g, _)| g)
            .collect();
        if let Some(m) = self.ui.marches.as_mut() {
            (m.recents, m.favoris, m.populaires) = (recents, favoris, populaires);
            (m.resultats, m.bourses, m.classes) = (resultats, bourses, classes);
        }
    }

    /// ₿ « CONTRE BTC »: every active market against BTC of every exchange's catalogue, the biggest coins first (the
    /// order of Hyperliquid's 24 h volumes the app already has), each coin's exchanges in the order of the search; 200
    /// at most.
    pub(crate) fn marches_contre_btc(&self) -> Vec<String> {
        let Some(st) = self.store.as_ref() else { return vec![] };
        let mut v: Vec<(usize, f64, usize, String)> = vec![];
        for (k, ex) in crate::actions::watchlist::ordre_btc().into_iter().enumerate() {
            let q = Search {
                query: "",
                exchange: Some(ex.id()),
                group: None,
                limit: 100_000,
            };
            for i in st.search(&q).unwrap_or_default() {
                if i.active && i.quote.eq_ignore_ascii_case("BTC") && !i.base.eq_ignore_ascii_case("BTC") {
                    let (tier, vol) = self.rang_volume(&i.base);
                    v.push((tier, -vol, k, i.id()));
                }
            }
        }
        v.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)).then(a.2.cmp(&b.2)).then(a.3.cmp(&b.3)));
        v.into_iter().take(200).map(|x| x.3).collect()
    }

    /// 📊 How much a coin trades, to put the most traded first (Jee, 23 Nov: ETH, XRP, SOL… before Bitfinex's small
    /// tokens BMN2, CALCPB, CH100): (0, its 24 h volume) when Hyperliquid trades it; else (1, -rank) for the big coins
    /// known to Argos ([`GROSSES`]), so the order holds before Hyperliquid's prices arrive; else (2, 0).
    pub(crate) fn rang_volume(&self, coin: &str) -> (usize, f64) {
        let c = coin.to_uppercase();
        if let Some(vol) = self.feeds.quotes.get(c.as_str()).map(|q| q.volume).filter(|v| *v > 0.0) {
            return (0, vol);
        }
        match GROSSES.iter().position(|g| *g == c) {
            Some(p) => (1, -(p as f64)),
            None => (2, 0.0),
        }
    }

    /// 📊 The panel's search results, most traded coin first ([`App::rang_volume`]); for one coin, the search's order
    /// is kept (Bitfinex first for a pair against BTC, the computed pair last).
    pub(crate) fn trier_par_volume(&self, found: &mut [argos_render::wlmenu::Found]) {
        let mut cles: std::collections::HashMap<String, (usize, f64)> = Default::default();
        for f in found.iter() {
            if !cles.contains_key(&f.id) {
                let coin = argos_data::paire::asset(&self.base_of(&f.id)).to_string();
                cles.insert(f.id.clone(), self.rang_volume(&coin));
            }
        }
        // (the computed pair « ratio:… » stays where the search put it: last)
        let fin = found.iter().position(|f| f.id.starts_with("ratio:")).unwrap_or(found.len());
        found[..fin].sort_by(|a, b| {
            let (x, y) = (cles[&a.id], cles[&b.id]);
            x.0.cmp(&y.0).then(y.1.total_cmp(&x.1))
        });
    }

    /// What the panel asks: search again, open a market (same path as the rows of « 🌐 Tous les marchés »), close.
    pub(crate) fn marches_faire(&mut self, c: MarcheClick) {
        match c {
            MarcheClick::None => {}
            MarcheClick::Fermer => self.ui.marches = None,
            MarcheClick::Chercher => self.marches_remplir(),
            MarcheClick::Ouvrir(id) => {
                self.ui.marches = None;
                return self.open_coin(id);
            }
            // 👆 « Ma watchlist › »: the whole watchlist, as the pill's touch opened it before the panel
            // 🖱 PC (pinned panel): the big Symboles panel, which the click on the pill opened before
            MarcheClick::Watchlist if !self.top.phone && !argos_render::actions::tactile() => {
                self.ui.marches = None;
                return self.symboles_volet();
            }
            MarcheClick::Watchlist => {
                self.ui.marches = None;
                (self.watch.prefs.open, self.watch.prefs.panel) = (true, argos_render::watchlist::Tool::Watchlist);
                self.save_watch();
            }
        }
        self.redraw();
    }

    /// 🖱 PC, a click on the symbol pill (Jee, 22 Nov: « the same panel as on hover, closed by a click outside »):
    /// pinned open panel → closed; else the panel opens (or stays) pinned under the pill.
    pub(crate) fn marches_clic_pastille(&mut self) {
        let Some(p) = self.pastille_symbole() else { return };
        match self.ui.marches.as_mut() {
            Some(m) if m.epingle => {
                self.ui.marches = None;
                self.ui.marches_survol.retenir();
            }
            Some(m) => m.epingle = true,
            None => {
                self.marches_ouvrir(p, false);
                if let Some(m) = self.ui.marches.as_mut() {
                    m.epingle = true;
                }
            }
        }
        self.redraw();
    }

    /// 🔎 The market panel's search field (window) while it is open: a zone where one types — a touch there gives the
    /// page's hidden field the focus, and the phone's keyboard opens (Jee, 24 Nov: « la barre de recherche ne marche
    /// pas » on the iPhone). (Only the browser has the page's hidden field: the desktop app reads it in tests.)
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) fn marches_zone_saisie(&self) -> Option<Rect> {
        self.ui.marches.as_ref().map(|m| m.champ_rect())
    }

    /// A click / touch while the panel is open (window); `false`: not for it (outside: it closes, the click goes on).
    pub(crate) fn marches_click(&mut self, x: f64, y: f64) -> bool {
        // 🖱 PC: the pill itself pins / closes the panel (not « outside »)
        let pc = !self.top.phone && !argos_render::actions::tactile();
        if pc && self.ui.marches.as_ref().is_some_and(|m| !m.rect.contains(x, y)) && self.pastille_symbole().is_some_and(|p| p.contains(x, y)) {
            self.marches_clic_pastille();
            return true;
        }
        let Some(m) = self.ui.marches.as_mut() else { return false };
        match m.click(x, y) {
            MarcheClick::Fermer => {
                self.ui.marches = None;
                self.redraw();
                false
            }
            c => {
                self.marches_faire(c);
                true
            }
        }
    }

    /// ⌨️ The panel's keyboard: text, Backspace, ↑ ↓, Enter, Escape (closes).
    pub(crate) fn key_marches(&mut self, event: winit::event::KeyEvent) {
        self.key_marches_with(&event.logical_key, event.text.as_deref());
    }

    /// ⌨ 📱 The same for a key or a text, from the computer's keyboard or the page's hidden field (the phone's
    /// keyboard, `App::touche_page`): the text typed filters the list, Backspace erases, Enter opens the market.
    pub(crate) fn key_marches_with(&mut self, key: &Key, text: Option<&str>) {
        let ctrl = self.pointer.mods.control_key();
        let Some(m) = self.ui.marches.as_mut() else { return };
        let c = match key {
            Key::Named(NamedKey::Escape) => MarcheClick::Fermer,
            Key::Named(NamedKey::Backspace) => m.effacer(),
            Key::Named(NamedKey::Enter) => m.entree(),
            Key::Named(NamedKey::ArrowDown) => {
                m.bouger(1);
                MarcheClick::None
            }
            Key::Named(NamedKey::ArrowUp) => {
                m.bouger(-1);
                MarcheClick::None
            }
            _ => match text.filter(|_| !ctrl) {
                Some(t) => m.taper(t),
                None => MarcheClick::None,
            },
        };
        self.marches_faire(c);
    }
}

/// The market of `cands` (one exchange's catalogue) with this `base` and kind, its quote the chart's `quote` first,
/// then USDT, USDC, USD; its id.
fn equivalent(base: &str, perp: bool, quote: Option<&str>, cands: &[argos_core::Instrument]) -> Option<String> {
    let rang = |q: &str| quote.into_iter().chain(["USDT", "USDC", "USD"]).position(|p| p.eq_ignore_ascii_case(q));
    cands
        .iter()
        .filter(|i| i.active && i.base.eq_ignore_ascii_case(base) && (i.kind == argos_core::MarketKind::Perp) == perp)
        .filter_map(|i| Some((rang(&i.quote)?, i)))
        .min_by_key(|(r, _)| *r)
        .map(|(_, i)| i.id())
}

/// The same market written two ways (« hl:BTC » in a list, « BTC » on the Hyperliquid list): one key.
fn cle_marche(id: &str) -> String {
    id.strip_prefix("hl:").unwrap_or(id).to_string()
}
