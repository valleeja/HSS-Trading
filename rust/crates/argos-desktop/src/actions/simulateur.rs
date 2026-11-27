//! 🧪 Simulateur démo : le branchement dans la fenêtre. Un seul moteur démo : le compte du volet Trading
//! (`self.demo`, `argos_core::portfolio` en centimes), ses ordres, son historique et son Gardien. Le Simulateur en est
//! la vue complète (ticket à puces, examen, courbe, statistiques, replay) ; le volet Trading, la vue courte.
//!
//! En direct, les ordres jouent au dernier prix (`demo_exits`). En ⏯ replay du Simulateur, ils jouent sur chaque
//! bougie révélée (`Portfolio::on_bar`, jamais de ticks) et les sorties du direct attendent ; à la fin du replay, ce
//! qui reste ouvert est fermé au dernier prix du replay.
//!
//! 📅 Le replay peut partir d'une date choisie (le passé se charge jusque-là, page par page) ; 💾 une séance se garde
//! pour plus tard (marché, bougie, vitesse, positions et ordres mis de côté hors du compte) et se reprend telle quelle.

use crate::*;
use argos_core::perp::{Bareme, Perp, TAUX_STANDARD_8H};
use argos_core::simu::{Replay as SimReplay, Seance};
use argos_core::Money;
use argos_propfirm::examen::{evaluer, Etat};
use argos_render::simulateur::{Bandeau, PerpVue, SimClick, Simulateur, Vue};

/// 💾 Fichier de la séance de replay gardée (`null` : aucune).
pub(crate) const SEANCE: &str = "seance_replay.json";

/// 🖥 Room kept under the Simulator in the Trading volet: its content stops above the volet's rounded bottom.
pub(crate) const SIM_BAS_VOLET: f64 = 12.0;

/// ⏯ Replay demandé à une date, sur un marché : il part quand le passé chargé atteint la date (ou le début de
/// l'historique). Avec une séance gardée, ses positions et ordres reviennent dans le compte au départ.
pub(crate) struct Visee {
    pub(crate) t: i64,
    pub(crate) ex: Exchange,
    pub(crate) base: String,
    pub(crate) interval: i64,
    pub(crate) seance: Option<Seance>,
}

/// « 12/03/2026 14:00 » : l'heure `t` (secondes UTC) décalée de `tz` secondes.
pub(crate) fn date_courte(t: i64, tz: i64) -> String {
    chrono::DateTime::from_timestamp(t + tz, 0).map_or_else(String::new, |d| d.format("%d/%m/%Y %H:%M").to_string())
}

/// « BTC · 15m · 12/03/2026 14:00 · 2 ouverts » : la séance gardée, telle que le volet la montre.
pub(crate) fn seance_texte(s: &Seance, tz: i64) -> String {
    let n = s.ouverts();
    let ouverts = match n {
        0 => String::new(),
        1 => " · 1 ouvert".into(),
        n => format!(" · {n} ouverts"),
    };
    format!("{} · {} · {}{ouverts}", s.base, Interval(s.interval).short(), date_courte(s.t, tz))
}

/// Jour « AAAA-MM-JJ » de l'heure `ms` dans le fuseau `tz` (la remise à zéro du jour de l'examen).
pub(crate) fn jour_dans(tz: &str, ms: i64) -> String {
    use chrono::Datelike;
    let s = ms.div_euclid(1000);
    chrono::DateTime::from_timestamp(s + argos_render::settings::tz_offset(tz, s).unwrap_or(0), 0)
        .map(|d| format!("{}-{:02}-{:02}", d.year(), d.month(), d.day()))
        .unwrap_or_default()
}

impl App {
    /// 🧪 Ouvre / ferme le volet du Simulateur. 🖥 PC : c'est l'onglet 🧪 du volet Trading (maquette du module 31) ;
    /// l'ouvrir ouvre ce volet, le refermer le referme (comme les autres icônes de la colonne).
    pub(crate) fn toggle_simulateur(&mut self) {
        if self.ui.sim.is_some() {
            if self.sim_en_volet() {
                (self.watch.prefs.open, self.watch.panels.trading.sim_onglet) = (false, false);
            }
            self.ui.sim = None;
        } else {
            self.ui.sim = Some(Simulateur::default());
            self.sim_ouvre_son_volet();
        }
        self.ui.raccourcis = None;
        self.redraw();
    }

    /// 🖥 PC: the Simulator lives in the Trading volet, under its header (the 🧪 tab of the module 31 mock-up; it used
    /// to float at the window's right edge, over the volet and the icon column): opening it opens that volet on its
    /// sheet, the layout's flags at once (its place is known before the next image). 📱 Phone, Zen, dual screen:
    /// nothing (it floats, as before).
    pub(crate) fn sim_ouvre_son_volet(&mut self) {
        if self.top.phone || self.ui.zen || self.watch.fixed_w.is_some() {
            return;
        }
        (self.watch.prefs.open, self.watch.prefs.panel, self.watch.volet_flottant) = (true, wl::Tool::Trading, true);
        let t = &mut self.watch.panels.trading;
        (t.cell, t.options, t.positions_vue, t.menu_validite, t.sim_onglet) = (true, false, false, false, true);
    }

    /// 🖥 The Simulator's place in the Trading volet: under the sheet's header, at the volet's width, above its rounded
    /// bottom; `None` when it floats (phone, Zen, double screen, volet closed or on another panel).
    pub(crate) fn sim_place_volet(&self) -> Option<Rect> {
        let w = &self.watch;
        if w.prefs.panel != wl::Tool::Trading || !w.panels.feuille_trading() {
            return None;
        }
        let v = self.volet_rect()?;
        let haut = w.panels.entete_fixe();
        Some(Rect::new(v.x, v.y + haut, v.w, (v.h - haut - SIM_BAS_VOLET).max(40.0)))
    }

    /// 🖥 The Simulator is open and shows in the Trading volet.
    pub(crate) fn sim_en_volet(&self) -> bool {
        self.ui.sim.is_some() && self.sim_place_volet().is_some()
    }

