//! 🎯 The « Stratégie » volet of the right bar (Jee, 23 Nov 2026): as soon as a strategy written in Argos Script (or
//! imported from Pine) is on the chart, its result is there, made again by itself when the chart or its settings
//! change (no « ▶️ Lancer »): its net result, its trades, its profit factor and its biggest drop, the curve of the
//! account, then its trades (the one still open in grey), month by month, and against TradingView's own list.
//! « 📊 Rapport complet » (and ⤢) opens the full report page. A simulation, not financial advice.

use crate::btpage::{couper, raison, VueTv, MONTHS};
use crate::dock::BtView;
use crate::palette as pal;
use crate::palette::{GREY, WHITE};
use crate::portfolio::TEAL;
use crate::shapes::{lines, poly_fill, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::Side;
use argos_script::lang::rapport::{baisse_max_pct, par_mois};
use tiny_skia::Pixmap;

/// What a click in the volet asks of the app.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StratClick {
    None,
    /// Redraw (a tab).
    Changed,
    /// 📜 ▾ The pill: choose among the chart's strategies (its menu under it).
    Choisir(Rect),
    /// 📊 The full report page.
    Rapport,
    /// ⬆ Comparer avec TradingView: the report page asks for TradingView's file.
    Tv,
    /// ⚙ The strategy's settings (its properties: capital, order size, costs).
    Reglages,
    /// 📱 ✕ of the sheet.
    Fermer,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Zone {
    Pilule,
    Onglet(usize),
    Rapport,
    Tv,
    Reglages,
    /// « … et N autres » under the rows: the full report.
    Autres,
    Fermer,
}

/// The tabs, as the mock-up writes them.
pub const ONGLETS: [&str; 3] = ["Trades", "Par mois", "⚖️ TradingView"];
const TAB_TV: usize = 2;
/// Rows shown under a tab (the full report has them all).
pub const RANGS: usize = 8;
const BOUTON_TV: &str = "⬆ Comparer avec TradingView";
const BOUTON_TV_CELL: &str = "⬆ Comparer avec TV";
const BOUTON_RAPPORT: &str = "📊 Rapport complet";
/// 📱 Room left at the bottom for the Argos AI round that stays over the sheets (bottom right).
const BAS_ROND_CELL: f64 = 64.0;

/// The volet's state: the result (from the app), the chart's strategies, the open tab and TradingView's list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct VoletStrategie {
    /// The result of the chosen strategy on the chart (`None`: not computed yet).
    pub vue: Option<BtView>,
    /// The strategies on the chart (their titles) and the chosen one (`id`: its script's id, kept by the app).
    pub noms: Vec<String>,
    pub choisie: usize,
    pub id: String,
    pub onglet: usize,
    /// ⚖️ TradingView's list compared with this result (from the report page).
    pub tv: Option<VueTv>,
    /// The chart's time zone by name (each date with its own summer time) and its offset now.
    pub fuseau: String,
    pub tz: i64,
    /// The chart's timeframe as the app writes it (« 4h »).
    pub unite: String,
}

/// Where everything is.
#[derive(Debug, Clone, PartialEq)]
struct Geo {
    x0: f64,
    w: f64,
    titre_y: f64,
    pilule: Rect,
    sous_y: f64,
    cartes: [Rect; 4],
    courbe: Rect,
    onglets: Vec<Rect>,
    /// Top of the first row (under the tabs; on ⚖️ under its pills).
    rangs_y: f64,
    rang_h: f64,
    n_rangs: usize,
    autres: Option<Rect>,
    rapport: Rect,
    tv: Rect,
    reglages: Option<Rect>,
    fermer: Rect,
    /// 🖥 ⤢: the full report (the mock-up's).
    agrandir: Option<Rect>,
    bas: f64,
}

/// « +$66 526 », « −$8 193 », « $0 »: whole dollars, signed.
fn usd0(v: f64) -> String {
    let s = crate::viewport::format_money(v, 0);
    if v >= 0.5 {
        format!("+{s}")
    } else {
        s
    }
}

fn signe(v: f64) -> Rgb {
    if v >= 0.0 {
        pal::up()
    } else {
        pal::down()
    }
}

/// « 6 janv. » (day and month) at `t` in a zone `dec` seconds from UTC; `an`: with the year.
fn jour(t: i64, dec: i64, an: bool) -> String {
    use chrono::Datelike;
    let d = chrono::DateTime::from_timestamp(t + dec, 0).unwrap_or_default();
    let y = if an { format!(" {}", d.year()) } else { String::new() };
    format!("{} {}{y}", d.day(), MONTHS[d.month0() as usize])
}

impl VoletStrategie {
    /// Seconds from UTC at `t` in the chart's zone (its summer time when it has a name).
    fn decalage(&self, t: i64) -> i64 {
        if self.fuseau.is_empty() {
            return self.tz;
        }
        crate::settings::tz_offset(&self.fuseau, t).unwrap_or(self.tz)
    }

