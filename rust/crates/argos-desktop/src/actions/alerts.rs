//! 🔔 Alertes : création (clic droit / appui long « Ajouter une alerte à … », bouton Alertes de la barre flottante),
//! ligne glissée sur la chart, surveillance sur les prix en direct de tous les symboles qui ont une alerte, et
//! déclenchement (bandeau, son et notification du navigateur si elle est permise).

use crate::*;
use argos_core::alerts::{line_id, Cond, Dir, Kind, Op, Src};
use argos_render::listmenu::{MenuGroup, MenuItem};

/// Modèles d'alertes multi-conditions (sur le symbole et l'unité de temps affichés) : nom, ET (sinon OU), conditions.
const TEMPLATES: [(&str, bool, &[Cond]); 8] = [
    (
        "RSI 14 sous 30 (survente)",
        true,
        &[Cond {
            l: Src::Rsi,
            op: Op::Below,
            r: Src::Num(30.0),
            r2: None,
        }],
    ),
    (
        "RSI 14 au-dessus de 70 (surachat)",
        true,
        &[Cond {
            l: Src::Rsi,
            op: Op::Above,
            r: Src::Num(70.0),
            r2: None,
        }],
    ),
    (
        "Prix croise au-dessus de l'EMA 50",
        true,
        &[Cond {
            l: Src::Close,
            op: Op::CrossUp,
            r: Src::Ema(50),
            r2: None,
        }],
    ),
    (
        "Prix croise en dessous de l'EMA 50",
        true,
        &[Cond {
            l: Src::Close,
            op: Op::CrossDown,
            r: Src::Ema(50),
            r2: None,
        }],
    ),
    (
        "EMA 20 croise au-dessus de l'EMA 50",
        true,
        &[Cond {
            l: Src::Ema(20),
            op: Op::CrossUp,
            r: Src::Ema(50),
            r2: None,
        }],
    ),
    (
        "EMA 20 croise en dessous de l'EMA 50",
        true,
        &[Cond {
            l: Src::Ema(20),
            op: Op::CrossDown,
            r: Src::Ema(50),
            r2: None,
        }],
    ),
    (
        "Volume au-dessus de la moyenne ET prix au-dessus de l'EMA 200",
        true,
        &[
            Cond {
                l: Src::Volume,
                op: Op::Above,
                r: Src::VolMa,
                r2: None,
            },
            Cond {
                l: Src::Close,
                op: Op::Above,
                r: Src::Ema(200),
                r2: None,
            },
        ],
    ),
    (
        "RSI 14 sous 30 OU prix croise en dessous de l'EMA 200",
        false,
        &[
            Cond {
                l: Src::Rsi,
                op: Op::Below,
                r: Src::Num(30.0),
                r2: None,
            },
            Cond {
                l: Src::Close,
                op: Op::CrossDown,
                r: Src::Ema(200),
                r2: None,
            },
        ],
    ),
];

/// Délais « à chaque fois » et validités proposés (secondes ; `None` : une fois / sans fin).
const EVERY: [(Option<i64>, &str); 3] = [(None, "Une fois"), (Some(900), "À chaque fois · 15 min"), (Some(3600), "À chaque fois · 1 h")];
const VALID: [(Option<i64>, &str); 3] = [(None, "Sans fin"), (Some(86_400), "24 h"), (Some(7 * 86_400), "7 jours")];

// identifiants du menu des alertes
const ID_PRICE: i64 = 1;
const ID_TPL: i64 = 10;
const ID_EVERY: i64 = 30;
const ID_VALID: i64 = 40;
const ID_COND: i64 = 200;
/// 🔔 Signaux nommés des indicateurs (alertes du Pine : breaks TBO, points Cipher…).
const ID_SIG: i64 = 300;
const ID_LINE: i64 = 100_000;
const ID_TOGGLE: i64 = 1_000_000;

/// 📈 Conditions proposées sur une ligne d'indicateur : texte, opérateur, valeurs à taper (0 = le prix, 1 = un
/// nombre, 2 = une zone « bas haut »).
const LINE_CONDS: [(&str, Op, u8); 8] = [
    ("croise au-dessus de… (valeur)", Op::CrossUp, 1),
    ("croise en dessous de… (valeur)", Op::CrossDown, 1),
    ("plus grand que… (valeur)", Op::Above, 1),
    ("plus petit que… (valeur)", Op::Below, 1),
    ("entre dans la zone… (bas haut)", Op::EntersZone, 2),
    ("sort de la zone… (bas haut)", Op::LeavesZone, 2),
    ("croise le prix vers le haut", Op::CrossUp, 0),
    ("croise le prix vers le bas", Op::CrossDown, 0),
];
const ID_DELETE: i64 = 2_000_000;

impl App {
    pub(crate) fn save_alerts(&self) {
        Self::write_prefs("alerts.json", &self.alerts);
    }

