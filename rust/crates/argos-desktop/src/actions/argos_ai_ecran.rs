//! 🤖 Argos AI level 1 on screen (« ok Argos AI », Jee 18 Nov): the « règles » mode, without network or Claude key.
//! The « Explique » pill of the Argos Lens card always opens it; without a Claude key, the corner pill, the legends' ✨
//! and the phone pill's Argos AI open it too (with a key, they keep talking to Claude as before). The text comes from
//! `argos_lens::explain::explain` on the last closed bar (the same snapshot and settings as the Lens card); its
//! drawings, checked against the visible window by `argos_lens::explain::check`, are kept by the app apart from the
//! user's drawings — never saved, never selectable — and go when the symbol or the interval changes, or with
//! « Effacer l'explication ».

use crate::*;
use argos_ai::DayLevels;
use argos_lens::explain::{self, Command, Window};
use argos_lens::Decimal;
use argos_render::assistant::KeyState;
use argos_render::indicators::Ind;

/// 🤖 What the « règles » mode's lines at the top of the panel read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ensemble {
    /// « ✨ Lire tous les indicateurs » (idea 1).
    Tout,
    /// « ✨ Accord des unités de temps » (idea 2).
    Accord,
    /// « ✨ Volatilité du moment » (idea 4).
    Volatilite,
    /// « ✨ Niveaux du jour » (idea 5).
    NiveauxJour,
}

/// 🤖 The explanation drawn on the chart: for which symbol and interval, and its checked drawings.
pub(crate) type Explication = (String, i64, Vec<Command>);

/// 🤖 What a reading needs from the screen that is computed for it (owned, borrowed by `App::donnees_ecran`): the
/// order-book walls, computed as the chart draws them, and the Anchored VWAP's source when it is another indicator's
/// output. `None` when the screen has none (or the indicator does not need it).
#[derive(Debug, Clone, Default)]
pub(crate) struct Calculs {
    murs: Option<Vec<argos_ai::Wall>>,
    source: Option<Vec<f64>>,
}

impl App {
    /// 🤖 No Claude key on this device: Argos AI answers with its rules.
    pub(crate) fn sans_cle_claude(&self) -> bool {
        self.ai.key == KeyState::None
    }

    /// 🤖 Opens the Argos AI panel in « règles » mode and explains the last closed bar.
    pub(crate) fn expliquer_regles(&mut self) {
        if !self.ui.ai_open {
            self.ouvrir_panneau_ai();
        }
        self.regles_expliquer();
        self.redraw();
    }

    /// 🤖 The legend's ✨ of an indicator without a Claude key (« ok lot 1 », Jee 18 Nov): the panel in « règles »
    /// mode with the indicator's name, then the reading of `argos_ai::read_with` on the closed bars and the screen's data (with rules: what it
    /// shows now, how to read it, what would change the reading; else its settings only), its settings with their
    /// label, help and role, and the reminder at the bottom — never another card.
    pub(crate) fn regles_indicateur(&mut self, i: Ind) {
        if !self.ui.ai_open {
            self.ouvrir_panneau_ai();
        }
        self.ai.regles = true;
        let texte = self.lecture_indicateur(i, false);
        self.ai.msgs.push((false, texte));
        self.ai.scroll = 0.0;
        self.redraw();
    }

    /// 🤖 « ＋ N autres réglages utiles »: the reading shown last, again, with all the settings its rules read.
    pub(crate) fn deplier_reglages(&mut self) {
        let Some(i) = self.ai.tous_reglages else { return };
        let texte = self.lecture_indicateur(i, true);
        let titre = format!("📏 {}\n", i.name());
        match self.ai.msgs.iter().rposition(|m| !m.0 && m.1.starts_with(&titre)) {
            Some(k) => self.ai.msgs[k].1 = texte,
            None => self.ai.msgs.push((false, texte)),
        }
        self.redraw();
    }

    /// 🤖 The text of an indicator's reading. Under a reading by rules, its settings on two levels (Jee 18 Nov,
    /// « Ok reglage court »): first those the reading names (a role), then — `tout`, or by « ＋ N autres réglages
    /// utiles » — the other keys its rules read; « ⚙ Tous les réglages » opens the ⚙. Settings only: all of them.
    fn lecture_indicateur(&mut self, i: Ind, tout: bool) -> String {
        self.lecture_par_id(i, &id_catalogue(i), tout)
    }

