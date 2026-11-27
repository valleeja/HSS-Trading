//! 🔔 Feuille des notifications du cell et de la tablette (bouton 🔔 de la maquette) : le fil des vrais événements
//! (alertes déclenchées, baleines, news, liquidations reçues, avertissement du Gardien PropFirm) et ce que fait chaque
//! toucher.

use crate::*;
use argos_render::notifs::{Evenement, NotifClick, Notifs};
use argos_render::watchlist as wl;

/// « 2,1 M$ », « 850 k$ ».
fn montant(usd: f64) -> String {
    if usd >= 1e6 {
        format!("{} M$", argos_render::viewport::format_number(usd / 1e6, 1))
    } else {
        format!("{} k$", argos_render::viewport::format_number(usd / 1e3, 0))
    }
}

impl App {
    /// 🔔 Les événements, du plus récent au plus ancien.
    pub(crate) fn notif_evenements(&self) -> Vec<Evenement> {
        let mut v: Vec<Evenement> = vec![];
        // 🛡 Gardien PropFirm : en premier quand il prévient (orange ou rouge)
        if let Some((t, c)) = self.gardien_pilule() {
            if c != argos_render::palette::up() {
                v.push(Evenement {
                    sorte: 5,
                    texte: format!("🛡 Gardien PropFirm : perte du jour {t}"),
                    ts: None,
                });
            }
        }
        let mut dates: Vec<Evenement> = vec![];
        dates.extend(self.alerts.history.iter().map(|f| Evenement {
            sorte: 1,
            texte: format!("🔔 {}", f.text),
            ts: Some(f.ts),
        }));
        // 🧭 ⭐ the followed Hyperliquid addresses that opened or closed a position (an alert)
        dates.extend(self.feeds.hl_alertes.iter().map(|(ts, t)| Evenement {
            sorte: 1,
            texte: t.clone(),
            ts: Some(*ts),
        }));
        dates.extend(self.watch.eco.news.iter().take(20).map(|n| Evenement {
            sorte: 2,
            texte: format!("📰 {}", n.title),
            ts: Some(n.ts),
        }));
        dates.extend(self.watch.whales.list.iter().take(20).map(|w| Evenement {
            sorte: 3,
            texte: format!("🐋 {} {} {}", w.coin, if w.buy { "achat" } else { "vente" }, montant(w.usd())),
            ts: Some(w.time_ms / 1000),
        }));
        dates.extend(self.feeds.liqs.iter().take(20).map(|l| Evenement {
            sorte: 4,
            texte: format!(
                "💥 {} {} liquidé {} ({})",
                l.coin,
                if l.long { "long" } else { "short" },
                montant(l.price * l.qty),
                l.venue
            ),
            ts: Some(l.time_ms / 1000),
        }));
        dates.sort_by_key(|e| std::cmp::Reverse(e.ts));
        v.extend(dates);
        // 🧪 mode essai (?essai=1) : les exemples de la maquette tant que rien n'est arrivé (jamais en usage normal)
        #[cfg(target_arch = "wasm32")]
        if v.is_empty() && crate::platform::web::essai() {
            v = argos_render::notifs::exemples(now_s());
        }
        v
    }

    /// 🖥 🔔 de la barre du haut : ouvre (ou ferme) le volet des notifications (le volet flottant, comme les autres
    /// volets ; la chart s'arrête à son bord) ; `_b` : le bouton.
    pub(crate) fn notifs_pop(&mut self, _b: Rect) {
        let ouvert = self.ui.notifs.as_ref().is_some_and(|n| n.volet) && self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Alertes;
        if ouvert {
            self.ui.notifs = None;
            self.watch.prefs.open = false;
        } else {
            (self.ui.list, self.ui.sheet, self.ui.argos, self.ui.reglages) = (None, None, None, None);
            (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Alertes);
            // (sa place exacte est reprise à chaque image par la mise en page)
            let v = self
                .volet_rect()
                .unwrap_or(Rect::new(self.side_x() - 428.0, crate::render::VOLET_HAUT, 420.0, 600.0));
            self.ui.notifs = Some(Notifs {
                pop: Some(v),
                volet: true,
                ..Default::default()
            });
        }
        self.save_watch();
        self.redraw();
    }

    /// 🔔 La feuille (ou le volet) reçoit-elle le point (fenêtre) ? Le volet : seulement ses clics ; la feuille du cell :
    /// tout (à côté, elle se ferme).
    pub(crate) fn notifs_sous(&self, x: f64, y: f64) -> bool {
        match self.ui.notifs.as_ref() {
            Some(n) if n.volet => n.pop.is_some_and(|p| p.contains(x, y)),
            Some(_) => true,
            None => false,
        }
    }

