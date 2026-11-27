//! 💼 Trading démo : marché, portfolio, ticket, ordres et sorties.

use crate::actions::journal::Note;
use crate::*;

/// 📅 An exit whose time is further than this from now (seconds) was made in a replay: its time is the market's.
const REPLAY_GAP: i64 = 60;

impl App {
    /// 🛒 Ouvre le Marketplace : aperçus calculés une fois sur les 160 dernières bougies (comme v1).
    pub(crate) fn open_market(&mut self) {
        // calcul sur 600 bougies (les moyennes ont le temps de démarrer), aperçu des 160 dernières
        let tail = &self.chart.bars[self.chart.bars.len().saturating_sub(600)..];
        let tf = self.chart.interval.secs();
        self.ui.mk_spark = argos_render::catalogue::officiels()
            .iter()
            .map(|o| {
                let (v, c) = argos_render::indicators::spark(o.0, tail, tf);
                (o.0, (v[v.len().saturating_sub(160)..].to_vec(), c))
            })
            .collect();
        self.ui.mk_page = Some(argos_render::marketplace::MkPage::new(self.ind.favs.clone()));
        // (opened from the Argos ▾ menu: shown at once, not at the next event)
        self.redraw();
    }

    /// 🛒 Clic dans le Marketplace.
    /// 🎓 Ouvre l'Académie (progression gardée dans `academy.json`).
    pub(crate) fn open_academy(&mut self) {
        (self.ui.list, self.ui.sheet, self.ui.argos) = (None, None, None);
        self.ui.ac_page = Some(argos_render::academy::AcademyPage::new(Self::read_prefs("academy.json")));
        self.redraw();
    }

    /// 📖 Ouvre la fiche « News éco → DXY / XAU » de l'Académie à la ligne de cette annonce (depuis l'Agenda).
    pub(crate) fn open_academy_news(&mut self, row: usize) {
        use argos_render::academy::{AcademyPage, NEWS_GUIDE};
        (self.ui.list, self.ui.sheet, self.ui.argos) = (None, None, None);
        let w = self.window.as_deref().map(ui_size).unwrap_or_default().width as f64;
        let mut p = AcademyPage::new(Self::read_prefs("academy.json"));
        if p.open_at(NEWS_GUIDE, 1, row, &argos_render::text::Text::default(), w) == argos_render::academy::AcClick::Changed {
            Self::write_prefs("academy.json", &p.progress);
        }
        self.ui.ac_page = Some(p);
        self.redraw();
    }

    /// 🎓 Clic dans l'Académie.
    pub(crate) fn academy_click(&mut self) {
        use argos_render::academy::AcClick;
        let Some(p) = self.ui.ac_page.as_mut() else { return };
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        match p.click(&argos_render::text::Text::default(), size.width as f64, size.height as f64, x, y) {
            AcClick::Close => self.ui.ac_page = None,
            AcClick::Changed => Self::write_prefs("academy.json", &p.progress),
            AcClick::None => {}
        }
    }

    pub(crate) fn market_click(&mut self) {
        use argos_render::marketplace::MkClick;
        let Some(m) = self.ui.mk_page.as_mut() else { return };
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        match m.click(&argos_render::text::Text::default(), size.width as f64, size.height as f64, &self.ind.on, x, y) {
            MkClick::Close => self.ui.mk_page = None,
            MkClick::Toggle(i) => self.toggle_indicator(i),
            MkClick::Favs => {
                self.ind.favs = m.favs.clone();
                self.ind.out.favs = self.ind.favs.clone();
                Self::write_prefs("market_fav.json", &m.favs);
            }
            MkClick::Soon(msg) => self.banner = Some(msg.into()),
            MkClick::Changed | MkClick::None => {}
        }
    }

