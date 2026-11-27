//! 🧪 Backtest de la chart ouverte, branché sur le moteur existant `argos_core::backtest` : une stratégie
//! (`argos_script::backtest`), un sens, un stop et un objectif en %. Entrée à l'ouverture de la bougie après le
//! signal, stop et objectif vérifiés à chaque bougie, risque de 0,5 % du capital par trade, frais 0,05 %.
//! Résultat : bandeau + carte 🧪 du dock. Une simulation, pas un conseil financier.
//! 📜 Or a strategy written in Argos Script (`strategy(...)`) on the chart: its own orders, sizes and costs (lot 6).

use crate::*;
use argos_core::alea::GRAINE;
use argos_core::backtest::{contre_hasard, run, BacktestConfig, Signal, TIRAGES};
use argos_render::listmenu::{MenuGroup, MenuItem};
use argos_script::backtest::{signals, Strategy};

/// Réglages gardés entre deux lancements (`backtest.json`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct BtPrefs {
    /// Index dans `Strategy::ALL`.
    pub(crate) strategy: usize,
    /// 0 les deux, 1 long seulement, 2 short seulement.
    pub(crate) dir: u8,
    pub(crate) stop_pct: f64,
    /// 0 : pas d'objectif (sortie au stop ou à la fin).
    pub(crate) target_pct: f64,
    /// Trades dessinés sur la chart.
    pub(crate) show: bool,
    /// Frais et glissement, en % du prix, à l'entrée et à la sortie.
    pub(crate) fee_pct: f64,
    pub(crate) slip_pct: f64,
    /// 📜 The id of the chosen strategy script (empty: `strategy`).
    pub(crate) script: String,
}

impl Default for BtPrefs {
    fn default() -> Self {
        Self {
            strategy: 0,
            dir: 0,
            stop_pct: 2.0,
            target_pct: 4.0,
            show: true,
            fee_pct: 0.05,
            slip_pct: 0.0,
            script: String::new(),
        }
    }
}

const STOPS: [f64; 4] = [1.0, 2.0, 3.0, 5.0];
/// Frais et glissement proposés (% du prix, à chaque entrée et sortie).
const FEES: [f64; 4] = [0.0, 0.02, 0.05, 0.1];
const SLIPS: [f64; 4] = [0.0, 0.01, 0.05, 0.1];
const TARGETS: [f64; 4] = [0.0, 2.0, 4.0, 6.0];
/// Identifiants des lignes du menu.
const RUN: i64 = 100;
const SHOW: i64 = 40;
const CLEAR: i64 = 41;
/// 📊 The full report page.
const RAPPORT: i64 = 42;
/// 📜 The strategy scripts of the chart: `SCRIPTS` + their index in `App::scripts` (at most 100).
const SCRIPTS: i64 = 200;

/// 🔎 Argos Lens, après les stratégies d'`argos_script::backtest` (même menu, index `Strategy::ALL.len()`).
const LENS: usize = Strategy::ALL.len();
const LENS_NAME: &str = "Argos Lens (setup ≥ 70)";

impl BtPrefs {
    /// La stratégie d'argos-script choisie (`None` = Argos Lens).
    fn strategy(&self) -> Option<Strategy> {
        Strategy::ALL.get(self.strategy.min(LENS)).copied()
    }

    fn name(&self) -> &'static str {
        self.strategy().map_or(LENS_NAME, Strategy::name)
    }

    fn text(&self) -> String {
        let dir = ["long et short", "long seulement", "short seulement"][self.dir.min(2) as usize];
        let (stop, tp) = if self.strategy().is_none() {
            ("stop 1,5 ATR".to_string(), "objectif 3R".to_string())
        } else if self.target_pct > 0.0 {
            (format!("stop {} %", self.stop_pct), format!("objectif {} %", self.target_pct))
        } else {
            (format!("stop {} %", self.stop_pct), "sans objectif".into())
        };
        format!("{dir} · {stop} · {tp} · frais {} % · glissement {} %", self.fee_pct, self.slip_pct)
    }
}

/// Signaux d'Argos Lens : un setup qui apparaît = un trade, au stop (1,5 ATR) et à la cible 2 (3R) du setup.
/// Bougies refusées par Lens (prix absurdes) : aucun signal.
fn lens_signals(bars: &[Bar], tf_s: i64) -> (Vec<i8>, Vec<Option<(f64, f64)>>) {
    let photos = argos_lens::photos(bars, tf_s, &argos_lens::Config::default(), None).unwrap_or_default();
    argos_lens::nouveaux_setups(&photos)
        .into_iter()
        .map(|s| match s {
            Some(s) => {
                let [_, stop, _, tp2] = s.niveaux_f64();
                (if s.side == argos_lens::Direction::Long { 1 } else { -1 }, Some((stop, tp2)))
            }
            None => (0, None),
        })
        .unzip()
}