    /// 🔔 A new alert (on): also its market, the chart's time frame and its creation time, for the alerts panel.
    pub(crate) fn add_alert(&mut self, sym: &str, kind: Kind, every: Option<i64>, until: Option<i64>, msg: &str) -> u64 {
        let id = self.alerts.add(sym, kind, every, until, msg);
        let (coin, tf) = (self.main_coin(), self.chart.interval.secs());
        if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) {
            (a.coin, a.tf, a.created) = (coin, Some(tf), Some(now_s()));
        }
        id
    }

    /// Options choisies dans le menu (délai, validité) pour la prochaine alerte.
    fn alert_opts(&self) -> (Option<i64>, Option<i64>) {
        let (e, v) = self.alerts_draft;
        (EVERY.get(e).and_then(|x| x.0), VALID.get(v).and_then(|x| x.0).map(|s| now_s() + s))
    }

    /// 🔔 Alerte de prix à `price` sur le symbole affiché (franchit dans les deux sens).
    pub(crate) fn add_price_alert(&mut self, price: f64) {
        let sym = self.symbol_label();
        let (every, until) = self.alert_opts();
        let id = self.add_alert(&sym, Kind::Price { price, dir: Dir::Both }, every, until, "");
        // le prix actuel sert de départ : franchir = passer de l'autre côté de la ligne
        let now_px = self.demo_price(&sym);
        if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) {
            a.seen = now_px;
        }
        self.save_alerts();
        platform::ask_notifications();
        self.banner = Some(format!(
            "🔔 Alerte créée : {sym} franchit {} (glisse la ligne pour la déplacer)",
            argos_render::viewport::format_price(price, 2)
        ));
        self.redraw();
    }

    /// 🔔 Ligne d'alerte du symbole affiché sous la souris (à 6 px près), dans la chart (en pause aussi) ; « Lignes
    /// d'alerte » décoché : sa forme courte (le nom, la poignée, le bout pointillé jusqu'à l'échelle).
    pub(crate) fn alert_at(&self) -> Option<u64> {
        let (mx, my) = self.pointer.mouse?;
        if mx > self.chart.viewport.plot_w() || my > self.chart.viewport.plot_h() || self.ui.context.is_open() || self.ui.list.is_some() {
            return None;
        }
        let sc = self.scale_now()?;
        let lignes = self.alerts.lignes(&self.symbol_label());
        if !self.top.prefs.lignes_alerte {
            return lignes.into_iter().map(|(id, ..)| id).find(|&id| {
                self.etiquette_de(id)
                    .is_some_and(|(_, _, y, p)| p.boite.contains(mx, my) || (mx >= p.poignee.x - 4.0 && (my - y).abs() <= 7.0))
            });
        }
        lignes.into_iter().find(|&(_, p, _)| (sc.y_of(p) - my).abs() <= 6.0).map(|(id, ..)| id)
    }

    /// 🔔 How the chart draws the alert lines: the paused ones (grey), the chosen colour, the short form.
    pub(crate) fn alertes_vue(&self) -> argos_render::alerte_etiquette::Vue {
        argos_render::alerte_etiquette::Vue {
            pause: self.alerts.lignes(&self.symbol_label()).into_iter().filter(|l| !l.2).map(|l| l.1).collect(),
            couleur: argos_render::colorpick::split(&self.top.prefs.couleur_alertes).map(|c| c.0),
            courtes: !self.top.prefs.lignes_alerte,
        }
    }

    /// 🔔 Ligne relâchée : l'alerte est gardée à son nouveau prix.
    pub(crate) fn alert_moved(&mut self, id: u64) {
        self.save_alerts();
        if let Some((_, p, _)) = self.alerts.lignes(&self.symbol_label()).into_iter().find(|(i, ..)| *i == id) {
            self.banner = Some(format!("🔔 Alerte déplacée à {}", argos_render::viewport::format_price(p, 2)));
        }
    }

    /// 📈 Lignes d'indicateurs disponibles pour une alerte : (nom, une valeur par bougie de la chart) — indicateurs
    /// actifs, scripts et modules démarrés.
    pub(crate) fn alert_lines(&self) -> Vec<(String, Vec<f64>)> {
        use argos_render::indicators::{output_series, source_outputs};
        let mut v: Vec<(String, Vec<f64>)> = source_outputs(&self.ind.on)
            .into_iter()
            .filter_map(|(ind, k, name)| Some((format!("{} · {name}", ind.name()), output_series(&self.ind.out, ind, k)?)))
            .collect();
        for s in &self.ind.out.scripts {
            for p in s.out.plots.iter().filter(|p| !p.title.is_empty()) {
                v.push((format!("{} · {}", s.title(), p.title), p.values.clone()));
            }
        }
        v
    }

    /// 🔔 Signaux nommés des indicateurs actifs, comme les `alertcondition` des scripts Pine : (nom, 1 sur les bougies
    /// où le signal arrive). TBO : 10 signaux ; Cipher : 9 ; AI Trend : 17 ; TVD : 11 ; Vector Zone : 13.
    pub(crate) fn alert_signals(&self) -> Vec<(String, Vec<f64>)> {
        use argos_render::indicators::Ind;
        let mut v = vec![];
        if let Some(t) = self.ind.out.tbo.as_ref().filter(|_| self.ind.on.contains(&Ind::Tbo)) {
            v.extend(t.alerts(&self.chart.bars).into_iter().map(|(n, s)| (format!("{} · {n}", Ind::Tbo.name()), s)));
        }
        if let Some(z) = self.ind.out.vz.as_ref().filter(|_| self.ind.on.contains(&Ind::VectorZone)) {
            v.extend(z.alerts().into_iter().map(|(n, s)| (format!("{} · {n}", Ind::VectorZone.name()), s)));
        }
        if let Some(t) = self.ind.out.tvd.as_ref().filter(|_| self.ind.on.contains(&Ind::Tvd)) {
            v.extend(t.alerts().into_iter().map(|(n, s)| (format!("{} · {n}", Ind::Tvd.name()), s)));
        }
        if let Some(a) = self.ind.out.ai.as_ref().filter(|_| self.ind.on.contains(&Ind::AiTrend)) {
            v.extend(
                a.alerts(&self.chart.bars)
                    .into_iter()
                    .map(|(n, s)| (format!("{} · {n}", Ind::AiTrend.name()), s)),
            );
        }
        if let Some(c) = self.ind.out.cipher.as_ref().filter(|_| self.ind.on.contains(&Ind::Cipher)) {
            v.extend(c.alerts().into_iter().map(|(n, s)| (format!("{} · {n}", Ind::Cipher.name()), s)));
        }
        // 📜 the alertcondition() of the user scripts on the chart
        for s in &self.ind.out.scripts {
            v.extend(s.out.alertconditions.iter().map(|(t, _, sig)| (format!("{} · {t}", s.title()), sig.clone())));
        }
        v
    }

    /// 🔔 Crée l'alerte « le signal arrive » (à la clôture de la bougie) sur le symbole et l'unité de temps affichés.
    fn add_signal_alert(&mut self, name: String) {
        let id = line_id(&name);
        let (every, until) = self.alert_opts();
        let (sym, tf) = (self.symbol_label(), self.chart.interval.secs());
        let cond = Cond {
            l: Src::Line(id),
            op: Op::Above,
            r: Src::Num(0.5),
            r2: None,
        };
        let aid = self.add_alert(
            &sym,
            Kind::Multi {
                tf,
                all: true,
                conds: vec![cond],
            },
            every,
            until,
            &name,
        );
        if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == aid) {
            a.lines = vec![(id, name.clone())];
        }
        self.save_alerts();
        platform::ask_notifications();
        self.banner = Some(format!(
            "🔔 Alerte créée : {name} sur {sym} {} (vérifiée à la clôture de chaque bougie, app ouverte)",
            self.chart.interval.short()
        ));
        self.redraw();
    }

    /// 📈 Valeur(s) tapée(s) pour la condition choisie sur la ligne d'indicateur (« 70 », « 30 70 »).
    pub(crate) fn alert_value_typed(&mut self, text: &str) {
        let nums: Vec<f64> = text
            .split(|c: char| c.is_whitespace() || c == ';' || c == '/')
            .filter_map(|w| w.trim().replace(',', ".").parse::<f64>().ok())
            .filter(|x| x.is_finite())
            .collect();
        let k = self.ui.alert_op.min(LINE_CONDS.len() - 1);
        let need = LINE_CONDS[k].2 as usize;
        if nums.len() < need {
            self.banner = Some(format!(
                "🔔 Écris {} nombre(s), par exemple « {} »",
                need,
                if need == 2 { "30 70" } else { "70" }
            ));
            return;
        }
        self.add_line_alert(k, &nums);
    }

    /// 📈 Crée l'alerte « ligne `op` valeur(s) » sur le symbole et l'unité de temps affichés.
    fn add_line_alert(&mut self, k: usize, nums: &[f64]) {
        let Some(name) = self.ui.alert_line.clone() else { return };
        let (_, op, need) = LINE_CONDS[k];
        let id = line_id(&name);
        let (r, r2) = match need {
            0 => (Src::Close, None),
            1 => (Src::Num(nums[0]), None),
            _ => (Src::Num(nums[0].min(nums[1])), Some(Src::Num(nums[0].max(nums[1])))),
        };
        let (every, until) = self.alert_opts();
        let (sym, tf) = (self.symbol_label(), self.chart.interval.secs());
        let aid = self.add_alert(
            &sym,
            Kind::Multi {
                tf,
                all: true,
                conds: vec![Cond { l: Src::Line(id), op, r, r2 }],
            },
            every,
            until,
            "",
        );
        if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == aid) {
            a.lines = vec![(id, name.clone())];
        }
        self.save_alerts();
        platform::ask_notifications();
        let text = self.alerts.list.iter().find(|a| a.id == aid).map(|a| a.text()).unwrap_or_default();
        self.banner = Some(format!("🔔 Alerte créée : {text} (vérifiée à la clôture de chaque bougie, app ouverte)"));
        self.redraw();
    }

    /// 🔔 Surveillance (à chaque prix reçu) : les alertes de prix de tous les symboles qui ont une alerte, et les
    /// alertes multi-conditions du symbole affiché sur ses bougies.
    pub(crate) fn alerts_watch(&mut self) {
        if self.alerts.list.is_empty() {
            return;
        }
        let now = now_s();
        self.alerts.expire(now);
        let mut fired = vec![];
        for sym in self.alerts.symbols() {
            if let Some(px) = self.demo_price(&sym) {
                fired.extend(self.alerts.check_price(&sym, px, now));
            }
        }
        let (sym, tf) = (self.symbol_label(), self.chart.interval.secs());
        // 📈 les lignes d'indicateurs de la chart affichée (alignées sur ses bougies, sinon ignorées ce tour-ci)
        let mut lines = self.alert_lines();
        lines.extend(self.alert_signals());
        let n = self.chart.bars.len();
        let ext = |id: u64| lines.iter().find(|(name, v)| line_id(name) == id && v.len() == n).map(|(_, v)| v.clone());
        fired.extend(self.alerts.check_bars_with(&sym, tf, &self.chart.bars, now, &ext));
        if !fired.is_empty() {
            self.save_alerts();
            let text = fired.iter().map(|f| f.text.clone()).collect::<Vec<_>>().join(" · ");
            self.banner = Some(format!("🔔 {text}"));
            // 🔊 the fired alerts' own sound, else the settings' one (silent only if all are « Son coupé »)
            let sons: Vec<Option<String>> = fired
                .iter()
                .map(|f| self.alerts.list.iter().find(|a| a.id == f.id).and_then(|a| a.son.clone()))
                .collect();
            let son = argos_core::alerts::son_a_jouer(sons.iter().map(|s| s.as_deref()), &self.top.prefs.son_alertes);
            platform::alert_notify("🔔 Alerte Argos", &text, son, self.top.prefs.volume_alertes);
            self.redraw();
        }
    }

    /// 🔔 Menu des alertes (bouton Alertes de la barre flottante) : nouvelle alerte, options, mes alertes, historique.
    pub(crate) fn alerts_menu(&self) -> Vec<MenuGroup> {
        let item = |id: i64, label: String, checked: bool, enabled: bool| MenuItem {
            id,
            label,
            enabled,
            checked,
            ..Default::default()
        };
        let sym = self.symbol_label();
        let tf = self.chart.interval.short();
        let px = self
            .chart
            .bars
            .last()
            .map(|b| argos_render::viewport::format_price(b.close, 2))
            .unwrap_or_default();
        let mut new = vec![item(ID_PRICE, format!("Alerte de prix à {px} ({sym})"), false, !px.is_empty())];
        new.extend(
            TEMPLATES
                .iter()
                .enumerate()
                .map(|(i, t)| item(ID_TPL + i as i64, format!("{} · {sym} {tf}", t.0), false, true)),
        );
        // 📈 sur un indicateur : la ligne d'abord, puis sa condition
        let lines = self.alert_lines();
        let mut on_line: Vec<MenuItem> = lines
            .iter()
            .enumerate()
            .map(|(k, (name, _))| item(ID_LINE + k as i64, format!("📈 {name}"), self.ui.alert_line.as_deref() == Some(name), true))
            .collect();
        if on_line.is_empty() {
            on_line.push(item(-1, "Ajoute un indicateur, un script ou un module à la chart".into(), false, false));
        }
        let mut groups = vec![
            MenuGroup {
                title: "NOUVELLE ALERTE".into(),
                items: new,
                strip: false,
            },
            MenuGroup {
                title: format!("SUR UN INDICATEUR · {sym} {tf}"),
                items: on_line,
                strip: false,
            },
            MenuGroup {
                title: "DÉCLENCHEMENT".into(),
                items: EVERY
                    .iter()
                    .enumerate()
                    .map(|(i, e)| item(ID_EVERY + i as i64, e.1.into(), self.alerts_draft.0 == i, true))
                    .collect(),
                strip: true,
            },
            MenuGroup {
                title: "VALIDITÉ".into(),
                items: VALID
                    .iter()
                    .enumerate()
                    .map(|(i, v)| item(ID_VALID + i as i64, v.1.into(), self.alerts_draft.1 == i, true))
                    .collect(),
                strip: true,
            },
        ];
        // 🔔 signaux nommés (alertes du Pine) : un toucher crée l'alerte
        let signals = self.alert_signals();
        if !signals.is_empty() {
            groups.insert(
                2,
                MenuGroup {
                    title: format!("SIGNAUX · {sym} {tf}"),
                    items: signals
                        .iter()
                        .enumerate()
                        .map(|(k, (name, _))| item(ID_SIG + k as i64, format!("🔔 {name}"), false, true))
                        .collect(),
                    strip: false,
                },
            );
        }
        if let Some(name) = self.ui.alert_line.as_ref().filter(|n| lines.iter().any(|l| &l.0 == *n)) {
            groups.insert(
                2,
                MenuGroup {
                    title: format!("CONDITION · {name}"),
                    items: LINE_CONDS
                        .iter()
                        .enumerate()
                        .map(|(k, c)| item(ID_COND + k as i64, c.0.into(), false, true))
                        .collect(),
                    strip: false,
                },
            );
        }
        if !self.alerts.list.is_empty() {
            groups.push(MenuGroup {
                title: "MES ALERTES (toucher = pause / reprise)".into(),
                items: self
                    .alerts
                    .list
                    .iter()
                    .enumerate()
                    .map(|(i, a)| item(ID_TOGGLE + i as i64, format!("{} {}", if a.on { "🔔" } else { "🔕" }, a.text()), a.on, true))
                    .collect(),
                strip: false,
            });
            groups.push(MenuGroup {
                title: "SUPPRIMER".into(),
                items: self
                    .alerts
                    .list
                    .iter()
                    .enumerate()
                    .map(|(i, a)| item(ID_DELETE + i as i64, format!("✕ {}", a.text()), false, true))
                    .collect(),
                strip: false,
            });
        }
        if !self.alerts.history.is_empty() {
            groups.push(MenuGroup {
                title: "HISTORIQUE".into(),
                items: self
                    .alerts
                    .history
                    .iter()
                    .take(10)
                    .map(|f| {
                        let t = chrono::DateTime::from_timestamp(f.ts + self.tz_offset_s(), 0)
                            .map(|d| d.format("%d/%m %H:%M").to_string())
                            .unwrap_or_default();
                        item(-1, format!("{t} · {}", f.text), false, false)
                    })
                    .collect(),
                strip: false,
            });
        }
        groups
    }

    /// 🔔 Choix dans le menu des alertes ; `true` si le menu doit rester ouvert (options, pause, suppression).
    pub(crate) fn alerts_pick(&mut self, id: i64) -> bool {
        match id {
            ID_PRICE => {
                if let Some(p) = self.chart.bars.last().map(|b| b.close) {
                    self.add_price_alert(p);
                }
                false
            }
            i if (ID_TPL..ID_TPL + TEMPLATES.len() as i64).contains(&i) => {
                let (name, all, conds) = TEMPLATES[(i - ID_TPL) as usize];
                let (every, until) = self.alert_opts();
                let sym = self.symbol_label();
                let tf = self.chart.interval.secs();
                self.add_alert(
                    &sym,
                    Kind::Multi {
                        tf,
                        all,
                        conds: conds.to_vec(),
                    },
                    every,
                    until,
                    "",
                );
                self.save_alerts();
                platform::ask_notifications();
                self.banner = Some(format!(
                    "🔔 Alerte créée : {name} sur {sym} {} (vérifiée à la clôture de chaque bougie)",
                    self.chart.interval.short()
                ));
                false
            }
            i if (ID_SIG..ID_SIG + 100).contains(&i) => {
                if let Some((name, _)) = self.alert_signals().into_iter().nth((i - ID_SIG) as usize) {
                    self.add_signal_alert(name);
                }
                false
            }
            i if (ID_LINE..ID_LINE + 10_000).contains(&i) => {
                self.ui.alert_line = self.alert_lines().get((i - ID_LINE) as usize).map(|l| l.0.clone());
                true
            }
            i if (ID_COND..ID_COND + LINE_CONDS.len() as i64).contains(&i) => {
                let k = (i - ID_COND) as usize;
                self.ui.alert_op = k;
                if LINE_CONDS[k].2 == 0 {
                    self.add_line_alert(k, &[]);
                    return false;
                }
                // valeur(s) à taper
                let anchor = Rect::new(self.rw() + 60.0, self.top_h() + 60.0, 40.0, 24.0);
                let hint = if LINE_CONDS[k].2 == 2 { "30 70" } else { "70" };
                self.ui.name_box = Some(NameBox::new(NameFor::AlertValue, hint, anchor, self.win_w()));
                self.ui.list = None;
                false
            }
            i if (ID_EVERY..ID_EVERY + EVERY.len() as i64).contains(&i) => {
                self.alerts_draft.0 = (i - ID_EVERY) as usize;
                true
            }
            i if (ID_VALID..ID_VALID + VALID.len() as i64).contains(&i) => {
                self.alerts_draft.1 = (i - ID_VALID) as usize;
                true
            }
            i if i >= ID_DELETE => {
                if (i - ID_DELETE) < self.alerts.list.len() as i64 {
                    self.alerts.list.remove((i - ID_DELETE) as usize);
                    self.save_alerts();
                }
                true
            }
            i if i >= ID_TOGGLE => {
                if let Some(a) = self.alerts.list.get_mut((i - ID_TOGGLE) as usize) {
                    a.on = !a.on;
                    (a.fired, a.seen) = (None, None);
                    rearmer_si_expiree(a);
                    self.save_alerts();
                }
                true
            }
            _ => true,
        }
    }
}

