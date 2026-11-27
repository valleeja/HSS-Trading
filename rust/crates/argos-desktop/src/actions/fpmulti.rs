//! 🏦 Footprint multi-bourses (maquette v1, « ok maquette footprint multi » de Jee le 22 nov. 2026) : la pièce de la
//! chart est suivie sur plusieurs bourses (celle de la chart et, en « Auto », les 4 plus gros volumes) ; leurs minutes
//! de vrais trades sont additionnées prix par prix (`argos_core::orderflow::fusionner`, prix recalés sur la bourse de
//! la chart) et le footprint est refait dessus. Le CVD et les Bulles restent sur la bourse de la chart. Le ⚙ de la
//! légende ouvre le panneau des bourses (`argos_render::fpmulti::Reglages`) : 🖥 sous la légende, 📱 en feuille.
//!
//! OKX compte ses trades de perpétuels en contrats : ils passent en pièces à l'addition (taille du contrat lue une fois,
//! la même qu'Argos Murs) ; tant qu'elle n'est pas connue, OKX n'est pas additionnée.

use crate::*;
use argos_core::orderflow::{Cle, FlowBar};
use argos_render::fpmulti::{self as fm, Clic, FpMulti, Reglages};
use argos_render::indicators::Ind;
use argos_render::text::Text;
use argos_script::indicators::flow::{FOOTPRINT_CASES, FOOTPRINT_PARAMS};

/// La bourse de chaque case du groupe « 🏦 Bourses » (même ordre que `FOOTPRINT_BOURSES`).
pub(crate) const BOURSES: [Exchange; 10] = [
    Exchange::Binance,
    Exchange::Bybit,
    Exchange::Okx,
    Exchange::Coinbase,
    Exchange::Hyperliquid,
    Exchange::Bitget,
    Exchange::Kraken,
    Exchange::Bitstamp,
    Exchange::Gate,
    Exchange::Mexc,
];

/// Minutes lues pour l'écart de prix d'une bourse avec la chart (sa médiane).
const ECART_N: usize = 30;

/// « Binance Perpétuel » : le nom d'une bourse et de son marché.
fn nom_marche(ex: Exchange) -> String {
    format!("{} {}", ex.name(), ex.market_kind())
}

impl App {
    fn fp_vals(&self) -> Vec<f64> {
        argos_render::indicators::values(&self.ind.set, Ind::Footprint)
    }

    fn fp_on(&self, cle: &str) -> bool {
        argos_script::inputs::Inputs {
            params: &FOOTPRINT_PARAMS,
            vals: &self.fp_vals(),
        }
        .on(cle)
    }

    /// La bourse de la chart, sa place dans la liste des cases et sa pièce, quand l'orderflow réel y est possible.
    fn fp_chart(&self) -> Option<(Exchange, Option<usize>, String)> {
        let cle = self.orderflow_cle()?;
        let ex = super::orderflow::bourse(&cle)?;
        Some((ex, BOURSES.iter().position(|e| *e == ex), cle.symbole))
    }

    /// Les bourses fusionnées (indices des cases), selon les réglages, pour la pièce de la chart.
    fn fp_choisies(&self) -> Vec<usize> {
        let Some((_, idx, coin)) = self.fp_chart() else { return vec![] };
        fm::choisies(&self.fp_vals(), idx, &|k| argos_data::orderflow::reel_pour(BOURSES[k], &coin))
    }

    /// 🏦 Les marchés du footprint multi-bourses, celui de la chart d'abord : (bourse, clé). Vide : Footprint éteint,
    /// orderflow réel impossible sur la chart, ou une seule bourse cochée (le footprint d'aujourd'hui).
    pub(crate) fn fp_cles(&self) -> Vec<(Exchange, Cle)> {
        if !self.ind.on.contains(&Ind::Footprint) {
            return vec![];
        }
        let (Some(cle), Some((ex, _, coin))) = (self.orderflow_cle(), self.fp_chart()) else {
            return vec![];
        };
        let mut v = vec![(ex, cle)];
        for k in self.fp_choisies() {
            let e = BOURSES[k];
            if e != ex {
                v.push((
                    e,
                    Cle {
                        exchange: e.id().to_string(),
                        symbole: coin.clone(),
                    },
                ));
            }
        }
        if v.len() < 2 {
            return vec![];
        }
        v
    }

