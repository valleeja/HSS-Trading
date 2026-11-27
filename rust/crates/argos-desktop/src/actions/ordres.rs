//! 💼 Ordres démo sur la chart (comme TradingView) : le branchement. Les lignes viennent du compte démo unique
//! (`self.demo`) : ce que le volet Trading, le Simulateur ou la chart y mettent apparaît tout seul ; un SL / TP
//! déplacé ici change le compte (et la fiche du Journal). Le dessin et les zones sont dans `argos_render::ordres`.

use crate::*;
use argos_core::Money;
use argos_render::ordres::{Apercu, Brouillon, OrdreChart, Prise, VueOrdres};

impl App {
    /// Les positions, ordres en attente et le brouillon du symbole affiché.
    pub(crate) fn vue_ordres(&self) -> VueOrdres {
        let sym = self.symbol_label();
        let usd = |a: f64, b: f64, q: f64| Money::from_f64((a - b).abs() * q);
        let mut ordres: Vec<OrdreChart> = self
            .demo
            .positions
            .iter()
            .filter(|p| p.base == sym)
            .map(|p| OrdreChart {
                id: p.id,
                attente: None,
                buy: p.side == argos_core::Side::Long,
                qty: p.qty,
                entree: p.entry,
                sl: p.stop,
                tp: p.target,
                pnl: self.demo_price(&p.base).map(|px| Money::from_f64(p.pnl(px))),
                perte_sl: p.stop.map_or(Money::ZERO, |s| usd(p.entry, s, p.qty)),
                gain_tp: p.target.map_or(Money::ZERO, |t| usd(t, p.entry, p.qty)),
                liq: p.liquidation().zip(p.perp.map(|x| x.levier)),
            })
            .collect();
        ordres.extend(self.demo.pending.iter().filter(|p| p.order.base == sym).map(|p| {
            let o = &p.order;
            OrdreChart {
                id: p.id,
                attente: Some(if p.kind == argos_core::portfolio::OrderKind::Limit {
                    "Limite"
                } else {
                    "Stop"
                }),
                buy: o.side == argos_core::Side::Long,
                qty: o.qty,
                entree: o.price,
                sl: o.stop,
                tp: o.target,
                pnl: None,
                perte_sl: o.stop.map_or(Money::ZERO, |s| usd(o.price, s, o.qty)),
                gain_tp: o.target.map_or(Money::ZERO, |t| usd(t, o.price, o.qty)),
                liq: None,
            }
        }));
        VueOrdres {
            ordres,
            brouillon: self.ui.brouillon.clone(),
            ticket: self.ticket_apercu(),
            erreur: self.pointer.order_drag.and_then(|(p, _)| self.pointer.order_err.map(|m| (p, m))),
        }
    }

    /// 🎫 The order the Trading ticket prepares, while its sheet shows the ticket: its lines on the chart (module 31).
    fn ticket_apercu(&self) -> Option<Apercu> {
        use argos_core::simu::Kind;
        let w = &self.watch;
        let t = &w.panels.trading;
        if !(w.prefs.open && w.prefs.panel == wl::Tool::Trading && w.panels.feuille_trading()) || t.positions_vue || t.prix <= 0.0 {
            return None;
        }
        // ₿ a pair against BTC: the sheet says « pas encore », so no ticket line on the chart nor widened scale
        if t.pas_btc || matches!(&self.chart.source, Source::Live { base, .. } if argos_data::paire::contre_btc(base)) {
            return None;
        }
        let buy = t.sens_affiche();
        let o = t.ordre(buy);
        let entree = match o.kind {
            Kind::Limit => Some(("Limite", o.price)),
            Kind::Stop => Some(("Stop", o.price)),
            Kind::Market => None,
        };
        Some(Apercu {
            buy,
            entree,
            depart: o.fill_price(t.prix),
            sl: o.sl,
            tp: o.tp,
            qty: o.qty,
        })
    }