/// Le backtest : signaux de la stratégie sur `bars`, puis le moteur.
pub(crate) fn backtest(bars: &[Bar], tf_s: i64, p: &BtPrefs, symbol: &str) -> argos_render::dock::BtView {
    let (sig, levels) = match p.strategy() {
        Some(s) => (signals(s, bars, tf_s), vec![]),
        None => lens_signals(bars, tf_s),
    };
    let cfg = BacktestConfig {
        fee_rate: p.fee_pct.max(0.0) / 100.0,
        slippage: p.slip_pct.max(0.0) / 100.0,
        ..BacktestConfig::default()
    };
    let (stop, tp, dir) = (p.stop_pct.max(0.1) / 100.0, p.target_pct.max(0.0) / 100.0, p.dir);
    let report = run(symbol, bars, &cfg, |closed| {
        let i = closed.len().checked_sub(1)?;
        let side = match (sig.get(i).copied().unwrap_or(0), dir) {
            (1, 0 | 1) => argos_core::Side::Long,
            (-1, 0 | 2) => argos_core::Side::Short,
            _ => return None,
        };
        // Argos Lens : le stop et la cible du setup ; sinon les % des réglages
        if let Some(&Some((stop, target))) = levels.get(i) {
            return Some(Signal {
                side,
                stop,
                target: Some(target),
            });
        }
        let c = closed[i].close;
        let k = side.sign();
        Some(Signal {
            side,
            stop: c * (1.0 - k * stop),
            target: (tp > 0.0).then_some(c * (1.0 + k * tp)),
        })
    });
    // achat-conservation depuis la première bougie qui a un signal
    let first = sig.iter().position(|s| *s != 0);
    let buy_hold_pct = first
        .and_then(|i| Some((bars.get(i + 1)?.open, bars.last()?.close)))
        .filter(|(o, _)| *o > 0.0)
        .map(|(o, c)| (c / o - 1.0) * 100.0);
    let hasard = contre_hasard(bars, &cfg, &report, TIRAGES, GRAINE);
    let detail = detail_des_trades(bars, cfg.start, &report.trades);
    argos_render::dock::BtView {
        name: p.name().to_string(),
        settings: p.text(),
        frais: String::new(),
        symbol: symbol.to_string(),
        bars: bars.len(),
        start: cfg.start,
        report,
        buy_hold_pct,
        hasard,
        script: false,
        detail,
        ouverte: None,
    }
}

/// 📊 The report's data of a built-in strategy: its trades (entries named after their side, exits after their
/// reason), the account at each bar from the trades closed so far.
fn detail_des_trades(bars: &[Bar], start: f64, trades: &[argos_core::portfolio::ClosedTrade]) -> argos_render::dock::BtDetail {
    use argos_script::lang::strategie::Trade;
    let bar = |t: i64| bars.partition_point(|b| b.time < t).min(bars.len().saturating_sub(1));
    let liste: Vec<Trade> = trades
        .iter()
        .map(|t| Trade {
            entree: if t.side == argos_core::Side::Long { "Achat" } else { "Vente" }.into(),
            sortie: t.reason.clone(),
            bar_in: bar(t.opened_at),
            bar_out: bar(t.closed_at),
            ferme: t.clone(),
        })
        .collect();
    let mut fermes = trades.iter().map(|t| (t.closed_at, t.net().to_f64())).collect::<Vec<_>>();
    fermes.sort_by_key(|x| x.0);
    let (mut k, mut compte) = (0, start);
    let equite = bars
        .iter()
        .map(|b| {
            while k < fermes.len() && fermes[k].0 <= b.time {
                compte += fermes[k].1;
                k += 1;
            }
            compte
        })
        .collect();
    argos_render::dock::BtDetail {
        lignes: argos_script::lang::rapport::lignes(&liste, bars),
        equite,
        temps: bars.iter().map(|b| b.time).collect(),
        dec: argos_render::viewport::data_decimals(bars),
        en_cours: vec![],
    }
}

/// 📜 The card of a strategy written in Argos Script, from its run on the chart (`b`): its own capital, order sizes and
/// costs (its ⚙), its closed trades (like TradingView, a position still open is not in the result), against chance
/// with its commission.
pub(crate) fn backtest_script(bars: &[Bar], nom: &str, b: &argos_script::lang::strategie::Bilan, symbol: &str) -> argos_render::dock::BtView {
    vue_script(bars, nom, b, symbol, None)
}