    /// 🏦 Ce qu'il manque pour additionner : la taille du contrat OKX de la pièce, et celle des bourses à contrats
    /// cochées (MEXC, Gate.io) ; chacune demandée une fois, la réponse arrive par `UserEvent::OrderflowContrats`.
    pub(crate) fn fp_demander(&mut self) {
        if !self.ind.on.contains(&Ind::Footprint) {
            return;
        }
        let Some((_, _, coin)) = self.fp_chart() else { return };
        let Some(proxy) = self.proxy.clone() else { return };
        if self.fp_choisies().iter().any(|k| BOURSES[*k] == Exchange::Okx)
            && argos_data::murs::okx_ctval_connu(&coin).is_none()
            && self.feeds.fp_ctval.insert(coin.clone())
        {
            let (proxy, c) = (proxy.clone(), coin.clone());
            argos_data::rt::spawn(move || async move {
                let http = argos_data::rt::Client::default();
                let res = argos_data::murs::okx_ctval(&http, &c).await.map(|_| ()).map_err(String::from);
                let _ = proxy.send_event(UserEvent::OrderflowContrats(Exchange::Okx, res));
            });
        }
        let cases: Vec<Exchange> = BOURSES.iter().zip(FOOTPRINT_CASES).filter(|(_, c)| self.fp_on(c)).map(|(e, _)| *e).collect();
        for ex in cases {
            if argos_data::orderflow::branchee(ex) && !argos_data::whales::taille_lisible(ex.name(), &coin) && self.feeds.contrats_demandes.insert(ex) {
                let proxy = proxy.clone();
                argos_data::rt::spawn(move || async move {
                    let res = argos_data::whaletop::charger_contrats(ex.name()).await;
                    let _ = proxy.send_event(UserEvent::OrderflowContrats(ex, res));
                });
            }
        }
    }

    /// 🏦 Les minutes d'un marché suivi depuis `t0` (fermées et en cours), tailles en pièces (OKX : × la taille du
    /// contrat) ; `None` : pas suivi, flux indisponible, ou taille du contrat OKX pas encore connue.
    fn fp_minutes(&self, ex: Exchange, cle: &Cle, t0: i64) -> Option<Vec<FlowBar>> {
        if self.feeds.flux_indisponibles.contains(&ex) {
            return None;
        }
        let f = self.feeds.orderflow.iter().find(|f| f.cle == *cle)?;
        let k = if ex == Exchange::Okx {
            argos_data::murs::okx_ctval_connu(&cle.symbole)?
        } else {
            1.0
        };
        let debut = f.minutes.partition_point(|b| b.time < t0);
        let mut m: Vec<FlowBar> = f.minutes[debut..].to_vec();
        m.extend(f.agg.en_cours());
        if k != 1.0 {
            for b in &mut m {
                for r in &mut b.rangees {
                    (r.1, r.2) = (r.1 * k, r.2 * k);
                }
            }
        }
        Some(m)
    }