    /// 🤖 The reading of `i` (its name, its settings) by `argos_ai` under the id `id` — `id_catalogue(i)` on the
    /// screen; the tests also give an id argos_ai does not read, for « réglages seulement » whatever Lens reads.
    pub(crate) fn lecture_par_id(&mut self, i: Ind, id: &str, tout: bool) -> String {
        let (groupes, params) = i.params();
        let reglages = self.reglages_par_nom(i);
        let par_nom: Vec<(&str, f64)> = reglages.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        let lecture = self.lecture_ecran(i, id, &par_nom);
        let mut texte = format!("📏 {}\n", i.name());
        for b in &lecture.blocks {
            texte.push_str(&format!("\n{}\n", b.title));
            for s in &b.sentences {
                texte.push_str(&format!("• {}\n", s.text));
            }
        }
        texte.push_str("\nRéglages\n");
        if lecture.settings.is_empty() {
            texte.push_str("• (aucun réglage)\n");
        }
        // (the style settings — colours, line width and type — say nothing about the reading: in ⚙ only)
        let style =
            |p: &argos_script::inputs::Param| matches!(p.kind, argos_script::inputs::Kind::Color) || groupes.get(p.group).is_some_and(|g| g.contains("Style"));
        // (« Ok reglage court », Jee 18 Nov: under a reading by rules, only the settings the rules read, in the order
        // of the reading — the « ⚙ Tous les réglages » button opens the others; settings only: all of them)
        let lus = argos_ai::keys_read(id);
        let par_regles = lecture.coverage == argos_ai::Coverage::Rules && !lus.is_empty();
        let utiles: Vec<&argos_ai::Setting> = lus.iter().filter_map(|k| lecture.settings.iter().find(|r| r.key == *k)).collect();
        let montres: Vec<&argos_ai::Setting> = if !par_regles {
            lecture.settings.iter().collect()
        } else if tout {
            utiles.clone()
        } else {
            utiles.iter().copied().filter(|r| r.role.is_some()).collect()
        };
        self.ai.tous_reglages = par_regles.then_some(i);
        self.ai.autres_reglages = if par_regles { utiles.len() - montres.len() } else { 0 };
        for r in montres {
            let k = reglages.iter().position(|(n, _)| *n == r.key);
            let p = k.and_then(|k| params.get(k));
            if !par_regles && r.role.is_none() && p.is_some_and(style) {
                continue;
            }
            let v = valeur_lisible(p, r.value);
            texte.push_str(&format!("• {} : {v}", p.map_or(r.key.as_str(), |p| p.label)));
            if let Some(role) = r.role {
                texte.push_str(&format!(" ({role})"));
            }
            if let Some(tip) = p.map(|p| p.tip).filter(|t| !t.is_empty()) {
                texte.push_str(&format!(" — {}", tip.replace('\n', " ")));
            }
            texte.push('\n');
        }
        texte.push_str(&format!("\n{}", lecture.reminder));
        texte
    }

    /// 🤖 The reading of `i` under the id `id` with its `reglages`, by `argos_ai::read_with` on the closed bars and what
    /// the screen holds besides them (`donnees_ecran`).
    pub(crate) fn lecture_ecran(&self, i: Ind, id: &str, reglages: &[(&str, f64)]) -> argos_ai::Reading {
        let c = self.calculs_ecran(i);
        argos_ai::read_with(id, self.bougies_fermees(), reglages, &self.donnees_ecran(&c))
    }

    /// 🤖 What `i`'s reading needs computed from the screen: the book walls for the heatmap, the external source for
    /// the Anchored VWAP (nothing for the others).
    pub(crate) fn calculs_ecran(&self, i: Ind) -> Calculs {
        Calculs {
            murs: (i == Ind::BookHeat).then(|| self.murs_du_carnet()).flatten(),
            source: (i == Ind::AnchoredVwap).then(|| self.source_externe()).flatten(),
        }
    }