    /// 🎫 📏 Zoom auto (mock-up of 22 Nov 2026), once per image: while the ticket shows, the price scale is widened
    /// just enough to show its Limite, TP and SL (📱 between the legend and the sheet) — only when one is out of view,
    /// never while one of its lines is dragged, never again once the scale is moved by hand (or its ratio locked);
    /// when the ticket closes, the scale from before comes back.
    pub(crate) fn ticket_echelle(&mut self) {
        let cle = (self.symbol_label(), self.chart.interval.secs());
        let sc = self.chart.settings.scale;
        let Some(apercu) = self.ticket_apercu() else {
            if let Some(z) = self.ui.ticket_zoom.take() {
                if z.cle == cle && (sc.manual, sc.lock_ratio) != z.avant {
                    (self.chart.settings.scale.manual, self.chart.settings.scale.lock_ratio) = z.avant;
                }
            }
            return;
        };
        if self.ui.ticket_zoom.as_ref().is_none_or(|z| z.cle != cle) {
            self.ui.ticket_zoom = Some(crate::state::TicketZoom {
                cle,
                avant: (sc.manual, sc.lock_ratio),
                ..Default::default()
            });
        }
        let Some(z) = self.ui.ticket_zoom.as_mut() else { return };
        // (moved by hand: bounds that are neither the ones from before nor the last ones set, or a ratio lock)
        if !z.main && (sc.manual != z.pose.or(z.avant.0) || sc.lock_ratio != z.avant.1) {
            z.main = true;
        }
        let tenu = matches!(self.pointer.order_drag, Some((Prise::TkEntree | Prise::TkSl | Prise::TkTp, _)));
        if z.main || sc.lock_ratio.is_some() || tenu {
            return;
        }
        let Some(s) = self.scale_now() else { return };
        // 📱 under the legend (it spans the width) and above the Trading sheet over the chart (glass)
        let h = s.height.max(1.0);
        let haut = if self.top.phone {
            (argos_render::indleg::bas_legende(&self.ind.out, self.ind.leg.0, true) + 8.0 - s.top) / h
        } else {
            0.0
        };
        let bas = self
            .ui
            .rows
            .map(|(feuille, _)| feuille)
            .filter(|&feuille| feuille < self.chart.viewport.plot_h())
            .map_or(1.0, |feuille| (feuille - s.top) / h);
        let n = s.garder_en_vue(&apercu.prix_en_vue(), (haut, bas));
        if (n.min, n.max) != (s.min, s.max) {
            self.chart.settings.scale.manual = Some((n.min, n.max));
            if let Some(z) = self.ui.ticket_zoom.as_mut() {
                z.pose = Some((n.min, n.max));
            }
        }
    }

    /// 💼 Un ordre préparé à `price` (menu de la chart) : SL à la distance du ticket du volet Trading (sinon 0,5 %),
    /// TP à 2 R, taille tirée du risque du ticket. Rien ne part avant le clic sur « Buy » / « Sell ».
    pub(crate) fn brouillon_nouveau(&mut self, price: f64, buy: bool) {
        let t = &self.watch.panels.trading;
        let d = if t.sl_pips > 0.0 && t.sl_pips < price { t.sl_pips } else { price * 0.005 };
        let s = if buy { 1.0 } else { -1.0 };
        self.ui.brouillon = Some(Brouillon {
            buy,
            entree: price,
            sl: price - s * d,
            tp: price + s * d * 2.0,
            qty: 0.0,
            genre: "Marché",
            perte_sl: Money::ZERO,
            gain_tp: Money::ZERO,
        });
        self.brouillon_maj();
        self.banner = Some(format!(
            "💼 Ordre préparé : déplace l'entrée, le SL ou le TP, puis clique « {} »",
            if buy { "Buy" } else { "Sell" }
        ));
        self.redraw();
    }

    /// Le genre (Limite / Stop selon le dernier prix), la taille et les montants du brouillon.
    fn brouillon_maj(&mut self) {
        let last = self.chart.bars.last().map_or(0.0, |b| b.close);
        let risque = Money::from_f64(self.watch.panels.trading.risk.max(0.0));
        let Some(b) = self.ui.brouillon.as_mut() else { return };
        b.genre = if (b.entree - last).abs() <= f64::EPSILON * last.abs().max(1.0) {
            "Marché"
        } else if (b.entree < last) == b.buy {
            "Limite"
        } else {
            "Stop"
        };
        b.qty = argos_core::simu::size_for_risk(risque, b.entree, b.sl);
        b.perte_sl = Money::from_f64((b.entree - b.sl).abs() * b.qty);
        b.gain_tp = Money::from_f64((b.tp - b.entree).abs() * b.qty);
    }

    /// Ce que la souris touche parmi les ordres de la chart.
    pub(crate) fn ordre_prise(&self) -> Option<Prise> {
        let (mx, my) = self.pointer.mouse?;
        if mx > self.chart.viewport.plot_w() || my > self.chart.viewport.plot_h() || self.ui.context.is_open() || self.ui.list.is_some() {
            return None;
        }
        let sc = self.scale_now()?;
        let v = self.vue_ordres();
        argos_render::ordres::prise(&argos_render::text::Text::default(), &|p| sc.y_of(p), self.chart.viewport.plot_w(), &v, mx, my)
    }

