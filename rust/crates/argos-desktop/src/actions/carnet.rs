//! 📖 The order-book panel (its volet 📖 and the column's module, mock-up v1 approved by Jee on 22 Nov 2026):
//! Carnet / Delta / DOM on the one book shared with the heatmap (`carnets.rs`: Hyperliquid's live stream, 5 s
//! snapshots of the books chosen, Bitfinex's own; Binance live from Argos Murs' stream while it runs), its menus, and
//! what it keeps between two books (Delta's moves, the middle price before, the DOM's centre, the walls already rung).

use crate::*;
use argos_core::carnet as cc;
use argos_data::murs::{self as dm, Carnet, Monnaie};
use argos_render::carnet::{self as rc, Bourse, CarnetClick, CarnetVue};
use argos_render::indicators::Ind;

/// What the panel keeps between two books.
#[derive(Default)]
pub(crate) struct CarnetPanneau {
    /// The coin and step Delta counts on (another one: Delta starts again).
    pub(crate) cle: String,
    pub(crate) delta: Option<cc::Delta>,
    /// The time of the last book given to Delta (ms).
    pub(crate) vu_ms: i64,
    /// The middle price of the book before, and whether it went up.
    pub(crate) mid_avant: f64,
    pub(crate) monte: Option<bool>,
    /// The DOM's ladder centre (0: not set; it stays put until ⌖ Centrer or the price leaves the ladder).
    pub(crate) centre: f64,
    /// Argos Murs' walls already seen (sell side, bucket): 🔊 rings for a new one only; `None`: not read yet.
    pub(crate) murs_vus: Option<Vec<(bool, i64)>>,
    /// Last rebuild from Binance's stream (≈ 5 times a second at most), and the last one at all (once a second
    /// without a new book: Delta's window slides, the DOM's coins traded grow).
    pub(crate) flux_at: Option<Instant>,
    pub(crate) maj_at: Option<Instant>,
}

/// The panel's menus (their pill).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CarnetMenu {
    Bourse,
    Pas,
    Echange,
    Reglages,
}

/// Levels per side kept for the panel (the tallest volet shows fewer).
const NIVEAUX_MAX: usize = 60;
/// ⚙ « Niveaux par côté » offered (0: as many as the panel holds).
const NIVEAUX: [usize; 5] = [0, 10, 15, 20, 30];
/// The DOM's ladder: the price may wander this many steps from its centre before it centres again by itself.
const DOM_DERIVE: f64 = 10.0;

/// Ids of the ⚙ menu.
const ID_USD: i64 = 0;
const ID_NIVEAUX: i64 = 10;
const ID_PALETTE: i64 = 20;
const ID_SON: i64 = 30;
/// Ids of the exchange menu: 0 the chart's, 1 + `Bourse::ALL`, then the currencies.
const ID_MONNAIE: i64 = 10;

/// 📖 A book as the panel reads it: (time ms, bids best first, asks best first, from Binance's live stream).
type Niveaux = (i64, Vec<(f64, f64)>, Vec<(f64, f64)>, bool);

impl App {
    /// Is the order-book volet open?
    pub(crate) fn carnet_volet_ouvert(&self) -> bool {
        self.watch.prefs.open && self.watch.prefs.panel == wl::Tool::Carnet
    }

    /// The step the panel was asked for at `px` (the one chosen, else the price's own).
    pub(crate) fn carnet_pas_choisi(&self, px: f64) -> f64 {
        cc::pas_du_prix(px, self.watch.prefs.carnet.pas)
    }

    /// 📖 Binance perp's book merged from Argos Murs' photos and stream, when the choice is Binance perp in USDT alone
    /// and the stream is connected for the chart's coin.
    fn carnet_flux_binance(&self) -> Option<argos_core::murs::VenueBook> {
        let ch = self.carnets_choix()?;
        if ch.carnets != [(Carnet::BinancePerp, Monnaie::Usdt)] {
            return None;
        }
        let coin = self.book_coin()?;
        let m = self.feeds.murs.as_deref().filter(|m| m.coin == coin)?;
        let f = m.flux.as_ref().filter(|f| f.connecte && f.carnet.span.is_some())?;
        let mid = self.chart.bars.last().map_or(m.last_px, |b| b.close);
        Some(f.carnet.livre("Binance", mid)).filter(|b| !b.bids.is_empty() && !b.asks.is_empty())
    }