    /// 🤖 What the screen holds besides the closed bars, for `argos_ai::read_with`: the chart's unit and the time, the
    /// real orderflow of the closed bars (`IndOut::reel`, empty: the chart estimates), AI Trend's smaller-unit bars,
    /// the tape's live state, the liquidation map when shown, and `c` (walls, external source).
    pub(crate) fn donnees_ecran<'a>(&'a self, c: &'a Calculs) -> argos_ai::ScreenData<'a> {
        let derniere = self.bougies_fermees().last().map_or(i64::MIN, |b| b.time);
        let reel = &self.ind.out.reel;
        argos_ai::ScreenData {
            bar_secs: Some(self.chart.interval.secs()),
            now: Some(now_s()),
            flow: Some(&reel[..reel.partition_point(|r| r.time <= derniere)]),
            lower: Some(&self.feeds.mtf_bars),
            ext_source: c.source.as_deref(),
            tape: self.ind.out.tape.as_ref(),
            walls: c.murs.as_deref(),
            liq: self.liq_on().then_some(self.feeds.liq_view.as_ref()).flatten().map(|l| argos_ai::LiqData {
                profile: &l.profile,
                notes: &l.notes,
            }),
        }
    }

    /// 🤖 The order-book walls as the chart draws them (argos-render, « 🧱 order-book walls »): middle price the last
    /// bar's close, the heatmap's « wallsN », « wallsMult », « wallsDepth ». `None` without a heatmap on screen.
    fn murs_du_carnet(&self) -> Option<Vec<argos_ai::Wall>> {
        let h = self.heat_on().then_some(self.feeds.heat.as_ref()).flatten()?;
        let mid = self.chart.bars.last()?.close;
        let v = argos_render::indicators::values(&self.ind.out.inputs, Ind::BookHeat);
        let p = argos_script::inputs::Inputs {
            params: &argos_render::bookheat::PARAMS,
            vals: &v,
        };
        Some(
            h.walls(mid, p.get("wallsN").max(1.0) as usize, p.get("wallsMult"), p.get("wallsDepth"))
                .into_iter()
                .map(|w| argos_ai::Wall {
                    price: w.price,
                    usd: w.usd,
                    sell: w.sell,
                })
                .collect(),
        )
    }

    /// 🤖 The Anchored VWAP's source when its setting points at another indicator's output: the same series the chart
    /// computes it with (`output_series`), cut to the closed bars. `None` for a price source or an output not computed.
    fn source_externe(&self) -> Option<Vec<f64>> {
        let v = argos_render::indicators::values(&self.ind.out.inputs, Ind::AnchoredVwap);
        let p = argos_script::inputs::Inputs {
            params: Ind::AnchoredVwap.params().1,
            vals: &v,
        };
        let argos_script::inputs::Src::Ext(k, o) = p.source("src") else {
            return None;
        };
        let mut serie = argos_render::indicators::output_series(&self.ind.out, *Ind::ALL.get(k)?, o)?;
        serie.truncate(self.bougies_fermees().len());
        Some(serie)
    }

    /// 🤖 An indicator's settings by name (`key`, or `key@k` for a key already taken), as `argos_ai::read` takes them.
    /// 🎨 Never the ⚙ Style of a built-in written in Argos Script (colour, width, line type of its plots): it says
    /// nothing about the reading, and Argos AI does not see it.
    pub(crate) fn reglages_par_nom(&self, i: Ind) -> Vec<(String, f64)> {
        let vals = argos_render::indicators::values(&self.ind.out.inputs, i);
        let fiche = argos_render::catalogue::fiche(i);
        let mut reglages: Vec<(String, f64)> = vec![];
        for (k, p) in i.params().1.iter().enumerate().filter(|(k, _)| !argos_script::catalogue::est_style(fiche, *k)) {
            let v = vals.get(k).copied().unwrap_or(p.def);
            reglages.push((nom_reglage(&reglages, p.key, k), v));
        }
        reglages
    }

    /// 🤖 The chart's closed bars (the bar under way is never read).
    pub(crate) fn bougies_fermees(&self) -> &[argos_core::Bar] {
        let tf = self.chart.interval.secs();
        let n = self.chart.bars.partition_point(|b| b.time.saturating_add(tf) <= now_s());
        &self.chart.bars[..n]
    }