    /// The rows of the open tab, newest first: (cells, all grey: a trade still open).
    fn rangs(&self, etroit: bool) -> Vec<(Vec<(String, Rgb)>, bool)> {
        let Some(v) = &self.vue else { return vec![] };
        let sens = |s: Side| {
            let long = s == Side::Long;
            (if long { "Long" } else { "Short" }.to_string(), if long { pal::up() } else { pal::down() })
        };
        match self.onglet {
            0 => {
                let n = v.detail.lignes.len();
                let ouverts = v.detail.en_cours.iter().enumerate().rev().map(|(k, (s, t, r))| {
                    let quand = format!("{} → en cours", jour(*t, self.decalage(*t), false));
                    (vec![((n + k + 1).to_string(), GREY), (sens(*s).0, GREY), (quand, GREY), (usd0(*r), GREY)], true)
                });
                let fermes = v.detail.lignes.iter().rev().map(|l| {
                    let net = l.net.to_f64();
                    let quand = format!(
                        "{} → {}",
                        jour(l.t_in, self.decalage(l.t_in), false),
                        jour(l.t_out, self.decalage(l.t_out), false)
                    );
                    (vec![(l.numero.to_string(), GREY), sens(l.side), (quand, WHITE), (usd0(net), signe(net))], false)
                });
                ouverts.chain(fermes).collect()
            }
            TAB_TV => {
                let Some(tv) = &self.tv else { return vec![] };
                tv.c.rangees
                    .iter()
                    .rev()
                    .map(|r| {
                        let a = r.argos.and_then(|k| v.detail.lignes.get(k));
                        let side = a.map(|a| a.side).or(r.tv.as_ref().map(|t| t.side)).unwrap_or(Side::Long);
                        let numero = match (a, &r.tv) {
                            (Some(a), _) => a.numero.to_string(),
                            (None, Some(t)) if t.numero > 0 => format!("TV {}", t.numero),
                            _ => "—".into(),
                        };
                        let resultat = a.map_or(("—".to_string(), GREY), |a| (usd0(a.net.to_f64()), signe(a.net.to_f64())));
                        let verdict = if r.pareil() {
                            ("✓ pareil".to_string(), pal::up())
                        } else {
                            (
                                format!("✗ {}", r.ecarts.first().map(|e| raison(e, v.detail.dec, true)).unwrap_or_default()),
                                pal::down(),
                            )
                        };
                        (vec![(numero, GREY), sens(side), verdict, resultat], false)
                    })
                    .collect()
            }
            _ => par_mois(&v.detail.lignes, v.start, self.tz)
                .iter()
                .rev()
                .map(|m| {
                    let net = m.net.to_f64();
                    let mois = format!("{} {}", MONTHS[(m.mois as usize).clamp(1, 12) - 1], m.annee);
                    let trades = if etroit {
                        format!("{} trade{}", m.trades, if m.trades > 1 { "s" } else { "" })
                    } else {
                        format!(
                            "{} trade{} · {} gagnant{}",
                            m.trades,
                            if m.trades > 1 { "s" } else { "" },
                            m.gagnants,
                            if m.gagnants > 1 { "s" } else { "" }
                        )
                    };
                    (
                        vec![
                            (mois, WHITE),
                            (trades, GREY),
                            (argos_core::nombres::format_pct(m.pct), signe(net)),
                            (usd0(net), signe(net)),
                        ],
                        false,
                    )
                })
                .collect(),
        }
    }