    /// 🛡 PropFirm (maquette) : le Simulateur ouvert sur ses règles PropFirm (scénarios d'examen, Gardien).
    pub(crate) fn ouvrir_propfirm(&mut self) {
        let mut s = self.ui.sim.take().unwrap_or_default();
        self.sim_ouvre_son_volet();
        let w = self.sim_rect().w;
        let text = argos_render::text::Text::default();
        self.sim_vue(|v| s.defile_vers(v, w, &text, "🎓 Scénario"));
        self.ui.sim = Some(s);
        self.ui.raccourcis = None;
        self.redraw();
    }

    /// 🧪 Le volet dans la fenêtre (🖥 dans le volet Trading, sous son en-tête).
    pub(crate) fn sim_rect(&self) -> Rect {
        if let Some(r) = self.sim_place_volet() {
            return r;
        }
        let r = self.raccourcis_rect();
        Simulateur::rect(r.x + r.w, r.y + r.h, r.y)
    }

    /// 🟠 Le bandeau DÉMO (en haut au centre).
    pub(crate) fn bandeau_rect(&self) -> Rect {
        self.bandeau_place().0
    }

    /// 🟠 Place du bandeau et son niveau de texte. 🖥 PC (demande de Jee) : dans la 1re rangée de la barre du haut, centré
    /// dans l'espace libre entre la mise en page et la recherche (texte raccourci si la place manque) ; ailleurs comme avant.
    pub(crate) fn bandeau_place(&self) -> (Rect, usize) {
        let text = argos_render::text::Text::default();
        let (solde, jour) = (self.demo.balance, self.sim_jour());
        // 🟠 filet n° 43: just under the top bar, over the chart right of the price legend (the longest text that
        // fits), never gone
        // (never over the legend's own lines: right of the title when it fits, else under the legend, at its left)
        let sous_la_barre = || {
            let (rw, th) = (self.rw(), self.top_h());
            let prix = self.legendes().first().map(|(_, r)| *r);
            // the chart's title as drawn (bold, 16) and its live dot: longer than the legend's box
            // (not `to_chart`: the chart keeps this band clear, `pilules_garder`)
            let nom = if matches!(self.ui.plein_ecran, Some(crate::actions::plein_ecran::PleinEcran::Panneau(_))) {
                self.title.clone()
            } else {
                self.titre_chart()
            };
            let titre = argos_render::text::Text::graisse(argos_render::text::Graisse::Bold).width(&nom, 16.0) as f64;
            let apres_titre = (rw + 12.0 + titre + 40.0).max(prix.map_or(rw, |r| r.x + r.w + 8.0));
            // (nor over an indicator's table, « Argos TVD Pro »…: the row stops before it)
            let tables: Vec<Rect> = argos_render::indicators::tableaux()
                .into_iter()
                .map(|t| Rect::new(rw + t.x, th + t.y, t.w, t.h))
                .collect();
            let fin_de = |y: f64, h: f64| {
                tables
                    .iter()
                    .filter(|t| t.y < y + h && y < t.y + t.h)
                    .map(|t| t.x - 8.0)
                    .fold(rw + self.chart.viewport.plot_w() - 8.0, f64::min)
            };
            // (📟 the legend's unfolded list: never over its rows — it went over the first one on the tablet
            // standing — beside them, else under them)
            let vp = self.chart.viewport;
            let liste = (self.ind.leg.0 && !self.top.phone)
                .then(|| argos_render::indleg::liste_rect(&text, &self.ind.on, &self.ind.out, vp.plot_h()))
                .flatten()
                .map(|r| Rect::new(rw + r.x, th + r.y, r.w, r.h));
            Bandeau::rect_dans(&text, apres_titre, fin_de(th, 36.0), th, 36.0, solde, jour)
                .or_else(|| {
                    // under the price legend, right of its « ⌄ N » (only candles under it)
                    let bas = prix.map_or(th + 100.0, |r| r.y + r.h) + 4.0;
                    let x0 = liste
                        .filter(|l| l.y < bas + 32.0 && bas < l.y + l.h)
                        .map_or(rw + 80.0, |l| (rw + 80.0).max(l.x + l.w + 8.0));
                    Bandeau::rect_dans(&text, x0, fin_de(bas, 32.0).min(x0 + 320.0), bas, 32.0, solde, jour)
                })
                .or_else(|| {
                    let bas = liste?.y + liste?.h + 4.0;
                    let x0 = rw + 80.0;
                    Bandeau::rect_dans(&text, x0, fin_de(bas, 32.0).min(x0 + 320.0), bas, 32.0, solde, jour)
                })
                .or_else(|| {
                    // under the price legend, right of its « ⌄ N » (only candles under it)
                    let bas = prix.map_or(th + 100.0, |r| r.y + r.h) + 4.0;
                    let x0 = rw + 80.0;
                    Bandeau::rect_dans(&text, x0, fin_de(bas, 32.0).min(x0 + 320.0), bas, 32.0, solde, jour)
                        // (a wide panel leaves ~150 points of chart, tablet standing: the row under the « ⌄ N »,
                        // the whole width of the chart, the short form if need be)
                        .or_else(|| {
                            // (nor over the folded drawing bar's ⋮⋮ grip, 4 panes on the tablet: the row goes under it)
                            let rang = |y: f64| Bandeau::rect_dans(&text, rw + 4.0, fin_de(y, 32.0), y, 32.0, solde, jour);
                            let (outils, _) = argos_render::toolbar::layout(&self.tools, &text);
                            let outils = Rect::new(rw + outils.x, th + outils.y, outils.w, outils.h);
                            let croise = |r: &Rect| {
                                outils.w > 0.0 && r.x < outils.x + outils.w && outils.x < r.x + r.w && r.y < outils.y + outils.h && outils.y < r.y + r.h
                            };
                            match rang(bas + 32.0) {
                                Some(p) if croise(&p.0) => rang(outils.y + outils.h + 4.0),
                                p => p,
                            }
                        })
                })
                .unwrap_or((Rect::new(0.0, 0.0, 0.0, 0.0), 2))
        };
        if let Some((x0, x1, y, h)) = self.top.espace_libre(self.top_w(), &text) {
            if let Some(p) = Bandeau::rect_dans(&text, x0, x1, y, h, solde, jour) {
                return p;
            }
            // (tablet lying, 1092 points by the finger: not even « DÉMO » fits in the first row)
            return sous_la_barre();
        }
        // (no room counted in the first row — tablet standing 819 points, tablet lying by the finger in the browser:
        // centred under the top bar, it went over the chart's title)
        sous_la_barre()
    }