/// ✅ What the confirmation box open does when confirmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AConfirmer {
    /// 🔔 Delete this alert (its id).
    SupprimerAlerte(u64),
    /// 🔔 « Vider l'historique » of the alerts panel.
    ViderHistorique,
}

impl App {
    /// 🔔 The label shown: the chosen alert line's (clicked or touched), else the one under the mouse; its id, its
    /// full name, its height on the chart and the label's places.
    pub(crate) fn etiquette_alerte(&self) -> Option<(u64, String, f64, argos_render::alerte_etiquette::Places)> {
        self.ui.alerte_sel.or(self.ui.alerte_survol).and_then(|id| self.etiquette_de(id))
    }

    /// 🔔 The mouse moved on the chart: the alert line under it (6 points around the line, like the ↕ cursor), or the
    /// one whose label it is on, keeps its label shown (PC, like TradingView); elsewhere, no label.
    pub(crate) fn survol_alerte(&mut self) {
        let sur_etiquette = |id: u64| {
            self.pointer
                .mouse
                .zip(self.etiquette_de(id))
                .is_some_and(|((x, y), (.., p))| p.boite.contains(x, y) || p.poignee.contains(x, y))
        };
        let garde = self.ui.alerte_survol.filter(|&id| sur_etiquette(id));
        self.ui.alerte_survol = garde.or_else(|| self.alert_at());
    }