    /// 🔔 Toucher dans la feuille (position de la fenêtre).
    pub(crate) fn notif_click(&mut self) {
        let (x, y) = self.pointer.win_mouse;
        // 🔔 the alerts panel (the feed only when ⚙ asked for it)
        if self.ui.notifs.as_ref().is_some_and(|n| !n.fil) {
            return self.alertes_click(x, y);
        }
        let ev = self.notif_evenements();
        let (w, h) = (self.win_w(), self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h());
        let text = argos_render::text::Text::default();
        let Some(p) = self.ui.notifs.as_mut() else { return };
        let ouvrir = |a: &mut Self, outil: wl::Tool| {
            a.ui.notifs = None;
            (a.watch.prefs.open, a.watch.prefs.panel) = (true, outil);
            a.save_watch();
        };
        match p.click(x, y, w, h, &text, &ev) {
            NotifClick::None | NotifClick::Changed => {}
            NotifClick::Fermer => {
                if self.ui.notifs.take().is_some_and(|n| n.volet) {
                    self.watch.prefs.open = false;
                    self.save_watch();
                }
            }
            NotifClick::Agrandir => {
                self.ui.volet_large = !self.ui.volet_large;
                let l = self.ui.volet_large;
                Self::interface2_changer(|i| i.volet_large = l);
            }
            // ＋ Nouvelle alerte, ⋯ Toutes les options : le menu des alertes (nouvelle, options, mes alertes, historique)
            NotifClick::NouvelleAlerte | NotifClick::Options => {
                let r = p.place(w, h, &ev);
                self.ui.notifs = None;
                return self.open_list(ListKind::Alerts, Rect::new(r.x + 9.0, r.y, 125.0, 1.0));
            }
            NotifClick::Evenement(i) => match ev.get(i).map(|e| e.sorte) {
                Some(1) => {
                    let r = p.place(w, h, &ev);
                    self.ui.notifs = None;
                    return self.open_list(ListKind::Alerts, Rect::new(r.x + 9.0, r.y, 125.0, 1.0));
                }
                Some(2) => ouvrir(self, wl::Tool::News),
                Some(3) => ouvrir(self, wl::Tool::Whales),
                Some(5) => ouvrir(self, wl::Tool::Trading),
                _ => {}
            },
            NotifClick::Bientot(m) => self.banner = Some(m.to_string()),
        }
        self.redraw();
    }
}

// ── 🔔 the Alertes panel (Jee's mock-up « alertes v1 », 22 Nov 2026) ─────────────────────────────────────────────

/// ⚙ of the alerts panel: its lines.
const RA_SON: i64 = 1;
const RA_LIGNES: i64 = 2;
const RA_COULEUR: i64 = 3;
const RA_OPTIONS: i64 = 4;
const RA_FIL: i64 = 5;
/// 🔔 When the feed of notifications was last opened (seconds UTC).
const FIL_VU: &str = "fil-vu.json";

/// 🔔 A row's name always reads « SOL/USDT » (base, then quote): a symbol still glued or written with - _ / or .P
/// (« SOLUSDT », « BTC-USDT », « BTCUSDC.P ») is split; a base already alone keeps the quote it has.
pub(crate) fn paire_lisible(base: &str, quote: &str) -> (String, String) {
    let glued = quote.is_empty() || base.contains(['-', '/', '_', '.']);
    match argos_data::tv::pair_parts(base).filter(|(_, q)| glued && (quote.is_empty() || q.eq_ignore_ascii_case(quote))) {
        Some((b, q)) => (b, q),
        None => (base.to_string(), quote.to_string()),
    }
}

/// 🔔 (base, quote, « Binance Spot ») of a chart id « <exchange>:<base> » (any exchange but Hyperliquid, whose rows
/// already read well); `None`: not a chart id.
pub(crate) fn alerte_marche_de_la_chart(coin: &str) -> Option<(String, String, String)> {
    let (e, base) = coin.split_once(':')?;
    let ex = Exchange::parse(e).filter(|x| *x != Exchange::Hyperliquid)?;
    // (a catalogue symbol still glued, « bybit:SOLUSDT », « okx:ETH-USDT-SWAP »: not a chart id, read as before)
    if base.is_empty() || (!base.contains('/') && argos_data::tv::pair_parts(base).is_some()) {
        return None;
    }
    let (a, q) = argos_data::paire::parts(base);
    let a = a.to_uppercase();
    let quote = match q {
        Some(q) => q.to_uppercase(),
        None => {
            // the exchange's own ticker without the coin (« SOLUSDT » → USDT, « SOL-USDT-SWAP » → USDT, « SOL/USD »)
            let t = ex.ticker(base);
            let reste = t.strip_prefix(&a).unwrap_or(&t).trim_start_matches(['-', '/', '_']);
            reste.split(['-', '_', '/']).next().unwrap_or("").to_string()
        }
    };
    let sorte = if ex.market_kind() == "Spot" { "Spot" } else { "Perp" };
    Some((a, quote, format!("{} {sorte}", ex.parent().name())))
}