    /// Heure du Simulateur (secondes) : celle de la bougie du replay, sinon maintenant.
    fn sim_now(&self) -> i64 {
        self.replay.as_ref().filter(|_| self.ui.sim_replay).map_or_else(now_s, |r| r.t)
    }

    /// Dernier prix du symbole de la chart (en replay : la bougie du replay, la chart étant coupée là).
    pub(crate) fn sim_last(&self) -> Option<f64> {
        self.chart.bars.last().map(|b| b.close)
    }

    /// Fuseau de la remise du jour (réglage du scénario, Europe centrale par défaut).
    fn sim_tz(&self) -> String {
        self.ui
            .sim
            .as_ref()
            .map_or_else(|| argos_propfirm::examen::FUSEAUX[0].to_string(), |s| s.regles.reset_tz.clone())
    }

    /// P&L du jour (depuis minuit dans le fuseau de la remise), net comme le solde : trades fermés aujourd'hui (coûts
    /// et funding déduits) et positions ouvertes (latent moins les coûts d'entrée et le funding déjà payés).
    pub(crate) fn sim_jour(&self) -> Money {
        let tz = self.sim_tz();
        let today = jour_dans(&tz, self.sim_now() * 1000);
        let closed: Money = self
            .demo
            .history
            .iter()
            .filter(|c| jour_dans(&tz, c.closed_at * 1000) == today)
            .map(|c| c.net())
            .sum();
        let open: Money = self
            .demo
            .positions
            .iter()
            .map(|p| self.demo_price(&p.base).map_or(Money::ZERO, |px| Money::from_f64(p.pnl(px))) - self.demo.deja_paye(p))
            .sum();
        closed + open
    }

    /// 🎓 Où en est l'examen (aucun trade : rien).
    pub(crate) fn sim_examen(&self) -> Option<Etat> {
        let s = self.ui.sim.as_ref()?;
        let a = &self.demo;
        if a.history.is_empty() && a.positions.is_empty() {
            return None;
        }
        // 💸 l'examen compte les frais, comme la courbe (nette)
        let closed: Vec<(i64, Money)> = a.history.iter().map(|c| (c.closed_at * 1000, c.net())).collect();
        let curve: Vec<(i64, Money)> = a.curve.iter().map(|&(t, e)| (t * 1000, e)).collect();
        let tz = s.regles.reset_tz.clone();
        Some(evaluer(&s.regles, a.start, &curve, &closed, &|ms| jour_dans(&tz, ms)))
    }

    /// Le replay du Simulateur vu par le volet (bougies montrées / toutes).
    fn sim_replay_vue(&self) -> (Option<SimReplay>, usize) {
        let r = self.replay.as_ref().filter(|_| self.ui.sim_replay).map(|r| SimReplay {
            shown: self.replay_times.iter().filter(|&&t| t <= r.t).count(),
            play: r.play,
            speed: r.speed,
        });
        (r, self.replay_times.len())
    }

    /// 📱 Le rond Argos AI posé sur le bas du volet : la hauteur qu'il couvre (plus un peu d'air), laissée sous le
    /// contenu pour que les dernières lignes (Drawdown max, Résultat) défilent au-dessus de lui ; 0 s'il est ailleurs.
    pub(crate) fn sim_marge_bas(&self) -> f64 {
        let (r, ai) = (self.sim_rect(), self.ai_bouton_rect());
        let dessus = ai.w > 0.0 && ai.x < r.x + r.w && ai.x + ai.w > r.x && ai.y < r.y + r.h && ai.y + ai.h > r.y;
        if dessus {
            (r.y + r.h - ai.y + 8.0).max(0.0)
        } else {
            0.0
        }
    }

    /// 💸 La bourse du marché ouvert et son barème (frais, écart, glissement) : les coûts d'un ordre démo placé ici.
    /// `None` : pas de vrai marché derrière (symbole calculé, série CoinMarketCap, fichier) : sans frais.
    pub(crate) fn couts_marche(&self) -> Option<(&'static str, argos_core::couts::Couts)> {
        use argos_data::couts::{bareme, DEX, OANDA};
        match &self.chart.source {
            Source::Live { exchange, base } | Source::Ratio { exchange, base, .. } => Some((exchange.name(), bareme(*exchange, base))),
            Source::Oanda { .. } => Some(("OANDA", OANDA)),
            Source::Dex { .. } => Some(("DEX", DEX)),
            Source::Synth { .. } | Source::Cap { .. } | Source::File(_) => None,
        }
    }

    /// ⚡ Le barème perpétuel du marché ouvert (levier maximum, marge de maintien, funding) ; `None` : au comptant, ou
    /// pas de vrai marché de bourse derrière (OANDA, DEX, symbole calculé, fichier).
    pub(crate) fn perp_marche(&self) -> Option<Bareme> {
        match &self.chart.source {
            Source::Live { exchange, base } => argos_data::perp::bareme(*exchange, base),
            _ => None,
        }
    }

    /// ⚡ Les conditions perpétuelles d'un ordre démo placé sur ce marché (Simulateur, volet Trading, chart) : le levier
    /// choisi (au Simulateur ou sous ⚙ du ticket : c'est le même ; ×10 tant qu'il n'a pas été changé), plafonné au
    /// maximum du marché.
    pub(crate) fn perp_ticket(&self) -> Option<Perp> {
        let levier = self.ui.sim.as_ref().map_or(self.watch.panels.trading.levier, |s| s.levier);
        self.perp_marche().map(|b| b.perp(levier))
    }