    /// The book the panel reads: (time ms, bids best first, asks best first, from Binance's live stream).
    fn carnet_niveaux(&self) -> Option<Niveaux> {
        if let Some(b) = self.carnet_flux_binance() {
            return Some((now_s() * 1000, b.bids, b.asks, true));
        }
        let b = self.feeds.book.as_ref()?;
        let l = |v: &[argos_data::book::Level]| v.iter().map(|x| (x.px, x.sz)).collect::<Vec<_>>();
        Some((b.time_ms, l(&b.bids), l(&b.asks), false))
    }

    /// 🧱 Is the book read Argos Murs' photos (it reads the same books already)?
    fn carnet_par_murs(&self) -> bool {
        self.feeds.carnets.as_ref().is_some_and(|s| s.murs_vu.is_some())
    }

    /// Are the trades of the chart's market those of the book read (Delta's « rempli »)?
    fn carnet_trades_du_livre(&self) -> bool {
        let Source::Live { exchange, .. } = &self.chart.source else { return false };
        if exchange.bitfinex().is_some() {
            return true;
        }
        match self.carnets_choix() {
            Some(ch) => ch.carnets.len() == 1 && dm::carnet_de(*exchange) == Some(ch.carnets[0].0),
            None => false,
        }
    }

    /// 📖 A new book (or a new piece of Binance's stream): into Delta, then the panel is built again.
    pub(crate) fn carnet_livre(&mut self) {
        if !self.carnet_montre() {
            if !self.feeds.carnet.cle.is_empty() || self.feeds.carnet.maj_at.is_some() {
                self.feeds.carnet = Default::default();
            }
            return;
        }
        if let Some((t, bids, asks, _)) = self.carnet_niveaux() {
            let px = match (bids.first(), asks.first()) {
                (Some(b), Some(a)) => (b.0 + a.0) / 2.0,
                _ => 0.0,
            };
            let fin = cc::finesse(&bids).max(cc::finesse(&asks));
            let pas = cc::pas_effectif(px, self.watch.prefs.carnet.pas, fin);
            let cle = format!("{}|{pas}", self.book_coin().unwrap_or_default());
            let s = &mut self.feeds.carnet;
            if s.cle != cle {
                (s.cle, s.delta, s.vu_ms, s.centre) = (cle, Some(cc::Delta::new(pas)), 0, 0.0);
            }
            if t != s.vu_ms {
                s.vu_ms = t;
                if let Some(d) = s.delta.as_mut() {
                    d.livre(t, &bids, &asks);
                }
            }
        }
        self.carnet_maj();
    }

    /// 📖 Each image: the panel is built again once a second (it also is at each new book); hidden, it forgets.
    pub(crate) fn carnet_suivre(&mut self) {
        let shown = self.carnet_montre();
        if (shown && self.feeds.carnet.maj_at.is_none_or(|t| t.elapsed().as_millis() >= 1000)) || (!shown && !self.feeds.carnet.cle.is_empty()) {
            self.carnet_livre();
        }
    }

    /// 📖 Binance's stream moved the merged book: the panel follows it (≈ 5 times a second at most) when it reads it.
    pub(crate) fn carnet_flux_bouge(&mut self) {
        if !self.carnet_montre() || self.feeds.carnet.flux_at.is_some_and(|t| t.elapsed().as_millis() < 200) {
            return;
        }
        if self.carnet_flux_binance().is_some() {
            self.feeds.carnet.flux_at = Some(Instant::now());
            self.carnet_livre();
            self.redraw();
        }
    }

    /// 📖 The chart's trades (already filtered to its market): Delta's « rempli » when the book read is that market's.
    pub(crate) fn carnet_trades(&mut self, list: &[(i64, f64, f64, bool)]) {
        if list.is_empty() || !self.carnet_montre() || !self.carnet_trades_du_livre() {
            return;
        }
        if let Some(d) = self.feeds.carnet.delta.as_mut() {
            for &(t, px, q, achat) in list {
                d.trade(t, px, q, achat);
            }
        }
    }