    /// 🔔 The label of the alert `id` (a price alert on the chart, its line inside the chart): « BTCUSDC.P Croisement
    /// 63 584,0 » like TradingView (the symbol as the top bar writes it, the condition, the price as on the chart),
    /// or its own message.
    pub(crate) fn etiquette_de(&self, id: u64) -> Option<(u64, String, f64, argos_render::alerte_etiquette::Places)> {
        let a = self.alerts.list.iter().find(|a| a.id == id && a.sym == self.symbol_label())?;
        let argos_core::alerts::Kind::Price { price, dir } = a.kind else {
            return None;
        };
        let y = self.scale_now()?.y_of(price);
        if !(0.0..self.chart.viewport.plot_h()).contains(&y) {
            return None;
        }
        // (its name: its message, else « BTCUSDC.P Croisement 63 584,0 » — the price written as on the chart)
        let nom = if a.msg.is_empty() {
            let px = argos_render::viewport::format_price(price, argos_render::viewport::data_decimals(&self.chart.bars));
            let condition = match dir {
                Dir::Up => "Croisement vers le haut",
                Dir::Down => "Croisement vers le bas",
                Dir::Both => "Croisement",
            };
            // (the symbol as TradingView writes it, without its exchange: « HYPERLIQUID:BTCUSDC.P » → « BTCUSDC.P »)
            let sym = self.top.symbol.rsplit(':').next().filter(|s| !s.is_empty()).unwrap_or(&a.sym);
            format!("{sym} {condition} {px}")
        } else {
            a.msg.clone()
        };
        // (off: it says why — paused by Jee, fired once, or expired)
        let nom = if a.on { nom } else { format!("{nom} {}", raison_arret(a, now_s())) };
        let t = argos_render::text::Text::default();
        let pw = self.chart.viewport.plot_w();
        let p = if self.top.prefs.lignes_alerte {
            argos_render::alerte_etiquette::places(&t, &nom, y, pw)
        } else {
            argos_render::alerte_etiquette::places_courte(&t, &nom, y, pw)
        };
        Some((id, nom, y, p))
    }