    /// ⚡ Le taux de funding (pour 8 h) d'une position démo sur `base` et s'il est réel : celui lu à la bourse de la
    /// chart, sinon celui de Hyperliquid (venu avec ses prix), sinon le taux de base (0,01 % / 8 h).
    pub(crate) fn funding_demo(&self, base: &str) -> (f64, bool) {
        let chart_hl = matches!(&self.chart.source, Source::Live { exchange: Exchange::Hyperliquid, base: b } if b == base);
        if let Some((_, t)) = self.feeds.funding_demo.get(base).filter(|_| !chart_hl) {
            return (*t, true);
        }
        match self.feeds.quotes.get(base).map(|q| q.funding * 8.0).filter(|t| t.is_finite()) {
            Some(t) => (t, true),
            None => (TAUX_STANDARD_8H, false),
        }
    }

    /// Ce que le volet montre, passé à `f`.
    pub(crate) fn sim_vue<R>(&self, f: impl FnOnce(&Vue) -> R) -> R {
        let ex = self.sim_examen();
        let (replay, total) = self.sim_replay_vue();
        let sym = self.symbol_label();
        let prix_de = |b: &str| self.demo_price(b);
        let v = Vue {
            acc: &self.demo,
            prix_de: &prix_de,
            gardien: self.watch.panels.trading.guardian,
            last: self.sim_last().unwrap_or(0.0),
            symbole: &sym,
            dec: argos_render::viewport::data_decimals(&self.chart.bars),
            jour: self.sim_jour(),
            examen: ex.as_ref(),
            replay,
            total,
            seance: self.ui.seance.as_ref().map(|s| seance_texte(s, self.tz_offset_s())),
            marge_bas: self.sim_marge_bas(),
            couts: self.couts_marche(),
            // ⚡ en replay : le taux fixe choisi au volet ; en direct : le taux de la bourse (ou celui de base)
            perp: self.perp_marche().map(|bareme| {
                let (taux_8h, reel) = match (replay.is_some(), self.ui.sim.as_ref()) {
                    (true, Some(s)) => (s.taux_replay(), false),
                    (true, None) => (TAUX_STANDARD_8H, false),
                    (false, _) => self.funding_demo(&sym),
                };
                PerpVue { bareme, taux_8h, reel }
            }),
            avis: self.avis_index(),
            en_volet: self.sim_place_volet().is_some(),
        };
        f(&v)
    }

    /// 🧪 Dessine le volet (et rend aussi sa place).
    pub(crate) fn sim_draw(&self, text: &argos_render::text::Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let s = self.ui.sim.as_ref()?;
        let r = self.sim_rect();
        let pm = self.sim_vue(|v| s.draw(text, r, v, self.pointer.win_mouse))?;
        Some((pm, r))
    }

    /// 📱 Téléphone et tablette : la pastille DÉMO vit dans la barre du bas (elle ne flotte plus sur la chart).
    pub(crate) fn demo_en_bas(&self) -> bool {
        // 🟠 filet n° 43: a tablet without the fixed bar under the chart (the PC frame: footer 0) had neither the
        // band nor the pill: the band stays at the top there
        self.top.phone || (self.ui.form.tablet() && self.chart.viewport.footer > 0)
    }

    /// 🟠 The DÉMO band where it lies over the chart (window place): 📟 the upright tablet's, under the top bar. 🖥 On
    /// the PC it is in the top bar, 📱 on the phone at the bottom: `None`. (B-018: the chart keeps it clear.)
    pub(crate) fn demo_sur_la_chart(&self) -> Option<Rect> {
        (!self.demo_en_bas()).then(|| self.bandeau_place().0)
    }

    /// 🟠 La pastille compacte : « DÉMO −580,22 $ » et la couleur du montant (vert gain, rouge perte).
    pub(crate) fn demo_pastille(&self) -> (String, argos_render::text::Rgb) {
        let j = self.sim_jour();
        (
            format!("DÉMO {}", argos_render::simulateur::argent(j, true)),
            argos_render::palette::pnl(j.to_f64()),
        )
    }

    /// 🟠 Le texte de la pastille de la barre du bas de la chart (tablette seulement).
    pub(crate) fn chart_demo(&self) -> Option<String> {
        (self.demo_en_bas() && !self.top.phone).then(|| self.demo_pastille().0)
    }

    /// 🟠 Toucher la pastille du bas : le Simulateur s'ouvre.
    pub(crate) fn demo_pastille_toucher(&mut self) {
        self.ui.sim.get_or_insert_with(Simulateur::default);
        self.ui.raccourcis = None;
        self.redraw();
    }

    /// 🟠 Dessine le bandeau DÉMO.
    pub(crate) fn bandeau_draw(&self, text: &argos_render::text::Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        if self.ui.accueil.is_some() || self.ui.zen || self.demo_en_bas() {
            return None;
        }
        let (r, niveau) = self.bandeau_place();
        if r.w <= 0.0 {
            return None;
        }
        Some((Bandeau::draw_n(text, r, self.demo.balance, self.sim_jour(), self.pointer.win_mouse, niveau)?, r))
    }

    /// 🟠 Clic sur le bandeau : ouvre le Simulateur ; ↺ l'ouvre sur la confirmation de remise à zéro.
    pub(crate) fn bandeau_click(&mut self, x: f64, y: f64) -> bool {
        let r = self.bandeau_rect();
        if self.ui.accueil.is_some() || self.ui.zen || self.demo_en_bas() || !r.contains(x, y) {
            return false;
        }
        let reset = Bandeau::reset(r).contains(x - r.x, y - r.y);
        let s = self.ui.sim.get_or_insert_with(Simulateur::default);
        s.confirme = reset;
        self.ui.raccourcis = None;
        self.redraw();
        true
    }