    /// 🤖 Lot 7 : ce que disaient les indicateurs affichés sur la chart à l'ouverture d'une position démo sur `base` (à
    /// l'achat si `long`), rangés par rapport à son sens par `argos_ai::read_for_trade` (le moteur de « ✨ Lire tous les
    /// indicateurs »), sur les bougies clôturées (`avant` : seulement celles d'avant cette heure, quand un ordre en
    /// attente part sur une bougie du replay). En direct, avec ce que l'écran a pour chacun ; en replay, les bougies
    /// seules (le carnet, la tape, la carte des liquidations sont ceux du direct). La chart montre un autre symbole :
    /// rien n'est lu, et la carte du trade le dit.
    pub(crate) fn lecture_a_l_entree(&self, base: &str, long: bool, avant: Option<i64>) -> argos_core::portfolio::Lecture {
        let sym = self.symbol_label();
        if sym != base {
            return argos_core::portfolio::Lecture {
                pas_lue: format!("la chart montrait {sym}"),
                ..Default::default()
            };
        }
        let affiches = self.indicateurs_affiches();
        let reglages: Vec<Vec<(&str, f64)>> = affiches.iter().map(|(_, _, _, r)| r.iter().map(|(n, v)| (n.as_str(), *v)).collect()).collect();
        let items: Vec<argos_ai::Item> = affiches
            .iter()
            .zip(&reglages)
            .map(|((_, id, name, _), settings)| argos_ai::Item { id, name, settings })
            .collect();
        let fermees = self.bougies_fermees();
        let bars = match avant {
            Some(t) => &fermees[..fermees.partition_point(|b| b.time < t)],
            None => fermees,
        };
        let calculs: Vec<Calculs> = affiches.iter().map(|(i, ..)| self.calculs_ecran(*i)).collect();
        let tf = self.chart.interval.secs();
        let donnees: Vec<argos_ai::ScreenData> = calculs
            .iter()
            .map(|c| {
                if self.replay.is_some() {
                    argos_ai::ScreenData {
                        bar_secs: Some(tf),
                        now: bars.last().map(|b| b.time + tf),
                        ext_source: c.source.as_deref().map(|s| &s[..s.len().min(bars.len())]),
                        ..Default::default()
                    }
                } else {
                    self.donnees_ecran(c)
                }
            })
            .collect();
        argos_ai::read_for_trade(&items, bars, &donnees, long)
    }

    /// 🤖 The indicators shown on the chart (not hidden by 👁), in the panel's order: (indicator, catalogue id, name,
    /// settings).
    fn indicateurs_affiches(&self) -> Vec<(Ind, String, String, Vec<(String, f64)>)> {
        self.ind
            .out
            .on
            .iter()
            .filter(|i| !self.ind.out.hidden.contains(i))
            .map(|&i| (i, id_catalogue(i), i.name().to_string(), self.reglages_par_nom(i)))
            .collect()
    }