    /// The exchange pill and the footer's words: (« Binance », « Binance USDT 1/1 · photo toutes les 5 s »).
    fn carnet_noms(&self) -> (String, String) {
        let Source::Live { exchange, .. } = &self.chart.source else {
            return (String::new(), String::new());
        };
        if let Some(perp) = exchange.bitfinex() {
            let n = usize::from(self.feeds.book.is_some());
            let genre = if perp { "Perpétuel" } else { "Spot" };
            return ("Bitfinex".into(), format!("Bitfinex {genre} {n}/1 · photo toutes les 5 s"));
        }
        let Some(ch) = self.carnets_choix() else {
            return (String::new(), String::new());
        };
        let mut noms: Vec<&str> = ch.carnets.iter().map(|(c, _)| c.name()).collect();
        noms.dedup();
        let pilule = if noms.len() > 1 {
            Bourse::Agrege.nom().to_string()
        } else {
            noms.first().copied().unwrap_or("").to_string()
        };
        let pied = match self.carnets_titre() {
            // (🧱 Argos Murs' photos, every 10 s)
            Some(t) if self.carnet_par_murs() => format!("{t} · photo toutes les 10 s"),
            Some(t) if self.carnets_instantanes() => format!("{t} · photo toutes les {} s", crate::actions::carnets::PERIOD_S),
            Some(t) => t,
            None => "Hyperliquid USDC".into(),
        };
        (pilule, pied)
    }

    /// 📖 The panel is built again from the book read (its tabs, Delta's window, the DOM's ladder and coins traded).
    pub(crate) fn carnet_maj(&mut self) {
        if !self.carnet_montre() {
            return;
        }
        self.feeds.carnet.maj_at = Some(Instant::now());
        let prefs = self.watch.prefs.carnet.clone();
        let (bourse, source) = self.carnet_noms();
        let mut vue = CarnetVue {
            dec: data_decimals(&self.chart.bars),
            unite: match self.book_coin().as_deref() {
                Some("BTC") => "₿".into(),
                Some(c) => c.to_string(),
                None => String::new(),
            },
            bourse,
            source,
            ..Default::default()
        };
        if let argos_render::dock::CarnetSource::Indisponible(m) = self.carnet_source() {
            vue.message = Some(m);
            self.watch.carnet = vue;
            return;
        }
        let Some((_, bids, asks, live)) = self.carnet_niveaux() else {
            // (its snapshots answered, none with a book: not « reading » any more)
            let refuse = self.feeds.carnets.as_ref().is_some_and(|s| s.at.is_some() && !s.busy && s.ok == 0);
            vue.message = Some(
                if refuse {
                    "Carnet indisponible pour le moment"
                } else {
                    "Lecture du carnet…"
                }
                .into(),
            );
            self.watch.carnet = vue;
            return;
        };
        let (Some(b0), Some(a0)) = (bids.first().copied(), asks.first().copied()) else {
            vue.message = Some("Carnet vide pour le moment".into());
            self.watch.carnet = vue;
            return;
        };
        let mid = (b0.0 + a0.0) / 2.0;
        let fin = cc::finesse(&bids).max(cc::finesse(&asks));
        let pas = cc::pas_effectif(mid, prefs.pas, fin);
        let n = if prefs.niveaux > 0 { prefs.niveaux } else { NIVEAUX_MAX };
        (vue.bids, vue.asks) = (cc::regrouper(&bids, pas, true, n), cc::regrouper(&asks, pas, false, n));
        (vue.pas, vue.mid, vue.ecart) = (pas, mid, (a0.0 - b0.0).max(0.0));
        // Hyperliquid's own stream, or Binance's merged one (Argos Murs' stream)
        vue.en_direct = live || (self.feeds.carnets.is_none() && self.feeds.book.is_some() && !self.carnets_instantanes());
        vue.meme_flux = live || self.carnet_par_murs();
        // ▲ / ▼: the middle price against the book before
        let now_ms = now_s() * 1000;
        let s = &mut self.feeds.carnet;
        if s.mid_avant > 0.0 && (mid - s.mid_avant).abs() > 1e-12 {
            s.monte = Some(mid > s.mid_avant);
        }
        (s.mid_avant, vue.monte) = (mid, s.monte);
        // Delta: what moved during the window
        if let Some(d) = s.delta.as_ref().filter(|d| (d.pas - pas).abs() <= pas * 1e-9) {
            let ms = prefs.fenetre.ms();
            (vue.delta, vue.net) = (d.fenetre(now_ms, ms), d.net(now_ms, ms));
        }
        // DOM: its centre stays until ⌖ Centrer, or until the price wanders off the ladder
        if s.centre <= 0.0 || (mid - s.centre).abs() > DOM_DERIVE * pas {
            s.centre = cc::case(mid, pas, true) as f64 * pas;
        }
        vue.centre = s.centre;
        vue.echange = cc::echange(&self.murs_minutes(), now_ms / 1000 - prefs.echange_min * 60, pas);
        // Argos Murs' walls on the chart (perp): in bold; 🔊 a new one rings when asked
        let murs: Vec<(bool, f64)> = self
            .murs_ecran()
            .map(|e| e.vue.pills.iter().filter(|p| !p.spot).map(|p| (!p.sell, p.price)).collect())
            .unwrap_or_default();
        let cles: Vec<(bool, i64)> = murs.iter().map(|(achat, px)| (!achat, cc::case(*px, pas, *achat))).collect();
        let nouveau = self.feeds.carnet.murs_vus.as_ref().is_some_and(|vus| cles.iter().any(|c| !vus.contains(c)));
        self.feeds.carnet.murs_vus = Some(cles);
        if nouveau && prefs.son_murs {
            let son = argos_core::alerts::son_a_jouer(std::iter::empty::<Option<&str>>(), &self.top.prefs.son_alertes);
            platform::alert_son(son, self.top.prefs.volume_alertes);
        }
        vue.murs = murs;
        self.watch.carnet = vue;
    }