    /// ⌨️ Escape with the Simulator open: first the month report's page, then the reset's confirmation, then the panel.
    pub(crate) fn sim_echap(&mut self) {
        match self.ui.sim.as_mut() {
            Some(s) if s.month.is_some() => s.month = None,
            Some(s) if s.confirme => s.confirme = false,
            Some(_) => (self.ui.sim, self.watch.panels.trading.sim_onglet) = (None, false),
            None => {}
        }
    }

    /// 🧪 Clic dans le volet.
    pub(crate) fn sim_click(&mut self, x: f64, y: f64) {
        let r = self.sim_rect();
        let text = argos_render::text::Text::default();
        // une copie, pas take() : la vue doit encore voir le volet (l'examen se lit dans ses règles), sinon le plan du
        // clic n'a pas les lignes de l'examen et les zones dessous sont décalées
        let Some(mut s) = self.ui.sim.clone() else { return };
        let c = self.sim_vue(|v| s.click(&text, r, v, x, y));
        // ⚡ one leverage for the demo orders: the one chosen here is the ticket's too
        self.watch.panels.trading.levier = s.levier;
        self.ui.sim = Some(s);
        self.sim_faire(c);
    }

    /// 🧪 Ce que demande un clic dans le volet.
    pub(crate) fn sim_faire(&mut self, c: SimClick) {
        match c {
            SimClick::None | SimClick::Changed => {}
            SimClick::Close => self.ui.sim = None,
            SimClick::Place => return self.sim_place(),
            SimClick::Gardien => {
                let t = &mut self.watch.panels.trading;
                t.guardian = !t.guardian;
                self.banner = Some(format!("🛡️ Gardien PropFirm {}", if t.guardian { "allumé" } else { "éteint" }));
                self.save_watch();
            }
            SimClick::Cancel(id) => return self.cancel_demo_order(id),
            SimClick::SansFrais(sans) => {
                self.demo.sans_frais = sans;
                Self::write_prefs("demo_account.json", &self.demo);
                self.banner = Some(match (sans, self.couts_marche()) {
                    (true, _) => "💸 Sans frais : les nouveaux ordres démo ne paient ni frais, ni écart, ni glissement".into(),
                    (false, Some((nom, _))) => format!("💸 Coûts réels : les nouveaux ordres démo paient les frais de {nom}, l'écart et le glissement"),
                    (false, None) => "💸 Coûts réels : ce marché n'a pas de barème, ses ordres restent sans frais".into(),
                });
            }
            SimClick::Moitie(id) => {
                let now = self.sim_now();
                let p = self.demo.positions.iter().find(|p| p.id == id).cloned();
                let px = p.as_ref().and_then(|p| self.demo_price(&p.base));
                if let Some((p, px)) = p.zip(px) {
                    if let Ok(c) = self.demo.close_part(id, p.qty / 2.0, px, now, "sortie de la moitié") {
                        self.demo_closed(&[c]);
                    }
                }
            }
            SimClick::StopEntree(id) => {
                let entry = self.demo.positions.iter().find(|p| p.id == id).map(|p| (p.entry, p.side, p.base.clone()));
                if let Some((entry, side, base)) = entry {
                    // seulement si le prix est déjà du bon côté : sinon le stop serait touché tout de suite
                    let gagne = self.demo_price(&base).is_some_and(|px| (px - entry) * side.sign() > 0.0);
                    self.banner = Some(if !gagne {
                        "⚖ Pas encore en gain : le stop à l'entrée sortirait tout de suite".into()
                    } else {
                        match self.demo.set_stop_entree(id) {
                            Ok(()) => {
                                Self::write_prefs("demo_account.json", &self.demo);
                                "⚖ Stop monté au prix d'entrée : ce trade ne peut plus perdre".into()
                            }
                            Err(e) => format!("⚖ {e}"),
                        }
                    });
                }
            }
            SimClick::CloseTrade(id) => {
                let now = self.sim_now();
                let px = self.demo.positions.iter().find(|p| p.id == id).and_then(|p| self.demo_price(&p.base));
                if let Some(c) = px.and_then(|px| self.demo.close(id, px, now, "fermé à la main").ok()) {
                    self.banner = Some(format!("🧪 Position fermée à la main : {:+.2} R", c.r()).replace('.', ","));
                    self.demo_closed(&[c]);
                }
            }
            SimClick::Reset => {
                self.demo.reset();
                self.banner = Some("↺ Compte démo remis à zéro".into());
                self.save_accounts();
            }
            SimClick::ReplayStart => return self.sim_replay_start(),
            SimClick::ReplayPlay => {
                if let Some(rp) = self.replay.as_mut() {
                    (rp.play, rp.pick) = (!rp.play, false);
                    self.replay_at = Instant::now();
                }
            }
            SimClick::ReplayStep => {
                if let Some(rp) = self.replay.as_mut() {
                    rp.play = false;
                }
                return self.replay_step(1);
            }
            SimClick::ReplaySpeed => {
                if let Some(rp) = self.replay.as_mut() {
                    rp.speed = SimReplay::next_speed(rp.speed);
                }
            }
            SimClick::ReplayStop => {
                self.sim_replay_end();
                self.replay = None;
                self.banner = Some("📡 Retour au direct".into());
                self.rebuild();
            }
            SimClick::ReplayDate => return self.sim_replay_date(),
            SimClick::SeanceSauver => return self.sim_seance_sauver(),
            SimClick::SeanceReprendre => return self.sim_seance_reprendre(),
            SimClick::SeanceOublier => return self.sim_seance_oublier(),
            SimClick::Lecon(id) => {
                self.month_close();
                return self.palette_cible(argos_render::recherche::Cible::Lecon(id.to_string()));
            }
            // (📅 the month page closes: the card shows where the page was)
            SimClick::Avis(k) => {
                self.month_close();
                self.avis_historique(k);
            }
            SimClick::MonthReport => self.month_toggle(),
        }
        self.redraw();
    }