    /// 🤖 « ✨ Lire tous les indicateurs » (idea 1, « ok fit en ordre 1 a 5 », Jee 18 Nov) and « ✨ Accord des unités de
    /// temps » (idea 2): the indicators shown on the chart, read together by `argos_ai` on the closed bars, in the
    /// panel (its blocks, then the reminder); a clear sentence when no indicator is shown, or no higher unit exists.
    /// « ✨ Volatilité du moment » (idea 4): `argos_ai::read_volatility` on the closed bars.
    pub(crate) fn lire_ensemble(&mut self, quoi: Ensemble) {
        if !self.ui.ai_open {
            self.ouvrir_panneau_ai();
        }
        self.ai.regles = true;
        self.ai.tous_reglages = None;
        self.ai.autres_reglages = 0;
        let affiches = self.indicateurs_affiches();
        let reglages: Vec<Vec<(&str, f64)>> = affiches.iter().map(|(_, _, _, r)| r.iter().map(|(n, v)| (n.as_str(), *v)).collect()).collect();
        let items: Vec<argos_ai::Item> = affiches
            .iter()
            .zip(&reglages)
            .map(|((_, id, name, _), settings)| argos_ai::Item { id, name, settings })
            .collect();
        let secs = self.chart.interval.secs();
        // « ✨ Niveaux du jour » (idea 5): yesterday's high and low and the day's VWAP (UTC day), shown like the others;
        // with a snapshot, its levels drawn on the chart like the Lens card's « Explique » (checked against the window)
        let mut niveaux = None;
        let (titre, vue) = if quoi == Ensemble::NiveauxJour {
            let d = argos_ai::read_day_levels(self.bougies_fermees());
            let vue = Some(d.overview.clone());
            niveaux = Some(d);
            ("✨ Niveaux du jour".to_string(), vue)
        } else if quoi == Ensemble::Volatilite {
            ("✨ Volatilité du moment".to_string(), Some(argos_ai::read_volatility(self.bougies_fermees())))
        } else if quoi == Ensemble::Accord {
            match argos_ai::higher_timeframe(secs) {
                Some(sup) => (
                    format!(
                        "✨ Accord des unités de temps : {} et {}",
                        argos_ai::timeframe_label(secs),
                        argos_ai::timeframe_label(sup)
                    ),
                    (!items.is_empty()).then(|| argos_ai::read_timeframes(&items, self.bougies_fermees(), secs, sup)),
                ),
                None => {
                    let m = format!(
                        "✨ Accord des unités de temps\n\nPas d'unité supérieure à {} : choisis une unité plus petite pour comparer.",
                        argos_ai::timeframe_label(secs)
                    );
                    self.ai.msgs.push((false, m));
                    self.ai.scroll = 0.0;
                    return self.redraw();
                }
            }
        } else {
            // (each indicator with what the screen holds for it, as its own reading; the comparison of units above
            // keeps the bars only: the screen's data is at the chart's unit, not the higher one)
            let calculs: Vec<Calculs> = affiches.iter().map(|(i, ..)| self.calculs_ecran(*i)).collect();
            let donnees: Vec<argos_ai::ScreenData> = calculs.iter().map(|c| self.donnees_ecran(c)).collect();
            (
                "✨ Tous les indicateurs de la chart".to_string(),
                (!items.is_empty()).then(|| argos_ai::read_together_with(&items, self.bougies_fermees(), &donnees)),
            )
        };
        let texte = match vue {
            None => format!("{titre}\n\nAucun indicateur affiché sur la chart : ajoute-en un avec ƒx (ou rends-le visible avec 👁), puis recommence."),
            Some(v) => {
                let mut t = format!("{titre}\n");
                for b in &v.blocks {
                    t.push_str(&format!("\n{}\n", b.title));
                    for s in &b.sentences {
                        t.push_str(&format!("• {}\n", s.text));
                    }
                }
                t.push_str(&format!("\n{}", v.reminder));
                t
            }
        };
        self.ai.msgs.push((false, texte));
        // 📏 the day's levels drawn on the chart (only those inside the visible window), with « Effacer l'explication »
        if let Some(DayLevels {
            explanation,
            snapshot: Some(snap),
            ..
        }) = &niveaux
        {
            let cmds = self.fenetre_visible().map_or_else(Vec::new, |w| explain::check(explanation, snap, &w));
            self.ui.explication = Some((self.symbol_label(), secs, cmds));
            self.ai.effacer = true;
        }
        self.ai.scroll = 0.0;
        self.redraw();
    }

    /// 🤖 The explanation in the panel and its drawings on the chart (or a clear message why there is none).
    pub(crate) fn regles_expliquer(&mut self) {
        self.ai.regles = true;
        self.ai.tous_reglages = None;
        self.ai.autres_reglages = 0;
        match self.explication_lens() {
            Ok((texte, cmds)) => {
                self.ai.msgs.push((false, texte));
                self.ui.explication = Some((self.symbol_label(), self.chart.interval.secs(), cmds));
            }
            Err(m) => {
                self.ai.msgs.push((false, m));
                self.ui.explication = None;
            }
        }
        self.ai.effacer = self.ui.explication.is_some();
        self.ai.scroll = 0.0;
    }