    /// 📖 A click in the panel (volet or column), its rects already in the window: a menu, a tab, Delta's window, the
    /// DOM's centre.
    pub(crate) fn carnet_click(&mut self, c: CarnetClick) {
        match c {
            CarnetClick::Bourse(r) => return self.open_list(ListKind::Carnet(CarnetMenu::Bourse), r),
            CarnetClick::Pas(r) => return self.open_list(ListKind::Carnet(CarnetMenu::Pas), r),
            CarnetClick::Echange(r) => return self.open_list(ListKind::Carnet(CarnetMenu::Echange), r),
            CarnetClick::Reglages(r) => return self.open_list(ListKind::Carnet(CarnetMenu::Reglages), r),
            CarnetClick::Onglet(o) => self.watch.prefs.carnet.onglet = o,
            CarnetClick::Fenetre(f) => self.watch.prefs.carnet.fenetre = f,
            CarnetClick::Centrer => self.feeds.carnet.centre = 0.0,
            CarnetClick::Fermer => {
                self.watch.prefs.open = false;
            }
            CarnetClick::Rien => return,
        }
        self.carnet_garder();
    }

    /// A click's rect moved into the window by `f` (the volet's or the column's place).
    pub(crate) fn carnet_dans_fenetre(&self, c: CarnetClick, f: impl Fn(&Self, Rect) -> Rect) -> CarnetClick {
        match c {
            CarnetClick::Bourse(r) => CarnetClick::Bourse(f(self, r)),
            CarnetClick::Pas(r) => CarnetClick::Pas(f(self, r)),
            CarnetClick::Echange(r) => CarnetClick::Echange(f(self, r)),
            CarnetClick::Reglages(r) => CarnetClick::Reglages(f(self, r)),
            c => c,
        }
    }

    /// The panel's settings saved, the panel built again and drawn.
    fn carnet_garder(&mut self) {
        self.save_watch();
        self.carnet_livre();
        self.redraw();
    }