    /// Where everything is in a volet `pw` wide (📱 `cell`: the phone's sheet).
    fn geo(&self, text: &Text, pw: f64, cell: bool) -> Geo {
        // (the mock-up's places: 🖥 the header lower, with its own ⤢ ✕ like the Trading sheet; 📱 under the sheet's
        // handle, the bottom cards without their small line)
        let (x0, w) = if cell { (14.0, pw - 28.0) } else { (16.0, pw - 30.0) };
        let (titre_y, pil_y, pil_h) = if cell { (38.0, 52.0, 28.0) } else { (58.0, 80.0, 30.0) };
        let nom = self.nom_pilule();
        let taille = if cell { 13.0 } else { 13.5 };
        let pil_w = (text.width(&nom, taille) as f64 + if cell { 26.0 } else { 28.0 }).min(w);
        let pilule = Rect::new(x0, pil_y, pil_w, pil_h);
        let sous_y = if cell { 97.0 } else { 130.0 };
        let (cartes_y, hauts, ecart, courbe_h) = if cell {
            (108.0, [64.0, 50.0], 7.0, 88.0)
        } else {
            (146.0, [76.0, 76.0], 10.0, 128.0)
        };
        let cw = (w - ecart) / 2.0;
        let carte = |k: usize| {
            let y = cartes_y + if k < 2 { 0.0 } else { hauts[0] + ecart };
            Rect::new(x0 + (k % 2) as f64 * (cw + ecart), y, cw, hauts[k / 2])
        };
        let cartes = [carte(0), carte(1), carte(2), carte(3)];
        let courbe = Rect::new(x0, cartes_y + hauts[0] + hauts[1] + 2.0 * ecart, w, courbe_h);
        let tab_y = courbe.y + courbe.h + 10.0;
        let (ts, tgap) = if cell { (13.0, 18.0) } else { (13.5, 20.0) };
        let mut tx = x0 + 2.0;
        let onglets = ONGLETS
            .iter()
            .map(|o| {
                let tw = text.width(o, ts) as f64;
                let r = Rect::new(tx, tab_y, tw, if cell { 24.0 } else { 26.0 });
                tx += tw + tgap;
                r
            })
            .collect::<Vec<_>>();
        let rang_h = if cell { 24.4 } else { 26.4 };
        let pilules_tv = if self.onglet == TAB_TV && self.tv.is_some() { 34.0 } else { 0.0 };
        let rangs_y = tab_y + if cell { 26.0 } else { 34.0 } + pilules_tv;
        let tous = self.rangs(cell).len();
        let n_rangs = tous.min(RANGS);
        // (no row: one or three lines of words instead)
        let texte_vide = if tous > 0 {
            0.0
        } else if self.onglet == TAB_TV && self.tv.is_none() {
            3.0 * 19.0 + 8.0
        } else {
            30.0
        };
        let mut y = rangs_y + n_rangs as f64 * rang_h + texte_vide;
        let autres = (tous > RANGS).then(|| {
            let r = Rect::new(x0, y + 2.0, w, 22.0);
            y += 24.0;
            r
        });
        y += if cell { 9.0 } else { 14.0 };
        let bh = 34.0;
        let (rapport, tv) = if cell {
            let bw = (w - 7.0) / 2.0;
            (Rect::new(x0, y, bw, bh), Rect::new(x0 + bw + 7.0, y, bw, bh))
        } else {
            let rw = text.width(BOUTON_RAPPORT, 13.0) as f64 + 30.0;
            let tw = text.width(BOUTON_TV, 13.0) as f64 + 30.0;
            (Rect::new(x0, y, rw, bh), Rect::new(x0 + w - tw, y, tw, bh))
        };
        y += bh;
        let reglages = (!cell).then(|| {
            let r = Rect::new(x0, y + 6.0, w, 30.0);
            y += 36.0;
            r
        });
        let fermer = if cell {
            Rect::new(pw - 14.0 - 30.0, 17.0, 30.0, 30.0)
        } else {
            Rect::new(pw - 47.0, 37.0, 30.0, 30.0)
        };
        let agrandir = (!cell).then(|| Rect::new(fermer.x - 38.0, fermer.y, 30.0, 30.0));
        Geo {
            x0,
            w,
            titre_y,
            pilule,
            sous_y,
            cartes,
            courbe,
            onglets,
            rangs_y,
            rang_h,
            n_rangs,
            autres,
            rapport,
            tv,
            reglages,
            fermer,
            agrandir,
            bas: y + if cell { 14.0 + BAS_ROND_CELL } else { 14.0 },
        }
    }

    /// « 📜 CA_-_RSI_Bol_Strat_1.1 ▾ » (▾ when there are several to choose from).
    fn nom_pilule(&self) -> String {
        let nom = self
            .noms
            .get(self.choisie)
            .cloned()
            .or_else(|| self.vue.as_ref().map(|v| v.name.trim_start_matches("📜 ").to_string()))
            .unwrap_or_default();
        format!("📜 {nom}{}", if self.noms.len() > 1 { " ▾" } else { "" })
    }

    fn zones(&self, text: &Text, pw: f64, cell: bool) -> Vec<(Rect, Zone)> {
        let g = self.geo(text, pw, cell);
        let mut z = vec![(g.pilule, Zone::Pilule), (g.rapport, Zone::Rapport), (g.tv, Zone::Tv)];
        z.extend(
            g.onglets
                .iter()
                .enumerate()
                .map(|(k, r)| (Rect::new(r.x - 4.0, r.y - 2.0, r.w + 8.0, r.h + 4.0), Zone::Onglet(k))),
        );
        z.extend(g.reglages.map(|r| (r, Zone::Reglages)));
        z.extend(g.autres.map(|r| (r, Zone::Autres)));
        z.push((g.fermer, Zone::Fermer));
        z.extend(g.agrandir.map(|r| (r, Zone::Autres)));
        z
    }

    /// Height of the content (the volet scrolls when it is taller than the window).
    pub fn content_h(&self, pw: f64, cell: bool) -> f64 {
        self.geo(&Text::default(), pw, cell).bas
    }