    /// Clic sur un ordre : ✕ (annuler / fermer), Buy / Sell (envoyer le brouillon), ou début du glisser d'une ligne.
    pub(crate) fn ordre_clic(&mut self, p: Prise) {
        match p {
            Prise::Fermer(id) => {
                if self.demo.pending.iter().any(|o| o.id == id) {
                    return self.cancel_demo_order(id);
                }
                let px = self.demo.positions.iter().find(|o| o.id == id).and_then(|o| self.demo_price(&o.base));
                if let Some(c) = px.and_then(|px| self.demo.close(id, px, now_s(), "fermé à la main").ok()) {
                    // 💸 le résultat net et les coûts du trade, comme les bandeaux du ticket
                    let couts = if c.fees > Money::ZERO {
                        format!(" (coûts {})", argos_render::simulateur::argent(c.fees, false))
                    } else {
                        String::new()
                    };
                    self.banner = Some(format!("💼 Position fermée : {}{couts}", argos_render::simulateur::argent(c.net(), true)));
                    self.demo_closed(&[c]);
                }
            }
            Prise::BrAnnuler => self.ui.brouillon = None,
            Prise::BrEnvoyer => return self.brouillon_envoyer(),
            // la ligne de liquidation se montre, elle ne se déplace pas
            Prise::Liq(_) => return,
            _ => {
                let depart = self.niveau_de(p).unwrap_or(0.0);
                self.pointer.order_drag = Some((p, depart));
                self.pointer.order_err = None;
            }
        }
        self.redraw();
    }

    /// Prix actuel de la ligne `p`.
    fn niveau_de(&self, p: Prise) -> Option<f64> {
        let b = self.ui.brouillon.as_ref();
        match p {
            Prise::Entree(id) | Prise::Sl(id) | Prise::Tp(id) => {
                let (_, e, s, t) = self.demo.niveaux(id)?;
                match p.niveau() {
                    None => Some(e),
                    Some(true) => s,
                    Some(false) => t,
                }
            }
            Prise::BrEntree => b.map(|b| b.entree),
            Prise::BrSl => b.map(|b| b.sl),
            Prise::BrTp => b.map(|b| b.tp),
            Prise::TkEntree => Some(self.watch.panels.trading.prix_ordre()),
            Prise::TkSl | Prise::TkTp => {
                let t = &self.watch.panels.trading;
                let o = t.ordre(t.sens_affiche());
                if p == Prise::TkSl {
                    o.sl
                } else {
                    o.tp
                }
            }
            _ => None,
        }
    }

    /// La ligne attrapée suit la souris ; du mauvais côté, la pastille rouge et le message s'affichent.
    pub(crate) fn ordre_glisse(&mut self, price: f64) {
        let Some((p, _)) = self.pointer.order_drag else { return };
        self.pointer.order_err = None;
        match p {
            // l'entrée d'un ordre en attente : son SL et son TP suivent ; celle d'une position ne bouge pas
            Prise::Entree(id) => self.demo.move_pending(id, price),
            Prise::Sl(id) | Prise::Tp(id) => {
                let stop = p.niveau() == Some(true);
                if let Some((side, e, _, _)) = self.demo.niveaux(id) {
                    self.pointer.order_err = argos_core::portfolio::level_error(side, e, stop, price);
                    let set = |s: &mut Option<f64>, t: &mut Option<f64>| *(if stop { s } else { t }) = Some(price);
                    if let Some(o) = self.demo.positions.iter_mut().find(|o| o.id == id) {
                        set(&mut o.stop, &mut o.target);
                    } else if let Some(o) = self.demo.pending.iter_mut().find(|o| o.id == id) {
                        set(&mut o.order.stop, &mut o.order.target);
                    }
                }
            }
            Prise::BrEntree | Prise::BrSl | Prise::BrTp => {
                if let Some(b) = self.ui.brouillon.as_mut() {
                    match p {
                        Prise::BrEntree => {
                            let d = price - b.entree;
                            (b.entree, b.sl, b.tp) = (price, b.sl + d, b.tp + d);
                        }
                        Prise::BrSl => b.sl = price,
                        _ => b.tp = price,
                    }
                    let side = if b.buy { argos_core::Side::Long } else { argos_core::Side::Short };
                    self.pointer.order_err = argos_core::portfolio::level_error(side, b.entree, p == Prise::BrSl, if p == Prise::BrSl { b.sl } else { b.tp })
                        .filter(|_| p != Prise::BrEntree);
                }
                self.brouillon_maj();
            }
            // 🎫 the ticket's lines: its price (TP and SL follow, they are distances), its SL, its TP
            Prise::TkEntree => {
                let t = &mut self.watch.panels.trading;
                t.px = price;
                t.set_qty(t.qty);
            }
            Prise::TkSl | Prise::TkTp => {
                let t = &mut self.watch.panels.trading;
                let buy = t.sens_affiche();
                let e = t.ordre(buy).fill_price(t.prix);
                let side = if buy { argos_core::Side::Long } else { argos_core::Side::Short };
                self.pointer.order_err = argos_core::portfolio::level_error(side, e, p == Prise::TkSl, price);
                t.poser_niveau(p == Prise::TkSl, price);
            }
            _ => {}
        }
        self.redraw();
    }

