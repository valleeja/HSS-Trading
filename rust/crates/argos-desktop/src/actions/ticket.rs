//! 🎫 Module 31 « Trading démo » : le ticket de la feuille Trading branché au compte démo. Chaque image, ce qu'il
//! montre (marché, Mid, disponible, pouvoir d'achat, position, bloc Compte, estimation) ; VENDRE / ACHETER ; les
//! champs tapés ; ⌖ sur la chart ; le curseur % tenu ; « Carnet › ». Compte démo seulement : rien ici ne parle à une
//! bourse.

use crate::*;
use argos_core::simu::{self, Placed};
use argos_render::panels::ticket::Champ;
use argos_render::wlmenu::{NameBox, NameFor};

impl App {
    /// 🎫 What the ticket shows, from the demo account and the market open on the chart (each frame, once the last
    /// price is in `trading.prix`). The quantity typed stays; its % of the buying power follows the prices.
    pub(crate) fn ticket_sync(&mut self) {
        let base = self.symbol_label();
        let last = self.watch.panels.trading.prix;
        let marche = match &self.chart.source {
            Source::Live { exchange, base } => {
                let n = argos_data::tv::name(*exchange, base);
                (n.split_once(':').map_or(n.clone(), |(_, s)| s.to_string()), exchange.name().to_string())
            }
            Source::Oanda { .. } => (base.clone(), "OANDA".to_string()),
            _ => (base.clone(), String::new()),
        };
        // Mid: the middle of the book's best bid and best ask (the book of this coin only), else the last price
        let carnet = self.feeds.book.as_ref().filter(|b| Some(&b.coin) == self.book_coin().as_ref());
        let meilleur = |l: Option<&argos_data::book::Level>| l.map(|l| l.px);
        let mid = simu::mid(
            meilleur(carnet.and_then(|b| b.bids.first())),
            meilleur(carnet.and_then(|b| b.asks.first())),
            last,
        );
        let levier_max = self.perp_marche().map_or(0.0, |b| b.levier_max);
        let (couts, perp) = (self.couts_marche().map(|c| c.1), self.perp_ticket());
        // the position on this market: its signed quantity and its P&L at the price of the moment
        let px = self.demo_price(&base);
        let mut position: Option<(f64, f64)> = None;
        for p in self.demo.positions.iter().filter(|p| p.base == base) {
            let (q, pnl) = position.unwrap_or_default();
            let s = if p.side == argos_core::Side::Long { 1.0 } else { -1.0 };
            position = Some((q + s * p.qty, pnl + px.map_or(0.0, |x| p.pnl(x))));
        }
        let compte = self.demo.summary(|b| self.demo_price(b));
        // ⚡ the funding of the PropFirm day (it ends at midnight in the reset's time zone); two days back at most
        let now = now_s();
        let j = &self.watch.panels.journal;
        let jour = j.propfirm_day(now * 1000);
        let funding = self.demo.funding_paid(|t| now - t < 2 * 86_400 && j.propfirm_day(t * 1000) == jour).to_f64();
        // 📱 the Argos AI round sits over the bottom of the sheet (Jee's choice B: everywhere): VENDRE / ACHETER, kept
        // at the bottom, stop before it (the sheet takes the phone's whole width)
        let ai = self.ai_bouton_rect();
        let bas = self.cell_h() - argos_render::panels::ticket::BAS_COLLE;
        let place_ai = if self.top.phone && ai.w > 0.0 && ai.y + ai.h > bas {
            (self.win_w() - ai.x).max(0.0)
        } else {
            0.0
        };
        // ₿ a pair against BTC: its prices are in BTC, the demo account in dollars: no ticket yet (a message)
        let pas_btc = matches!(&self.chart.source, Source::Live { base, .. } if argos_data::paire::contre_btc(base));
        let t = &mut self.watch.panels.trading;
        t.pas_btc = pas_btc;
        t.place_ai = place_ai;
        t.marche = marche;
        (t.mid, t.levier_max, t.position, t.compte, t.funding_jour) = (mid, levier_max, position, compte, funding);
        t.dispo = self.demo.disponible().to_f64();
        t.pouvoir = simu::buying_power(&self.demo, perp).to_f64();
        t.reducible = (self.demo.reducible(&base, true), self.demo.reducible(&base, false));
        t.set_qty(t.qty);
        t.estimate = simu::estimate(&self.demo, &t.ordre(t.sens_affiche()), &base, last, couts, perp);
        // ⚡ one leverage for the demo orders: the Simulateur shows the ticket's
        if let Some(s) = self.ui.sim.as_mut() {
            s.levier = t.levier;
        }
    }