/// [`backtest_script`]; `hasard`: against chance already drawn for the same closed trades (🎯 the volet made again at
/// each move of the last bar: its 1 000 draws are not drawn again).
fn vue_script(
    bars: &[Bar],
    nom: &str,
    b: &argos_script::lang::strategie::Bilan,
    symbol: &str,
    hasard: Option<Option<argos_core::backtest::Hasard>>,
) -> argos_render::dock::BtView {
    use argos_script::lang::strategie::Taille;
    let r = &b.reglages;
    let report = argos_core::backtest::rapport(
        r.capital,
        argos_core::Money::from_f64(r.capital) + b.net,
        b.trades.iter().map(|t| t.ferme.clone()).collect(),
        &[],
    );
    let cfg = BacktestConfig {
        start: r.capital,
        fee_rate: r.commission_pct / 100.0,
        slippage: 0.0,
        ..BacktestConfig::default()
    };
    let first = b.trades.iter().map(|t| t.bar_in).chain(b.ouverts.iter().map(|o| o.bar)).min();
    let buy_hold_pct = first
        .and_then(|i| Some((bars.get(i)?.open, bars.last()?.close)))
        .filter(|(o, _)| *o > 0.0)
        .map(|(o, c)| (c / o - 1.0) * 100.0);
    let hasard = hasard.unwrap_or_else(|| contre_hasard(bars, &cfg, &report, TIRAGES, GRAINE));
    let taille = match r.taille {
        Taille::Fixe => format!("{} contrat{}", r.valeur, if r.valeur > 1.0 { "s" } else { "" }),
        Taille::Montant => format!("{} $ par ordre", r.valeur),
        Taille::PctEquite => format!("{} % du compte", r.valeur),
    };
    // (two short rows in the card: what is left at its default is not said; pyramiding 0 and 1 both allow one entry,
    // as on TradingView, so it is said from 2 on)
    let settings = format!("{} $ · {taille}", argos_core::nombres::format_number(r.capital, 0));
    let mut frais = format!("{} %", r.commission_pct);
    if r.glissement > 0.0 {
        frais.push_str(&format!(" · glissement {} tick{}", r.glissement, if r.glissement > 1.0 { "s" } else { "" }));
    }
    if r.pyramiding > 1 {
        frais.push_str(&format!(" · pyramiding {}", r.pyramiding));
    }
    if r.a_la_cloture {
        frais.push_str(" · ordres à la clôture");
    }
    let ouverte = (!b.ouverts.is_empty()).then(|| b.ouverts.iter().map(|o| o.resultat).sum());
    argos_render::dock::BtView {
        name: format!("📜 {nom}"),
        settings,
        frais,
        symbol: symbol.to_string(),
        bars: bars.len(),
        start: r.capital,
        report,
        buy_hold_pct,
        hasard,
        script: true,
        detail: argos_render::dock::BtDetail {
            lignes: argos_script::lang::rapport::lignes(&b.trades, bars),
            equite: b.equite.clone(),
            temps: bars.iter().map(|b| b.time).collect(),
            dec: argos_render::viewport::data_decimals(bars),
            en_cours: b.ouverts.iter().filter_map(|o| Some((o.side, bars.get(o.bar)?.time, o.resultat))).collect(),
        },
        ouverte,
    }
}

/// 🎯 What the Stratégie volet's last result was made from: the same, and only its open trade has moved (the last bar):
/// against chance is not drawn again.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StratCle {
    id: String,
    symbole: String,
    unite: i64,
    bougies: usize,
    debut: i64,
    trades: usize,
    net: argos_core::Money,
    reglages: argos_script::lang::strategie::Reglages,
}

impl App {
    /// 🧪 Trades du dernier backtest à dessiner (vide si masqués, ou si la chart a changé de symbole).
    /// (📜 a strategy script draws its own fills.)
    pub(crate) fn bt_marks(&self) -> Vec<argos_render::TradeMark> {
        let Some(v) = self
            .ui
            .bt
            .as_ref()
            .filter(|v| self.ui.bt_prefs.show && !v.script && v.symbol == self.symbol_label())
        else {
            return vec![];
        };
        v.report
            .trades
            .iter()
            .map(|t| {
                let long = t.side == argos_core::Side::Long;
                argos_render::TradeMark {
                    entry_t: t.opened_at,
                    exit_t: t.closed_at,
                    long,
                    entry: t.entry,
                    exit: t.exit,
                    pct: (t.exit / t.entry - 1.0) * 100.0 * if long { 1.0 } else { -1.0 },
                }
            })
            .collect()
    }

    /// 📜 The strategies written in Argos Script on the chart: (index in `scripts`, its drawing with its run).
    fn strategies_de_la_chart(&self) -> Vec<(usize, &argos_render::scriptview::ScriptView)> {
        self.scripts
            .iter()
            .enumerate()
            .filter(|(k, s)| s.on && *k < 100)
            .filter_map(|(k, s)| {
                let v = self.ind.out.scripts.iter().find(|v| v.name == s.name && v.out.strategie.is_some())?;
                Some((k, v))
            })
            .collect()
    }

    /// 📜 The chosen strategy script, if it is on the chart.
    fn strategie_choisie(&self) -> Option<&argos_render::scriptview::ScriptView> {
        let id = &self.ui.bt_prefs.script;
        self.strategies_de_la_chart()
            .into_iter()
            .find(|(k, _)| !id.is_empty() && self.scripts[*k].id == *id)
            .map(|(_, v)| v)
    }