    /// 🤖 The explanation of the last closed bar: the Lens card's snapshot if it is on the chart, else computed now
    /// with the Lens settings; its text, and its drawings kept inside the visible window.
    fn explication_lens(&self) -> Result<(String, Vec<Command>), String> {
        let vals = argos_render::indicators::values(&self.ind.set, Ind::Lens);
        let tf = self.chart.interval.secs();
        let calcul;
        let vue = match self.ind.out.lens.as_ref().filter(|v| v.photo.is_some()) {
            Some(v) => v,
            None => {
                calcul = argos_render::indicators::lens::compute(&self.chart.bars, tf, now_s(), &vals);
                &calcul
            }
        };
        let Some((i, snap)) = vue.photo.as_ref() else {
            return Err(match &vue.erreur {
                Some(e) => format!("📏 Argos AI · règles : pas d'explication pour cette chart ({e})."),
                None => "📏 Argos AI · règles : pas encore assez de bougies fermées pour expliquer la chart.".into(),
            });
        };
        let ouverture = self.chart.bars.get(*i).map_or(0, |b| b.time);
        let e = explain::explain(snap, &argos_lens::reglages::config(&vals), ouverture, tf);
        let mut texte = String::from("📏 Argos AI · règles (sans réseau, sans clé)\n");
        for b in &e.blocks {
            texte.push_str(&format!("\n{}\n", b.title));
            for s in &b.sentences {
                texte.push_str(&format!("• {}\n", s.text));
            }
        }
        texte.push_str(&format!("\n{}", e.reminder));
        let cmds = self.fenetre_visible().map_or_else(Vec::new, |w| explain::check(&e, snap, &w));
        Ok((texte, cmds))
    }

    /// 🤖 The chart's visible window: the times of its left and right edges, the prices of the price zone's bottom and
    /// top.
    fn fenetre_visible(&self) -> Option<Window> {
        let f = self.frame_now()?;
        let g = f.geo();
        let sc = self.scale_now()?;
        let vp = &self.chart.viewport;
        let price_h = (vp.plot_h() - self.ind.out.osc_h()).max(1.0);
        let d = |p: f64| Decimal::try_from(p).ok();
        let (haut, bas) = (sc.price_of(0.0), sc.price_of(price_h));
        Some(Window {
            time_from: g.t_of(0.0),
            time_to: g.t_of(vp.plot_w()),
            price_low: d(haut.min(bas))?,
            price_high: d(haut.max(bas))?,
        })
    }

    /// 🤖 The explanation's drawings for this image: only for the symbol and interval they were made for.
    pub(crate) fn explication_vue(&self) -> &[Command] {
        self.ui
            .explication
            .as_ref()
            .filter(|(s, t, _)| *s == self.symbol_label() && *t == self.chart.interval.secs())
            .map_or(&[], |(_, _, c)| c.as_slice())
    }

    /// 🤖 The symbol or the interval changed: the explanation goes (and its « Effacer » button).
    pub(crate) fn explication_a_jour(&mut self) {
        let (sym, tf) = (self.symbol_label(), self.chart.interval.secs());
        if self.ui.explication.as_ref().is_some_and(|(s, t, _)| *s != sym || *t != tf) {
            self.effacer_explication();
        }
    }

    /// 🤖 « Effacer l'explication »: its drawings go.
    pub(crate) fn effacer_explication(&mut self) {
        self.ui.explication = None;
        self.ai.effacer = false;
    }

    /// 🤖 The press (mouse or finger) on the Lens card's « Explique » pill (chart coordinates): 40 points around it on
    /// the phone and the tablet. `true` if it was.
    pub(crate) fn sur_explique(&self, x: f64, y: f64) -> bool {
        let r = if self.top.phone {
            argos_render::indicators::lens::bouton_explique_doigt()
        } else {
            argos_render::indicators::lens::bouton_explique()
        };
        r.is_some_and(|r| r.contains(x, y))
    }
}

/// 🤖 An indicator's id in the Argos Script catalogue (`catalogue::Fiche::id`: the name of its `Ind` variant, « Rsi »,
/// « Macd », « EmaCross », « Vwap »…).
pub(crate) fn id_catalogue(i: Ind) -> String {
    serde_json::to_string(&i).unwrap_or_default().trim_matches('"').to_string()
}