    /// A click at (`x`, `y`) of the content.
    pub fn click(&mut self, text: &Text, pw: f64, x: f64, y: f64, cell: bool) -> StratClick {
        let Some((r, z)) = self.zones(text, pw, cell).into_iter().find(|(r, _)| r.contains(x, y)) else {
            return StratClick::None;
        };
        match z {
            Zone::Pilule if self.noms.len() > 1 => StratClick::Choisir(r),
            Zone::Pilule => StratClick::None,
            Zone::Onglet(k) => {
                self.onglet = k;
                StratClick::Changed
            }
            Zone::Rapport | Zone::Autres => StratClick::Rapport,
            Zone::Tv => StratClick::Tv,
            Zone::Reglages => StratClick::Reglages,
            Zone::Fermer => StratClick::Fermer,
        }
    }

    /// Draws the content (`pw` wide, as high as the pixmap) on the volet's background.
    pub fn draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, mouse: Option<(f64, f64)>, cell: bool) {
        let g = self.geo(text, pw, cell);
        let survol = |r: Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        let gras = Text::graisse(Graisse::Bold);
        let moyen = Text::graisse(Graisse::Medium);
        let k = if cell { 0.94 } else { 1.0 };
        // 🎯 Stratégie
        let ts = 16.0;
        text.draw(pm, "🎯", (g.x0 as f32, (g.titre_y) as f32), ts, WHITE, Align::Left);
        gras.draw(pm, "Stratégie", ((g.x0 + 25.0) as f32, g.titre_y as f32), ts, WHITE, Align::Left);
        // ⤢ (the full report) and ✕, the volet's round buttons
        let r = g.fermer;
        round_fill(
            pm,
            r,
            r.h / 2.0,
            crate::jetons::VERRE2.0,
            if survol(r) { 0.16 } else { crate::jetons::VERRE2.1 },
        );
        text.draw(pm, "✕", (r.cx() as f32, (r.cy() + 5.0) as f32), 14.0, crate::jetons::TEXTE, Align::Center);
        if let Some(r) = g.agrandir {
            round_fill(
                pm,
                r,
                r.h / 2.0,
                crate::jetons::VERRE2.0,
                if survol(r) { 0.16 } else { crate::jetons::VERRE2.1 },
            );
            // (⤢ is not in the font: two arrows on a diagonal, drawn like the other volets')
            lines(
                pm,
                r.cx(),
                r.cy(),
                &[
                    &[(-5.0, 5.0), (5.0, -5.0)],
                    &[(1.0, -5.0), (5.0, -5.0), (5.0, -1.0)],
                    &[(-5.0, 1.0), (-5.0, 5.0), (-1.0, 5.0)],
                ],
                crate::jetons::TEXTE,
                1.3,
            );
        }
        // 📜 the strategy (▾: choose)
        let p = g.pilule;
        round_fill(
            pm,
            p,
            p.h / 2.0,
            crate::jetons::ACCENT,
            if survol(p) && self.noms.len() > 1 { 0.32 } else { 0.22 },
        );
        let nom = self.nom_pilule();
        let taille = if cell { 13.0 } else { 13.5 };
        let nom = couper(text, &nom, taille, p.w - 22.0);
        text.draw(
            pm,
            &nom,
            ((p.x + 12.0) as f32, (p.cy() + taille as f64 * 0.36) as f32),
            taille,
            WHITE,
            Align::Left,
        );
        let Some(v) = &self.vue else {
            text.draw(pm, "Calcul en cours…", (g.x0 as f32, g.sous_y as f32), 12.0, GREY, Align::Left);
            return;
        };
        // the chart it ran on
        let bougies = crate::viewport::format_number(v.bars as f64, 0);
        let fin = if cell {
            "se refait tout seul"
        } else {
            "se refait tout seul quand la chart change"
        };
        let sous = [v.symbol.as_str(), self.unite.as_str(), &format!("{bougies} bougies"), fin]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · ");
        let ss = if cell { 11.5 } else { 12.0 };
        text.draw(pm, &couper(text, &sous, ss, g.w), ((g.x0 + 2.0) as f32, g.sous_y as f32), ss, GREY, Align::Left);
        // the four cards
        let r = &v.report;
        let l = &v.detail.lignes;
        let net = r.net_pnl.to_f64();
        let gains: Vec<f64> = l.iter().map(|x| x.net.to_f64()).filter(|x| *x > 0.0).collect();
        let gain_moyen = if gains.is_empty() {
            0.0
        } else {
            gains.iter().sum::<f64>() / gains.len() as f64
        };
        let baisse = if v.detail.equite.is_empty() {
            r.max_drawdown_pct
        } else {
            baisse_max_pct(&v.detail.equite)
        };
        let en_cours = v.detail.en_cours.len();
        let mut trades_sous = format!("{} % gagnants", crate::viewport::format_number(r.win_rate, 1));
        if en_cours > 0 && !cell {
            trades_sous.push_str(&format!(" · {en_cours} en cours"));
        }
        let pf = if l.is_empty() {
            ("—".to_string(), WHITE)
        } else if r.profit_factor.is_finite() {
            (
                crate::viewport::format_number(r.profit_factor, 2),
                if r.profit_factor >= 1.0 { pal::up() } else { pal::down() },
            )
        } else {
            ("∞".to_string(), pal::up())
        };
        let cartes: [(&str, (String, Rgb), String); 4] = [
            (
                "Résultat net",
                (usd0(net), if l.is_empty() { WHITE } else { signe(net) }),
                format!("{} du capital", argos_core::nombres::format_pct(net / v.start.max(1e-9) * 100.0)),
            ),
            ("Trades", (l.len().to_string(), WHITE), trades_sous),
            (
                "Profit factor",
                pf,
                if gains.is_empty() {
                    String::new()
                } else {
                    format!("gain moyen {}", crate::viewport::format_money(gain_moyen, 0))
                },
            ),
            (
                "Baisse max",
                (
                    format!("−{} %", crate::viewport::format_number(baisse, 2)),
                    if baisse > 0.0 { pal::down() } else { WHITE },
                ),
                v.buy_hold_pct
                    .map_or(String::new(), |b| format!("buy & hold {}", argos_core::nombres::format_pct(b))),
            ),
        ];
        let (dy_l, dy_n, dy_s, tl, tn) = if cell {
            (18.0, 40.0, 56.0, 11.5, 17.0)
        } else {
            (21.0, 47.0, 66.0, 12.0, 19.0)
        };
        for (c, (titre, (valeur, couleur), sous)) in g.cartes.iter().zip(cartes.iter()) {
            carte(pm, *c);
            let x = (c.x + if cell { 11.0 } else { 14.0 }) as f32;
            text.draw(pm, titre, (x, (c.y + dy_l) as f32), tl, GREY, Align::Left);
            moyen.draw(pm, valeur, (x, (c.y + dy_n) as f32), tn, *couleur, Align::Left);
            if !sous.is_empty() && c.h > 56.0 {
                let s = couper(text, sous, tl, c.w - 2.0 * (x as f64 - c.x));
                text.draw(pm, &s, (x, (c.y + dy_s) as f32), tl, GREY, Align::Left);
            }
        }
        // the curve of the account, from the first trade
        self.draw_courbe(pm, text, g.courbe, v, cell);
        // the tabs
        let ts = if cell { 13.0 } else { 13.5 };
        let base = g.onglets.first().map_or(0.0, |r| r.y + r.h);
        crate::fill(pm, g.x0, base, g.w, 1.0, pal::edge(), 1.0);
        for (k, r) in g.onglets.iter().enumerate() {
            let on = k == self.onglet;
            let c = if on || survol(*r) { WHITE } else { GREY };
            text.draw(pm, ONGLETS[k], (r.x as f32, (r.y + r.h - 8.0) as f32), ts, c, Align::Left);
            if on {
                crate::fill(pm, r.x, base - 1.0, r.w, 2.0, TEAL, 1.0);
            }
        }
        // ⚖️ its pills
        if self.onglet == TAB_TV {
            if let Some(tv) = &self.tv {
                let y = base + 8.0;
                let mut x = g.x0;
                let pluriel = |n: usize, un: &str, plusieurs: &str| if n > 1 { plusieurs.to_string() } else { un.to_string() };
                let c = &tv.c;
                for (s, coul) in [
                    (format!("✓ {} {}", c.pareils, pluriel(c.pareils, "pareil", "pareils")), pal::up()),
                    (format!("✗ {} {}", c.differents, pluriel(c.differents, "différent", "différents")), pal::down()),
                ] {
                    if s.starts_with("✗ 0 ") {
                        continue;
                    }
                    let w = text.width(&s, 12.5) as f64 + 20.0;
                    let r = Rect::new(x, y, w, 24.0);
                    round_fill(pm, r, 12.0, coul, 0.13);
                    round_stroke(pm, r, 12.0, coul, 0.45);
                    text.draw(pm, &s, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, coul, Align::Center);
                    x += w + 8.0;
                }
                let reste = couper(text, &format!("sur {} · {}", c.rangees.len(), tv.fichier), 12.0, g.x0 + g.w - x - 4.0);
                text.draw(pm, &reste, ((x + 2.0) as f32, (y + 16.5) as f32), 12.0, GREY, Align::Left);
            }
        }
        // the rows
        let rangs = self.rangs(cell);
        let rs = if cell { 12.5 } else { 13.0 };
        let cols = if self.onglet == 1 {
            // (month · trades · % · result)
            [
                g.x0 + 2.0,
                g.x0 + if cell { 84.0 } else { 94.0 },
                g.x0 + g.w - if cell { 78.0 } else { 92.0 },
                g.x0 + g.w - 2.0,
            ]
        } else {
            [g.x0 + 2.0, g.x0 + 30.0 * k, g.x0 + if cell { 76.0 } else { 88.0 }, g.x0 + g.w - 2.0]
        };
        for (i, (cells, _)) in rangs.iter().take(g.n_rangs).enumerate() {
            let y = g.rangs_y + i as f64 * g.rang_h + g.rang_h * 0.68;
            for (j, (s, c)) in cells.iter().enumerate() {
                let (x, a) = match (self.onglet, j) {
                    (1, 2) | (_, 3) => (cols[j], Align::Right),
                    _ => (cols[j], Align::Left),
                };
                // (a cell never runs over the next one: ⚖️ what differs is cut before the result)
                let place = match j {
                    2 if self.onglet != 1 => cols[3] - cols[2] - if cell { 74.0 } else { 86.0 },
                    _ => g.w,
                };
                let s = couper(text, s, rs, place);
                text.draw(pm, &s, (x as f32, y as f32), rs, *c, a);
            }
        }
        if rangs.is_empty() {
            let mots: &[&str] = match (self.onglet, &self.tv) {
                (TAB_TV, None) => &[
                    "Compare chaque trade avec ceux de TradingView :",
                    "exporte sa liste des trades en CSV,",
                    "puis touche « ⬆ Comparer avec TradingView ».",
                ],
                (1, _) => &["Aucun mois avec un trade fermé."],
                (TAB_TV, Some(_)) => &["Aucun trade à comparer sur la même période."],
                _ => &["Aucun trade pour l'instant sur ces bougies."],
            };
            for (i, m) in mots.iter().enumerate() {
                let m = if cell { m.replace("TradingView ».", "TV ».") } else { m.to_string() };
                text.draw(
                    pm,
                    &couper(text, &m, rs, g.w),
                    ((g.x0 + 2.0) as f32, (g.rangs_y + 18.0 + i as f64 * 19.0) as f32),
                    rs,
                    GREY,
                    Align::Left,
                );
            }
        }
        if let Some(r) = g.autres {
            let n = rangs.len() - g.n_rangs;
            let s = format!("… et {n} autre{} dans le rapport complet ›", if n > 1 { "s" } else { "" });
            text.draw(
                pm,
                &s,
                ((r.x + 2.0) as f32, (r.y + 15.0) as f32),
                12.0,
                if survol(r) { WHITE } else { GREY },
                Align::Left,
            );
        }
        // the buttons
        let b = g.rapport;
        round_fill(pm, b, b.h / 2.0, TEAL, if survol(b) { 0.26 } else { 0.16 });
        round_stroke(pm, b, b.h / 2.0, TEAL, 1.0);
        let bs = if cell { 12.5 } else { 13.0 };
        text.draw(
            pm,
            BOUTON_RAPPORT,
            (b.cx() as f32, (b.cy() + bs as f64 * 0.36) as f32),
            bs,
            Rgb(0xbf, 0xf5, 0xec),
            Align::Center,
        );
        let b = g.tv;
        round_fill(pm, b, b.h / 2.0, crate::jetons::VERRE2.0, if survol(b) { 0.12 } else { 0.05 });
        round_stroke(pm, b, b.h / 2.0, if survol(b) { TEAL } else { pal::edge() }, 1.0);
        let s = if cell { BOUTON_TV_CELL } else { BOUTON_TV };
        text.draw(pm, s, (b.cx() as f32, (b.cy() + bs as f64 * 0.36) as f32), bs, WHITE, Align::Center);
        // ⚙ all the settings
        if let Some(r) = g.reglages {
            if survol(r) {
                round_fill(pm, r, 10.0, crate::jetons::VERRE2.0, crate::jetons::VERRE2.1);
            }
            let y = (r.cy() + 5.0) as f32;
            crate::phosphor::draw(pm, "gear", r.x + 12.0, r.cy(), 17, crate::jetons::TEXTE);
            text.draw(pm, "Tous les réglages", ((r.x + 28.0) as f32, y), 13.5, crate::jetons::TEXTE, Align::Left);
            let x = r.x + 28.0 + text.width("Tous les réglages", 13.5) as f64 + 10.0;
            text.draw(pm, "capital, taille, frais", (x as f32, y), 12.0, GREY, Align::Left);
            lines(pm, r.x + r.w - 10.0, r.cy(), &[&[(-2.0, -4.0), (2.0, 0.0), (-2.0, 4.0)]], GREY, 1.4);
        }
    }

    /// The account's curve (the account at each bar's close, its open trade counted), from the first trade's bar.
    fn draw_courbe(&self, pm: &mut Pixmap, text: &Text, c: Rect, v: &BtView, cell: bool) {
        carte(pm, c);
        let tl = if cell { 11.5 } else { 12.0 };
        let (dy, pad) = if cell { (17.0, 9.0) } else { (20.0, 10.0) };
        text.draw(pm, "Courbe du compte", ((c.x + pad + 2.0) as f32, (c.y + dy) as f32), tl, GREY, Align::Left);
        let debut = v
            .detail
            .lignes
            .first()
            .map(|l| l.bar_in)
            .into_iter()
            .chain(v.detail.en_cours.iter().filter_map(|(_, t, _)| v.detail.temps.iter().position(|x| x == t)))
            .min();
        let e = &v.detail.equite;
        let Some(debut) = debut.filter(|d| e.len() > d + 1) else {
            return;
        };
        if let Some(t) = v.detail.temps.get(debut) {
            let s = format!("depuis le {}", jour(*t, self.decalage(*t), true));
            text.draw(pm, &s, ((c.x + c.w - pad - 2.0) as f32, (c.y + dy) as f32), tl, GREY, Align::Right);
        }
        let e = &e[debut..];
        let zone = Rect::new(c.x + pad, c.y + dy + 9.0, c.w - 2.0 * pad, c.h - dy - 9.0 - pad + 2.0);
        let n = e.len();
        let (lo, hi) = e.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), p| (a.min(*p), b.max(*p)));
        let (lo, hi) = if hi > lo {
            (lo - (hi - lo) * 0.06, hi + (hi - lo) * 0.06)
        } else {
            (lo - 1.0, hi + 1.0)
        };
        let x = |i: usize| (zone.x + i as f64 / (n - 1) as f64 * zone.w) as f32;
        let y = |p: f64| (zone.y + (hi - p) / (hi - lo) * zone.h) as f32;
        let pas = (n / (zone.w.max(1.0) as usize * 2)).max(1);
        let ligne: Vec<(f32, f32)> = (0..n).step_by(pas).chain([n - 1]).map(|i| (x(i), y(e[i]))).collect();
        let couleur = signe(e[n - 1] - e[0]);
        let mut aire = ligne.clone();
        aire.push((x(n - 1), (zone.y + zone.h) as f32));
        aire.push((x(0), (zone.y + zone.h) as f32));
        poly_fill(pm, 0.0, 0.0, &aire, couleur, 0.18);
        lines(pm, 0.0, 0.0, &[&ligne], couleur, 1.5);
        // the start: a dotted line at the capital
        let yc = y(v.start) as f64;
        if (zone.y..=zone.y + zone.h).contains(&yc) {
            let mut xx = zone.x;
            while xx < zone.x + zone.w {
                crate::fill(pm, xx, yc, 3.0, 1.0, GREY, 0.5);
                xx += 7.0;
            }
        }
    }
}