    /// 🎫 VENDRE (`buy` false) / ACHETER : the ticket's order goes to the demo account. 🛡 The Gardien PropFirm (when
    /// on) checks it first, its SL required (a reduce-only order only takes risk off); a position opened is noted in
    /// the journal, without SL when there is none (Argos AI then says « Entré sans stop »).
    pub(crate) fn passer_ticket(&mut self, buy: bool) {
        // 🔐 journal out of reach (PC, keychain): the order would open without being noted — refused
        if !self.jsync.pret {
            self.banner = Some("🔐 Ordre démo refusé : journal indisponible (rien ne serait noté)".into());
            return;
        }
        let Some(last) = self.chart.bars.last().map(|b| b.close) else {
            self.banner = Some("💼 Pas encore de prix : attends les bougies".into());
            return;
        };
        let t = self.watch.panels.trading.clone();
        let o = t.ordre(buy);
        if o.qty <= 0.0 {
            self.banner = Some("🎫 Entre une taille : la quantité, la valeur ou le curseur %".into());
            return;
        }
        let (base, now) = (self.symbol_label(), now_s());
        let entree = o.fill_price(last);
        let risque = o.sl.map_or(0.0, |s| (entree - s).abs() * o.qty);
        if t.guardian && !o.reduce_only {
            // (the Gardien checks the risk at the SL: it stays required while it is on, the TP/SL box unticked or not)
            if o.sl.is_none() {
                self.banner = Some("🛡️ Gardien PropFirm : ordre bloqué (SL obligatoire : coche TP/SL)".into());
                return;
            }
            let news = argos_render::eco::news_live(&self.watch.eco.agenda, &base, now);
            let j = &self.watch.panels.journal;
            if let Some(why) = j.guard(risque, now * 1000, self.demo.positions.len(), o.sl.is_some(), news.as_deref()) {
                self.banner = Some(format!("🛡️ Gardien PropFirm : ordre bloqué ({why})"));
                return;
            }
        }
        let fmt = |p: f64| argos_render::viewport::format_price(p, 2);
        let qte = format!("{} {}", super::trading::fmt_qty(o.qty), base);
        let avant = self.demo.balance;
        // 💸 the costs of the market's exchange (unless « sans frais »); ⚡ perpetual: leverage and isolated margin
        let (couts, perp) = (self.couts_marche().map(|c| c.1), self.perp_ticket());
        match o.place(&mut self.demo, &base, last, now, couts, perp) {
            Ok(Placed::Opened(id)) => {
                // 🤖 what the chart's indicators said at the opening (lot 7: the trade's Argos AI card)
                self.noter_lecture(id, None);
                let p = self.demo.positions.iter().find(|p| p.id == id).cloned();
                let entree = p.as_ref().map_or(entree, |p| p.entry);
                self.journal_open(&base, buy, entree, o.sl, o.tp, risque, now);
                let payes = avant - self.demo.balance;
                let couts = if payes > argos_core::Money::ZERO {
                    format!(", coûts d'entrée {}", argos_render::simulateur::argent(payes, false))
                } else {
                    String::new()
                };
                // ⚡ perpetual: the leverage (after the side) and the liquidation price
                let (levier, liq) = p
                    .and_then(|p| Some((p.perp?, p.liquidation()?)))
                    .map_or((String::new(), String::new()), |(x, liq)| {
                        (format!(" {}", argos_core::perp::levier_txt(x.levier)), format!(" · liq. {}", fmt(liq)))
                    });
                let sans_sl = if o.sl.is_none() { " · sans SL" } else { "" };
                self.banner = Some(format!(
                    "💼 {}{levier} {base} ouvert à {} ({qte}{couts}){liq}{sans_sl} : inscrit au portfolio et au journal",
                    if buy { "▲ Long" } else { "▼ Short" },
                    fmt(entree),
                ));
            }
            Ok(Placed::Resting(_)) => {
                let genre = if o.kind == simu::Kind::Limit { "Limite" } else { "Stop" };
                let duree = if o.kind == simu::Kind::Limit {
                    format!(" ({})", o.validity.name())
                } else {
                    String::new()
                };
                self.banner = Some(if o.reduce_only {
                    format!(
                        "🔻 Reduce-only : {} {genre} de {qte} à {} : elle réduira ta position quand le prix l'atteindra",
                        if buy { "achat" } else { "vente" },
                        fmt(o.price)
                    )
                } else {
                    format!(
                        "💼 {} {genre} {qte} placé à {}{duree} : il s'exécutera quand le prix l'atteindra",
                        if buy { "Achat" } else { "Vente" },
                        fmt(o.price)
                    )
                });
            }
            // 🔻 reduce-only filled now: the trades it closed go to the journal (their banner says so)
            Ok(Placed::Reduced(closed)) => {
                self.demo_closed(&closed);
            }
            Err(e) => {
                self.banner = Some(format!("💼 Ordre refusé : {e}"));
                return;
            }
        }
        Self::write_prefs("demo_account.json", &self.demo);
    }