    /// 🔔 Draws the chosen alert line's label on the chart image; « Lignes d'alerte » unchecked: the short form of
    /// every alert line, always (an alert is never invisible).
    pub(crate) fn dessiner_etiquette_alerte(&self, pm: &mut tiny_skia::Pixmap) {
        use argos_render::alerte_etiquette as ae;
        let t = argos_render::text::Text::default();
        let pw = self.chart.viewport.plot_w();
        // (⛶ a panel full screen: nothing of the prices)
        if matches!(self.ui.plein_ecran, Some(crate::actions::plein_ecran::PleinEcran::Panneau(_))) {
            return;
        }
        let vue = self.alertes_vue();
        let en_pause = |id: u64| self.alerts.list.iter().any(|a| a.id == id && !a.on);
        if vue.courtes {
            for (id, ..) in self.alerts.lignes(&self.symbol_label()) {
                if let Some((_, nom, y, _)) = self.etiquette_de(id) {
                    let pause = en_pause(id);
                    ae::draw_courte(pm, &t, &nom, y, pw, self.pointer.mouse, vue.ligne(pause), vue.poignee(pause));
                }
            }
            return;
        }
        if let Some((id, nom, y, _)) = self.etiquette_alerte() {
            ae::draw(pm, &t, &nom, y, pw, self.pointer.mouse, vue.poignee(en_pause(id)));
        }
    }