    /// Menu 🧪 : stratégie, sens, stop, objectif, puis « ▶️ Lancer ».
    /// 📜 With a strategy script chosen: no direction, stop, target or costs here (they are the script's, in its ⚙).
    pub(crate) fn backtest_menu(&self) -> Vec<MenuGroup> {
        let p = &self.ui.bt_prefs;
        let script = self.strategie_choisie().is_some();
        let item = |id: i64, label: String, checked: bool| MenuItem {
            id,
            label,
            enabled: true,
            checked,
            ..Default::default()
        };
        let mut g = vec![
            MenuGroup {
                title: "🧪 STRATÉGIE".into(),
                items: Strategy::ALL
                    .iter()
                    .map(|s| s.name())
                    .chain([LENS_NAME])
                    .enumerate()
                    .map(|(k, s)| item(k as i64, s.into(), !script && k == p.strategy.min(LENS)))
                    .chain(
                        self.strategies_de_la_chart()
                            .into_iter()
                            .map(|(k, v)| item(SCRIPTS + k as i64, format!("📜 {}", v.title()), script && self.scripts[k].id == p.script)),
                    )
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: "SENS".into(),
                items: ["Long et short", "Long seulement", "Short seulement"]
                    .iter()
                    .enumerate()
                    .map(|(k, l)| item(10 + k as i64, (*l).into(), k as u8 == p.dir))
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: "STOP".into(),
                items: STOPS
                    .iter()
                    .enumerate()
                    .map(|(k, v)| item(20 + k as i64, format!("{v} %"), *v == p.stop_pct))
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: "OBJECTIF".into(),
                items: TARGETS
                    .iter()
                    .enumerate()
                    .map(|(k, v)| item(30 + k as i64, if *v > 0.0 { format!("{v} %") } else { "Aucun".into() }, *v == p.target_pct))
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: "FRAIS (entrée et sortie)".into(),
                items: FEES
                    .iter()
                    .enumerate()
                    .map(|(k, v)| item(50 + k as i64, format!("{v} %"), *v == p.fee_pct))
                    .collect(),
                strip: true,
            },
            MenuGroup {
                title: "GLISSEMENT (entrée et sortie)".into(),
                items: SLIPS
                    .iter()
                    .enumerate()
                    .map(|(k, v)| item(60 + k as i64, format!("{v} %"), *v == p.slip_pct))
                    .collect(),
                strip: true,
            },
            MenuGroup {
                title: "CHART".into(),
                items: vec![
                    item(SHOW, "👁 Trades sur la chart".into(), p.show),
                    item(CLEAR, "🗑 Effacer le résultat".into(), false),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    item(RUN, format!("▶️ Lancer sur {} bougies", self.chart.bars.len()), false),
                    // 📊 after a run: the full report page
                    MenuItem {
                        enabled: self.ui.bt.is_some(),
                        ..item(RAPPORT, "📊 Rapport complet".into(), false)
                    },
                ],
                strip: false,
            },
        ];
        if script {
            // 📜 no direction, stop, target, costs or arrows here: they are the script's (its ⚙, its own arrows)
            g.retain(|g| g.title.starts_with('🧪') || g.title.is_empty());
            g.insert(
                1,
                MenuGroup {
                    title: "RÉGLAGES".into(),
                    items: vec![MenuItem {
                        id: -1,
                        label: "Capital, taille, frais : dans le ⚙ de la stratégie".into(),
                        enabled: false,
                        ..Default::default()
                    }],
                    strip: false,
                },
            );
        }
        g
    }

    /// Ligne choisie dans le menu 🧪 : `true` = le menu reste ouvert (réglage changé).
    pub(crate) fn backtest_pick(&mut self, id: i64) -> bool {
        let script = usize::try_from(id - SCRIPTS).ok().and_then(|k| self.scripts.get(k)).map(|s| s.id.clone());
        let p = &mut self.ui.bt_prefs;
        match id {
            0..=9 => {
                p.strategy = (id as usize).min(LENS);
                p.script.clear();
            }
            SCRIPTS..=299 => match script {
                Some(s) => p.script = s,
                None => return true,
            },
            10..=12 => p.dir = (id - 10) as u8,
            20..=23 => p.stop_pct = STOPS[(id - 20) as usize],
            30..=33 => p.target_pct = TARGETS[(id - 30) as usize],
            50..=53 => p.fee_pct = FEES[(id - 50) as usize],
            60..=63 => p.slip_pct = SLIPS[(id - 60) as usize],
            SHOW => p.show = !p.show,
            CLEAR => {
                self.ui.bt = None;
                self.redraw();
                return true;
            }
            RUN => {
                self.run_backtest();
                return false;
            }
            RAPPORT if self.ui.bt.is_some() => {
                // (its dates in the chart's zone, each with its summer time, like TradingView's list)
                let mut page = argos_render::btpage::BtPage::new();
                page.fuseau = self.fuseau().to_string();
                self.ui.bt_page = Some(page);
                return false;
            }
            _ => return true,
        }
        Self::write_prefs("backtest.json", &self.ui.bt_prefs);
        true
    }

    /// Lance le backtest sur la chart ouverte.
    pub(crate) fn run_backtest(&mut self) {
        if self.chart.bars.len() < 60 {
            self.banner = Some("🧪 Pas assez de bougies pour un backtest (60 au moins)".into());
            return self.redraw();
        }
        let v = if self.ui.bt_prefs.script.is_empty() {
            backtest(&self.chart.bars, self.chart.interval.secs(), &self.ui.bt_prefs, &self.symbol_label())
        } else {
            // 📜 the strategy script, as it ran on the chart (with the values of its ⚙)
            let Some((nom, b)) = self.strategie_choisie().and_then(|v| Some((v.title().to_string(), v.out.strategie.as_ref()?))) else {
                self.banner = Some("🧪 Ta stratégie n'est plus sur la chart : ajoute-la, puis relance".into());
                return self.redraw();
            };
            backtest_script(&self.chart.bars, &nom, b, &self.symbol_label())
        };
        self.banner = Some(format!(
            "{} · 📊 rapport complet dans le menu 🧪 · simulation, pas un conseil financier",
            argos_render::dock::bt_summary(&v)
        ));
        self.ui.bt = Some(v);
        self.redraw();
    }
}