    /// 💼 Clic dans la page Portfolio.
    pub(crate) fn portfolio_click(&mut self) {
        use argos_render::portfolio::PfClick;
        let Some(p) = self.ui.pf_page.as_mut() else { return };
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        match p.click(&argos_render::text::Text::default(), size.width as f64, size.height as f64, x, y) {
            PfClick::Close => self.ui.pf_page = None,
            // ouvrir Hyperliquid dans le navigateur (Argos ne signe rien : l'action se fait là-bas, avec ton wallet)
            PfClick::Open(url) => {
                platform::open_url(url);
            }
            PfClick::Fund(v) => {
                // compte démo : dépôt / retrait d'argent fictif (pas sous 0)
                self.demo.balance = (self.demo.balance + argos_core::Money::from_f64(v)).max(argos_core::Money::ZERO);
                Self::write_prefs("demo_account.json", &self.demo);
            }
            PfClick::Soon(msg) => self.banner = Some(msg.into()),
            PfClick::Accounts => {
                let (x, y) = self.pointer.win_mouse;
                self.ui.pf_page = None;
                self.open_list(ListKind::Accounts, Rect::new(x - 120.0, y, 240.0, 1.0));
            }
            PfClick::Changed | PfClick::None => {}
        }
    }

    /// 💼 Le ticket démo suit la position dessinée `i` : côté, stop, risque, entrée / stop / cible.
    pub(crate) fn ticket_from_drawing(&mut self, i: usize) {
        use argos_render::position;
        let Some(d) = self.chart.drawings.get(i).filter(|d| position::is_position(&d.tool)) else {
            return;
        };
        let c = position::calc(d, self.chart.bars.last().map(|b| b.close), position::tick_of(&self.chart.bars));
        let t = &mut self.watch.panels.trading;
        (t.buy, t.sl_pips, t.risk, t.to_lots, t.pip_value) = (c.long, (c.entry - c.stop).abs(), c.risk, true, 1.0);
        t.plan = Some((c.entry, c.stop, c.target));
        self.ticket_draw = Some(i);
    }

    /// 💼 La position dessinée suit le ticket (côté, stop, risque ; la cible garde son RR) ; s'il n'y en a pas, elle est
    /// créée à la dernière bougie.
    pub(crate) fn drawing_from_ticket(&mut self) {
        use argos_render::position;
        let t = self.watch.panels.trading.clone();
        let Some(last) = self.chart.bars.last().copied() else { return };
        let dir = if t.buy { 1.0 } else { -1.0 };
        let i = match self
            .ticket_draw
            .filter(|&i| self.chart.drawings.get(i).is_some_and(|d| position::is_position(&d.tool)))
        {
            Some(i) => i,
            None => {
                let mut d = self.new_drawing(if t.buy { "long" } else { "short" }, Point { t: last.time, p: last.close });
                d.pts.push(Point {
                    t: last.time + 20 * self.chart.interval.secs(),
                    p: last.close - dir * t.sl_pips.max(f64::EPSILON),
                });
                position::normalize(&mut d);
                let cfg = &self.watch.panels.journal.cfg;
                (d.style.account, d.style.lot) = (cfg.account, 1.0);
                self.chart.drawings.push(d);
                self.chart.drawings.len() - 1
            }
        };
        let d = &mut self.chart.drawings[i];
        if (d.tool == "long") != t.buy {
            position::invert(d);
        }
        position::normalize(d);
        let e = d.pts[0].p;
        let old = (e - d.pts[1].p).abs();
        let rr = if old > 0.0 { (d.pts[2].p - e).abs() / old } else { d.style.rr };
        d.pts[1].p = e - dir * t.sl_pips;
        d.pts[2].p = e + dir * t.sl_pips * rr;
        (d.style.risk, d.style.risk_pct) = (t.risk, false);
        self.ticket_from_drawing(i);
        self.save();
    }