    /// 🧪 Place l'ordre du ticket du Simulateur (même chemin que le ticket du volet Trading).
    pub(crate) fn sim_place(&mut self) {
        let Some(last) = self.sim_last() else {
            self.banner = Some("🧪 Pas encore de prix : attends les bougies".into());
            return self.redraw();
        };
        let Some(s) = self.ui.sim.clone() else { return };
        let t = s.ticket(last, self.demo.balance);
        let now = self.sim_now();
        let avant: Vec<u64> = self.demo_ids();
        if let Some((a, b)) = s.tickets_oco(last, self.demo.balance) {
            // 🔀 OCO : les deux côtés de la cassure, liés
            if !self.jsync.pret {
                self.banner = Some("🔐 Ordre démo refusé : journal indisponible (rien ne serait noté)".into());
                return self.redraw();
            }
            if let Some(why) = self.demo_garde(&a, last, now) {
                self.banner = Some(format!("🛡️ Gardien PropFirm : ordre bloqué ({why})"));
                return self.redraw();
            }
            let base = self.symbol_label();
            let couts = self.couts_marche().map(|c| c.1);
            let perp = self.perp_ticket();
            self.banner = Some(match argos_core::simu::place_oco(&a, &b, &mut self.demo, &base, last, now, couts, perp) {
                Ok(_) => {
                    let fmt = |p: f64| argos_render::viewport::format_price(p, 2);
                    format!(
                        "🔀 OCO placé : Buy Stop {} et Sell Stop {} (quand l'un part, l'autre est annulé)",
                        fmt(a.price),
                        fmt(b.price)
                    )
                }
                Err(e) => format!("💼 Ordre refusé : {e}"),
            });
        } else {
            self.demo_ticket(t, last, now);
        }
        // 🎛 la gestion choisie au ticket va aux nouveaux ordres et positions (un OCO : chacun celle de son côté)
        if !s.gestion(&t).vide() {
            for id in self.demo_ids().into_iter().filter(|id| !avant.contains(id)) {
                let Some((side, ..)) = self.demo.niveaux(id) else { continue };
                let mut cote = s.clone();
                cote.buy = side == argos_core::Side::Long;
                let g = cote.gestion(&cote.ticket(last, self.demo.balance));
                if let Err(e) = self.demo.gerer(id, g) {
                    self.banner = Some(format!("🎛 Gestion refusée : {e}"));
                }
            }
        }
        Self::write_prefs("demo_account.json", &self.demo);
        self.redraw();
    }