impl App {
    /// 🎯 After each run of the chart's scripts: the Stratégie volet follows the strategies on the chart (its 🎯 in the
    /// bar while there is one), its result made again by itself (no « ▶️ Lancer »); a strategy added to the chart opens
    /// the volet on it (🖥, once). The report page and the 🧪 card showing the same strategy follow too.
    pub(crate) fn strategie_a_jour(&mut self) {
        let liste: Vec<(String, String)> = self
            .strategies_de_la_chart()
            .into_iter()
            .map(|(k, v)| (self.scripts[k].id.clone(), v.title().to_string()))
            .collect();
        // (what is on the chart when the app starts was not « added »)
        if self.ui.strat_vus.is_none() {
            self.ui.strat_vus = Some(self.scripts.iter().filter(|s| s.on).map(|s| s.id.clone()).collect());
        }
        if liste.is_empty() {
            let vol = &mut self.watch.strategie;
            if !vol.noms.is_empty() || vol.vue.is_some() {
                (vol.noms, vol.vue, vol.tv) = (vec![], None, None);
                self.ui.strat_cle = None;
                if self.watch.prefs.panel == wl::Tool::Strategie {
                    (self.watch.prefs.open, self.watch.prefs.panel) = (false, wl::Tool::Watchlist);
                }
            }
            return;
        }
        let vus = self.ui.strat_vus.get_or_insert_with(Default::default);
        let nouveau = liste.iter().find(|(id, _)| !vus.contains(id)).map(|(id, _)| id.clone());
        vus.extend(liste.iter().map(|(id, _)| id.clone()));
        // the chosen one: the one just added, else the volet's, else the 🧪 menu's, else the first
        let id = nouveau
            .clone()
            .or_else(|| Some(self.watch.strategie.id.clone()).filter(|i| liste.iter().any(|(x, _)| x == i)))
            .or_else(|| Some(self.ui.bt_prefs.script.clone()).filter(|i| liste.iter().any(|(x, _)| x == i)))
            .unwrap_or_else(|| liste[0].0.clone());
        if nouveau.is_some() && !self.top.phone {
            self.ui.strat_ouvrir = true;
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
        let (fuseau, tz, unite) = (self.fuseau().to_string(), self.tz_offset_s(), self.chart.interval.short());
        let vol = &mut self.watch.strategie;
        vol.choisie = liste.iter().position(|(i, _)| *i == id).unwrap_or(0);
        vol.noms = liste.into_iter().map(|(_, n)| n).collect();
        (vol.id, vol.fuseau, vol.tz, vol.unite) = (id.clone(), fuseau, tz, unite);
        let symbole = self.symbol_label();
        let bars = &self.chart.bars;
        let Some((nom, b)) = self
            .strategies_de_la_chart()
            .into_iter()
            .find(|(k, _)| self.scripts[*k].id == id)
            .and_then(|(_, v)| Some((v.title().to_string(), v.out.strategie.as_ref()?)))
        else {
            return;
        };
        let cle = StratCle {
            id,
            symbole,
            unite: self.chart.interval.secs(),
            bougies: bars.len(),
            debut: bars.first().map_or(0, |b| b.time),
            trades: b.trades.len(),
            net: b.net,
            reglages: b.reglages.clone(),
        };
        let avant = self.ui.strat_cle.as_ref();
        // (only the open trade moved: against chance as it was)
        let hasard = (avant == Some(&cle)).then(|| self.watch.strategie.vue.as_ref().and_then(|v| v.hasard.clone()));
        // (another strategy, symbol, timeframe or settings: TradingView's list was another one's)
        let autre = avant.is_none_or(|a| (&a.id, &a.symbole, a.unite, &a.reglages) != (&cle.id, &cle.symbole, cle.unite, &cle.reglages));
        let vue = vue_script(bars, &nom, b, &cle.symbole, hasard);
        if autre {
            self.watch.strategie.tv = None;
        }
        if self.ui.bt.as_ref().is_some_and(|v| v.script && v.name == vue.name && v.symbol == vue.symbol) {
            self.ui.bt = Some(vue.clone());
        }
        self.watch.strategie.vue = Some(vue);
        self.ui.strat_cle = Some(cle);
    }

    /// 🎯 The volet asked to open by a strategy just added (🖥): open at the start of the frame, before the layout.
    pub(crate) fn strategie_ouvrir(&mut self) {
        if std::mem::take(&mut self.ui.strat_ouvrir) && !self.watch.strategie.noms.is_empty() {
            (self.watch.prefs.open, self.watch.prefs.panel) = (true, wl::Tool::Strategie);
            self.watch.haut_du_volet();
        }
    }

    /// 🎯 What the Stratégie volet asks.
    pub(crate) fn strategie_click(&mut self, c: argos_render::strategie::StratClick) {
        use argos_render::strategie::StratClick as S;
        match c {
            S::Rapport => self.strategie_rapport(),
            S::Tv => {
                self.strategie_rapport();
                if self.ui.bt_page.is_some() {
                    self.rapport_tv();
                }
            }
            S::Reglages => {
                let id = self.watch.strategie.id.clone();
                let slot = self.scripts.iter().position(|s| s.id == id && s.on);
                let k = slot.and_then(|s| {
                    self.ind
                        .out
                        .scripts
                        .iter()
                        .position(|v| v.name == self.scripts[s].name && v.out.strategie.is_some())
                });
                if let (Some(slot), Some(k)) = (slot, k) {
                    self.reglages_du_script(slot, k);
                }
            }
            S::Choisir(r) => {
                let r = match self.volet_rect() {
                    Some(v) => Rect::new(v.x + r.x, v.y + r.y, r.w, r.h),
                    None => self.side_rect(r),
                };
                self.open_list(ListKind::Strategies, r);
            }
            S::None | S::Changed | S::Fermer => {}
        }
    }

    /// 🎯 📊 The full report page of the volet's result (on its open tab; ⚖️ with TradingView's list already read).
    fn strategie_rapport(&mut self) {
        let Some(v) = self.watch.strategie.vue.clone() else {
            self.banner = Some("🎯 La stratégie se calcule encore : un instant".into());
            return;
        };
        self.ui.bt = Some(v);
        let mut page = argos_render::btpage::BtPage::new();
        page.fuseau = self.fuseau().to_string();
        page.tv = self.watch.strategie.tv.clone();
        page.tab = match self.watch.strategie.onglet {
            2 if page.tv.is_none() => 0,
            k => k.min(2),
        };
        self.ui.bt_page = Some(page);
    }

    /// 🎯 📜 ▾ The strategies on the chart (the volet's pill).
    pub(crate) fn strategies_menu(&self) -> Vec<MenuGroup> {
        let id = &self.watch.strategie.id;
        vec![MenuGroup {
            title: "STRATÉGIE DU VOLET".into(),
            items: self
                .strategies_de_la_chart()
                .into_iter()
                .enumerate()
                .map(|(i, (k, v))| MenuItem {
                    id: i as i64,
                    label: format!("📜 {}", v.title()),
                    enabled: true,
                    checked: self.scripts[k].id == *id,
                    ..Default::default()
                })
                .collect(),
            strip: false,
        }]
    }

    /// 🎯 A strategy chosen for the volet: its result now.
    pub(crate) fn strategie_pick(&mut self, i: i64) {
        let id = usize::try_from(i)
            .ok()
            .and_then(|i| self.strategies_de_la_chart().get(i).map(|(k, _)| self.scripts[*k].id.clone()));
        if let Some(id) = id {
            self.watch.strategie.id = id;
            self.watch.strategie.onglet = self.watch.strategie.onglet.min(1);
            self.ui.strat_cle = None;
            self.strategie_a_jour();
            self.watch.haut_du_volet();
        }
        self.redraw();
    }
}

impl App {
    /// 📊 A click in the report page.
    pub(crate) fn rapport_click(&mut self) {
        use argos_render::btpage::BtClick;
        let Some(p) = self.ui.bt_page.as_mut() else { return };
        let size = self.window.as_deref().map(ui_size).unwrap_or_default();
        let (x, y) = self.pointer.win_mouse;
        match p.click(&argos_render::text::Text::default(), size.width as f64, size.height as f64, x, y) {
            BtClick::Close => self.ui.bt_page = None,
            BtClick::Csv => self.rapport_csv(),
            BtClick::Tv => self.rapport_tv(),
            BtClick::Changed | BtClick::None => {}
        }
    }

