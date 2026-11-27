//! Fiche d'un trade du journal : champs, notes, clics et dessin.

use super::*;

/// Ce qu'un clic (ou une touche) dans la fiche demande à la fenêtre.
#[derive(Debug, Clone, PartialEq)]
pub enum FormClick {
    None,
    Changed,
    Close,
    /// Enregistrer ce trade (nouveau ou modifié).
    Save(Box<JTrade>),
    /// Supprimer le trade de cet id.
    Delete(String),
    /// Message à montrer (champ manquant).
    Warn(&'static str),
}

/// Champs de la fiche, dans l'ordre de v1 : (étiquette, genre).
#[derive(Debug, Clone, Copy, PartialEq)]
enum F {
    Text,
    Num,
    Side,
    Rate,
    Date,
}

const FIELDS: [(&str, F); 14] = [
    ("Symbole", F::Text),
    ("Sens", F::Side),
    ("Unité de temps", F::Text),
    ("Stratégie", F::Text),
    ("Entrée", F::Num),
    ("Stop loss", F::Num),
    ("Take profit", F::Num),
    ("Sortie (vide = ouvert)", F::Num),
    ("Risque $", F::Num),
    ("Frais $", F::Num),
    ("P&L réel $ (optionnel)", F::Num),
    ("Note /5", F::Rate),
    ("Entrée le", F::Date),
    ("Sortie le", F::Date),
];
/// Index du champ Notes (après les 14 champs).
const NOTES: usize = 14;
const FW: f64 = 760.0;
const FH: f64 = 700.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FHit {
    Close,
    Field(usize),
    /// ▾ de Stratégie : stratégie suivante de la liste.
    NextStrat,
    Conf(usize),
    Emo(usize),
    Delete,
    Cancel,
    Save,
}

/// La fiche ouverte (fenêtre par-dessus l'app).
#[derive(Debug, Clone, PartialEq)]
pub struct JForm {
    pub rect: Rect,
    trade: JTrade,
    pub is_new: bool,
    /// Texte des champs (index de `FIELDS`, puis les notes).
    vals: Vec<String>,
    focus: Option<usize>,
    strats: Vec<String>,
    armed: bool,
}

fn num_txt(v: Option<f64>) -> String {
    v.map(|x| {
        let s = format!("{x:.8}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    })
    .unwrap_or_default()
}

impl JForm {
    /// Fiche de `t` (nouveau trade si `is_new`), centrée dans la fenêtre `win_w` × `win_h`.
    pub fn new(t: JTrade, is_new: bool, strats: Vec<String>, win_w: f64, win_h: f64) -> Self {
        let rect = Rect::new(
            ((win_w - FW) / 2.0).max(6.0),
            ((win_h - FH) / 2.0).max(6.0),
            FW.min(win_w - 12.0),
            FH.min(win_h - 12.0),
        );
        let vals = vec![
            t.sym.clone(),
            String::new(),
            t.tf.clone(),
            t.strat.clone(),
            num_txt(t.entry),
            num_txt(t.sl),
            num_txt(t.tp),
            num_txt(t.exit),
            num_txt(t.risk),
            num_txt(t.fees),
            num_txt(t.pnl),
            String::new(),
            date_txt(t.entry_ts),
            t.exit_ts.map(date_txt).unwrap_or_default(),
            t.notes.clone(),
        ];
        Self {
            rect,
            trade: t,
            is_new,
            vals,
            focus: None,
            strats,
            armed: false,
        }
    }

    /// Le trade tel que la fiche le montre (champs relus).
    pub fn trade(&self) -> JTrade {
        let mut t = self.trade.clone();
        let n = |k: usize| self.vals[k].trim().replace(',', ".").parse::<f64>().ok().filter(|v| v.is_finite());
        t.sym = self.vals[0].trim().to_string();
        t.tf = self.vals[2].trim().to_string();
        t.strat = self.vals[3].trim().to_string();
        (t.entry, t.sl, t.tp, t.exit, t.risk, t.fees, t.pnl) = (n(4), n(5), n(6), n(7), n(8), n(9), n(10));
        t.entry_ts = parse_date(&self.vals[12]).unwrap_or(t.entry_ts);
        t.exit_ts = parse_date(&self.vals[13]);
        if t.exit.is_some() && t.exit_ts.is_none() {
            t.exit_ts = Some(now_ms()); // sortie notée sans date : maintenant (comme v1)
        }
        t.notes = self.vals[NOTES].clone();
        t
    }

    fn cols(&self) -> (f64, f64) {
        let inner = self.rect.w - 48.0;
        (inner, (inner - 2.0 * 12.0) / 3.0)
    }

    /// Zones de la fiche (coordonnées de la fenêtre de l'app) ; aussi le haut des confluences et des émotions.
    pub(super) fn zones(&self, text: &Text) -> (Vec<(Rect, FHit)>, f64, f64, f64) {
        let r = self.rect;
        let (inner, cw) = self.cols();
        let mut z = vec![(Rect::new(r.x + r.w - 44.0, r.y + 14.0, 30.0, 30.0), FHit::Close)];
        let x0 = r.x + 24.0;
        let top = r.y + 64.0;
        for k in 0..FIELDS.len() {
            let fr = Rect::new(x0 + (k % 3) as f64 * (cw + 12.0), top + (k / 3) as f64 * 54.0 + 18.0, cw, 30.0);
            z.push((fr, FHit::Field(k)));
            if k == 3 {
                z.push((Rect::new(fr.x + fr.w - 28.0, fr.y, 28.0, 30.0), FHit::NextStrat));
            }
        }
        let calc_y = top + 5.0 * 54.0 + 4.0;
        // confluences, puis émotions : des pastilles qui passent à la ligne
        let mut y = calc_y + 40.0 + 20.0;
        let conf_y = y - 20.0;
        let chips = |labels: &mut dyn Iterator<Item = (String, FHit)>, y: &mut f64, z: &mut Vec<(Rect, FHit)>| {
            let mut x = x0;
            for (l, hit) in labels {
                let w = text.width(&l, 12.5) as f64 + 22.0;
                if x + w > x0 + inner {
                    x = x0;
                    *y += 34.0;
                }
                z.push((Rect::new(x, *y, w, 28.0), hit));
                x += w + 6.0;
            }
            *y += 34.0;
        };
        chips(&mut CONF.iter().enumerate().map(|(k, c)| (c.to_string(), FHit::Conf(k))), &mut y, &mut z);
        let emo_y = y + 4.0;
        y += 24.0;
        chips(&mut EMOS.iter().enumerate().map(|(k, e)| (e.1.to_string(), FHit::Emo(k))), &mut y, &mut z);
        z.push((Rect::new(x0, y + 6.0, inner, (r.y + r.h - 64.0 - y - 12.0).max(40.0)), FHit::Field(NOTES)));
        let fy = r.y + r.h - 52.0;
        if !self.is_new {
            z.push((Rect::new(x0, fy, if self.armed { 200.0 } else { 110.0 }, 36.0), FHit::Delete));
        }
        z.push((Rect::new(r.x + r.w - 24.0 - 130.0 - 8.0 - 100.0, fy, 100.0, 36.0), FHit::Cancel));
        z.push((Rect::new(r.x + r.w - 24.0 - 130.0, fy, 130.0, 36.0), FHit::Save));
        (z, calc_y, conf_y, emo_y)
    }

    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    pub fn click(&mut self, x: f64, y: f64, text: &Text) -> FormClick {
        let (z, ..) = self.zones(text);
        // ▾ et les boutons d'abord (par-dessus leur champ)
        let hit = z
            .iter()
            .filter(|(r, _)| r.contains(x, y))
            .map(|(_, h)| *h)
            .min_by_key(|h| matches!(h, FHit::Field(_)));
        if hit != Some(FHit::Delete) {
            self.armed = false;
        }
        self.focus = None;
        match hit {
            Some(FHit::Close) | Some(FHit::Cancel) => FormClick::Close,
            Some(FHit::Save) => self.save(),
            Some(FHit::Delete) if self.armed => FormClick::Delete(self.trade.id.clone()),
            Some(FHit::Delete) => {
                self.armed = true;
                FormClick::Changed
            }
            Some(FHit::NextStrat) if !self.strats.is_empty() => {
                let k = self.strats.iter().position(|s| *s == self.vals[3]).map_or(0, |k| (k + 1) % self.strats.len());
                self.vals[3] = self.strats[k].clone();
                FormClick::Changed
            }
            Some(FHit::Field(k)) => {
                match FIELDS.get(k).map(|f| f.1) {
                    Some(F::Side) => self.trade.side = if self.trade.short() { "long".into() } else { "short".into() },
                    Some(F::Rate) => {
                        self.trade.rate = match self.trade.rate.map(|r| r as u8) {
                            Some(5) => None,
                            Some(r) => Some((r + 1) as f64),
                            None => Some(1.0),
                        }
                    }
                    _ => self.focus = Some(k),
                }
                FormClick::Changed
            }
            Some(FHit::Conf(k)) => {
                let c = CONF[k].to_string();
                if self.trade.conf.contains(&c) {
                    self.trade.conf.retain(|x| *x != c);
                } else {
                    self.trade.conf.push(c);
                }
                FormClick::Changed
            }
            Some(FHit::Emo(k)) => {
                self.trade.emo = if self.trade.emo == EMOS[k].0 { String::new() } else { EMOS[k].0.into() };
                FormClick::Changed
            }
            _ => FormClick::Changed,
        }
    }

    fn save(&mut self) -> FormClick {
        let t = self.trade();
        if t.entry.is_none() || t.sl.is_none() {
            return FormClick::Warn("Il faut au moins l'entrée et le stop loss");
        }
        if [12, 13].iter().any(|&k| !self.vals[k].trim().is_empty() && parse_date(&self.vals[k]).is_none()) {
            return FormClick::Warn("Date invalide : écris-la comme AAAA-MM-JJ HH:MM");
        }
        FormClick::Save(Box::new(t))
    }

    pub fn type_char(&mut self, c: char) {
        if let Some(k) = self.focus {
            if c != '\n' && c != '\r' && !c.is_control() {
                self.vals[k].push(c);
            }
        }
    }

    pub fn backspace(&mut self) {
        if let Some(k) = self.focus {
            self.vals[k].pop();
        }
    }

    /// Tab : champ suivant (les champs à taper seulement).
    pub fn tab(&mut self) {
        let typed: Vec<usize> = (0..=NOTES)
            .filter(|&k| !matches!(FIELDS.get(k).map(|f| f.1), Some(F::Side | F::Rate)))
            .collect();
        let at = self.focus.and_then(|f| typed.iter().position(|&k| k == f)).map_or(0, |p| (p + 1) % typed.len());
        self.focus = Some(typed[at]);
    }

    /// Entrée : enregistrer.
    pub fn enter(&mut self) -> FormClick {
        self.focus = None;
        self.save()
    }

    /// Échap : quitte le champ en cours, sinon ferme (renvoie `true`).
    pub fn escape(&mut self) -> bool {
        self.focus.take().is_none()
    }

    pub fn draw(&self, text: &Text, mouse: (f64, f64)) -> Option<(Pixmap, Rect)> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w as u32, r.h as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::jetons::fond_fenetre(&mut pm, loc(r), 12.0);
        round_stroke(&mut pm, loc(r), 12.0, pal::edge(), 1.0);
        let t = self.trade();
        let title = if self.is_new {
            "📓 Nouveau trade".to_string()
        } else {
            format!("📓 Trade {}", t.sym)
        };
        text.draw(&mut pm, &title, (24.0, 40.0), 19.0, WHITE, Align::Left);
        let (z, calc_y, conf_y, emo_y) = self.zones(text);
        let hover = |q: &Rect| q.contains(mouse.0, mouse.1);
        for (k, (name, _)) in FIELDS.iter().enumerate() {
            if let Some((q, _)) = z.iter().find(|(_, h)| *h == FHit::Field(k)) {
                text.draw(&mut pm, name, ((q.x - r.x) as f32, (q.y - r.y - 6.0) as f32), 12.0, GREY, Align::Left);
            }
        }
        // ligne de calcul (comme `jCalc`)
        let calc = [
            ("RR prévu", t.plan().map_or("—".into(), |p| format!("{p:.2}")), WHITE),
            ("Résultat", r_txt(t.r()), tone(t.r())),
            ("P&L", money(t.pnl()), tone(t.pnl())),
            ("Session", sess_of(t.entry_ts).to_string(), WHITE),
        ];
        let mut cx = 24.0;
        let cr = Rect::new(24.0, calc_y - r.y, r.w - 48.0, 32.0);
        round_fill(&mut pm, cr, 6.0, pal::field(), 1.0);
        for (name, val, c) in calc {
            text.draw(&mut pm, name, ((cx + 12.0) as f32, (cr.cy() + 5.0) as f32), 13.0, GREY, Align::Left);
            let nw = text.width(name, 13.0) as f64 + 18.0;
            text.draw(&mut pm, &val, ((cx + nw) as f32, (cr.cy() + 5.0) as f32), 13.0, c, Align::Left);
            cx += nw + text.width(&val, 13.0) as f64 + 26.0;
        }
        text.draw(&mut pm, "Confluences", (24.0, (conf_y - r.y + 14.0) as f32), 12.5, GREY, Align::Left);
        text.draw(&mut pm, "Émotion", (24.0, (emo_y - r.y + 14.0) as f32), 12.5, GREY, Align::Left);
        for (q, h) in &z {
            let l = loc(*q);
            match *h {
                FHit::Close => lines(&mut pm, l.cx(), l.cy(), &[&[(-6.0, -6.0), (6.0, 6.0)], &[(6.0, -6.0), (-6.0, 6.0)]], GREY, 1.6),
                FHit::Field(k) => {
                    let on = self.focus == Some(k);
                    round_fill(&mut pm, l, 6.0, pal::field(), 1.0);
                    round_stroke(
                        &mut pm,
                        l,
                        6.0,
                        if on {
                            crate::palette::accent()
                        } else if hover(q) {
                            GREY
                        } else {
                            pal::edge()
                        },
                        1.0,
                    );
                    let (s, c) = match FIELDS.get(k).map(|f| f.1) {
                        Some(F::Side) => (
                            if t.short() { "▼ Short" } else { "▲ Long" }.to_string(),
                            if t.short() { crate::palette::down() } else { crate::palette::up() },
                        ),
                        Some(F::Rate) => (t.rate.map_or("—".into(), |r| "⭐".repeat(r as usize)), WHITE),
                        _ if self.vals[k].is_empty() && !on => (
                            match k {
                                10 => "calculé avec le R",
                                NOTES => "Notes : pourquoi l'entrée, gestion, leçon…",
                                12 | 13 => "AAAA-MM-JJ HH:MM",
                                _ => "",
                            }
                            .to_string(),
                            crate::palette::DIM,
                        ),
                        _ => (format!("{}{}", self.vals[k], if on { "|" } else { "" }), WHITE),
                    };
                    if k == NOTES {
                        for (j, line) in wrap(text, &s, 13.5, l.w - 20.0).iter().enumerate().take(((l.h - 8.0) / 19.0).max(1.0) as usize) {
                            text.draw(
                                &mut pm,
                                line,
                                ((l.x + 10.0) as f32, (l.y + 20.0 + j as f64 * 19.0) as f32),
                                13.5,
                                c,
                                Align::Left,
                            );
                        }
                    } else {
                        let room = l.w - if k == 3 { 40.0 } else { 18.0 };
                        let shown = if on && text.width(&s, 13.5) as f64 > room {
                            text.fit(&s.chars().rev().collect::<String>(), 13.5, room).chars().rev().collect()
                        } else {
                            text.fit(&s, 13.5, room)
                        };
                        text.draw(&mut pm, &shown, ((l.x + 10.0) as f32, (l.cy() + 5.0) as f32), 13.5, c, Align::Left);
                    }
                }
                FHit::NextStrat => lines(
                    &mut pm,
                    l.cx(),
                    l.cy(),
                    &[&[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]],
                    if hover(q) { WHITE } else { GREY },
                    1.4,
                ),
                FHit::Conf(_) | FHit::Emo(_) => {
                    let (label, on) = match *h {
                        FHit::Conf(k) => (CONF[k], t.conf.iter().any(|c| c == CONF[k])),
                        FHit::Emo(k) => (EMOS[k].1, t.emo == EMOS[k].0),
                        _ => unreachable!(),
                    };
                    round_fill(
                        &mut pm,
                        l,
                        14.0,
                        if on {
                            pal::on()
                        } else if hover(q) {
                            pal::hover()
                        } else {
                            pal::panel()
                        },
                        1.0,
                    );
                    round_stroke(&mut pm, l, 14.0, if on { crate::palette::accent() } else { pal::edge() }, 1.0);
                    text.draw(&mut pm, label, (l.cx() as f32, (l.cy() + 4.5) as f32), 12.5, WHITE, Align::Center);
                }
                FHit::Delete => {
                    round_fill(&mut pm, l, 8.0, crate::palette::down(), if hover(q) { 0.3 } else { 0.18 });
                    round_stroke(&mut pm, l, 8.0, crate::palette::down(), 1.0);
                    text.draw(
                        &mut pm,
                        if self.armed { "Confirmer la suppression" } else { "Supprimer" },
                        (l.cx() as f32, (l.cy() + 5.0) as f32),
                        13.5,
                        WHITE,
                        Align::Center,
                    );
                }
                FHit::Cancel => {
                    round_fill(&mut pm, l, 8.0, if hover(q) { pal::hover() } else { pal::panel() }, 1.0);
                    round_stroke(&mut pm, l, 8.0, pal::edge(), 1.0);
                    text.draw(&mut pm, "Annuler", (l.cx() as f32, (l.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
                }
                FHit::Save => {
                    round_fill(&mut pm, l, 8.0, crate::palette::accent(), if hover(q) { 0.85 } else { 1.0 });
                    text.draw(&mut pm, "Enregistrer", (l.cx() as f32, (l.cy() + 5.0) as f32), 14.0, WHITE, Align::Center);
                }
            }
        }
        Some((pm, r))
    }
}