    /// 🔔 A press on the chosen line's label (chart coordinates): 🗑 asks « Supprimer cette alerte? », the handle
    /// grabs the line, the name does nothing; elsewhere the label goes. `true` if the press was the label's.
    pub(crate) fn etiquette_alerte_appui(&mut self, x: f64, y: f64) -> bool {
        let Some((id, nom, _, p)) = self.etiquette_alerte() else {
            self.ui.alerte_sel = None;
            return false;
        };
        if p.poubelle.contains(x, y) {
            self.demander_suppression_alerte(id, &nom);
            return true;
        }
        let g = p.poignee;
        // (the handle: a touch zone of 40 around it)
        if (x - g.cx()).abs() <= 20.0 && (y - g.cy()).abs() <= 20.0 {
            (self.pointer.alert_drag, self.pointer.alert_appui) = (Some(id), None);
            return true;
        }
        if p.boite.contains(x, y) {
            return true;
        }
        self.ui.alerte_sel = None;
        false
    }

    /// 🔔 « Supprimer cette alerte? » for the alert `id` named `nom` (its 🗑, or « Effacer » in its menu).
    pub(crate) fn demander_suppression_alerte(&mut self, id: u64, nom: &str) {
        use argos_render::i18n::tr;
        let c = argos_render::confirmer::Confirmer {
            titre: tr("Supprimer cette alerte?").into_owned(),
            // (non-breaking spaces: « and » stay with the name)
            texte: format!("{} «\u{a0}{nom}\u{a0}».", tr("Cette action supprimera définitivement votre alerte")),
            ok: tr("Effacer").into_owned(),
            danger: true,
        };
        self.ui.confirmer = Some((c, AConfirmer::SupprimerAlerte(id)));
    }

    /// 🔔 « Vider l'historique? » (the alerts panel): the same box as « Supprimer cette alerte? ».
    pub(crate) fn demander_vider_historique(&mut self) {
        use argos_render::i18n::tr;
        // (the panel's history: its firings, not the notifications feed)
        let n = self.alerts.declenchements.len();
        let c = argos_render::confirmer::Confirmer {
            titre: tr("Vider l'historique?").into_owned(),
            texte: format!(
                "{} ({n}). {}",
                tr("Cette action effacera définitivement les déclenchements de l'historique"),
                tr("Tes alertes restent.")
            ),
            ok: tr("Vider").into_owned(),
            danger: true,
        };
        self.ui.confirmer = Some((c, AConfirmer::ViderHistorique));
    }

    /// ✅ A click or touch while the confirmation box is open (window coordinates): it takes them all.
    pub(crate) fn confirmer_clic(&mut self, wx: f64, wy: f64) {
        let Some((c, a)) = self.ui.confirmer.clone() else { return };
        let (w, h) = self.taille_points();
        match c.clic(&argos_render::text::Text::default(), w, h, wx, wy) {
            Some(argos_render::confirmer::ConfirmerClic::Ok) => {
                self.ui.confirmer = None;
                match a {
                    AConfirmer::SupprimerAlerte(id) => {
                        self.alerts.list.retain(|x| x.id != id);
                        self.ui.alerte_sel = None;
                        self.save_alerts();
                    }
                    AConfirmer::ViderHistorique => {
                        self.alerts.clear_history();
                        self.save_alerts();
                    }
                }
            }
            Some(argos_render::confirmer::ConfirmerClic::Annuler) => self.ui.confirmer = None,
            None => {}
        }
        self.redraw();
    }

    /// The window's size in points (the confirmation box is centred in it).
    pub(crate) fn taille_points(&self) -> (f64, f64) {
        let h = if self.top.phone { self.cell_h() } else { self.top_h() + self.area.1 as f64 };
        (self.win_w(), h)
    }
}

