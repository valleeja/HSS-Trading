//! 🧮 Orderflow réel : les trades des bourses branchées (`argos_data::orderflow::reel_pour` : Hyperliquid, Binance…)
//! sont rangés par minute et par rangée de prix (`argos_core::orderflow`) et les minutes fermées sont gardées dans le
//! cache (`argos_data::orderflow`). Hyperliquid passe par le flux du carnet (le même WebSocket que les modules 📖 ⚡,
//! jamais une 2e connexion) ; chaque autre bourse a **une** connexion pour toutes ses pièces suivies. Marchés suivis :
//! celui de la chart (si sa bourse est branchée) et ● jusqu'à `MAX_ENREGISTRES` marchés enregistrés en continu (liste
//! gardée dans `orderflow.json`, bouton « ● Enregistrer » du module ⚡ Trades). La clé est toujours bourse + symbole.

use crate::*;
use argos_core::orderflow::{Agregateur, Cle, FlowBar, FlowStore, TradeFlux};
use argos_data::orderflow::FlowCache;

/// Minutes fermées gardées en mémoire par marché, pour l'écran (3 jours).
pub(crate) const EN_MEMOIRE: usize = 3 * 24 * 60;

/// L'orderflow réel d'un marché suivi.
pub(crate) struct FluxReel {
    pub(crate) cle: Cle,
    pub(crate) agg: Agregateur,
    /// Minutes fermées (relues du cache puis reçues), dans l'ordre.
    pub(crate) minutes: Vec<FlowBar>,
    /// 🩺 Trades received from its exchange, and those put in this market (page diagnostic, `orderflow_diag`).
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(crate) recus: (usize, usize),
}

/// Le cache de l'orderflow (un dossier temporaire dans les tests).
pub(crate) fn store() -> Option<FlowCache> {
    let dir = if cfg!(test) {
        // dans le dossier de compilation du projet (rust/target/tmp), jamais dans /tmp
        Some(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/tmp")
                .join(format!("orderflow-{}", std::process::id())),
        )
    } else {
        argos_data::orderflow::dir()
    };
    dir.map(|dir| FlowCache { dir })
}

/// 🔁 Bitfinex's real trades of `pieces` (chart bases, « XRP/BTC »), read every [`BITFINEX_PERIODE_MS`]: only those
/// not seen yet go to `on_event` (`LiveEvent::Whales`), `Connected` first; stops when `on_event` says `false` (another
/// list of markets). In the browser the wait pauses while the page is hidden (`rt::sleep`).
///
/// [`BITFINEX_PERIODE_MS`]: argos_data::orderflow::BITFINEX_PERIODE_MS
fn bitfinex_trades(ex: Exchange, pieces: Vec<String>, on_event: impl Fn(LiveEvent) -> bool + Send + 'static) {
    let perp = ex == Exchange::Bitfinex;
    argos_data::rt::spawn(move || async move {
        use argos_data::Http as _;
        let http = argos_data::rt::Client::default();
        let mut vus: Vec<Option<u64>> = vec![None; pieces.len()];
        if !on_event(LiveEvent::Connected) {
            return;
        }
        loop {
            for (base, vu) in pieces.iter().zip(vus.iter_mut()) {
                let argos_data::exchanges::Request::Get(url) = argos_data::exchanges::bitfinex::trades_request(base, perp) else {
                    continue;
                };
                let Ok(list) = http
                    .get(&url)
                    .await
                    .and_then(|b| argos_data::exchanges::bitfinex::parse_trades(&b, &base.to_uppercase()))
                else {
                    continue;
                };
                let (nouveaux, haut) = argos_data::orderflow::bitfinex_nouveaux(list, *vu);
                *vu = haut;
                if !nouveaux.is_empty() && !on_event(LiveEvent::Whales(nouveaux)) {
                    return;
                }
            }
            argos_data::rt::sleep(std::time::Duration::from_millis(argos_data::orderflow::BITFINEX_PERIODE_MS)).await;
            // (a list of markets that changed: the next event is refused, nothing more is asked)
            if !on_event(LiveEvent::Whales(vec![])) {
                return;
            }
        }
    });
}

/// La bourse d'une clé.
pub(crate) fn bourse(cle: &Cle) -> Option<Exchange> {
    Exchange::ALL.iter().copied().find(|e| e.id() == cle.exchange)
}