/// A card of the volet: a slightly lighter glass with its thin edge.
fn carte(pm: &mut Pixmap, r: Rect) {
    round_fill(pm, r, 12.0, crate::jetons::CARTE.0, crate::jetons::CARTE.1 * 1.6);
    round_stroke(pm, r, 12.0, crate::jetons::sur_fond(crate::jetons::LIGNE), 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dock::BtDetail;
    use argos_core::backtest::rapport;
    use argos_core::portfolio::ClosedTrade;
    use argos_core::Money;
    use argos_script::lang::rapport::Ligne;

    /// A strategy's result: `n` trades (one a day from 1 Nov 2025), won and lost in turn, and a long still open.
    fn volet(n: usize) -> VoletStrategie {
        let t0 = 1_761_955_200; // 2025-11-01 00:00 UTC
        let trades: Vec<ClosedTrade> = (0..n)
            .map(|k| ClosedTrade {
                base: "BTC".into(),
                side: if k % 2 == 0 { Side::Long } else { Side::Short },
                qty: 1.0,
                entry: 100.0,
                exit: 110.0,
                pnl: Money::from_f64(if k % 2 == 0 { 10.0 } else { -5.0 }),
                fees: Money::ZERO,
                opened_at: t0 + k as i64 * 86_400,
                closed_at: t0 + k as i64 * 86_400 + 43_200,
                reason: "X".into(),
                risk: Money::ZERO,
                partiel: false,
                glissement: Money::ZERO,
                funding: Money::ZERO,
                levier: None,
                plan: None,
                real_closed_at: None,
            })
            .collect();
        let net: Money = trades.iter().map(|t| t.net()).sum();
        let mut cumul = Money::ZERO;
        let lignes = trades
            .iter()
            .enumerate()
            .map(|(k, t)| {
                cumul += t.net();
                Ligne {
                    numero: k + 1,
                    side: t.side,
                    entree: "L".into(),
                    sortie: "X".into(),
                    bar_in: 2 * k,
                    bar_out: 2 * k + 1,
                    t_in: t.opened_at,
                    t_out: t.closed_at,
                    prix_in: t.entry,
                    prix_out: t.exit,
                    qty: t.qty,
                    net: t.net(),
                    net_pct: t.net().to_f64(),
                    cumul,
                    run_up: 0.0,
                    drawdown: 0.0,
                }
            })
            .collect();
        let temps: Vec<i64> = (0..2 * n as i64 + 2).map(|k| t0 + k * 43_200).collect();
        let equite = (0..temps.len()).map(|k| 1000.0 + if k % 4 == 1 { 10.0 } else { 3.0 } * k as f64).collect();
        let vue = BtView {
            name: "📜 Ma stratégie".into(),
            settings: String::new(),
            frais: String::new(),
            symbol: "BTC".into(),
            bars: temps.len(),
            start: 1000.0,
            report: rapport(1000.0, Money::from_f64(1000.0) + net, trades, &[]),
            buy_hold_pct: Some(-9.25),
            hasard: None,
            script: true,
            detail: BtDetail {
                lignes,
                equite,
                en_cours: vec![(Side::Long, temps[temps.len() - 2], -26.0)],
                temps,
                dec: 2,
            },
            ouverte: Some(-26.0),
        };
        VoletStrategie {
            vue: Some(vue),
            noms: vec!["Ma stratégie".into()],
            fuseau: "UTC".into(),
            unite: "4h".into(),
            ..Default::default()
        }
    }

    /// Its rows: the open trade first (grey, « → en cours »), then the closed ones, newest first; at most 8 under the
    /// tab, the others in the full report (« … et N autres »); month by month; ⚖️ without TradingView's list, words
    /// that say how.
    #[test]
    fn rangs_du_volet() {
        let mut v = volet(12);
        let r = v.rangs(false);
        assert_eq!(r.len(), 13);
        assert!(r[0].1 && r[0].0[0].0 == "13" && r[0].0[2].0.ends_with("→ en cours") && r[0].0[3].0 == "−$26");
        assert_eq!((r[1].0[0].0.as_str(), r[1].0[1].0.as_str(), r[1].0[3].0.as_str()), ("12", "Short", "−$5"));
        assert_eq!(r[2].0[3].0, "+$10");
        assert_eq!(r[12].0[2].0, "1 nov. → 1 nov.");
        let t = Text::default();
        let g = v.geo(&t, 420.0, false);
        assert_eq!(g.n_rangs, RANGS);
        assert!(g.autres.is_some(), "the others: in the full report");
        v.onglet = 1;
        let m = v.rangs(false);
        assert_eq!(m.len(), 1, "one month");
        assert_eq!(m[0].0[0].0, "nov. 2025");
        v.onglet = TAB_TV;
        assert!(v.rangs(false).is_empty());
        assert!(v.geo(&t, 420.0, false).autres.is_none());
    }

    /// 🖥 Everything fits in the volet of a 900-high window (no scroll), 📱 the sheet scrolls a little; every button
    /// says what it asks; ▾ only with several strategies.
    #[test]
    fn clics_du_volet() {
        let t = Text::default();
        let mut v = volet(7);
        assert!(v.content_h(420.0, false) <= 828.0, "{}", v.content_h(420.0, false));
        for cell in [false, true] {
            let pw = if cell { 402.0 } else { 420.0 };
            let g = v.geo(&t, pw, cell);
            let c = |v: &mut VoletStrategie, r: Rect| v.click(&t, pw, r.cx(), r.cy(), cell);
            assert_eq!(c(&mut v, g.rapport), StratClick::Rapport);
            assert_eq!(c(&mut v, g.tv), StratClick::Tv);
            assert_eq!(c(&mut v, g.fermer), StratClick::Fermer);
            assert_eq!(c(&mut v, g.pilule), StratClick::None, "one strategy: nothing to choose");
            assert_eq!(c(&mut v, g.onglets[1]), StratClick::Changed);
            assert_eq!(v.onglet, 1);
            v.onglet = 0;
            if let Some(r) = g.agrandir {
                assert_eq!(c(&mut v, r), StratClick::Rapport, "⤢: the full report");
            }
            if let Some(r) = g.reglages {
                assert_eq!(c(&mut v, r), StratClick::Reglages);
            }
            assert_eq!(g.reglages.is_none() && g.agrandir.is_none(), cell);
            let mut pm = crate::px::new(pw as u32, v.content_h(pw, cell) as u32).unwrap();
            v.draw(&mut pm, &t, pw, Some((g.rapport.cx(), g.rapport.cy())), cell);
        }
        v.noms.push("Autre".into());
        assert!(v.nom_pilule().ends_with(" ▾"));
        let g = v.geo(&t, 420.0, false);
        assert!(matches!(v.click(&t, 420.0, g.pilule.cx(), g.pilule.cy(), false), StratClick::Choisir(_)));
        // not computed yet: its name, « Calcul en cours… »
        v.vue = None;
        let mut pm = crate::px::new(420, 300).unwrap();
        v.draw(&mut pm, &t, 420.0, None, false);
    }
}