    /// 💼 Ordre démo du volet Trading (vue courte du Simulateur) : Market au dernier prix, Limit / Stop au prix du
    /// ticket ; le stop et la cible de la position dessinée (sinon le stop du ticket et une cible à 2 R) ; le risque de
    /// ta stratégie donne la taille. Même chemin que le Simulateur : [`App::demo_ticket`].
    pub(crate) fn place_demo_order(&mut self) {
        use argos_core::simu::{Kind, Ticket};
        let t = self.watch.panels.trading.clone();
        let Some(last) = self.chart.bars.last().map(|b| b.close) else {
            self.banner = Some("💼 Pas encore de prix : attends les bougies".into());
            return self.redraw();
        };
        let dir = if t.buy { 1.0 } else { -1.0 };
        let kind = match t.tab {
            0 => Kind::Market,
            1 => Kind::Limit,
            _ => Kind::Stop,
        };
        let entry = if kind == Kind::Market || t.px <= 0.0 { last } else { t.px };
        let (sl, tp) = match t.plan {
            Some((_, s, g)) if (entry - s) * dir > 0.0 && (g - entry) * dir > 0.0 => (s, g),
            _ => (entry - dir * t.sl_pips, entry + dir * t.sl_pips * 2.0),
        };
        let ticket = Ticket {
            buy: t.buy,
            kind,
            price: entry,
            sl,
            tp,
            risk: argos_core::Money::from_f64(t.risk),
        };
        self.demo_ticket(ticket, last, now_s());
        self.redraw();
    }