    /// 🎫 A field of the ticket touched: its box opens with the value it shows (📱 near the top: the keyboard comes up
    /// from the bottom).
    pub(crate) fn saisir_ticket(&mut self, c: Champ) {
        let anchor = if self.top.phone {
            Rect::new(0.0, self.top_h() + 60.0, self.win_w(), 1.0)
        } else {
            let (x, y) = self.pointer.win_mouse;
            Rect::new(x - 160.0, y + 10.0, 320.0, 1.0)
        };
        let v = self.watch.panels.trading.champ_txt(c);
        self.ui.name_box = Some(NameBox::new(NameFor::Ticket(c), &v, anchor, self.win_w()));
    }

    /// 🎫 The value typed in a field's box: the ticket takes it, or the banner says what is wrong.
    pub(crate) fn ticket_typed(&mut self, c: Champ, s: &str) {
        if let Err(e) = self.watch.panels.trading.saisir(c, s) {
            self.banner = Some(format!("🎫 {} : {e}", c.titre().trim_start_matches(|ch: char| !ch.is_alphabetic())));
        }
    }

    /// ⌖ The ticket waits for a price: the banner says to touch the chart.
    pub(crate) fn ticket_viser(&mut self) {
        self.banner = Some(argos_render::actions::sans_touches("⊕ Touche la chart au prix voulu (Échap : annuler)"));
    }

    /// ⌖ The touch on the chart at `price`: the order's price takes this level (to the chart's tick); the size stays.
    pub(crate) fn ticket_cible(&mut self, price: f64) {
        let tick = argos_render::position::tick_of(&self.chart.bars);
        let t = &mut self.watch.panels.trading;
        t.px = if tick > 0.0 { (price / tick).round() * tick } else { price };
        t.cible = false;
        t.set_qty(t.qty);
        self.banner = Some(format!("⊕ Prix de l'ordre : {}", argos_render::viewport::format_price(t.px, 2)));
        self.redraw();
    }

    /// 📖 « Carnet › » : 🖥 the Book module turns on in the column of modules (the sheet stays open); 📱 the phone has
    /// no column.
    pub(crate) fn ticket_carnet(&mut self) {
        use argos_render::dock::Module;
        if self.top.phone {
            self.banner = Some("📖 Le carnet d'ordres est un module de la colonne : sur grand écran (PC, tablette en paysage)".into());
            return;
        }
        let d = &mut self.watch.dock;
        if let Some(i) = d.items.iter_mut().find(|i| i.module == Module::Book) {
            i.on = true;
        }
        d.on = true;
        Self::write_prefs("dock.json", &self.watch.dock);
        if !self.watch.dock_big {
            self.banner = Some("📖 Carnet ajouté à la colonne de modules : elle se montre dans une fenêtre plus large".into());
        }
    }
}