// 🔔 the menu of an alert line (right-click, or long press on phone and tablet)
pub(crate) const AL_PAUSE: i64 = 1;
const AL_EDITER: i64 = 2;
const AL_EFFACER: i64 = 3;
pub(crate) const AL_LIGNES: i64 = 4;
pub(crate) const AL_COULEUR: i64 = 5;
const AL_SON: i64 = 6;
/// 🔊 « Son des alertes » (Réglages): ▶ Écouter, volume − and + (the sounds are 0 – 7).
const SA_ECOUTER: i64 = 100;
const SA_MOINS: i64 = 101;
const SA_PLUS: i64 = 102;
/// 🎨 The shared palette open for the alerts' colour (not a drawing's): its « drawing index ».
pub(crate) const COULEUR_ALERTES: usize = usize::MAX;
/// 🔔 The alert window of drawings open to edit a price alert (not on a drawing): its « drawing index ».
pub(crate) const ALERTE_DE_PRIX: usize = usize::MAX;

impl App {
    /// 🔔 The menu of the alert line `id` (Jee, 19 Nov, like TradingView): ⏸ Mettre en pause (▶ Reprendre),
    /// Éditer…, « Son… », 🗑 Effacer; then « Lignes d'alerte » (checked: the full-width lines) and « Modifier la
    /// couleur des alertes… ».
    pub(crate) fn menu_ligne_alerte(&self, id: u64) -> Option<Vec<MenuGroup>> {
        let a = self.alerts.list.iter().find(|a| a.id == id)?;
        let item = |id: i64, label: &str, icon: &'static str, checked: bool| MenuItem {
            id,
            label: label.into(),
            enabled: true,
            icon: Some(icon),
            checked,
            ..Default::default()
        };
        let (pause, pause_ic) = if a.on { ("Mettre en pause", "pause") } else { ("Reprendre", "play") };
        let lignes = self.top.prefs.lignes_alerte;
        Some(vec![
            MenuGroup {
                title: String::new(),
                items: vec![
                    item(AL_PAUSE, pause, pause_ic, false),
                    item(AL_EDITER, "Éditer…", "pencil", false),
                    item(AL_SON, "Son…", "son", false),
                    item(AL_EFFACER, "Effacer", "trash", false),
                ],
                strip: false,
            },
            MenuGroup {
                title: String::new(),
                items: vec![
                    item(AL_LIGNES, "Lignes d'alerte", if lignes { "case-cochee" } else { "case" }, lignes),
                    item(AL_COULEUR, "Modifier la couleur des alertes…", "🎨", false),
                ],
                strip: false,
            },
        ])
    }

    /// 🔔 A line picked in the menu of the alert line `id` (`anchor`: where the menu was opened, window coordinates).
    pub(crate) fn ligne_alerte_choix(&mut self, id: u64, pick: i64, anchor: argos_render::shapes::Rect) {
        match pick {
            AL_PAUSE => {
                if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) {
                    // (as « MES ALERTES »: resumed, it starts again from the next price)
                    a.on = !a.on;
                    (a.fired, a.seen) = (None, None);
                    rearmer_si_expiree(a);
                    self.banner = Some(if a.on { "▶ Alerte reprise" } else { "⏸️ Alerte en pause" }.into());
                    self.save_alerts();
                }
            }
            AL_EDITER => self.ouvrir_edition_alerte(id),
            AL_SON => return self.open_list(ListKind::SonAlerte(id), anchor),
            AL_EFFACER => {
                if let Some((_, nom, ..)) = self.etiquette_de(id) {
                    self.demander_suppression_alerte(id, &nom);
                } else if let Some(nom) = self.alerts.list.iter().find(|a| a.id == id).map(|a| a.text()) {
                    self.demander_suppression_alerte(id, &nom);
                }
            }
            AL_LIGNES => {
                self.top.prefs.lignes_alerte = !self.top.prefs.lignes_alerte;
                self.save_top();
            }
            AL_COULEUR => {
                let cur = if self.top.prefs.couleur_alertes.is_empty() {
                    argos_render::colorpick::join(argos_render::palette::GOLD, 1.0)
                } else {
                    self.top.prefs.couleur_alertes.clone()
                };
                let custom: Vec<String> = Self::read_prefs(crate::actions::drawbar::COULEURS_PERSO);
                let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
                let cp = argos_render::colorpick::ColorPick::new(anchor, &cur, custom, ww, wh);
                self.ui.colorpick = Some((COULEUR_ALERTES, argos_render::objdlg::Which::Line, cp));
            }
            _ => {}
        }
        self.redraw();
    }

    /// 🔔 « Éditer… »: the alert window of drawings, prefilled with this price alert (« Enregistrer »).
    pub(crate) fn ouvrir_edition_alerte(&mut self, id: u64) {
        let Some(a) = self.alerts.list.iter().find(|a| a.id == id) else { return };
        let Kind::Price { price, dir } = a.kind else { return };
        let sym = self.top.symbol.rsplit(':').next().filter(|s| !s.is_empty()).unwrap_or(&a.sym).to_string();
        let dec = argos_render::viewport::data_decimals(&self.chart.bars);
        let (ww, wh) = (self.win_w(), self.area.1 as f64 + self.top_h());
        let tf = self.chart.interval.secs();
        let w = argos_render::drawalert::DrawAlert::editer(
            id,
            &sym,
            tf,
            &self.chart.interval.short(),
            price,
            dir,
            dec,
            &a.msg,
            a.son.as_deref(),
            now_s(),
            ww,
            wh,
        );
        self.ui.list = None;
        self.ui.colorpick = None;
        self.ui.draw_alert = Some((ALERTE_DE_PRIX, w));
    }

    /// 🔔 « Enregistrer »: the same alert (same id, its pause kept) at its new price, condition, message and sound.
    pub(crate) fn alerte_enregistree(&mut self, id: u64, price: f64, dir: Dir, msg: String, son: Option<String>) {
        let sym = self.symbol_label();
        let now_px = self.demo_price(&sym);
        if let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) {
            a.kind = Kind::Price { price, dir };
            (a.msg, a.son) = (msg, son);
            // (the current price is the start again: crossing = going to the other side of the line)
            (a.seen, a.fired) = (now_px, None);
            self.save_alerts();
            self.banner = Some(format!("🔔 Alerte modifiée : {}", argos_render::viewport::format_price(price, 2)));
        }
    }
}