    /// 🏦 Après le calcul des indicateurs : le footprint refait sur les trades des bourses additionnées (rien quand le
    /// footprint n'a qu'une bourse, ou que l'unité de la chart n'est pas un nombre entier de minutes).
    pub(crate) fn fp_calculer(&mut self) {
        self.ind.out.fp_multi = None;
        let bars = &self.chart.bars;
        let tf = self.chart.interval.secs();
        if self.ind.out.footprint.is_none() || !(tf >= 60 && tf % 60 == 0 && 86_400 % tf == 0) {
            return;
        }
        let cles = self.fp_cles();
        if cles.len() < 2 {
            return;
        }
        let vals = self.fp_vals();
        let p = argos_script::inputs::Inputs {
            params: &FOOTPRINT_PARAMS,
            vals: &vals,
        };
        let nb = p.get("nb").round() as usize;
        // seulement ce que le footprint montre (ses `nb` dernières bougies), et de quoi lire l'écart de prix
        let t0 = bars
            .get(bars.len().saturating_sub(nb))
            .map_or(0, |b| b.time)
            .min(now_s() - ECART_N as i64 * 60 - 60);
        let (dollars, aligne) = (p.pick("unite") == 1, p.on("aligne"));
        let minutes: Vec<Option<Vec<FlowBar>>> = cles.iter().map(|(ex, c)| self.fp_minutes(*ex, c, t0)).collect();
        let Some(reference) = minutes[0].clone() else { return };
        let (mut noms, mut barres, mut ecarts) = (vec![], vec![], vec![]);
        for ((ex, _), m) in cles.iter().zip(minutes) {
            let Some(m) = m else { continue };
            let couleur = BOURSES.iter().position(|e| e == ex).map_or(fm::AUTRE, |k| fm::COULEURS[k]);
            ecarts.push(if aligne && !barres.is_empty() {
                argos_core::orderflow::ecart(&reference, &m, ECART_N)
            } else {
                0.0
            });
            noms.push((ex.name().to_string(), couleur));
            barres.push(argos_core::orderflow::regrouper(&m, tf));
        }
        let coin = cles[0].1.symbole.clone();
        let (m, fusion) = FpMulti::de(noms, barres, ecarts, dollars, &coin, &nom_marche(cles[0].0));
        self.ind.out.footprint = Some(argos_script::indicators::flow::footprint_multi(bars, &vals, &fusion));
        self.ind.out.fp_multi = Some(std::sync::Arc::new(m));
    }

    // ───────────────────────────── ⚙ ─────────────────────────────

    /// 🏦 Ce que le ⚙ montre (`None` : Footprint éteint).
    fn fp_reglages_etat(&self) -> Option<Reglages> {
        if !self.ind.on.contains(&Ind::Footprint) {
            return None;
        }
        let chart = self.fp_chart();
        let vals = self.fp_vals();
        let p = argos_script::inputs::Inputs {
            params: &FOOTPRINT_PARAMS,
            vals: &vals,
        };
        let piece = chart.as_ref().map_or_else(|| "Pièces".to_string(), |c| c.2.clone());
        let reference = match &self.chart.source {
            Source::Live { exchange, .. } => nom_marche(*exchange),
            _ => "la bourse de la chart".to_string(),
        };
        // sans orderflow réel sur la chart : les cases telles qu'elles sont réglées
        let choisies = if chart.is_some() {
            self.fp_choisies()
        } else {
            (0..BOURSES.len()).filter(|k| p.on(FOOTPRINT_CASES[*k])).collect()
        };
        Some(Reglages {
            auto: p.on("auto"),
            choisies,
            chart: chart.and_then(|c| c.1),
            dollars: p.pick("unite") == 1,
            aligne: p.on("aligne"),
            parts: p.on("parts"),
            piece,
            reference,
        })
    }

    /// 🏦 Le ⚙ et sa place dans la fenêtre, quand il est ouvert : 🖥 sous la ligne du Footprint de la légende ; 📱 la
    /// feuille en bas de l'écran.
    pub(crate) fn fp_place(&self, text: &Text) -> Option<(Reglages, Rect)> {
        if !self.ui.fp_reglages || self.page_ouverte() {
            return None;
        }
        let r = self.fp_reglages_etat()?;
        let out = &self.ind.out;
        let sous = argos_render::indleg::haut_ligne(out, Ind::Footprint)
            .filter(|_| self.ind.leg.0)
            .map_or_else(|| argos_render::indleg::bas_legende(out, self.ind.leg.0, false), |y| y + 26.0)
            + 7.0;
        let win_h = self.top_h() + self.area.1 as f64;
        let place = r.place(text, self.rw() + 8.0, self.top_h() + sous, self.win_w(), win_h, self.top.phone);
        Some((r, place))
    }