    /// 📊 Wheel or finger on the report page (`dy` > 0: down).
    pub(crate) fn rapport_molette(&mut self, dy: f64) {
        let (tz, size) = (self.tz_offset_s(), self.window.as_deref().map(ui_size).unwrap_or_default());
        let (Some(p), Some(v)) = (self.ui.bt_page.as_mut(), self.ui.bt.as_ref()) else {
            return;
        };
        let n = p.lignes(v, tz);
        p.wheel(dy, &argos_render::text::Text::default(), size.width as f64, size.height as f64, n);
    }

    /// 📊 ⬇️ Export CSV: the list of trades in TradingView's columns (« List of trades »), in the Downloads.
    pub(crate) fn rapport_csv(&mut self) {
        let Some(v) = self.ui.bt.as_ref() else { return };
        let csv = argos_script::lang::rapport::csv(&v.detail.lignes, v.start, "USD", self.tz_offset_s());
        // (letters and digits only: « 📜 Exemple EMA 9 / 21 » → « exemple-ema-9-21 »)
        let propre = |s: &str| {
            s.split(|c: char| !c.is_alphanumeric())
                .filter(|m| !m.is_empty())
                .collect::<Vec<_>>()
                .join("-")
                .to_lowercase()
        };
        let name = format!("argos-backtest-{}-{}.csv", propre(&v.name), propre(&v.symbol));
        self.banner = Some(match platform::save_download(&name, csv.as_bytes()) {
            Ok(dir) => format!("💾 {name} enregistré dans {dir}"),
            Err(e) => format!("Écriture impossible ({e})"),
        });
    }

    /// ⚖️ ⬆ Comparer avec TradingView: the page opens its file picker; the desktop app takes the latest CSV of the
    /// Downloads (not one Argos wrote).
    pub(crate) fn rapport_tv(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.ui.bt_tv_choix = true;
            let _ = platform::web::take_picked();
            platform::web::call_page("argosPick", &[".csv,text/csv,text/plain"]);
            self.banner = Some(format!("📂 Choisis {TV_FICHIER}"));
        }
        #[cfg(not(target_arch = "wasm32"))]
        match platform::dernier_csv_telecharge() {
            Some((nom, octets)) => self.rapport_tv_lire(&nom, &octets),
            None => self.banner = Some(format!("⚠️ Aucun fichier CSV dans tes Téléchargements : exporte {TV_FICHIER}")),
        }
    }