impl App {
    /// 🔔 A market's round, name and exchange for a row (`coin` empty: an alert saved before the panel, only its
    /// symbol is known): (base, quote, « Binance Perp »).
    fn alerte_marche(&self, coin: &str, sym: &str) -> (String, String, String) {
        if coin.is_empty() {
            let (base, quote) = paire_lisible(sym, "");
            return (base, quote, String::new());
        }
        // 🔔 filet n° 33: an alert keeps the chart's id (« binance-spot:SOL », « bitfinex-spot:XRP/BTC »), not the
        // catalogue's: read from the exchange itself (« SOL/USDT · Binance Spot », « XRP/BTC · Bitfinex Spot »)
        if let Some(m) = alerte_marche_de_la_chart(coin) {
            let catalogue = self.store.as_ref().and_then(|s| s.instrument(coin).ok().flatten());
            if catalogue.is_none() {
                return m;
            }
        }
        let l = self.marche_ligne(coin);
        let bourse = if l.bourse.is_empty() {
            String::new()
        } else {
            format!("{} {}", l.bourse, if l.perp { "Perp" } else { "Spot" })
        };
        let base = if l.base.is_empty() { sym.to_string() } else { l.base };
        let (base, quote) = paire_lisible(&base, &l.quote);
        (base, quote, bourse)
    }

    /// 🔔 The rows of the alerts panel: the alerts (on first, then the newest), and the history (newest first).
    pub(crate) fn alertes_donnees(&self) -> argos_render::alertes::Donnees {
        use argos_render::alertes::Ligne;
        let now = now_s();
        let ut = |tf: Option<i64>| tf.map(|t| argos_core::Interval(t).short()).unwrap_or_default();
        fn dec<T: ToString>(d: Option<T>) -> Option<f64> {
            d.and_then(|d| d.to_string().parse::<f64>().ok())
        }
        let mut actives: Vec<Ligne> = self
            .alerts
            .list
            .iter()
            .map(|a| {
                let (base, quote, bourse) = self.alerte_marche(&a.coin, &a.sym);
                let (sens, prix) = a.sens();
                let tf = match a.kind {
                    argos_core::alerts::Kind::Multi { tf, .. } => Some(tf),
                    argos_core::alerts::Kind::Price { .. } => a.tf,
                };
                let arret = match crate::actions::alerts::raison_arret(a, now) {
                    "(expirée)" => "Expirée",
                    "(déclenchée)" => "Déclenchée",
                    _ => "En pause",
                };
                Ligne {
                    cle: a.id,
                    coin: a.coin.clone(),
                    base,
                    quote,
                    bourse,
                    ut: ut(tf),
                    ts: a.created,
                    sens,
                    fleche: Some(a.sens_du_prix()),
                    prix,
                    frequence: if a.every.is_some() { "Chaque fois" } else { "Une fois" }.into(),
                    pause: !a.on,
                    arret: arret.into(),
                    declenche: None,
                }
            })
            .collect();
        actives.sort_by_key(|l| (l.pause, std::cmp::Reverse(l.ts.unwrap_or(i64::MIN))));
        let historique = self
            .alerts
            .declenchements
            .iter()
            .enumerate()
            .map(|(i, d)| {
                let (base, quote, bourse) = self.alerte_marche(&d.coin, &d.sym);
                Ligne {
                    cle: i as u64,
                    coin: d.coin.clone(),
                    base,
                    quote,
                    bourse,
                    ut: ut(d.tf),
                    ts: Some(d.ts),
                    sens: d.sens,
                    // (the alert still known: its price's condition; else ↕)
                    fleche: self.alerts.list.iter().find(|a| a.id == d.id).map(|a| a.sens_du_prix()),
                    prix: dec(d.target),
                    frequence: if d.every { "Chaque fois" } else { "Une fois" }.into(),
                    pause: false,
                    arret: String::new(),
                    declenche: dec(d.price),
                }
            })
            .collect();
        argos_render::alertes::Donnees { actives, historique }
    }