// 🔊 alert sounds
impl App {
    /// 🔊 Plays `son` now at the settings' volume (`None`: the settings' sound).
    pub(crate) fn ecouter_son(&self, son: Option<&str>) {
        let p = &self.top.prefs;
        platform::alert_son(argos_core::alerts::son_connu(son.unwrap_or(&p.son_alertes)), p.volume_alertes);
    }

    /// 🔊 « Son… » of the alert line `id`: « Par défaut » (the settings' sound), the 8 sounds, « Son coupé »; checked
    /// on its choice.
    pub(crate) fn menu_son_alerte(&self, id: u64) -> Option<Vec<MenuGroup>> {
        let a = self.alerts.list.iter().find(|a| a.id == id)?;
        let items = argos_core::alerts::choix_sons()
            .into_iter()
            .enumerate()
            .map(|(k, (label, son))| MenuItem {
                id: k as i64,
                label: if son.is_none() {
                    format!("{label} ({})", self.top.prefs.son_alertes)
                } else {
                    label.into()
                },
                enabled: true,
                checked: a.son.as_deref() == son,
                ..Default::default()
            })
            .collect();
        Some(vec![MenuGroup {
            title: "SON DE L'ALERTE".into(),
            items,
            strip: false,
        }])
    }

    /// 🔊 A choice of « Son… » for the alert `id`: saved in alerts.json, and played (not « Son coupé »).
    pub(crate) fn son_alerte_choix(&mut self, id: u64, pick: i64) {
        let Some((label, son)) = usize::try_from(pick).ok().and_then(|k| argos_core::alerts::choix_sons().get(k).copied()) else {
            return;
        };
        let Some(a) = self.alerts.list.iter_mut().find(|a| a.id == id) else { return };
        a.son = son.map(str::to_string);
        self.save_alerts();
        self.ecouter_son(son);
        self.banner = Some(format!("🔊 Son de l'alerte : {label}"));
    }

    /// 🔊 « Son des alertes » (Réglages): the 8 sounds (checked: the default one), ▶ Écouter, the volume − / +.
    pub(crate) fn menu_son_alertes(&self) -> Vec<MenuGroup> {
        let p = &self.top.prefs;
        let item = |id: i64, label: &str, checked: bool, enabled: bool| MenuItem {
            id,
            label: label.into(),
            enabled,
            checked,
            ..Default::default()
        };
        vec![
            MenuGroup {
                title: "SON DES ALERTES".into(),
                items: argos_core::alerts::SONS
                    .iter()
                    .enumerate()
                    .map(|(k, s)| item(k as i64, s, *s == p.son_alertes, true))
                    .chain(std::iter::once(item(SA_ECOUTER, "▶ Écouter", false, true)))
                    .collect(),
                strip: false,
            },
            MenuGroup {
                title: format!("VOLUME : {} %", p.volume_alertes),
                items: vec![
                    item(SA_MOINS, "− Moins fort", false, p.volume_alertes > 0),
                    item(SA_PLUS, "+ Plus fort", false, p.volume_alertes < 100),
                ],
                strip: false,
            },
        ]
    }

    /// 🔊 A line of « Son des alertes »: a sound (kept, and played), ▶ (played), the volume by 10 (kept, and played).
    pub(crate) fn son_alertes_choix(&mut self, pick: i64) {
        let p = &mut self.top.prefs;
        match pick {
            SA_ECOUTER => {}
            SA_MOINS => p.volume_alertes = p.volume_alertes.saturating_sub(10),
            SA_PLUS => p.volume_alertes = p.volume_alertes.saturating_add(10).min(100),
            k => match usize::try_from(k).ok().and_then(|k| argos_core::alerts::SONS.get(k)) {
                Some(s) => p.son_alertes = s.to_string(),
                None => return,
            },
        }
        if pick != SA_ECOUTER {
            self.save_top();
        }
        self.ecouter_son(None);
    }
}

/// 🔔 Why an alert that is off is off, as its label says it: « (expirée) » when its validity ended, « (déclenchée) »
/// for a once-only alert that already fired, else « (en pause) » (paused by Jee: pausing clears `fired`).
pub(crate) fn raison_arret(a: &argos_core::alerts::Alert, now: i64) -> &'static str {
    if a.until.is_some_and(|u| u <= now) {
        "(expirée)"
    } else if a.every.is_none() && a.fired.is_some() {
        "(déclenchée)"
    } else {
        "(en pause)"
    }
}

/// 🔔 « Reprendre » rearms in every case: an expired alert turned back on loses its past end of validity.
fn rearmer_si_expiree(a: &mut argos_core::alerts::Alert) {
    if a.on && a.until.is_some_and(|u| u <= now_s()) {
        a.until = None;
    }
}
