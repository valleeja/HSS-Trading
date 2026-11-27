//! 🔔 « Créer une alerte sur » a drawing (Jee's captures, like TradingView): a glass window with the symbol and the
//! timeframe; Condition — the price or an indicator line, the operation (Croisement, vers le haut, vers le bas, Plus
//! grand que, Moins que), the target: the drawing (its line followed at each bar) or a value; « + Ajouter une
//! condition »; Déclenchement, Expiration, Son (▶ to listen), Message; Annuler / Créer. One table of zones (`layout`) for drawing and
//! clicks.

use crate::shapes::{lines, round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_core::alerts::{choix_sons, Cond, Dir, Kind, Op, Src};
use tiny_skia::Pixmap;

const W: f64 = 600.0;
const PAD: f64 = 24.0;
const LEFT: f64 = 180.0;
const FIELD: f64 = 40.0;
const BLOCK: f64 = 150.0;
/// Conditions at most.
const MAX_CONDS: usize = 3;

/// The operations offered, in the list's order.
const OPS: [(Op, &str); 5] = [
    (Op::Cross, "Croisement"),
    (Op::CrossUp, "Croisement vers le haut"),
    (Op::CrossDown, "Croisement vers le bas"),
    (Op::Above, "Plus grand que"),
    (Op::Below, "Moins que"),
];
/// Déclenchement: (label, once per bar?).
const TRIGGERS: [(&str, bool); 2] = [("Une fois par barre", true), ("Une seule fois", false)];
/// Expiration: (label, seconds; `None`: never).
const EXPIRY: [(&str, Option<i64>); 5] = [
    ("1 jour", Some(86_400)),
    ("7 jours", Some(7 * 86_400)),
    ("1 mois", Some(30 * 86_400)),
    ("3 mois", Some(90 * 86_400)),
    ("Sans fin", None),
];

/// One condition as chosen: its source (0 the price, k the indicator line k − 1), operation, and target (`None`:
/// the drawing; `Some(text)`: a value being typed).
#[derive(Debug, Clone, PartialEq)]
struct CondUi {
    src: usize,
    op: usize,
    value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Pop {
    Src(usize),
    Op(usize),
    Target(usize),
    Trigger,
    Expiry,
    Son,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Hit {
    Close,
    Src(usize),
    Op(usize),
    Target(usize),
    Value(usize),
    RemoveCond(usize),
    AddCond,
    Trigger,
    Expiry,
    /// 🔊 The alert's sound (a list), and ▶ to listen to it.
    Son,
    Ecouter,
    Message,
    Cancel,
    Create,
    /// A line of the open list.
    Row(usize),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Focus {
    Value(usize),
    Message,
}

/// What a click did.
#[derive(Debug, Clone, PartialEq)]
pub enum DaClick {
    None,
    Cancel,
    /// Create this alert: what to watch, « every » (once per bar: the timeframe; once: `None`), until, message, the
    /// indicator lines used (for its text).
    Create {
        kind: Kind,
        every: Option<i64>,
        until: Option<i64>,
        msg: String,
        lines: Vec<(u64, String)>,
        /// 🔊 Its sound (`None`: the settings' sound).
        son: Option<String>,
    },
    /// 🔔 « Enregistrer »: the price alert `id` edited (its condition, price and message; empty: its automatic name).
    Save {
        id: u64,
        price: f64,
        dir: Dir,
        msg: String,
        son: Option<String>,
    },
    /// 🔊 ▶: play this sound now (`None`: the settings' sound).
    Ecouter(Option<String>),
}

/// The window.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawAlert {
    pub rect: Rect,
    sym: String,
    tf: i64,
    tf_label: String,
    /// The drawing's name and its line.
    name: String,
    line: Src,
    /// Indicator lines offered as sources: (id, name).
    sources: Vec<(u64, String)>,
    conds: Vec<CondUi>,
    trigger: usize,
    expiry: usize,
    /// 🔊 The chosen line of `choix_sons()` (0: « Par défaut »).
    son: usize,
    msg: String,
    msg_edited: bool,
    focus: Option<Focus>,
    pop: Option<Pop>,
    now: i64,
    /// 🔔 Editing the price alert of this id (right-click on its line ▸ « Éditer… »): the price only, Croisement /
    /// vers le haut / vers le bas, a value, the message; « Enregistrer ».
    edition: Option<u64>,
}

impl DrawAlert {
    /// The window for a drawing `name` whose line is `line`, on `sym` and timeframe `tf` (`tf_label`, e.g. « 4h »),
    /// centred in the window (`win_w` × `win_h`); `sources`: the indicator lines offered; `now`: seconds UTC.
    #[allow(clippy::too_many_arguments)]
    pub fn new(name: &str, line: Src, sym: &str, tf: i64, tf_label: &str, sources: Vec<(u64, String)>, now: i64, win_w: f64, win_h: f64) -> Self {
        let mut a = Self {
            rect: Rect::default(),
            sym: sym.into(),
            tf,
            tf_label: tf_label.into(),
            name: name.into(),
            line,
            sources,
            conds: vec![CondUi { src: 0, op: 0, value: None }],
            trigger: 0,
            expiry: 2,
            son: 0,
            msg: String::new(),
            msg_edited: false,
            focus: None,
            pop: None,
            now,
            edition: None,
        };
        a.fit(win_w, win_h);
        a.msg = a.auto_msg();
        a
    }

    /// 🔔 The same window to edit the price alert `id` on `sym`: prefilled with its condition (`dir`), its price
    /// (written with `decimals`), its message (empty: the automatic one, kept automatic) and its sound `son`.
    #[allow(clippy::too_many_arguments)]
    pub fn editer(
        id: u64,
        sym: &str,
        tf: i64,
        tf_label: &str,
        price: f64,
        dir: Dir,
        decimals: usize,
        msg: &str,
        son: Option<&str>,
        now: i64,
        win_w: f64,
        win_h: f64,
    ) -> Self {
        let line = Src::Trend {
            t0: now,
            p0: price,
            t1: now,
            p1: price,
        };
        let mut a = Self::new("Valeur", line, sym, tf, tf_label, vec![], now, win_w, win_h);
        a.edition = Some(id);
        a.conds[0].op = match dir {
            Dir::Both => 0,
            Dir::Up => 1,
            Dir::Down => 2,
        };
        a.conds[0].value = Some(format!("{price:.decimals$}"));
        a.son = choix_sons().iter().position(|c| c.1 == son).unwrap_or(0);
        a.rect = Rect::default();
        a.fit(win_w, win_h);
        if msg.is_empty() {
            a.msg = a.auto_msg();
        } else {
            (a.msg, a.msg_edited) = (msg.to_string(), true);
        }
        a
    }

    /// The operations offered (editing a price alert: the crossings only).
    fn ops(&self) -> &'static [(Op, &'static str)] {
        if self.edition.is_some() {
            &OPS[..3]
        } else {
            &OPS
        }
    }

    fn height(&self) -> f64 {
        let rangs = if self.edition.is_some() { 2.0 } else { 4.0 };
        112.0 + self.conds.len() as f64 * BLOCK + 40.0 + rangs * 48.0 + 90.0
    }

    fn fit(&mut self, win_w: f64, win_h: f64) {
        let (w, h) = (W.min(win_w - 12.0), self.height().min(win_h - 12.0));
        let x = if self.rect.w > 0.0 { self.rect.x } else { ((win_w - w) / 2.0).max(6.0) };
        let y = if self.rect.w > 0.0 {
            self.rect.y.min(win_h - h - 6.0).max(6.0)
        } else {
            ((win_h - h) / 2.0).max(6.0)
        };
        self.rect = Rect::new(x, y, w, h);
    }

    fn src_name(&self, k: usize) -> String {
        if k == 0 {
            "Prix".into()
        } else {
            self.sources.get(k - 1).map_or_else(String::new, |s| s.1.clone())
        }
    }

    /// The default message: « BTC, 4h Croisement rayon horizontal ».
    fn auto_msg(&self) -> String {
        let c = &self.conds[0];
        let target = c.value.as_deref().map_or_else(|| self.name.to_lowercase(), |v| v.to_string());
        format!("{}, {} {} {target}", self.sym, self.tf_label, OPS[c.op].1)
    }

    fn at(&self, x: f64, y: f64, w: f64, h: f64) -> Rect {
        Rect::new(self.rect.x + x, self.rect.y + y, w, h)
    }

    fn layout(&self) -> Vec<(Rect, Hit)> {
        let r = self.rect;
        let fw = r.w - LEFT - PAD;
        let mut out = vec![(self.at(r.w - 50.0, 18.0, 32.0, 32.0), Hit::Close)];
        for (k, c) in self.conds.iter().enumerate() {
            let y = 112.0 + k as f64 * BLOCK;
            if k > 0 {
                out.push((self.at(LEFT - 40.0, y + 6.0, 28.0, 28.0), Hit::RemoveCond(k)));
            }
            out.push((self.at(LEFT, y, fw, FIELD), Hit::Src(k)));
            out.push((self.at(LEFT, y + 50.0, fw, FIELD), Hit::Op(k)));
            if c.value.is_some() {
                out.push((self.at(LEFT, y + 100.0, fw / 2.0 - 5.0, FIELD), Hit::Target(k)));
                out.push((self.at(LEFT + fw / 2.0 + 5.0, y + 100.0, fw / 2.0 - 5.0, FIELD), Hit::Value(k)));
            } else {
                out.push((self.at(LEFT, y + 100.0, fw, FIELD), Hit::Target(k)));
            }
        }
        let mut y = 112.0 + self.conds.len() as f64 * BLOCK;
        if self.conds.len() < MAX_CONDS && self.edition.is_none() {
            out.push((self.at(LEFT, y - 4.0, 220.0, 30.0), Hit::AddCond));
        }
        y += 50.0;
        if self.edition.is_none() {
            out.push((self.at(LEFT, y, fw, 34.0), Hit::Trigger));
            out.push((self.at(LEFT, y + 48.0, fw, 34.0), Hit::Expiry));
            y += 96.0;
        }
        out.push((self.at(LEFT, y, fw - 50.0, 34.0), Hit::Son));
        out.push((self.at(LEFT + fw - 40.0, y, 40.0, 34.0), Hit::Ecouter));
        y += 48.0;
        out.push((self.at(LEFT, y, fw, 34.0), Hit::Message));
        // (« Enregistrer » is longer than « Créer »)
        let bw = if self.edition.is_some() { 124.0 } else { 94.0 };
        out.push((self.at(r.w - PAD - bw - 106.0, r.h - 62.0, 96.0, 40.0), Hit::Cancel));
        out.push((self.at(r.w - PAD - bw, r.h - 62.0, bw, 40.0), Hit::Create));
        out
    }

    fn pop_rows(&self) -> Vec<String> {
        match self.pop {
            Some(Pop::Src(_)) => (0..=self.sources.len()).map(|k| self.src_name(k)).collect(),
            Some(Pop::Op(_)) => self.ops().iter().map(|o| o.1.to_string()).collect(),
            Some(Pop::Target(_)) => vec![self.name.clone(), "Valeur".into()],
            Some(Pop::Trigger) => TRIGGERS.iter().map(|t| t.0.to_string()).collect(),
            Some(Pop::Expiry) => EXPIRY.iter().map(|e| e.0.to_string()).collect(),
            Some(Pop::Son) => choix_sons().iter().map(|c| c.0.to_string()).collect(),
            None => vec![],
        }
    }

    /// The open list: its rows (under its field, or above it when there is no room).
    fn pop_layout(&self) -> Vec<(Rect, Hit)> {
        let Some(p) = self.pop else { return vec![] };
        let want = match p {
            Pop::Src(k) => Hit::Src(k),
            Pop::Op(k) => Hit::Op(k),
            Pop::Target(k) => Hit::Target(k),
            Pop::Trigger => Hit::Trigger,
            Pop::Expiry => Hit::Expiry,
            Pop::Son => Hit::Son,
        };
        let Some((a, _)) = self.layout().into_iter().find(|(_, h)| *h == want) else {
            return vec![];
        };
        let n = self.pop_rows().len();
        let h = n as f64 * 40.0;
        let y0 = if a.y + a.h + 6.0 + h <= self.rect.y + self.rect.h - 6.0 {
            a.y + a.h + 6.0
        } else {
            (a.y - 6.0 - h).max(self.rect.y + 6.0)
        };
        (0..n)
            .map(|k| (Rect::new(a.x, y0 + k as f64 * 40.0, a.w.max(260.0), 38.0), Hit::Row(k)))
            .collect()
    }

    /// The price of the drawing at `t` (for the « Valeur » field's first value).
    fn line_at(&self, t: i64) -> f64 {
        match self.line {
            Src::Trend { t0, p0, t1, p1 } if t1 != t0 => p0 + (p1 - p0) * (t - t0) as f64 / (t1 - t0) as f64,
            Src::Trend { p0, .. } => p0,
            _ => 0.0,
        }
    }

    /// A click (window coordinates).
    pub fn click(&mut self, x: f64, y: f64) -> DaClick {
        if self.pop.is_some() {
            let row = self.pop_layout().into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, h)| h);
            let p = self.pop.take();
            if let (Some(Hit::Row(k)), Some(p)) = (row, p) {
                match p {
                    Pop::Src(c) => self.conds[c].src = k,
                    Pop::Op(c) => self.conds[c].op = k,
                    Pop::Target(c) => {
                        self.conds[c].value = (k == 1).then(|| format!("{:.2}", self.line_at(self.now)));
                    }
                    Pop::Trigger => self.trigger = k,
                    Pop::Expiry => self.expiry = k,
                    Pop::Son => self.son = k,
                }
                if !self.msg_edited {
                    self.msg = self.auto_msg();
                }
                return DaClick::None;
            }
        }
        if !self.rect.contains(x, y) {
            return DaClick::None;
        }
        let Some((_, hit)) = self.layout().into_iter().find(|(r, _)| r.contains(x, y)) else {
            self.focus = None;
            return DaClick::None;
        };
        self.focus = None;
        match hit {
            Hit::Close | Hit::Cancel => return DaClick::Cancel,
            Hit::Create => return self.create(),
            Hit::Src(k) => self.pop = Some(Pop::Src(k)),
            Hit::Op(k) => self.pop = Some(Pop::Op(k)),
            // (editing a price alert: its target is always a value)
            Hit::Target(_) if self.edition.is_some() => {}
            Hit::Target(k) => self.pop = Some(Pop::Target(k)),
            Hit::Trigger => self.pop = Some(Pop::Trigger),
            Hit::Expiry => self.pop = Some(Pop::Expiry),
            Hit::Son => self.pop = Some(Pop::Son),
            Hit::Ecouter => return DaClick::Ecouter(self.son_choisi()),
            Hit::Value(k) => self.focus = Some(Focus::Value(k)),
            Hit::Message => self.focus = Some(Focus::Message),
            Hit::AddCond => {
                self.conds.push(CondUi { src: 0, op: 0, value: None });
                let (ww, wh) = (self.rect.x * 2.0 + self.rect.w, self.rect.y * 2.0 + self.rect.h);
                self.fit(ww.max(W + 12.0), wh.max(self.height() + 12.0));
            }
            Hit::RemoveCond(k) => {
                self.conds.remove(k);
                self.rect.h = self.height().min(self.rect.h);
            }
            Hit::Row(_) => {}
        }
        DaClick::None
    }

    /// Keeps the window in the window (`win_w` × `win_h`) after a resize or an added condition.
    pub fn refit(&mut self, win_w: f64, win_h: f64) {
        self.fit(win_w, win_h);
    }

    /// 🔊 The chosen sound, as saved on the alert (`None`: « Par défaut »).
    fn son_choisi(&self) -> Option<String> {
        choix_sons().get(self.son).and_then(|c| c.1).map(str::to_string)
    }

    fn create(&self) -> DaClick {
        if let Some(id) = self.edition {
            let c = &self.conds[0];
            let price = match &c.value {
                None => self.line_at(self.now),
                Some(v) => match v.trim().replace(',', ".").replace(' ', "").parse::<f64>() {
                    Ok(x) if x.is_finite() && x > 0.0 => x,
                    _ => return DaClick::None,
                },
            };
            let dir = match c.op {
                1 => Dir::Up,
                2 => Dir::Down,
                _ => Dir::Both,
            };
            let msg = if self.msg_edited { self.msg.clone() } else { String::new() };
            return DaClick::Save {
                id,
                price,
                dir,
                msg,
                son: self.son_choisi(),
            };
        }
        let mut conds = vec![];
        let mut used = vec![];
        for c in &self.conds {
            let l = if c.src == 0 {
                Src::Close
            } else {
                let (id, name) = self.sources[c.src - 1].clone();
                used.push((id, name));
                Src::Line(id)
            };
            let r = match &c.value {
                None => self.line,
                Some(v) => match v.trim().replace(',', ".").replace(' ', "").parse::<f64>() {
                    Ok(x) if x.is_finite() => Src::Num(x),
                    _ => return DaClick::None,
                },
            };
            conds.push(Cond {
                l,
                op: OPS[c.op].0,
                r,
                r2: None,
            });
        }
        DaClick::Create {
            kind: Kind::Multi { tf: self.tf, all: true, conds },
            every: TRIGGERS[self.trigger].1.then_some(self.tf),
            until: EXPIRY[self.expiry].1.map(|s| self.now + s),
            msg: self.msg.clone(),
            lines: used,
            son: self.son_choisi(),
        }
    }

    pub fn typing(&self) -> bool {
        self.focus.is_some()
    }

    pub fn type_char(&mut self, c: char) {
        match self.focus {
            Some(Focus::Value(k)) if c.is_ascii_digit() || c == '.' || c == ',' || (c == '-' && self.conds[k].value.as_ref().is_some_and(|v| v.is_empty())) => {
                if let Some(v) = self.conds[k].value.as_mut() {
                    v.push(c);
                }
            }
            Some(Focus::Message) if !c.is_control() && self.msg.chars().count() < 200 => {
                self.msg.push(c);
                self.msg_edited = true;
            }
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.focus {
            Some(Focus::Value(k)) => {
                if let Some(v) = self.conds[k].value.as_mut() {
                    v.pop();
                }
            }
            Some(Focus::Message) => {
                self.msg.pop();
                self.msg_edited = true;
            }
            None => {}
        }
    }

    /// The expiry as a date (« 17 décembre 2026 à 05:13 »), or « Sans fin ».
    fn expiry_text(&self) -> String {
        let Some(s) = EXPIRY[self.expiry].1 else { return "Sans fin".into() };
        const MOIS: [&str; 12] = [
            "janvier",
            "février",
            "mars",
            "avril",
            "mai",
            "juin",
            "juillet",
            "août",
            "septembre",
            "octobre",
            "novembre",
            "décembre",
        ];
        chrono::DateTime::from_timestamp(self.now + s, 0).map_or_else(String::new, |t| {
            use chrono::{Datelike, Timelike};
            format!("{} {} {} à {:02}:{:02}", t.day(), MOIS[t.month0() as usize], t.year(), t.hour(), t.minute())
        })
    }

    /// The window and its open list (window coordinates).
    pub fn draw(&self, text: &Text, mouse: (f64, f64)) -> Option<Pixmap> {
        let r = self.rect;
        let mut pm = crate::px::new(r.w.ceil() as u32, r.h.ceil() as u32)?;
        let loc = |q: Rect| Rect::new(q.x - r.x, q.y - r.y, q.w, q.h);
        crate::shapes::menu_card(&mut pm, loc(r));
        let ink = crate::jetons::TEXTE;
        let grey = crate::palette::GREY;
        let edge = crate::jetons::sur_fond(crate::jetons::LISERE);
        let bold = Text::graisse(Graisse::Bold);
        // (editing, like TradingView: « Modifier l'alerte sur BTCUSDC.P »)
        let titre = if self.edition.is_some() {
            format!("Modifier l'alerte sur {}", self.sym)
        } else {
            "Créer une alerte sur".to_string()
        };
        bold.draw(&mut pm, &text.fit(&titre, 20.0, r.w - 2.0 * PAD), (PAD as f32, 44.0), 20.0, ink, Align::Left);
        text.draw(&mut pm, &format!("{}, {}", self.sym, self.tf_label), (PAD as f32, 76.0), 15.0, ink, Align::Left);
        crate::fill(&mut pm, 0.0, 96.0, r.w, 1.0, edge, 1.0);
        text.draw(&mut pm, "Condition", (PAD as f32, 138.0), 14.0, grey, Align::Left);
        let after = 112.0 + self.conds.len() as f64 * BLOCK;
        crate::fill(&mut pm, PAD, after + 32.0, r.w - 2.0 * PAD, 1.0, edge, 1.0);
        let labels: &[&str] = if self.edition.is_some() {
            &["Son", "Message"]
        } else {
            &["Déclenchement", "Expiration", "Son", "Message"]
        };
        for (k, label) in labels.iter().enumerate() {
            text.draw(&mut pm, label, (PAD as f32, (after + 72.0 + k as f64 * 48.0) as f32), 14.0, grey, Align::Left);
        }
        let hover = |q: &Rect| q.contains(mouse.0, mouse.1) && self.pop.is_none();
        let field = |pm: &mut Pixmap, q: Rect, s: &str, active: bool, chevron: bool| {
            round_fill(pm, q, 8.0, crate::jetons::sur_fond(crate::jetons::CHAMP), 1.0);
            round_stroke(pm, q, 8.0, if active { crate::palette::accent() } else { edge }, if active { 1.6 } else { 1.0 });
            text.draw(
                pm,
                &text.fit(s, 14.5, q.w - 44.0),
                ((q.x + 12.0) as f32, (q.cy() + 5.0) as f32),
                14.5,
                ink,
                Align::Left,
            );
            if chevron {
                lines(pm, q.x + q.w - 18.0, q.cy(), &[&[(-4.0, -2.0), (0.0, 2.0), (4.0, -2.0)]], ink, 1.4);
            }
        };
        for (q, hit) in self.layout() {
            let lq = loc(q);
            let h = hover(&q);
            match hit {
                Hit::Close => lines(
                    &mut pm,
                    lq.cx(),
                    lq.cy(),
                    &[&[(-7.0, -7.0), (7.0, 7.0)], &[(7.0, -7.0), (-7.0, 7.0)]],
                    if h { ink } else { grey },
                    1.6,
                ),
                Hit::Src(k) => field(&mut pm, lq, &self.src_name(self.conds[k].src), self.pop == Some(Pop::Src(k)), true),
                Hit::Op(k) => {
                    field(
                        &mut pm,
                        Rect::new(lq.x + 30.0, lq.y, lq.w - 30.0, lq.h),
                        OPS[self.conds[k].op].1,
                        self.pop == Some(Pop::Op(k)),
                        true,
                    );
                    round_stroke(
                        &mut pm,
                        lq,
                        8.0,
                        if self.pop == Some(Pop::Op(k)) { crate::palette::accent() } else { edge },
                        1.0,
                    );
                    // the crossing icon
                    lines(
                        &mut pm,
                        lq.x + 20.0,
                        lq.cy(),
                        &[&[(-8.0, 8.0), (8.0, -8.0)], &[(-8.0, -8.0), (8.0, 8.0)]],
                        ink,
                        1.3,
                    );
                }
                Hit::Target(k) => {
                    let s = if self.conds[k].value.is_some() {
                        "Valeur".to_string()
                    } else {
                        self.name.clone()
                    };
                    field(&mut pm, lq, &s, self.pop == Some(Pop::Target(k)), true);
                }
                Hit::Value(k) => {
                    let v = self.conds[k].value.clone().unwrap_or_default();
                    field(&mut pm, lq, &v, self.focus == Some(Focus::Value(k)), false);
                }
                Hit::RemoveCond(_) => lines(
                    &mut pm,
                    lq.cx(),
                    lq.cy(),
                    &[&[(-5.0, -5.0), (5.0, 5.0)], &[(5.0, -5.0), (-5.0, 5.0)]],
                    grey,
                    1.4,
                ),
                Hit::AddCond => {
                    let blue = Rgb(0x5b, 0x9c, 0xf6);
                    text.draw(
                        &mut pm,
                        "+  Ajouter une condition",
                        ((lq.x + 4.0) as f32, (lq.cy() + 5.0) as f32),
                        14.5,
                        blue,
                        Align::Left,
                    );
                }
                Hit::Trigger => text.draw(
                    &mut pm,
                    &format!("{}  ⌄", TRIGGERS[self.trigger].0),
                    (lq.x as f32, (lq.cy() + 5.0) as f32),
                    14.5,
                    ink,
                    Align::Left,
                ),
                Hit::Expiry => text.draw(
                    &mut pm,
                    &format!("{}  ⌄", self.expiry_text()),
                    (lq.x as f32, (lq.cy() + 5.0) as f32),
                    14.5,
                    ink,
                    Align::Left,
                ),
                Hit::Son => text.draw(
                    &mut pm,
                    &format!("{}  ⌄", choix_sons().get(self.son).map_or("", |c| c.0)),
                    (lq.x as f32, (lq.cy() + 5.0) as f32),
                    14.5,
                    ink,
                    Align::Left,
                ),
                // ▶ listen: a round button with a triangle
                Hit::Ecouter => {
                    round_stroke(&mut pm, lq, lq.h / 2.0, if h { ink } else { edge }, 1.2);
                    lines(
                        &mut pm,
                        lq.cx() + 1.0,
                        lq.cy(),
                        &[&[(-4.0, -6.0), (6.0, 0.0), (-4.0, 6.0), (-4.0, -6.0)]],
                        ink,
                        1.4,
                    );
                }
                Hit::Message => {
                    let on = self.focus == Some(Focus::Message);
                    if on {
                        round_stroke(&mut pm, lq, 8.0, crate::palette::accent(), 1.4);
                    }
                    text.draw(
                        &mut pm,
                        &text.fit(&self.msg, 14.5, lq.w - 16.0),
                        ((lq.x + 6.0) as f32, (lq.cy() + 5.0) as f32),
                        14.5,
                        ink,
                        Align::Left,
                    );
                }
                Hit::Cancel => {
                    round_stroke(&mut pm, lq, 10.0, if h { ink } else { edge }, 1.2);
                    text.draw(&mut pm, "Annuler", (lq.cx() as f32, (lq.cy() + 5.5) as f32), 15.0, ink, Align::Center);
                }
                Hit::Create => {
                    round_fill(&mut pm, lq, 10.0, crate::jetons::TEXTE, 1.0);
                    text.draw(
                        &mut pm,
                        if self.edition.is_some() { "Enregistrer" } else { "Créer" },
                        (lq.cx() as f32, (lq.cy() + 5.5) as f32),
                        15.0,
                        crate::jetons::FOND,
                        Align::Center,
                    );
                }
                Hit::Row(_) => {}
            }
        }
        // the open list, over the window
        let rows = self.pop_rows();
        let z = self.pop_layout();
        if let (Some((first, _)), Some((last, _))) = (z.first(), z.last()) {
            let frame = loc(Rect::new(first.x - 4.0, first.y - 4.0, first.w + 8.0, last.y + last.h - first.y + 8.0));
            round_fill(&mut pm, frame, 10.0, crate::palette::panel(), 1.0);
            round_stroke(&mut pm, frame, 10.0, edge, 1.0);
            let chosen = match self.pop {
                Some(Pop::Src(c)) => self.conds[c].src,
                Some(Pop::Op(c)) => self.conds[c].op,
                Some(Pop::Target(c)) => usize::from(self.conds[c].value.is_some()),
                Some(Pop::Trigger) => self.trigger,
                Some(Pop::Expiry) => self.expiry,
                Some(Pop::Son) => self.son,
                None => usize::MAX,
            };
            for ((q, _), (k, s)) in z.iter().zip(rows.iter().enumerate()) {
                let lq = loc(*q);
                if k == chosen {
                    round_fill(&mut pm, lq, 8.0, crate::zoompad::CHOISI.0, crate::zoompad::CHOISI.1);
                } else if q.contains(mouse.0, mouse.1) {
                    round_fill(&mut pm, lq, 8.0, crate::jetons::BLANC, 0.06);
                }
                text.draw(
                    &mut pm,
                    &text.fit(s, 14.5, lq.w - 20.0),
                    ((lq.x + 12.0) as f32, (lq.cy() + 5.0) as f32),
                    14.5,
                    ink,
                    Align::Left,
                );
            }
        }
        Some(pm)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win() -> DrawAlert {
        let line = Src::Trend {
            t0: 100,
            p0: 65_000.0,
            t1: 100,
            p1: 0.0,
        };
        DrawAlert::new(
            "Rayon horizontal",
            line,
            "BTC",
            14_400,
            "4h",
            vec![(7, "RSI 14 · RSI".into())],
            1_000,
            1440.0,
            900.0,
        )
    }

    fn press(a: &mut DrawAlert, h: Hit) -> DaClick {
        let r = a.layout().into_iter().find(|(_, k)| *k == h).map(|(r, _)| r).expect("zone");
        a.click(r.cx(), r.cy())
    }

    fn row(a: &mut DrawAlert, k: usize) {
        let r = a.pop_layout()[k].0;
        a.click(r.cx(), r.cy());
    }

    #[test]
    fn alerte_sur_le_dessin_par_defaut() {
        let mut a = win();
        assert_eq!(a.msg, "BTC, 4h Croisement rayon horizontal");
        let DaClick::Create { kind, every, until, .. } = press(&mut a, Hit::Create) else {
            panic!("create")
        };
        let Kind::Multi { tf, conds, .. } = kind else { panic!() };
        assert_eq!((tf, every, until), (14_400, Some(14_400), Some(1_000 + 30 * 86_400)), "once per bar, a month");
        assert_eq!(
            conds[0],
            Cond {
                l: Src::Close,
                op: Op::Cross,
                r: Src::Trend {
                    t0: 100,
                    p0: 65_000.0,
                    t1: 100,
                    p1: 0.0
                },
                r2: None
            }
        );
    }

    #[test]
    fn indicateur_operation_valeur_et_conditions() {
        let mut a = win();
        // RSI as the source, « Plus grand que », a value 70
        press(&mut a, Hit::Src(0));
        row(&mut a, 1);
        press(&mut a, Hit::Op(0));
        row(&mut a, 3);
        press(&mut a, Hit::Target(0));
        row(&mut a, 1);
        press(&mut a, Hit::Value(0));
        (0..12).for_each(|_| a.backspace());
        "70".chars().for_each(|c| a.type_char(c));
        // a 2nd condition, then removed
        press(&mut a, Hit::AddCond);
        assert_eq!(a.conds.len(), 2);
        press(&mut a, Hit::RemoveCond(1));
        assert_eq!(a.conds.len(), 1);
        // once, never ending
        press(&mut a, Hit::Trigger);
        row(&mut a, 1);
        press(&mut a, Hit::Expiry);
        row(&mut a, 4);
        let DaClick::Create { kind, every, until, lines, .. } = press(&mut a, Hit::Create) else {
            panic!()
        };
        let Kind::Multi { conds, .. } = kind else { panic!() };
        assert_eq!(
            conds[0],
            Cond {
                l: Src::Line(7),
                op: Op::Above,
                r: Src::Num(70.0),
                r2: None
            }
        );
        assert_eq!((every, until), (None, None));
        assert_eq!(lines, vec![(7, "RSI 14 · RSI".to_string())]);
        assert!(a.draw(&Text::default(), (0.0, 0.0)).is_some());
        assert_eq!(press(&mut a, Hit::Cancel), DaClick::Cancel);
    }

    /// 🔔 Editing a price alert: prefilled (vers le bas, its price, its automatic message kept automatic), the
    /// crossings only, no drawing target, « Enregistrer » gives the same id; « Annuler » nothing.
    #[test]
    fn editer_une_alerte_de_prix() {
        let mut a = DrawAlert::editer(42, "BTC", 14_400, "4h", 63_584.0, Dir::Down, 1, "", Some("Cloche"), 1_000, 1440.0, 900.0);
        assert_eq!(a.conds[0].value.as_deref(), Some("63584.0"));
        assert!(a.layout().iter().all(|(_, h)| !matches!(h, Hit::AddCond | Hit::Trigger | Hit::Expiry)));
        press(&mut a, Hit::Op(0));
        assert_eq!(a.pop_rows().len(), 3, "the crossings only");
        row(&mut a, 1);
        press(&mut a, Hit::Target(0));
        assert!(a.pop.is_none(), "the target stays a value");
        press(&mut a, Hit::Value(0));
        (0..12).for_each(|_| a.backspace());
        "64000".chars().for_each(|c| a.type_char(c));
        assert!(a.draw(&Text::default(), (0.0, 0.0)).is_some());
        assert_eq!(
            press(&mut a, Hit::Create),
            DaClick::Save {
                id: 42,
                price: 64_000.0,
                dir: Dir::Up,
                msg: String::new(),
                son: Some("Cloche".into())
            }
        );
        assert_eq!(press(&mut a, Hit::Cancel), DaClick::Cancel);
        let mut b = DrawAlert::editer(7, "BTC", 14_400, "4h", 100.0, Dir::Both, 2, "mon message", None, 1_000, 1440.0, 900.0);
        let DaClick::Save { msg, dir, son, .. } = press(&mut b, Hit::Create) else {
            panic!()
        };
        assert_eq!((msg.as_str(), dir), ("mon message", Dir::Both), "its own message kept");
        assert_eq!(son, None, "no sound of its own: « Par défaut »");
    }

    /// 🔊 The « Son » line: « Par défaut » first, the 8 sounds, « Son coupé »; the choice goes with « Créer »;
    /// ▶ plays the chosen sound.
    #[test]
    fn son_de_l_alerte() {
        let mut a = win();
        assert_eq!(press(&mut a, Hit::Ecouter), DaClick::Ecouter(None), "« Par défaut »");
        press(&mut a, Hit::Son);
        let rows = a.pop_rows();
        assert_eq!((rows.len(), rows[0].as_str(), rows[9].as_str()), (10, "Par défaut", "Son coupé"));
        row(&mut a, 3);
        assert_eq!(press(&mut a, Hit::Ecouter), DaClick::Ecouter(Some("Cloche".into())));
        assert!(a.draw(&Text::default(), (0.0, 0.0)).is_some());
        let DaClick::Create { son, .. } = press(&mut a, Hit::Create) else { panic!() };
        assert_eq!(son.as_deref(), Some("Cloche"));
        press(&mut a, Hit::Son);
        row(&mut a, 9);
        let DaClick::Create { son, .. } = press(&mut a, Hit::Create) else { panic!() };
        assert_eq!(son.as_deref(), Some(argos_core::alerts::SON_COUPE));
    }
}