    /// The menu of one of the panel's pills.
    pub(crate) fn carnet_menu(&self, m: CarnetMenu) -> Vec<MenuGroup> {
        let item = |id: i64, label: String, checked: bool| MenuItem {
            id,
            label,
            enabled: true,
            checked,
            ..Default::default()
        };
        let groupe = |title: &str, items: Vec<MenuItem>| MenuGroup {
            title: title.into(),
            items,
            strip: false,
        };
        let p = &self.watch.prefs.carnet;
        match m {
            CarnetMenu::Bourse => {
                let v = argos_render::indicators::values(&self.ind.set, Ind::BookHeat);
                let inp = argos_script::inputs::Inputs {
                    params: &argos_render::bookheat::PARAMS,
                    vals: &v,
                };
                let auto = inp.pick("source") == 0;
                let coches = ["cBinance", "cBybit", "cOkx", "cBitunix", "cHyperliquid"].map(|k| inp.on(k));
                let mut l = vec![item(0, "Bourse de la chart".into(), auto)];
                l.extend(
                    Bourse::ALL
                        .iter()
                        .enumerate()
                        .map(|(k, b)| item(1 + k as i64, b.nom().into(), !auto && Bourse::de_coches(coches) == Some(*b))),
                );
                let monnaie = inp.pick("monnaie");
                let mo = argos_render::bookheat::MONNAIES
                    .iter()
                    .enumerate()
                    .map(|(k, n)| item(ID_MONNAIE + k as i64, n.to_string(), k == monnaie))
                    .collect();
                vec![groupe("CARNET LU (AUSSI CELUI DE LA HEATMAP)", l), groupe("MONNAIE", mo)]
            }
            CarnetMenu::Pas => {
                let w = &self.watch.carnet;
                let px = if w.mid > 0.0 {
                    w.mid
                } else {
                    self.chart.bars.last().map_or(0.0, |b| b.close)
                };
                let l = cc::pas_possibles(px)
                    .into_iter()
                    .enumerate()
                    .map(|(k, q)| item(k as i64, rc::pas_nom(q), (q - w.pas).abs() <= q * 1e-9))
                    .collect();
                vec![groupe("PAS DE PRIX", l)]
            }
            CarnetMenu::Echange => {
                let l = rc::ECHANGES_MIN
                    .iter()
                    .enumerate()
                    .map(|(k, m)| item(k as i64, rc::duree_nom(*m), *m == p.echange_min))
                    .collect();
                vec![groupe("ÉCHANGÉ SUR", l)]
            }
            CarnetMenu::Reglages => {
                let unite = if self.watch.carnet.unite.is_empty() {
                    "pièces".to_string()
                } else {
                    self.watch.carnet.unite.clone()
                };
                let niveaux = NIVEAUX
                    .iter()
                    .enumerate()
                    .map(|(k, n)| {
                        item(
                            ID_NIVEAUX + k as i64,
                            if *n == 0 { "Auto (tout le panneau)".into() } else { n.to_string() },
                            *n == p.niveaux,
                        )
                    })
                    .collect();
                let couleurs = rc::PALETTES
                    .iter()
                    .enumerate()
                    .map(|(k, n)| item(ID_PALETTE + k as i64, n.to_string(), k == p.palette))
                    .collect();
                vec![
                    groupe(
                        "TAILLES",
                        vec![item(ID_USD, "En dollars".into(), p.usd), item(ID_USD + 1, format!("En {unite}"), !p.usd)],
                    ),
                    groupe("NIVEAUX PAR CÔTÉ", niveaux),
                    groupe("COULEURS", couleurs),
                    groupe("SON", vec![item(ID_SON, "🔊 Son des gros murs (Argos Murs)".into(), p.son_murs)]),
                ]
            }
        }
    }

    /// A line of one of the panel's menus was chosen.
    pub(crate) fn carnet_pick(&mut self, m: CarnetMenu, id: i64) {
        match m {
            CarnetMenu::Bourse => {
                let mut v = argos_render::indicators::values(&self.ind.set, Ind::BookHeat);
                let params = &argos_render::bookheat::PARAMS;
                let mut set = |cle: &str, x: f64| {
                    if let Some(k) = params.iter().position(|p| p.key == cle) {
                        if let Some(s) = v.get_mut(k) {
                            *s = x;
                        }
                    }
                };
                if id == 0 {
                    set("source", 0.0);
                } else if let Some(b) = Bourse::ALL.get(id as usize - 1).filter(|_| id < ID_MONNAIE) {
                    set("source", 1.0);
                    for (k, on) in ["cBinance", "cBybit", "cOkx", "cBitunix", "cHyperliquid"].iter().zip(b.coches()) {
                        set(k, if on { 1.0 } else { 0.0 });
                    }
                } else if id >= ID_MONNAIE {
                    set("monnaie", (id - ID_MONNAIE) as f64);
                }
                self.ind.set.insert(Ind::BookHeat, v);
                self.ind.key = Default::default();
                self.garder_reglages();
                // another book: read it now (the heatmap starts again on it too)
                self.feeds.carnets = None;
                self.carnets_refresh();
            }
            CarnetMenu::Pas => {
                let px = if self.watch.carnet.mid > 0.0 {
                    self.watch.carnet.mid
                } else {
                    self.chart.bars.last().map_or(0.0, |b| b.close)
                };
                if let Some(q) = cc::pas_possibles(px).get(id as usize) {
                    self.watch.prefs.carnet.pas = *q;
                }
            }
            CarnetMenu::Echange => {
                if let Some(m) = rc::ECHANGES_MIN.get(id as usize) {
                    self.watch.prefs.carnet.echange_min = *m;
                }
            }
            CarnetMenu::Reglages => {
                let p = &mut self.watch.prefs.carnet;
                match id {
                    ID_USD => p.usd = true,
                    1 => p.usd = false,
                    ID_SON => p.son_murs = !p.son_murs,
                    i if (ID_PALETTE..ID_SON).contains(&i) => p.palette = ((i - ID_PALETTE) as usize).min(rc::PALETTES.len() - 1),
                    i if (ID_NIVEAUX..ID_PALETTE).contains(&i) => p.niveaux = NIVEAUX.get((i - ID_NIVEAUX) as usize).copied().unwrap_or(0),
                    _ => {}
                }
            }
        }
        self.carnet_garder();
    }
}