/// 🤖 A setting's value as the ⚙ shows it, from its kind in the catalogue (Jee 18 Nov: « Show WaveTrend : 1 »): a
/// tick box « oui » / « non », a list or a source its option's name, a colour « #rrggbb », a number as it is.
fn valeur_lisible(p: Option<&argos_script::inputs::Param>, v: f64) -> String {
    use argos_script::inputs::{Kind, SOURCES};
    let nombre = || if v.fract() == 0.0 { format!("{v:.0}") } else { format!("{v}") };
    match p.map(|p| p.kind) {
        Some(Kind::Bool) => (if v != 0.0 { "oui" } else { "non" }).to_string(),
        Some(Kind::Choice(o)) => o.get(v as usize).map_or_else(nombre, |s| (*s).to_string()),
        Some(Kind::Source) => SOURCES
            .get(v as usize)
            .map_or_else(|| "la sortie d'un autre indicateur".to_string(), |s| (*s).to_string()),
        Some(Kind::Color) => format!("#{:06x}", (v as u32) & 0xff_ffff),
        _ => nombre(),
    }
}

/// 🤖 A setting's name, as the catalogue's `par_nom` writes it: its `Param::key`, or `key@k` (`k`: its position) when
/// the key is already taken in the same table.
fn nom_reglage(deja: &[(String, f64)], key: &str, k: usize) -> String {
    if deja.iter().any(|(n, _)| n == key) {
        format!("{key}@{k}")
    } else {
        key.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🤖 The ids and setting names are the Argos Script catalogue's (`catalogue.rs` of argos-script-1: the variant
    /// name; `par_nom`: `key`, then `key@k` for a key taken twice), for the 4 indicators of « lot 1 ».
    #[test]
    fn ids_et_reglages_du_catalogue() {
        let ids: Vec<String> = [Ind::Rsi, Ind::Macd, Ind::EmaCross, Ind::EmaMulti, Ind::Vwap]
            .into_iter()
            .map(id_catalogue)
            .collect();
        assert_eq!(ids, ["Rsi", "Macd", "EmaCross", "EmaMulti", "Vwap"]);
        let noms = |i: Ind| {
            let mut out: Vec<(String, f64)> = vec![];
            for (k, p) in i.params().1.iter().enumerate() {
                out.push((nom_reglage(&out, p.key, k), p.def));
            }
            out.into_iter().map(|(n, _)| n).collect::<Vec<_>>()
        };
        assert_eq!(&noms(Ind::Rsi)[..3], ["len", "haut", "bas"]);
        assert_eq!(&noms(Ind::Macd)[..3], ["fast", "slow", "signal"]);
        assert_eq!(noms(Ind::Vwap), ["hl3", "col", "ep1", "ty1"]);
        // a key taken twice in one table: `key@k` the second time, as par_nom
        assert_eq!(nom_reglage(&[("col".into(), 0.0)], "col", 5), "col@5");
        assert_eq!(nom_reglage(&[("col".into(), 0.0)], "ep1", 2), "ep1");
    }

    /// 🤖 Jee 18 Nov (« Show WaveTrend : 1 » on Cipher): each setting as the ⚙ shows it, from its kind in the catalogue.
    #[test]
    fn valeurs_comme_le_reglage() {
        use argos_script::inputs::{check, choice, color, num, Kind, Param};
        let oui = check("a", 0, "Show WaveTrend", true);
        assert_eq!(valeur_lisible(Some(&oui), 1.0), "oui");
        assert_eq!(valeur_lisible(Some(&oui), 0.0), "non");
        let liste = choice("b", 0, "Mode", 0, &["Normal", "Scalping", "Swing"]);
        assert_eq!(valeur_lisible(Some(&liste), 2.0), "Swing");
        let source = Param {
            kind: Kind::Source,
            ..num("c", 0, "Source", 0.0, 0.0, 6.0, 1.0)
        };
        assert_eq!(valeur_lisible(Some(&source), 5.0), "(H + L + C)/3");
        assert_eq!(valeur_lisible(Some(&source), 0.0), "Clôture");
        assert_eq!(valeur_lisible(Some(&color("d", 0, "Couleur", 0x2962ff)), 0x2962ff as f64), "#2962ff");
        let n = num("e", 0, "Longueur", 14.0, 1.0, 500.0, 1.0);
        assert_eq!(valeur_lisible(Some(&n), 14.0), "14");
        assert_eq!(valeur_lisible(Some(&n), 2.5), "2.5");
        assert_eq!(valeur_lisible(None, 3.0), "3");
    }
}