    /// 🏦 Dessine le ⚙ (et rend sa place).
    pub(crate) fn fp_draw(&self, text: &Text) -> Option<(tiny_skia::Pixmap, Rect)> {
        let (r, q) = self.fp_place(text)?;
        let survol = self.survol(Some(self.pointer.win_mouse)).map(|(x, y)| (x - q.x, y - q.y));
        Some((r.draw(text, q.w, self.top.phone, survol)?, q))
    }

    /// 🏦 Le ⚙ est-il sous ce point (fenêtre), sans rien au-dessus de lui ?
    pub(crate) fn fp_sous(&self, x: f64, y: f64) -> bool {
        self.ui.fp_reglages
            && !self.ui.context.is_open()
            && self.ui.goto.is_none()
            && !self.couche_haute(x, y)
            && self.fp_place(&Text::default()).is_some_and(|(_, r)| r.contains(x, y))
    }

    /// 🏦 La main sur ce qui se clique du ⚙ (`None` : pas sur lui).
    pub(crate) fn fp_main(&self, x: f64, y: f64) -> Option<bool> {
        if !self.fp_sous(x, y) {
            return None;
        }
        let text = Text::default();
        let (r, q) = self.fp_place(&text)?;
        Some(r.click(&text, q.w, self.top.phone, x - q.x, y - q.y) != Clic::Rien)
    }

    /// 🏦 ⚙ de la légende du Footprint : ouvre ou ferme le panneau des bourses.
    pub(crate) fn fp_basculer(&mut self) {
        self.ui.fp_reglages = !self.ui.fp_reglages;
        self.ind.sel = None;
        self.redraw();
    }

    /// Règle une valeur du Footprint (recalcul tout de suite, gardée).
    fn fp_regler(&mut self, cle: &str, x: f64) {
        let mut v = self.fp_vals();
        if let Some(k) = FOOTPRINT_PARAMS.iter().position(|p| p.key == cle) {
            v[k] = x;
        }
        self.ind.set.insert(Ind::Footprint, v);
        self.ind.key = Default::default();
        self.garder_reglages();
    }

    /// « Auto » décoché (ou une pastille touchée en « Auto ») : les cases prennent les bourses montrées, pour que
    /// rien ne change à l'écran avant le choix suivant.
    fn fp_figer(&mut self) {
        let montrees = self.fp_reglages_etat().map(|r| r.choisies).unwrap_or_default();
        for (k, c) in FOOTPRINT_CASES.iter().enumerate() {
            self.fp_regler(c, f64::from(u8::from(montrees.contains(&k))));
        }
        self.fp_regler("auto", 0.0);
    }

    /// 🏦 Clic dans le ⚙.
    pub(crate) fn fp_click(&mut self, x: f64, y: f64) {
        let text = Text::default();
        let Some((r, q)) = self.fp_place(&text) else { return };
        match r.click(&text, q.w, self.top.phone, x - q.x, y - q.y) {
            Clic::Rien => return,
            Clic::Fermer => self.ui.fp_reglages = false,
            Clic::Auto if r.auto => self.fp_figer(),
            Clic::Auto => self.fp_regler("auto", 1.0),
            // la bourse de la chart reste : c'est elle qui donne les prix
            Clic::Bourse(k) if r.chart == Some(k) => {
                self.banner = Some(format!("🏦 {} est la bourse de la chart : toujours dans le footprint", BOURSES[k].name()));
            }
            Clic::Bourse(k) => {
                if r.auto {
                    self.fp_figer();
                }
                let on = self.fp_on(FOOTPRINT_CASES[k]);
                self.fp_regler(FOOTPRINT_CASES[k], f64::from(u8::from(!on)));
            }
            Clic::Unite(d) => self.fp_regler("unite", f64::from(u8::from(d))),
            Clic::Aligne => self.fp_regler("aligne", f64::from(u8::from(!r.aligne))),
            Clic::Parts => self.fp_regler("parts", f64::from(u8::from(!r.parts))),
            Clic::Tous => {
                self.ui.fp_reglages = false;
                self.open_ind_dlg(Ind::Footprint);
            }
        }
        self.orderflow_suivre();
        self.redraw();
    }
}