    /// ⚖️ TradingView's file read and compared with the backtest: the ⚖️ TradingView tab opens on it.
    pub(crate) fn rapport_tv_lire(&mut self, nom: &str, octets: &[u8]) {
        let (tz, tf, fuseau) = (self.tz_offset_s(), self.chart.interval.secs(), self.fuseau().to_string());
        // (the chart's zone with its summer time: TradingView writes the file in its chart's zone)
        let zone = move |t: i64| argos_render::settings::tz_offset(&fuseau, t).unwrap_or(tz);
        let (Some(v), Some(page)) = (self.ui.bt.as_ref(), self.ui.bt_page.as_mut()) else {
            return;
        };
        let liste = match argos_script::lang::comparer::lire_csv(&String::from_utf8_lossy(octets)) {
            Ok(l) => l,
            Err(e) => {
                self.banner = Some(format!("⚠️ {nom} : {e}"));
                return self.redraw();
            }
        };
        let tf = pas_des_barres(&v.detail.temps).unwrap_or(tf);
        let tick = 10f64.powi(-(v.detail.dec as i32));
        let c = argos_script::lang::comparer::comparer(&v.detail.lignes, &liste, &v.detail.temps, tf, &zone, tick);
        self.banner = Some(format!(
            "⚖️ TradingView : {} pareil{}, {} différent{} sur {}",
            c.pareils,
            if c.pareils > 1 { "s" } else { "" },
            c.differents,
            if c.differents > 1 { "s" } else { "" },
            c.rangees.len()
        ));
        let vue = argos_render::btpage::VueTv { fichier: nom.to_string(), c };
        // 🎯 the same result as the Stratégie volet's: its ⚖️ tab shows the list too
        let meme = self.watch.strategie.vue.as_ref().is_some_and(|w| w.name == v.name && w.symbol == v.symbol);
        page.montrer_tv(vue.clone());
        if meme {
            (self.watch.strategie.tv, self.watch.strategie.onglet) = (Some(vue), 2);
        }
        self.redraw();
    }

    /// ⚖️ On each loop wait, in the page: TradingView's file chosen. `true`: still waiting (fast wake-up).
    pub(crate) fn rapport_tv_tick(&mut self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            if !self.ui.bt_tv_choix {
                return false;
            }
            if self.ui.bt_page.is_none() {
                self.ui.bt_tv_choix = false;
                return false;
            }
            let Some((nom, b64)) = platform::web::take_picked().pop() else { return true };
            self.ui.bt_tv_choix = false;
            match argos_core::retours::base64_lire(&b64) {
                Some(o) => self.rapport_tv_lire(&nom, &o),
                None => self.banner = Some(format!("⚠️ {nom} : fichier illisible")),
            }
            false
        }
        #[cfg(not(target_arch = "wasm32"))]
        false
    }
}

/// ⚖️ What to export from TradingView, said in the banner.
const TV_FICHIER: &str = "la liste des trades de TradingView (Testeur de stratégie › Liste des transactions › export CSV)";