    /// 💼🧪 Le seul chemin d'un ordre démo (volet Trading et Simulateur) : le Gardien PropFirm (s'il est allumé)
    /// bloque un ordre qui ferait dépasser tes limites, puis le ticket est vérifié (SL et TP obligatoires) et placé
    /// au compte démo ; une position ouverte s'inscrit au journal. `now` : secondes (l'heure du replay en replay).
    pub(crate) fn demo_ticket(&mut self, t: argos_core::simu::Ticket, last: f64, now: i64) {
        // 🔐 journal injoignable (PC, trousseau) : l'ordre serait ouvert sans être noté au journal — refusé
        if !self.jsync.pret {
            self.banner = Some("🔐 Ordre démo refusé : journal indisponible (rien ne serait noté)".into());
            return;
        }
        let base = self.symbol_label();
        let entry = if t.kind == argos_core::simu::Kind::Market { last } else { t.price };
        if let Some(why) = self.demo_garde(&t, last, now) {
            self.banner = Some(format!("🛡️ Gardien PropFirm : ordre bloqué ({why})"));
            return;
        }
        let fmt = |p: f64| argos_render::viewport::format_price(p, 2);
        let avant = self.demo.balance;
        // 💸 les coûts de la bourse du marché ouvert (sauf « sans frais ») ; ⚡ perpétuel : levier et marge isolée
        let (couts, perp) = (self.couts_marche().map(|c| c.1), self.perp_ticket());
        match t.place(&mut self.demo, &base, last, now, couts, perp) {
            Ok((id, true)) => {
                // 🤖 ce que disaient les indicateurs de la chart à l'ouverture (lot 7 : la carte Argos AI du trade)
                self.noter_lecture(id, None);
                self.journal_open(&base, t.buy, entry, Some(t.sl), Some(t.tp), t.risk.to_f64(), now);
                let payes = avant - self.demo.balance;
                let couts = if payes > argos_core::Money::ZERO {
                    format!(", coûts d'entrée {}", argos_render::simulateur::argent(payes, false))
                } else {
                    String::new()
                };
                // ⚡ perpétuel : le levier (après le sens) et le prix de liquidation, en court (la marge est au volet)
                let (levier, liq) = self
                    .demo
                    .positions
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| Some((p.perp?, p.liquidation()?)))
                    .map_or((String::new(), String::new()), |(x, liq)| {
                        (format!(" {}", argos_core::perp::levier_txt(x.levier)), format!(" · liq. {}", fmt(liq)))
                    });
                self.banner = Some(format!(
                    "💼 {}{levier} {base} ouvert à {} (risque {}{couts}){liq} : inscrit au portfolio et au journal",
                    if t.buy { "▲ Long" } else { "▼ Short" },
                    fmt(entry),
                    argos_render::simulateur::argent(t.risk, false)
                ));
            }
            Ok((_, false)) => {
                self.banner = Some(format!(
                    "💼 {} {} {base} placé à {} (risque {}) : il s'exécutera quand le prix l'atteindra",
                    if t.buy { "Buy" } else { "Sell" },
                    if t.kind == argos_core::simu::Kind::Limit { "Limit" } else { "Stop" },
                    fmt(entry),
                    argos_render::simulateur::argent(t.risk, false)
                ));
            }
            Err(e) => {
                self.banner = Some(format!("💼 Ordre refusé : {e}"));
                return;
            }
        }
        Self::write_prefs("demo_account.json", &self.demo);
    }

    /// 🛡️ Le Gardien PropFirm (s'il est allumé) refuse-t-il ce ticket ? La raison, sinon `None`.
    pub(crate) fn demo_garde(&self, t: &argos_core::simu::Ticket, last: f64, now: i64) -> Option<String> {
        if !self.watch.panels.trading.guardian {
            return None;
        }
        let base = self.symbol_label();
        let entry = if t.kind == argos_core::simu::Kind::Market { last } else { t.price };
        let has_sl = t.sl > 0.0 && (entry - t.sl) * if t.buy { 1.0 } else { -1.0 } > 0.0;
        let news = argos_render::eco::news_live(&self.watch.eco.agenda, &base, now);
        let j = &self.watch.panels.journal;
        j.guard(t.risk.to_f64(), now * 1000, self.demo.positions.len(), has_sl, news.as_deref())
    }

    /// 💼 Une position démo ouverte s'inscrit au journal (identifiant `demo-<symbole>-<heure>`, retrouvé à la sortie) ;
    /// sans SL ou sans TP (🎫 case TP/SL décochée), la fiche n'en a pas.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn journal_open(&mut self, base: &str, buy: bool, entry: f64, stop: Option<f64>, target: Option<f64>, risk: f64, now: i64) {
        let j = argos_core::journal::JTrade {
            id: format!("demo-{base}-{now}"),
            sym: base.into(),
            side: if buy { "long" } else { "short" }.into(),
            tf: self.chart.interval.short(),
            strat: "Démo".into(),
            entry: Some(entry),
            sl: stop,
            tp: target,
            risk: Some(risk),
            entry_ts: now * 1000,
            notes: "Ouvert par le ticket démo d'Argos".into(),
            ..Default::default()
        };
        self.watch.panels.journal.trades.push(j);
        self.journal_ecrire();
    }

    /// 💼 Ordres en attente atteints par le prix : ils deviennent des positions (au journal) ; le gardien PropFirm
    /// (s'il est allumé) revérifie au moment de l'exécution et annule ceux qui briseraient les règles.
    fn demo_pending(&mut self) {
        if self.demo.pending.is_empty() {
            return;
        }
        let now = now_s();
        let mut bases: Vec<String> = self.demo.pending.iter().map(|p| p.order.base.clone()).collect();
        bases.sort();
        bases.dedup();
        let guardian = self.watch.panels.trading.guardian;
        let mut msgs = vec![];
        for base in bases {
            let Some(px) = self.demo_price(&base) else { continue };
            let news = argos_render::eco::news_live(&self.watch.eco.agenda, &base, now);
            let journal = self.watch.panels.journal.clone();
            let open = self.demo.positions.len();
            let r = self.demo.check_pending(&base, px, px, now, |o| {
                let risk = o.stop.map_or(0.0, |s| (o.price - s).abs() * o.qty);
                guardian
                    .then(|| journal.guard(risk, now * 1000, open, o.stop.is_some(), news.as_deref()))
                    .flatten()
            });
            for (_, pos) in &r.filled {
                self.noter_lecture(*pos, None);
                if let Some(p) = self.demo.positions.iter().find(|p| p.id == *pos).cloned() {
                    let buy = p.side == argos_core::Side::Long;
                    self.journal_open(&p.base, buy, p.entry, p.stop, p.target, p.risk(), now);
                    msgs.push(format!(
                        "💼 Ordre exécuté : {} {} à {}",
                        if buy { "▲ Long" } else { "▼ Short" },
                        p.base,
                        argos_render::viewport::format_price(p.entry, 2)
                    ));
                }
            }
            for (_, why) in &r.refused {
                msgs.push(format!("🛡️ Ordre en attente annulé à l'exécution ({why})"));
            }
            if !r.expired.is_empty() {
                msgs.push(format!("💼 {} ordre(s) expiré(s)", r.expired.len()));
            }
            if !r.oco.is_empty() {
                msgs.push("🔀 L'autre ordre de l'OCO est annulé".into());
            }
            // 🔻 reduce-only orders reached: the trades they closed go to the journal; those with nothing left to
            // reduce are cancelled
            if !r.closed.is_empty() {
                let note = self.demo_closed(&r.closed);
                let net: argos_core::Money = r.closed.iter().map(|c| c.net()).sum();
                msgs.push(format!(
                    "🔻 Reduce-only exécuté : {base} réduit, {}{}",
                    argos_render::simulateur::argent(net, true),
                    note.suffixe()
                ));
            }
            if !r.nothing_to_reduce.is_empty() {
                msgs.push(format!("🔻 {}, ordre annulé", argos_core::portfolio::NOTHING_TO_REDUCE));
            }
        }
        if !msgs.is_empty() {
            Self::write_prefs("demo_account.json", &self.demo);
            self.banner = Some(msgs.join(" · "));
        }
    }

    /// 💼 Annule un ordre en attente (✕ du panneau).
    pub(crate) fn cancel_demo_order(&mut self, id: u64) {
        if self.demo.cancel(id) {
            Self::write_prefs("demo_account.json", &self.demo);
            self.banner = Some("💼 Ordre en attente annulé".into());
        }
        self.redraw();
    }

    /// 💼 Prix actuel d'un symbole démo : la chart si c'est lui, sinon le prix Hyperliquid de la watchlist (rafraîchi
    /// aux 3 s pour la crypto, 30 s pour HIP-3), y compris l'équivalent HIP-3 d'un symbole OANDA (XAUUSD → xyz:GOLD).
    pub(crate) fn demo_price(&self, base: &str) -> Option<f64> {
        if base == self.symbol_label() {
            if let Some(b) = self.chart.bars.last() {
                return Some(b.close);
            }
        }
        let mark = |k: &str| self.feeds.quotes.get(k).and_then(|q| q.mark.parse::<f64>().ok()).filter(|p| *p > 0.0);
        let hip3 = || {
            argos_data::oanda::OANDA
                .iter()
                .find(|o| o.0.replace('_', "") == base)
                .and_then(|o| o.4)
                .and_then(mark)
        };
        // (₿ a pair against BTC: never the coin's dollar price of Hyperliquid)
        let coin = || {
            (!argos_data::paire::contre_btc(base))
                .then(|| argos_data::whales::coin_of(base))
                .and_then(|c| mark(&c))
        };
        mark(base).or_else(hip3).or_else(coin)
    }

    /// 💼 Stop ou cible touché : chaque position démo est vérifiée à son propre prix (même un symbole qui n'est pas sur
    /// la chart) ; la position se ferme et le journal reçoit la sortie et le P&L. `true` si quelque chose a fermé.
    pub(crate) fn demo_exits(&mut self) -> bool {
        // ⏯ replay du Simulateur : le compte joue sur les bougies du replay, pas sur les prix du direct
        if self.ui.sim_replay {
            return false;
        }
        let mut bases: Vec<String> = self.demo.positions.iter().map(|p| p.base.clone()).collect();
        bases.sort();
        bases.dedup();
        let now = now_s();
        let mut closed = vec![];
        for base in &bases {
            if let Some(px) = self.demo_price(base) {
                closed.extend(self.demo.check_exits(base, px, px, now));
            }
        }
        self.demo_closed(&closed);
        self.demo_pending();
        // ⚡ funding des perpétuels ouverts : chaque échéance passée (aussi celles d'une app fermée entre-temps), au taux
        // de la bourse (sinon celui de base) et au prix actuel
        let marches: std::collections::HashMap<String, (f64, f64)> = bases
            .iter()
            .filter(|b| self.demo.positions.iter().any(|p| p.base == **b && p.perp.is_some()))
            .filter_map(|b| Some((b.clone(), (self.funding_demo(b).0, self.demo_price(b)?))))
            .collect();
        let payes = if marches.is_empty() {
            vec![]
        } else {
            self.demo.payer_funding(now, |b| marches.get(b).map(|m| m.0), |b| marches.get(b).map(|m| m.1))
        };
        if !payes.is_empty() {
            Self::write_prefs("demo_account.json", &self.demo);
            if self.banner.is_none() {
                let l: Vec<String> = payes
                    .iter()
                    .map(|(b, m)| format!("{b} {}", argos_render::simulateur::argent(argos_core::Money::ZERO - *m, true)))
                    .collect();
                self.banner = Some(format!("⚡ Funding démo : {}", l.join(" · ")));
            }
        }
        let flat = self.guard_watch();
        !closed.is_empty() || !payes.is_empty() || flat
    }

    /// 🛡️ Pilule du Gardien PropFirm dans la barre du haut : ses vrais chiffres (positions démo ouvertes comprises),
    /// verte, orange ou rouge selon son niveau ; `None` quand le Gardien est éteint.
    pub(crate) fn gardien_pilule(&self) -> Option<(String, argos_render::text::Rgb)> {
        use argos_render::journal::GuardLevel;
        if !self.watch.panels.trading.guardian {
            return None;
        }
        let floating: f64 = self.demo.positions.iter().filter_map(|p| self.demo_price(&p.base).map(|px| p.pnl(px))).sum();
        let (t, niveau) = self.watch.panels.journal.pilule(floating, self.demo.positions.len(), now_s() * 1000);
        let c = match niveau {
            GuardLevel::Ok => argos_render::palette::up(),
            GuardLevel::Half => argos_render::palette::GOLD,
            GuardLevel::Warn => argos_render::palette::ORANGE,
            GuardLevel::Bad => argos_render::palette::down(),
        };
        Some((t, c))
    }

    /// 🛡️ Surveillance du gardien PropFirm (s'il est allumé) : si la perte du jour ou la perte totale, positions
    /// ouvertes comprises, touche sa limite, tout se ferme (positions et ordres en attente) et la journée est
    /// verrouillée jusqu'à minuit (heure du reset). Avant ça, un bandeau prévient une fois par palier et par jour (50 %
    /// puis le seuil d'alerte). `true` si des positions ont été fermées.
    pub(crate) fn guard_watch(&mut self) -> bool {
        if !self.watch.panels.trading.guardian || self.demo.positions.is_empty() {
            return false;
        }
        let now = now_s();
        // une position sans prix en direct compte pour 0 (on ne ferme pas sur un prix inconnu)
        let prices = self.demo_prices();
        let floating: f64 = self.demo.positions.iter().filter_map(|p| prices.get(&p.base).map(|&px| p.pnl(px))).sum();
        let Some(hit) = self.watch.panels.journal.breach(floating, now * 1000) else {
            let open = self.demo.positions.len();
            if let Some(pct) = self.watch.panels.journal.warn_once(floating, open, now * 1000) {
                self.banner = Some(format!("⚠️ Gardien PropFirm : tu es à {pct:.0} % de ta limite de perte"));
            }
            return false;
        };
        let (closed, _) = self.demo.close_all(|b| prices.get(b).copied(), now, "gardien PropFirm");
        self.watch.panels.journal.cfg.lock = self.watch.panels.journal.propfirm_day(now * 1000);
        let note = self.demo_closed(&closed);
        let verrou = self.journal_noter();
        // (a trade or the lock not noted: said at the end, the closing and the lock stay)
        let avis = [note, verrou]
            .into_iter()
            .find(|n| matches!(n, Note::NonNotee(_)))
            .map_or(String::new(), |n| n.suffixe());
        self.banner = Some(format!(
            "🛡️ Gardien PropFirm : {hit} atteinte → {} position(s) fermée(s), journée verrouillée jusqu'à minuit (heure du reset){avis}",
            closed.len()
        ));
        !closed.is_empty()
    }

    /// 🔓 « Déverrouiller aujourd'hui » (carte du gardien) : demande la raison (boîte de saisie).
    pub(crate) fn guard_unlock_demander(&mut self) {
        let anchor = Rect::new(self.win_w() - self.watch.pw(), self.top_h() + 60.0, self.watch.pw(), 1.0);
        self.ui.name_box = Some(NameBox::new(NameFor::Raison, "", anchor, self.win_w()));
    }

    /// 🔓 La raison est écrite : le verrou du jour saute et la trace (jour, heure, raison) est gardée dans le journal
    /// chiffré ; sans raison, la journée reste verrouillée.
    pub(crate) fn guard_unlock(&mut self, raison: &str) {
        match self.watch.panels.journal.unlock(raison, now_s() * 1000) {
            Ok(()) => {
                self.journal_ecrire();
                self.banner = Some("🔓 Déverrouillée aujourd'hui · raison notée au Journal".into());
            }
            Err(e) => self.banner = Some(e),
        }
    }

    /// 💼 Prix actuel de chaque symbole démo ouvert (absent : pas de prix en direct).
    pub(crate) fn demo_prices(&self) -> std::collections::HashMap<String, f64> {
        self.demo
            .positions
            .iter()
            .filter_map(|p| self.demo_price(&p.base).map(|px| (p.base.clone(), px)))
            .collect()
    }

    /// 💼 « Tout fermer » (bouton du panneau Trading démo) : toutes les positions démo au prix actuel et tous les
    /// ordres en attente annulés.
    pub(crate) fn close_demo_positions(&mut self) {
        if self.demo.positions.is_empty() && self.demo.pending.is_empty() {
            self.banner = Some("💼 Aucune position ni ordre démo".into());
            return self.redraw();
        }
        let prices = self.demo_prices();
        let open = self.demo.positions.len();
        let (closed, cancelled) = self.demo.close_all(|b| prices.get(b).copied(), now_s(), "fermé à la main");
        let left = open - closed.len();
        let pnl = argos_render::simulateur::argent(closed.iter().map(|c| c.net()).sum(), true);
        // the closing is always done (it lowers the risk), journal or not; the banner says if it is noted
        let pause = self.watch.panels.journal.cfg.pause_until;
        let note = self.demo_closed(&closed);
        let mut parts = vec![];
        if !closed.is_empty() {
            parts.push(format!("{} position(s) fermée(s) : {pnl}{}", closed.len(), note.suffixe()));
        }
        if cancelled > 0 {
            parts.push(format!("{cancelled} ordre(s) en attente annulé(s)"));
        }
        if left > 0 {
            parts.push(format!("{left} sans prix en direct, encore ouverte(s)"));
        }
        // le bandeau de la pause anti-tilt (s'il en vient une) passe devant
        if self.watch.panels.journal.cfg.pause_until == pause {
            self.banner = Some(format!("💼 {}", parts.join(" ; ")));
        }
        if closed.is_empty() {
            Self::write_prefs("demo_account.json", &self.demo);
        }
        self.redraw();
    }

    /// 💼 Inscrit des sorties au journal (sortie, heure, P&L), regarde la pause anti-tilt et sauvegarde le compte ;
    /// rend ce qu'il en est au journal (`Note`) : un trade sans sa fiche dans le journal n'y est pas noté.
    pub(crate) fn demo_closed(&mut self, closed: &[argos_core::portfolio::ClosedTrade]) -> Note {
        let mut absentes = 0;
        let mut auto = None;
        for c in closed {
            // 🎯 résultat net du trade entier (coûts déduits) et ses coûts : ses sorties partielles comprises
            let parts = || self.demo.history.iter().filter(|h| h.base == c.base && h.opened_at == c.opened_at);
            let total: argos_core::Money = parts().map(|h| h.net()).sum();
            // ⚡ le funding payé (ou reçu) compte avec les coûts
            let frais: argos_core::Money = parts().map(|h| h.fees + h.funding).sum();
            if let Some(j) = self
                .watch
                .panels
                .journal
                .trades
                .iter_mut()
                // (two trades opened in the same second share their key: the one still open takes the exit)
                .find(|t| t.id == format!("demo-{}-{}", c.base, c.opened_at) && t.exit.is_none())
            {
                let px = argos_render::viewport::format_price(c.exit, 2);
                if c.partiel {
                    // le trade reste ouvert au journal : la sortie partielle est notée
                    j.notes = format!(
                        "{} · ✂ {} sorti à {px} ({}) : {}",
                        j.notes,
                        fmt_qty(c.qty),
                        c.reason,
                        argos_render::simulateur::argent(c.net(), true)
                    );
                } else {
                    (j.exit, j.exit_ts, j.pnl) = (Some(c.exit), Some(c.closed_at * 1000), Some(total.to_f64()));
                    // 💸 frais, écart et glissement du trade entier (le P&L noté est net)
                    j.fees = (frais != argos_core::Money::ZERO).then(|| frais.to_f64());
                    j.notes = format!("{} · fermé : {}", j.notes, c.reason);
                }
            } else {
                absentes += 1;
            }
            // 💸 the trade's costs (fees, spread, slippage) in the banner; the result shown is net
            let couts = if c.fees > argos_core::Money::ZERO {
                format!(" (coûts {})", argos_render::simulateur::argent(c.fees, false))
            } else {
                String::new()
            };
            // the banner waits for the journal write (its suffix says whether the trade was noted)
            if c.partiel {
                auto = Some(format!(
                    "✂ {} : {} sorti ({}) : {}{couts}",
                    c.base,
                    fmt_qty(c.qty),
                    c.reason,
                    argos_render::simulateur::argent(c.net(), true)
                ));
            } else if c.reason != "fermé à la main" {
                auto = Some(format!(
                    "💼 {} fermé ({}) : {}{couts}",
                    c.base,
                    c.reason,
                    argos_render::simulateur::argent(c.net(), true)
                ));
            }
        }
        if closed.is_empty() {
            return Note::Notee;
        }
        // 📅 lot 8: an exit of the replay (its time is the market's, back then) keeps the real time it was made at:
        // the month report puts it in the month it was really traded
        let now = now_s();
        for c in closed.iter().filter(|c| (now - c.closed_at).abs() > REPLAY_GAP) {
            if let Some(h) = self.demo.history.iter_mut().rev().find(|h| *h == c) {
                h.real_closed_at = Some(now);
            }
        }
        // 🤖 lot 7 : la carte « Argos AI · ton trade » du dernier trade fini
        self.avis_apres(closed);
        let pause = self.watch.panels.journal.cfg.pause_until;
        self.tilt_check(closed);
        Self::write_prefs("demo_account.json", &self.demo);
        let note = match self.journal_noter() {
            Note::NonNotee(pourquoi) => Note::NonNotee(pourquoi),
            _ if absentes > 0 => Note::NonNotee("trade absent du journal".into()),
            n => n,
        };
        // the banner of an automatic closing (stop, target, end of replay) once the write is known; the anti-tilt
        // pause's banner (if one starts) goes first
        if let Some(m) = auto.filter(|_| self.watch.panels.journal.cfg.pause_until == pause) {
            self.banner = Some(format!("{m}{}", note.suffixe()));
        }
        note
    }

    /// 🧊 Pause anti-tilt (gardien allumé), juste après des trades démo fermés : elle démarre après `tilt_n` pertes
    /// de suite (ou un trade noté « revenge », si l'option est cochée) et bloque les nouveaux ordres `tilt_min`
    /// minutes, à l'heure de ces trades (celle du replay en replay) ; les positions et ordres déjà posés restent.
    fn tilt_check(&mut self, closed: &[argos_core::portfolio::ClosedTrade]) {
        if !self.watch.panels.trading.guardian {
            return;
        }
        let j = &self.watch.panels.journal;
        let revenge = closed.iter().any(|c| {
            j.trades
                .iter()
                .any(|t| t.id == format!("demo-{}-{}", c.base, c.opened_at) && t.emo == "revenge")
        });
        let now = closed.iter().map(|c| c.closed_at).max().unwrap_or(0) * 1000;
        let Some((until, why)) = j.cfg.tilt_start(&self.demo.history, closed, revenge, now) else {
            return;
        };
        let cfg = &mut self.watch.panels.journal.cfg;
        cfg.pause_until = cfg.pause_until.max(until);
        self.banner = Some(format!("🧊 Pause anti-tilt : {why}, ordres bloqués {} min", cfg.tilt_min));
    }
}

/// Une quantité courte pour le journal : « 0,5 », « 12,25 » (au plus 4 décimales, sans zéros inutiles).
pub(super) fn fmt_qty(q: f64) -> String {
    let t = format!("{q:.4}");
    t.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
}