    /// 🔔 A click in the alerts panel (window point).
    pub(crate) fn alertes_click(&mut self, x: f64, y: f64) {
        use argos_render::alertes::AClic;
        let d = self.alertes_donnees();
        let (w, h) = (self.win_w(), self.top_h() + self.area.1 as f64 + argos_render::ecrans::dock_h());
        let text = argos_render::text::Text::default();
        let Some(n) = self.ui.notifs.as_mut() else { return };
        let r = n.alertes_place(w, h);
        match n.alertes.click(&text, r, &d, x, y) {
            AClic::None | AClic::Changed => {}
            AClic::Fermer => {
                if self.ui.notifs.take().is_some_and(|n| n.volet) {
                    self.watch.prefs.open = false;
                    self.save_watch();
                }
            }
            // + : the alerts menu of before (new alert, its options, my alerts)
            AClic::Nouvelle(b) => return self.open_list(ListKind::Alerts, b),
            AClic::Reglages(b) => return self.open_list(ListKind::ReglagesAlertes, b),
            // an alert: the menu of its line (pause, Éditer…, Son…, Effacer), as on the chart
            AClic::Active(id, q) => return self.open_list(ListKind::AlertLine(id), Rect::new(q.x + q.w - 230.0, q.y + q.h / 2.0, 220.0, 1.0)),
            AClic::Vider => self.demander_vider_historique(),
            AClic::Trade(i) => return self.alerte_trade(i as usize),
        }
        self.redraw();
    }

    /// 🔔 « Trade » on a history row (`i`: its index): that market on the chart, then the demo Trading ticket.
    pub(crate) fn alerte_trade(&mut self, i: usize) {
        let Some(d) = self.alerts.declenchements.get(i).cloned() else { return };
        let coin = if d.coin.is_empty() { format!("hl:{}", d.sym) } else { d.coin };
        if let Some(n) = self.ui.notifs.as_mut() {
            n.alertes.touche = None;
        }
        self.ui.notifs = None;
        if coin != self.main_coin() {
            self.open_coin(coin);
        }
        (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Trading);
        self.save_watch();
        self.redraw();
    }

    /// 🔔 The feed's events not seen yet: newer than when ⚙ « Fil des notifications » was last opened (kept in
    /// fil-vu.json), the PropFirm warning in progress included.
    pub(crate) fn fil_non_lus(&self) -> usize {
        let vu: i64 = Self::read_prefs(FIL_VU);
        self.notif_evenements().iter().filter(|e| e.ts.is_none_or(|t| t > vu)).count()
    }

    /// 🔔 « Fil des notifications (news, baleines…) », with its unread count so nothing is missed.
    fn fil_libelle(&self) -> String {
        match self.fil_non_lus() {
            0 => "Fil des notifications (news, baleines…)".into(),
            1 => "Fil des notifications (news, baleines…) · 1 non lu".into(),
            n => format!("Fil des notifications (news, baleines…) · {n} non lus"),
        }
    }

    /// ⚙ of the alerts panel: every alert setting of before, in one place.
    pub(crate) fn menu_reglages_alertes(&self) -> Vec<argos_render::listmenu::MenuGroup> {
        use argos_render::listmenu::{MenuGroup, MenuItem};
        let item = |id: i64, label: &str, icon: &'static str, checked: bool| MenuItem {
            id,
            label: label.into(),
            enabled: true,
            icon: Some(icon),
            checked,
            ..Default::default()
        };
        let lignes = self.top.prefs.lignes_alerte;
        vec![
            MenuGroup {
                title: "RÉGLAGES DES ALERTES".into(),
                items: vec![
                    item(RA_SON, "Son des alertes…", "son", false),
                    item(RA_LIGNES, "Lignes d'alerte", if lignes { "case-cochee" } else { "case" }, lignes),
                    item(RA_COULEUR, "Modifier la couleur des alertes…", "🎨", false),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    item(RA_OPTIONS, "Déclenchement, validité, mes alertes…", "bell", false),
                    item(RA_FIL, &self.fil_libelle(), "newspaper", false),
                ],
                strip: false,
            },
        ]
    }

    /// ⚙ A line picked (`anchor`: where the menu was opened).
    pub(crate) fn reglages_alertes_choix(&mut self, id: i64, anchor: Rect) {
        match id {
            RA_SON => return self.open_list(ListKind::SonAlertes, anchor),
            RA_LIGNES => return self.ligne_alerte_choix(0, crate::actions::alerts::AL_LIGNES, anchor),
            RA_COULEUR => return self.ligne_alerte_choix(0, crate::actions::alerts::AL_COULEUR, anchor),
            RA_OPTIONS => return self.open_list(ListKind::Alerts, anchor),
            RA_FIL => {
                if let Some(n) = self.ui.notifs.as_mut() {
                    n.fil = true;
                }
                Self::write_prefs(FIL_VU, &now_s());
            }
            _ => {}
        }
        self.redraw();
    }
}