    /// Ligne relâchée : du mauvais côté, elle revient à sa place (avec le message) ; sinon le compte est gardé et la
    /// fiche du Journal suit.
    pub(crate) fn ordre_lache(&mut self) {
        let Some((p, depart)) = self.pointer.order_drag.take() else { return };
        if let Some(m) = self.pointer.order_err.take() {
            if p.glisse() && p != Prise::BrEntree {
                self.ordre_glisse_sans_controle(p, depart);
            }
            self.banner = Some(format!("⚠️ {m}"));
            return self.redraw();
        }
        // 🤖 lot 7 : la ligne d'une position lâchée à un autre prix est une retouche du plan à la main (gardée pour la
        // carte Argos AI du trade) : remise à son départ, puis posée par `set_level`, qui note la retouche
        if let Prise::Sl(id) | Prise::Tp(id) = p {
            let stop = p.niveau() == Some(true);
            let fin = self.demo.positions.iter_mut().find(|o| o.id == id).and_then(|o| {
                let niveau = if stop { &mut o.stop } else { &mut o.target };
                niveau.replace(depart)
            });
            if let Some(fin) = fin {
                let _ = self.demo.set_level(id, stop, fin);
            }
        }
        if let Prise::Sl(id) | Prise::Tp(id) | Prise::Entree(id) = p {
            self.journal_niveaux(id);
            Self::write_prefs("demo_account.json", &self.demo);
            self.banner = Some(match p {
                Prise::Entree(_) => "💼 Ordre en attente déplacé (SL et TP suivent)".into(),
                Prise::Sl(_) => "🛡️ Stop Loss déplacé".into(),
                _ => "🎯 Take Profit déplacé".into(),
            });
        }
        self.redraw();
    }

    /// Remet une ligne à `price` sans contrôle (retour à sa place d'avant le glisser).
    fn ordre_glisse_sans_controle(&mut self, p: Prise, price: f64) {
        self.pointer.order_drag = Some((p, price));
        self.ordre_glisse(price);
        (self.pointer.order_drag, self.pointer.order_err) = (None, None);
    }

    /// 📓 La fiche du Journal d'une position démo suit son SL / TP.
    fn journal_niveaux(&mut self, id: u64) {
        let Some(p) = self.demo.positions.iter().find(|p| p.id == id) else { return };
        let key = format!("demo-{}-{}", p.base, p.opened_at);
        if let Some(t) = self.watch.panels.journal.trades.iter_mut().find(|t| t.id == key) {
            (t.sl, t.tp) = (p.stop, p.target);
            self.journal_ecrire();
        }
    }

    /// 💼 Bouton « Buy » / « Sell » du brouillon : l'ordre part au compte démo par le chemin unique (Gardien, SL / TP
    /// obligatoires, journal), en Limite, Stop ou au marché selon son prix.
    fn brouillon_envoyer(&mut self) {
        use argos_core::simu::{Kind, Ticket};
        let Some(b) = self.ui.brouillon.clone() else { return };
        let Some(last) = self.chart.bars.last().map(|b| b.close) else { return };
        let kind = match b.genre {
            "Limite" => Kind::Limit,
            "Stop" => Kind::Stop,
            _ => Kind::Market,
        };
        let avant = self.demo.positions.len() + self.demo.pending.len();
        let t = Ticket {
            buy: b.buy,
            kind,
            price: b.entree,
            sl: b.sl,
            tp: b.tp,
            risk: Money::from_f64(self.watch.panels.trading.risk.max(0.0)),
        };
        self.demo_ticket(t, last, now_s());
        if self.demo.positions.len() + self.demo.pending.len() > avant {
            self.ui.brouillon = None;
        }
        self.redraw();
    }
}