/// The bars' step (seconds): the smallest gap between two of the first ones (a gap of the market is longer).
fn pas_des_barres(temps: &[i64]) -> Option<i64> {
    temps.windows(2).take(200).map(|w| w[1] - w[0]).filter(|d| *d > 0).min()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 📜 The card of a strategy stays short: what is at its default is not said (pyramiding 0 or 1, no slippage),
    /// the rest is; a position still open at the last bar has its own row, not in the result (as TradingView).
    #[test]
    fn carte_d_une_strategie_courte_et_position_ouverte() {
        let bars: Vec<Bar> = (0..60)
            .map(|i| Bar {
                time: i * 3600,
                open: 100.0 + i as f64,
                high: 101.0 + i as f64,
                low: 99.0 + i as f64,
                close: 100.5 + i as f64,
                volume: 1.0,
            })
            .collect();
        let carte = |entete: &str| {
            let src = format!("//@version=6\n{entete}\nif bar_index == 10\n    strategy.entry(\"L\", strategy.long)\n");
            let o = argos_script::lang::run(&src, &bars, 3600, &Default::default()).expect("compiles");
            backtest_script(&bars, "T", o.strategie.as_ref().expect("a strategy"), "BTC")
        };
        let capital = argos_core::nombres::format_number(10_000.0, 0);
        let v = carte("strategy(\"T\", initial_capital = 10000, pyramiding = 1, commission_value = 0.05)");
        assert_eq!((v.settings.as_str(), v.frais.as_str()), (format!("{capital} $ · 1 contrat").as_str(), "0.05 %"));
        let v = carte("strategy(\"T\", initial_capital = 10000, pyramiding = 3, slippage = 2)");
        assert_eq!(v.frais, "0 % · glissement 2 ticks · pyramiding 3");
        let rows = argos_render::dock::bt_rows(&v, 0);
        let noms: Vec<&str> = rows.iter().take(5).map(|r| r.0.as_str()).collect();
        assert_eq!(noms, ["Stratégie", "Capital · taille", "Frais", "Symbole · bougies", "Position ouverte"]);
        // bought at bar 11's open (111 + 2 ticks), still open at the last close: its own row, out of the result
        let ouverte = v.ouverte.expect("a position still open");
        assert!(ouverte > 0.0 && v.report.trades.is_empty(), "{ouverte}");
        let ligne = rows.iter().find(|r| r.0 == "Position ouverte").expect("its row");
        assert_eq!(ligne.1, format!("{ouverte:+.2} $ · hors résultat"));
        assert!(
            rows.len() as f64 * argos_render::dock::KV_ROW <= argos_render::dock::bt_h(),
            "every row fits the card"
        );
        assert_eq!(v.detail.dec, 1, "the bars' own decimals for the report's prices");
    }

    #[test]
    fn backtest_sur_tendance_et_reglages() {
        // hausse puis baisse en dents de scie : il y a des trades, et le rapport est cohérent
        let bars: Vec<Bar> = (0..400)
            .map(|i| {
                let c = 100.0 + (i as f64 * 0.08).sin() * 20.0 + i as f64 * 0.05;
                Bar {
                    time: i * 3600,
                    open: c,
                    high: c + 1.5,
                    low: c - 1.5,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect();
        let p = BtPrefs::default();
        let v = backtest(&bars, 3600, &p, "BTC");
        let r = &v.report;
        assert!(!r.trades.is_empty());
        assert!((0.0..=100.0).contains(&r.win_rate));
        assert_eq!(r.end_balance, argos_core::Money::from_f64(v.start) + r.net_pnl, "au centime près, exactement");
        // 📊 the report page's data: every trade in the list, the account at each bar ending at the final balance
        assert_eq!(v.detail.lignes.len(), r.trades.len());
        assert_eq!((v.detail.equite.len(), v.detail.temps.len()), (bars.len(), bars.len()));
        assert!((v.detail.equite.last().unwrap() - r.end_balance.to_f64()).abs() < 1e-6);
        assert_eq!(v.detail.lignes.last().unwrap().cumul, r.net_pnl);
        assert!(v.detail.lignes.iter().all(|l| l.bar_out >= l.bar_in && l.entree.len() > 2));
        assert_eq!(v.settings, "long et short · stop 2 % · objectif 4 % · frais 0.05 % · glissement 0 %");
        // frais et glissement en plus : moins bien, jamais mieux ; R moyen cohérent avec le signe du résultat
        let costly = backtest(
            &bars,
            3600,
            &BtPrefs {
                fee_pct: 0.1,
                slip_pct: 0.1,
                ..p.clone()
            },
            "BTC",
        );
        assert!(costly.report.net_pnl < r.net_pnl);
        assert!(r.avg_r.is_finite());
        assert_eq!(
            r.avg_r > 0.0,
            r.trades.iter().map(|t| t.pnl - t.fees).sum::<argos_core::Money>() > argos_core::Money::ZERO
        );
        // long seulement : aucun short
        let long = backtest(&bars, 3600, &BtPrefs { dir: 1, ..p.clone() }, "BTC");
        assert!(long.report.trades.iter().all(|t| t.side == argos_core::Side::Long));
        let rows = argos_render::dock::bt_rows(&v, 0);
        assert_eq!(rows[0].1, "Croisement EMA 9 / 21");
        // 🎲 contre le hasard : 1 000 tirages, même résultat à chaque fois (graine fixe), une ligne dans la carte
        let h = v.hasard.as_ref().expect("des trades : un test contre le hasard");
        assert_eq!(h.tirages, TIRAGES);
        assert!((1..=TIRAGES + 1).contains(&h.rang), "rang de 1 à tirages + 1");
        assert_eq!(backtest(&bars, 3600, &p, "BTC").hasard.as_ref(), Some(h), "déterministe");
        let ligne = rows.iter().find(|r| r.0 == "🎲 Contre le hasard").expect("ligne 🎲");
        assert_eq!(ligne.1, format!("mieux que {:.0} % des tirages", h.battus_pct));
        assert!(argos_render::dock::bt_summary(&v).contains("trades"));
    }

    /// 🔎 Argos Lens dans la carte 🧪 : un trade par setup qui apparaît, au stop et à la cible 2 du setup ; contre le
    /// hasard comme les autres stratégies.
    #[test]
    fn backtest_argos_lens() {
        let bars: Vec<Bar> = (0..600)
            .map(|i| {
                let c = 200.0 + (i as f64 / 40.0).sin() * 30.0 + (i as f64 / 3.0).sin() * 1.5;
                Bar {
                    time: 1_699_920_000 + i * 3600,
                    open: c - 0.3,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 100.0 + (i % 7) as f64 * 15.0,
                }
            })
            .collect();
        let p = BtPrefs {
            strategy: LENS,
            ..BtPrefs::default()
        };
        let v = backtest(&bars, 3600, &p, "XAUUSD");
        assert_eq!(v.name, LENS_NAME);
        assert_eq!(v.settings, "long et short · stop 1,5 ATR · objectif 3R · frais 0.05 % · glissement 0 %");
        let photos = argos_lens::photos(&bars, 3600, &argos_lens::Config::default(), None).unwrap();
        let setups: Vec<_> = argos_lens::nouveaux_setups(&photos).into_iter().flatten().collect();
        assert!(!setups.is_empty() && !v.report.trades.is_empty());
        assert!(v.report.trades.len() <= setups.len(), "au plus un trade par setup (une position à la fois)");
        assert!(v.hasard.is_some(), "contre le hasard aussi");
        assert_eq!(backtest(&bars, 3600, &p, "XAUUSD").report, v.report, "déterministe");
        // le menu propose Lens après les stratégies d'argos-script ; un index trop grand retombe sur Lens
        assert_eq!(BtPrefs { strategy: 99, ..p }.name(), LENS_NAME);
        assert_eq!(BtPrefs::default().name(), Strategy::ALL[0].name());
    }
}