impl App {
    /// La clé du marché de la chart, si l'orderflow réel y est possible (bourse branchée, taille des trades lisible).
    pub(crate) fn orderflow_cle(&self) -> Option<Cle> {
        let Source::Live { exchange, base } = &self.chart.source else { return None };
        // (₿ a pair against BTC: the estimate, the trade streams follow the USDT / USD market; Bitfinex: its real trades of
        // the exact market, keyed by the chart's base, « XRP/BTC » included)
        let bitfinex = exchange.bitfinex().is_some();
        (bitfinex || !argos_data::paire::contre_btc(base))
            .then(|| if bitfinex { base.to_uppercase() } else { argos_data::whales::coin_of(base) })
            .filter(|c| !c.is_empty() && argos_data::orderflow::reel_pour(*exchange, c))
            .map(|symbole| Cle {
                exchange: exchange.id().to_string(),
                symbole,
            })
    }

    /// 🧮 Chart sur une bourse branchée à contrats dont la taille du contrat de la pièce est inconnue : la demande
    /// (une fois par bourse) ; la réponse arrive par `UserEvent::OrderflowContrats`. En attendant : l'estimé.
    fn orderflow_demander_contrats(&mut self) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        let (ex, coin) = (*exchange, argos_data::whales::coin_of(base));
        if argos_data::paire::contre_btc(base)
            || !argos_data::orderflow::branchee(ex)
            || argos_data::orderflow::reel_pour(ex, &coin)
            || !self.feeds.contrats_demandes.insert(ex)
        {
            return;
        }
        if let Some(proxy) = self.proxy.clone() {
            argos_data::rt::spawn(move || async move {
                let res = argos_data::whaletop::charger_contrats(ex.name()).await;
                let _ = proxy.send_event(UserEvent::OrderflowContrats(ex, res));
            });
        }
    }

    /// 🧮 Tailles de contrat lues (ou l'erreur) : la chart passe à l'orderflow réel si sa pièce est maintenant connue ;
    /// sinon, un message clair, et l'estimé reste affiché.
    pub(crate) fn orderflow_contrats_lus(&mut self, ex: Exchange, res: Result<(), String>) {
        let Source::Live { exchange, base } = &self.chart.source else { return };
        if *exchange != ex {
            return;
        }
        let coin = argos_data::whales::coin_of(base);
        if argos_data::orderflow::reel_pour(ex, &coin) {
            return self.orderflow_suivre();
        }
        let pourquoi = res.err().map_or(String::new(), |e| format!(" ({e})"));
        self.banner = Some(format!(
            "🧮 {} {coin} : taille du contrat inconnue : footprint réel indisponible, estimé affiché{pourquoi}",
            ex.name()
        ));
    }

    /// 🧮 L'orderflow réel du marché de la chart à son unité de temps (minutes gardées en mémoire et minute en cours,
    /// additionnées), trié par heure, pour le footprint et le CVD. Vide si le marché n'est pas suivi ou si l'unité
    /// n'est pas un nombre entier de minutes qui divise la journée (secondes, semaine, mois).
    pub(crate) fn orderflow_chart(&self) -> Vec<FlowBar> {
        let tf = self.chart.interval.secs();
        // 🔑 flux indisponible (jeton refusé…) : rien de réel, l'estimé
        let suivi = self
            .orderflow_cle()
            .filter(|cle| bourse(cle).is_some_and(|e| !self.feeds.flux_indisponibles.contains(&e)))
            .and_then(|cle| self.feeds.orderflow.iter().find(|f| f.cle == cle));
        let Some(f) = suivi.filter(|_| tf >= 60 && tf % 60 == 0 && 86_400 % tf == 0) else {
            return vec![];
        };
        let mut minutes = f.minutes.clone();
        minutes.extend(f.agg.en_cours());
        argos_core::orderflow::regrouper(&minutes, tf)
    }

    /// Suit les marchés voulus (chart + ● enregistrés) : un marché qui n'est plus voulu est rangé (sa minute en cours
    /// incomplète), un nouveau repart du pas déjà gardé ; puis les minutes fermées sont écrites dans le cache.
    pub(crate) fn orderflow_suivre(&mut self) {
        self.orderflow_suivre_a(now_s() * 1000);
    }

    /// `orderflow_suivre` à l'heure `now_ms`.
    pub(crate) fn orderflow_suivre_a(&mut self, now_ms: i64) {
        self.orderflow_demander_contrats();
        self.fp_demander();
        let mut voulus = self.feeds.enregistres.liste.clone();
        voulus.extend(self.orderflow_cle().filter(|c| !voulus.contains(c)));
        // 🏦 footprint multi-bourses : la pièce de la chart sur les autres bourses additionnées
        for (_, c) in self.fp_cles() {
            if !voulus.contains(&c) {
                voulus.push(c);
            }
        }
        let (garder, partir): (Vec<FluxReel>, Vec<FluxReel>) = std::mem::take(&mut self.feeds.orderflow).into_iter().partition(|f| voulus.contains(&f.cle));
        for mut f in partir {
            let fin = f.agg.tout_fermer(now_ms);
            Self::orderflow_ecrire(&f.cle, &fin);
        }
        self.feeds.orderflow = garder;
        let now = now_ms / 1000;
        for cle in voulus {
            if self.feeds.orderflow.iter().any(|f| f.cle == cle) {
                continue;
            }
            let minutes = store().map(|s| s.lire(&cle, now - EN_MEMOIRE as i64 * 60, now + 60)).unwrap_or_default();
            let mut agg = Agregateur::new(minutes.last().map(|b| b.pas));
            agg.coupure(now_ms); // démarrage : la minute en cours a commencé sans nous
            if let Some(mut s) = store() {
                s.limiter(now);
            }
            self.feeds.orderflow.push(FluxReel {
                cle,
                agg,
                minutes,
                #[cfg(any(test, target_arch = "wasm32"))]
                recus: (0, 0),
            });
        }
        for f in &mut self.feeds.orderflow {
            let fermees = f.agg.fermees(now_ms);
            if !fermees.is_empty() {
                Self::orderflow_ecrire(&f.cle, &fermees);
                f.minutes.extend(fermees);
                let trop = f.minutes.len().saturating_sub(EN_MEMOIRE);
                f.minutes.drain(..trop);
            }
        }
        self.orderflow_flux();
        #[cfg(target_arch = "wasm32")]
        platform::web::orderflow(&self.orderflow_diag());
    }

    /// 🩺 Read-only diagnostic of the real orderflow, for the page (`window.argosOrderflow`): per followed market, its
    /// exchange and coin, the trades received from the exchange and those put in this market, the closed minutes, the
    /// minutes in progress, the last minute put (seconds) and the last bar of the chart (seconds). Counters and times
    /// only.
    #[cfg(any(test, target_arch = "wasm32"))]
    pub(crate) fn orderflow_diag(&self) -> String {
        let marches: Vec<serde_json::Value> = self
            .feeds
            .orderflow
            .iter()
            .map(|f| {
                let en_cours = f.agg.en_cours();
                serde_json::json!({
                    "bourse": f.cle.exchange,
                    "piece": f.cle.symbole,
                    "recus": f.recus.0,
                    "ranges": f.recus.1,
                    "fermees": f.minutes.len(),
                    "en_cours": en_cours.len(),
                    "derniere_minute": en_cours.last().or(f.minutes.last()).map(|b| b.time),
                })
            })
            .collect();
        serde_json::json!({ "marches": marches, "derniere_bougie": self.chart.bars.last().map(|b| b.time) }).to_string()
    }

    /// Les pièces suivies d'une bourse.
    pub(crate) fn orderflow_pieces(&self, ex: Exchange) -> Vec<String> {
        self.feeds
            .orderflow
            .iter()
            .filter(|f| f.cle.exchange == ex.id())
            .map(|f| f.cle.symbole.clone())
            .collect()
    }

    /// 🏦 Une connexion par bourse branchée (sauf Hyperliquid, dans le flux du carnet) pour toutes ses pièces suivies ;
    /// relancées quand la liste change.
    fn orderflow_flux(&mut self) {
        let voulus: Vec<(Exchange, Vec<String>)> = Exchange::ALL
            .iter()
            .filter(|e| **e != Exchange::Hyperliquid)
            .map(|&e| (e, self.orderflow_pieces(e)))
            .filter(|(e, p)| !p.is_empty() && argos_data::orderflow::branchee(*e))
            .collect();
        if voulus == self.feeds.orderflow_flux {
            return;
        }
        // no event loop yet (the Home page opens at launch before it): nothing is opened, so nothing is noted as open;
        // the next call, once the loop exists, opens the connections
        let Some(proxy) = self.proxy.clone() else { return };
        let gen = self.feeds.orderflow_gen.fetch_add(1, Ordering::SeqCst) + 1;
        for (ex, pieces) in &voulus {
            let (proxy, current, ex) = (proxy.clone(), self.feeds.orderflow_gen.clone(), *ex);
            let parse = |t: &str| Some(argos_data::whales::parse_all(t)).filter(|l| !l.is_empty()).map(LiveEvent::Whales);
            let on_event = move |ev| current.load(Ordering::SeqCst) == gen && proxy.send_event(UserEvent::Orderflow(ex, ev)).is_ok();
            match argos_data::orderflow::source(ex, pieces) {
                Some(sub) => {
                    live::spawn(sub, parse, on_event);
                }
                // KuCoin : son adresse demande un jeton public (sans clé), redemandé à chaque reconnexion
                None if ex == Exchange::Kucoin => {
                    let coins = pieces.clone();
                    live::spawn_jeton(move || argos_data::whales::kucoin_preparer(coins.clone()), parse, on_event);
                }
                // 🔁 Bitfinex: its last trades read every 5 s (in the browser through the PC server's relay)
                None if ex.bitfinex().is_some() => bitfinex_trades(ex, pieces.clone(), on_event),
                None => {}
            }
        }
        self.feeds.orderflow_flux = voulus;
    }

    fn orderflow_ecrire(cle: &Cle, minutes: &[FlowBar]) {
        if let (Some(mut s), false) = (store(), minutes.is_empty()) {
            let _ = s.ecrire(cle, minutes);
        }
    }

    /// ⚡ Where the Trades module reads: the exchange of the chart when its real trades flow is open (Hyperliquid
    /// through the book flow, the others through their orderflow connection); otherwise Hyperliquid, and the exchange
    /// of the chart second, so the module says it is a fallback.
    pub(crate) fn tape_source(&self) -> (Exchange, Option<Exchange>) {
        match &self.chart.source {
            Source::Live { exchange, .. } if *exchange == Exchange::Hyperliquid => (Exchange::Hyperliquid, None),
            Source::Live { exchange, .. } if self.orderflow_cle().is_some() && !self.feeds.flux_indisponibles.contains(exchange) => (*exchange, None),
            Source::Live { exchange, .. } => (Exchange::Hyperliquid, Some(*exchange)),
            _ => (Exchange::Hyperliquid, None),
        }
    }

    /// ⚡ Trades of `ex` for the Trades module: kept only when `ex` is its source, for the coin of the chart, the most
    /// recent first (100 at most); a new source starts an empty list.
    ///
    /// 🔁 Just after its stream is cut or (re)connected, an exchange gives its last trades again. Of that first lot,
    /// only what the list does not hold yet is added: a trade whose number is above the highest one held, or, without
    /// numbers, whose moment is after the newest one held. It is what the order flow does with that lot
    /// (`Agregateur::ajouter`). The other lots are added whole, and trades are never compared one to one: two real
    /// trades can be the same (same moment, price, size and side).
    pub(crate) fn tape_ajouter(&mut self, ex: Exchange, mut list: Vec<argos_data::whales::Whale>) {
        let Some(de) = self.tape_cle().filter(|(e, _)| *e == ex) else { return };
        list.retain(|t| t.coin == de.1);
        if self.feeds.tape_de.as_ref() != Some(&de) {
            (self.feeds.tape, self.feeds.tape_de, self.feeds.tape_apres_connexion) = (vec![], Some(de), false);
        }
        // (a lot with no trade of this coin is not the first lot of the list: the next one still is)
        if list.is_empty() {
            return;
        }
        if std::mem::take(&mut self.feeds.tape_apres_connexion) {
            let held = &self.feeds.tape;
            let (number, moment) = (held.iter().filter_map(|t| t.id).max(), held.iter().map(|t| t.time_ms).max());
            list.retain(|t| match (t.id, number) {
                (Some(id), Some(highest)) => id > highest,
                _ => moment.is_none_or(|newest| t.time_ms > newest),
            });
        }
        list.sort_by_key(|t| -t.time_ms);
        list.append(&mut self.feeds.tape);
        list.truncate(100);
        self.feeds.tape = list;
    }

    /// ⚡ The exchange and coin the Trades module reads now (`None`: no coin).
    fn tape_cle(&self) -> Option<(Exchange, String)> {
        let ex = self.tape_source().0;
        let coin = if ex == Exchange::Hyperliquid {
            self.feeds.flow_coin.clone()
        } else {
            self.orderflow_cle().map(|c| c.symbole)
        };
        coin.map(|c| (ex, c))
    }

    /// ⚡ The trades shown by the Trades module: those of its current source only (none just after the source changed).
    pub(crate) fn tape_vue(&self) -> &[argos_data::whales::Whale] {
        if self.feeds.tape_de.is_some() && self.feeds.tape_de == self.tape_cle() {
            &self.feeds.tape
        } else {
            &[]
        }
    }

    /// Trades reçus d'une bourse : chacun rangé dans le marché suivi de sa pièce.
    pub(crate) fn orderflow_trades(&mut self, ex: Exchange, list: &[argos_data::whales::Whale]) {
        // 📖 Delta's « rempli »: the chart's market's trades
        if let Some(cle) = self.orderflow_cle().filter(|c| c.exchange == ex.id()) {
            let l: Vec<(i64, f64, f64, bool)> = list
                .iter()
                .filter(|t| t.coin == cle.symbole)
                .map(|t| (t.time_ms, t.price, t.size, t.buy))
                .collect();
            self.carnet_trades(&l);
        }
        for f in self.feeds.orderflow.iter_mut().filter(|f| f.cle.exchange == ex.id()) {
            let trades: Vec<TradeFlux> = list
                .iter()
                .filter(|t| t.coin == f.cle.symbole)
                .map(|t| TradeFlux {
                    time_ms: t.time_ms,
                    prix: t.price,
                    taille: t.size,
                    achat: t.buy,
                    id: t.id,
                })
                .collect();
            #[cfg(any(test, target_arch = "wasm32"))]
            {
                f.recus = (f.recus.0 + list.len(), f.recus.1 + trades.len());
            }
            if !trades.is_empty() {
                f.agg.ajouter(&trades);
            }
        }
    }

    /// Flux d'une bourse connecté ou coupé : la minute en cours de ses marchés suivis devient incomplète. ⚡ When the
    /// Trades module reads this exchange, its next lot is the first one after the cut (`tape_ajouter`).
    pub(crate) fn orderflow_coupure(&mut self, ex: Exchange) {
        let now_ms = now_s() * 1000;
        for f in self.feeds.orderflow.iter_mut().filter(|f| f.cle.exchange == ex.id()) {
            f.agg.coupure(now_ms);
        }
        if self.feeds.tape_de.as_ref().is_some_and(|(de, _)| *de == ex) {
            self.feeds.tape_apres_connexion = true;
        }
    }

    /// Événement du flux d'une bourse branchée (sauf Hyperliquid).
    pub(crate) fn orderflow_evenement(&mut self, ex: Exchange, ev: LiveEvent) {
        match ev {
            LiveEvent::Whales(list) => {
                self.orderflow_trades(ex, &list);
                self.tape_ajouter(ex, list);
            }
            LiveEvent::Connected => {
                self.feeds.flux_indisponibles.remove(&ex);
                self.orderflow_coupure(ex);
            }
            LiveEvent::Disconnected(_) => self.orderflow_coupure(ex),
            // 🔑 le flux ne s'ouvre pas (jeton refusé…) : l'estimé, un message clair une fois, et un nouvel essai plus tard
            LiveEvent::Indisponible(e) => {
                self.orderflow_coupure(ex);
                if self.feeds.flux_indisponibles.insert(ex) {
                    self.banner = Some(format!("🧮 {} : flux indisponible, estimé affiché ({e})", ex.name()));
                }
            }
            _ => {}
        }
    }

    /// Le marché de la chart est-il ● enregistré en continu ? `None` : pas d'orderflow réel possible ici.
    pub(crate) fn orderflow_enregistre(&self) -> Option<bool> {
        self.orderflow_cle().map(|c| self.feeds.enregistres.contient(&c))
    }

    /// ● « Enregistrer » (module ⚡ Trades) : ajoute le marché de la chart à la liste, ou l'en retire. Ses données
    /// déjà gardées restent jusqu'aux limites du cache.
    pub(crate) fn orderflow_basculer(&mut self) {
        let Some(cle) = self.orderflow_cle() else { return };
        let nom = format!("{} ({})", cle.symbole, bourse(&cle).map_or("?", |e| e.name()));
        self.banner = Some(match self.feeds.enregistres.basculer(cle) {
            Ok(true) => format!("● {nom} enregistré en continu tant qu'Argos est ouvert (même pas affiché)"),
            Ok(false) => format!("○ {nom} n'est plus enregistré en continu (ses données gardées restent)"),
            Err(e) => format!("● {e}"),
        });
        Self::write_prefs("orderflow.json", &self.feeds.enregistres);
        self.refresh_eco();
        self.redraw();
    }
}