    /// ⏯ Pourquoi un nouveau replay ne peut pas partir : des positions ou ordres démo du direct (leurs prix ne doivent
    /// pas jouer sur le passé), ou une séance gardée qui a encore des positions (elle serait perdue en sauvant).
    fn sim_replay_refus(&self) -> Option<&'static str> {
        if !self.demo.positions.is_empty() || !self.demo.pending.is_empty() {
            return Some("🧪 Ferme d'abord les positions et ordres démo du direct");
        }
        if self.ui.seance.as_ref().is_some_and(|s| s.ouverts() > 0) {
            return Some("💾 Une séance gardée a des positions : reprends-la ou oublie-la d'abord");
        }
        None
    }

    /// Le marché crypto ouvert (exchange, symbole, unité en secondes) ; `None` pour les autres sources.
    fn sim_marche(&self) -> Option<(Exchange, String, i64)> {
        match &self.chart.source {
            Source::Live { exchange, base } => Some((*exchange, base.clone(), self.chart.interval.secs())),
            _ => None,
        }
    }

    /// 📅 « Choisir la date » : la fenêtre « Aller à » s'ouvre ; la date choisie est le départ du replay.
    pub(crate) fn sim_replay_date(&mut self) {
        if let Some(m) = self.sim_replay_refus() {
            self.banner = Some(m.into());
            return self.redraw();
        }
        let Some(last) = self.chart.bars.last().map(|b| b.time) else {
            self.banner = Some("⏳ Attends que les bougies soient chargées".into());
            return self.redraw();
        };
        if self.sim_marche().is_none() {
            self.banner = Some("📅 Replay à une date : seulement sur un marché crypto en direct".into());
            return self.redraw();
        }
        self.ui.goto = Some(argos_render::goto::GoTo::open(last, now_s(), self.tz_offset_s()));
        self.ui.goto_replay = true;
        self.banner = Some("📅 Choisis la date de départ du replay".into());
        self.redraw();
    }

    /// 📅 Replay à partir de `t` (secondes UTC) : sur le marché de la séance s'il y en a une (la chart y va), sinon
    /// sur celui de la chart. Le passé se charge jusqu'à `t`, puis le replay part (`sim_replay_poursuivre`).
    pub(crate) fn sim_replay_depuis(&mut self, t: i64, seance: Option<Seance>) {
        let cible = match &seance {
            Some(s) => Exchange::parse(&s.exchange).map(|e| (e, s.base.clone(), s.interval)),
            None => self.sim_marche(),
        };
        let Some((ex, base, iv)) = cible else {
            self.banner = Some("📅 Replay à une date : seulement sur un marché crypto en direct".into());
            return self.redraw();
        };
        // un replay de la chart (pas celui du Simulateur) en cours : on le quitte
        if self.replay.is_some() && !self.ui.sim_replay {
            self.replay = None;
            self.rebuild();
        }
        if self.sim_marche().as_ref() != Some(&(ex, base.clone(), iv)) {
            // la chart va sur le marché de la séance (l'ancien garde sa session et son cache)
            self.save();
            self.write_cache();
            if self.chart.interval.secs() != iv {
                self.chart.interval = Interval(iv);
                self.tools.interval = iv;
                self.save_tools();
            }
            self.open_live(ex, base.clone());
        }
        self.banner = Some(format!("⏳ Chargement des bougies jusqu'au {}", date_courte(t, self.tz_offset_s())));
        self.ui.replay_vise = Some(Visee {
            t,
            ex,
            base,
            interval: iv,
            seance,
        });
        self.sim_replay_poursuivre();
        self.redraw();
    }

    /// ⏯ Un replay attend sa date : encore une page du passé, ou le départ (date atteinte, début de l'historique,
    /// mémoire pleine). Appelé à l'arrivée des bougies et de chaque page du passé.
    pub(crate) fn sim_replay_poursuivre(&mut self) {
        // ⏪ a change of unit during a replay waits for its past too
        self.replay_ut_poursuivre();
        let Some(v) = self.ui.replay_vise.as_ref() else { return };
        let (t, marche) = (v.t, (v.ex, v.base.clone(), v.interval));
        if self.sim_marche() != Some(marche) {
            // un autre marché ouvert entre-temps : le replay ne part pas (la séance gardée reste gardée)
            self.ui.replay_vise = None;
            self.banner = Some("📅 Replay annulé : le marché a changé".into());
            return;
        }
        if self.chart.loading.is_some() || self.chart.raw.is_empty() {
            return; // les bougies arrivent
        }
        self.older_key_sync();
        if self.chart.older_busy {
            return; // une page du passé arrive
        }
        let atteint = self.chart.raw.first().is_some_and(|b| b.time <= t) || self.chart.history_start || self.chart.raw.len() >= crate::actions::load::MAX_RAW;
        if !atteint && self.request_older(crate::actions::load::OLDER_PAGE) {
            return;
        }
        if let Some(v) = self.ui.replay_vise.take() {
            self.sim_replay_lancer(v);
        }
    }

    /// ⏯ Départ du replay sur la bougie de `v.t` (la première si l'historique commence après) ; une séance reprise
    /// remet ses positions et ordres et sa vitesse.
    fn sim_replay_lancer(&mut self, v: Visee) {
        let n = self.chart.bars.len();
        if n < 2 {
            self.banner = Some("⏳ Pas assez de bougies pour un replay".into());
            return self.redraw();
        }
        if !self.demo.positions.is_empty() || !self.demo.pending.is_empty() {
            self.banner = Some("🧪 Ferme d'abord les positions et ordres démo du direct".into());
            return self.redraw();
        }
        let i = argos_render::goto::index_at_time(&self.chart.bars, v.t).unwrap_or(0).min(n - 2);
        let t0 = self.chart.bars[i].time;
        let tz = self.tz_offset_s();
        let mut rp = argos_render::replay::Replay::new(t0);
        rp.pick = false;
        rp.speed = v.seance.as_ref().map_or(1, |s| s.speed);
        if !SimReplay::SPEEDS.contains(&rp.speed) {
            rp.speed = 1;
        }
        self.ui.sim_last_t = t0;
        self.replay = Some(rp);
        self.ui.sim_replay = true;
        self.banner = Some(if let Some(s) = v.seance {
            // le replay est déjà là : les prix du direct ne touchent pas ce qui revient
            let n = s.ouverts();
            self.demo.remettre(s.positions, s.pending);
            Self::write_prefs("demo_account.json", &self.demo);
            self.ui.seance = None;
            Self::write_prefs(SEANCE, &None::<Seance>);
            format!("▶ Séance reprise au {} ({n} ouvert(s)) : ▶ pour jouer", date_courte(t0, tz))
        } else if t0 > v.t + self.chart.interval.secs() {
            format!("📜 L'historique chargé commence le {} : le replay part de là", date_courte(t0, tz))
        } else {
            format!("⏯️ Replay depuis le {} : ▶ pour jouer, +1 bougie pour avancer", date_courte(t0, tz))
        });
        self.ui.sim.get_or_insert_with(Simulateur::default);
        self.sim_ouvre_son_volet();
        self.rebuild();
        self.redraw();
    }

    /// 💾 « Sauver et quitter » : la séance est gardée (marché, bougie, vitesse) ; ses positions et ordres sortent du
    /// compte sans être fermés, et la chart revient au direct.
    pub(crate) fn sim_seance_sauver(&mut self) {
        let Some(rp) = self.replay.as_ref().filter(|_| self.ui.sim_replay) else {
            return;
        };
        let (t, speed) = (rp.t, rp.speed);
        let Some((ex, base, interval)) = self.sim_marche() else {
            self.banner = Some("💾 Séance gardée seulement sur un marché crypto en direct".into());
            return self.redraw();
        };
        let prix = self.sim_last().unwrap_or(0.0);
        let (positions, pending) = self.demo.mettre_de_cote();
        let s = Seance {
            exchange: ex.id().into(),
            base,
            interval,
            t,
            speed,
            prix,
            positions,
            pending,
            saved_at: now_s(),
        };
        Self::write_prefs(SEANCE, &s);
        Self::write_prefs("demo_account.json", &self.demo);
        self.banner = Some(format!(
            "💾 Séance gardée ({}) : reprends-la dans le Simulateur",
            seance_texte(&s, self.tz_offset_s())
        ));
        self.ui.seance = Some(s);
        self.ui.sim_replay = false;
        self.replay = None;
        self.rebuild();
        self.redraw();
    }

    /// ▶ « Reprendre la séance » : la chart va sur son marché, charge jusqu'à sa bougie, et le replay repart là.
    pub(crate) fn sim_seance_reprendre(&mut self) {
        let Some(s) = self.ui.seance.clone() else { return };
        if !self.demo.positions.is_empty() || !self.demo.pending.is_empty() {
            self.banner = Some("🧪 Ferme d'abord les positions et ordres démo du direct".into());
            return self.redraw();
        }
        if self.replay.is_some() {
            return;
        }
        self.sim_replay_depuis(s.t, Some(s));
    }

    /// 🗑 « Oublier » : ses positions ferment au dernier prix de la séance (inscrit au journal), ses ordres sont
    /// annulés, et la séance n'est plus gardée.
    pub(crate) fn sim_seance_oublier(&mut self) {
        let Some(s) = self.ui.seance.take() else { return };
        Self::write_prefs(SEANCE, &None::<Seance>);
        if !s.positions.is_empty() && s.prix > 0.0 {
            // seules les positions de la séance passent par le compte, le temps de les fermer
            let (vivantes, attente) = self.demo.mettre_de_cote();
            self.demo.remettre(s.positions, vec![]);
            let (closed, _) = self.demo.close_all(|_| Some(s.prix), s.t, "séance oubliée");
            self.demo.remettre(vivantes, attente);
            self.demo_closed(&closed);
        }
        self.banner = Some("🗑 Séance oubliée".into());
        Self::write_prefs("demo_account.json", &self.demo);
        self.redraw();
    }

    /// ⏯ Replay du Simulateur : le replay de la chart, à ×1 (puis ×2, ×5, ×10), départ 100 bougies avant la fin.
    pub(crate) fn sim_replay_start(&mut self) {
        let n = self.chart.bars.len();
        if n < 20 {
            self.banner = Some("⏳ Attends que les bougies soient chargées".into());
            return self.redraw();
        }
        if let Some(m) = self.sim_replay_refus() {
            self.banner = Some(m.into());
            return self.redraw();
        }
        let start = n.saturating_sub(100).max(n / 2).min(n - 1);
        let mut rp = argos_render::replay::Replay::new(self.chart.bars[start].time);
        (rp.pick, rp.speed) = (false, 1);
        self.ui.sim_last_t = rp.t;
        self.replay = Some(rp);
        self.ui.sim_replay = true;
        self.banner = Some("⏯️ Replay du Simulateur : ▶ pour jouer, +1 bougie pour avancer".into());
        self.rebuild();
        self.redraw();
    }

    /// ⏯ Fin du replay du Simulateur : ce qui reste ouvert ferme au dernier prix du replay, les ordres en attente
    /// sont annulés (leurs prix sont du passé : ils ne doivent pas toucher le direct).
    pub(crate) fn sim_replay_end(&mut self) {
        if !self.ui.sim_replay {
            return;
        }
        let now = self.sim_now();
        let prices = self.demo_prices();
        let (closed, _) = self.demo.close_all(|b| prices.get(b).copied(), now, "fin du replay");
        self.ui.sim_replay = false;
        self.demo_closed(&closed);
    }

    /// ⏯ Bougies révélées par le replay du Simulateur et pas encore jouées : chacune joue les ordres démo du symbole
    /// de la chart (le Gardien revérifie à l'exécution). Appelé à chaque `rebuild` (chaque pas du replay).
    pub(crate) fn sim_tick(&mut self) {
        let Some(fin) = self.replay.as_ref().filter(|_| self.ui.sim_replay).map(|r| r.t) else {
            return;
        };
        let last_t = self.ui.sim_last_t;
        let bars: Vec<Bar> = self.chart.bars.iter().filter(|b| b.time > last_t && b.time <= fin).cloned().collect();
        let Some(dernier) = bars.last().map(|b| b.time) else { return };
        self.ui.sim_last_t = dernier;
        if self.demo.positions.is_empty() && self.demo.pending.is_empty() {
            return;
        }
        let base = self.symbol_label();
        let guardian = self.watch.panels.trading.guardian;
        let journal = self.watch.panels.journal.clone();
        // ⚡ le funding du replay : le taux fixe choisi au volet, payé à chaque échéance au prix de clôture de la bougie
        let taux = self.ui.sim.as_ref().map_or(TAUX_STANDARD_8H, |s| s.taux_replay());
        let mut fermes = vec![];
        for b in &bars {
            let open = self.demo.positions.len();
            let (out, closed) = self.demo.on_bar(&base, b, |o| {
                let risk = o.stop.map_or(0.0, |s| (o.price - s).abs() * o.qty);
                guardian.then(|| journal.guard(risk, b.time * 1000, open, o.stop.is_some(), None)).flatten()
            });
            for (_, pos) in &out.filled {
                // 🤖 ce que disaient les indicateurs à l'ouverture : les bougies d'avant celle où l'ordre part
                self.noter_lecture(*pos, Some(b.time));
                if let Some(p) = self.demo.positions.iter().find(|p| p.id == *pos).cloned() {
                    let buy = p.side == argos_core::Side::Long;
                    self.journal_open(&p.base, buy, p.entry, p.stop, p.target, p.risk(), b.time);
                }
            }
            fermes.extend(closed);
            self.demo.payer_funding(b.time, |_| Some(taux), |_| Some(b.close));
        }
        if let Some(c) = fermes.last() {
            self.banner = Some(
                format!(
                    "{} Trade démo fermé ({}) : {:+.2} R",
                    // 💸 gagné ou perdu une fois les coûts payés ; le R reste celui des prix (comme le journal)
                    if c.net() < Money::ZERO { "❌" } else { "✅" },
                    c.reason,
                    c.r()
                )
                .replace('.', ","),
            );
        }
        self.demo_closed(&fermes);
        Self::write_prefs("demo_account.json", &self.demo);
    }

    /// Identifiants des positions et ordres démo.
    fn demo_ids(&self) -> Vec<u64> {
        self.demo.positions.iter().map(|p| p.id).chain(self.demo.pending.iter().map(|p| p.id)).collect()
    }

    /// 🧪 Molette sur le volet.
    pub(crate) fn sim_wheel(&mut self, dy: f64) -> bool {
        let r = self.sim_rect();
        if self.ui.sim.is_none() || !r.contains(self.pointer.win_mouse.0, self.pointer.win_mouse.1) {
            return false;
        }
        // 🖥 une copie, pas take() : sans le volet, la vue n'a pas l'examen (ses règles sont dans le volet) et la limite
        // de défilement était calculée sur un plan plus court que celui dessiné (Résultat et Historique hors d'atteinte)
        let Some(mut s) = self.ui.sim.clone() else { return false };
        self.sim_vue(|v| s.molette(dy, r, v));
        self.ui.sim = Some(s);
        self.redraw();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jour_a_minuit_europe_centrale() {
        // 2026-11-08 23:30 UTC : déjà le 9 à Prague (UTC+1), encore le 8 à New York
        let ms = 1_794_180_600_000;
        assert_eq!(jour_dans("Europe/Prague", ms), "2026-11-09");
        assert_eq!(jour_dans("America/New_York", ms), "2026-11-08");
        assert_eq!(jour_dans("UTC", ms), "2026-11-08");
    }
}
