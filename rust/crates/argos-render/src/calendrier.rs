//! 📅 The Calendrier (Jee's mock-up « calendrier v1 », 22 Nov 2026): Aujourd'hui / Semaine, the impact and currency
//! chips (`IMPACTS`, `CURRENCIES`), the column header Heure · Annonce · Réel · Prévu · Préc., the line « HH:MM ·
//! maintenant » between past and coming releases, the « Réel » in green (better than expected for the economy and
//! the markets) or red (worse), under a past high-impact release the chart's reaction (« BTC −1,2 % en 5 min »), and
//! at the bottom « Heure de New York » and the sources. A click on a release opens its card ([`Carte`]).

use crate::eco::{high_impact, Eco, EcoClick, CURRENCIES, IMPACTS};
use crate::jetons as j;
use crate::shapes::{round_fill, round_stroke, Rect};
use crate::text::{Align, Graisse, Rgb, Text};
use argos_data::eco::Event;
use argos_data::reel;
use tiny_skia::Pixmap;

const PAD: f64 = 14.0;
const HEAD: f64 = 48.0;
const PUCE_H: f64 = 28.0;
const COLS: f64 = 28.0;
const JOUR: f64 = 30.0;
const LIGNE: f64 = 50.0;
const REACTION: f64 = 26.0;
const MAINTENANT: f64 = 24.0;
const PIED: f64 = 40.0;

/// The short « what it is » and « why it matters » of the common releases (French), by words of the Forex Factory
/// title (lower case); an unknown release has neither.
pub const FICHES: &[(&[&str], &str, &str)] = &[
    (
        &["core cpi"],
        "L'inflation des prix à la consommation sans l'alimentation ni l'énergie, ce que la Fed suit de près.",
        "Plus haut que prévu : la Fed garde ses taux hauts plus longtemps. Souvent mauvais pour BTC et les actions, bon pour le dollar.",
    ),
    (
        &["cpi"],
        "L'inflation des prix payés par les ménages, sur un mois ou un an.",
        "Plus haut que prévu : moins de baisses de taux. Souvent mauvais pour BTC et les actions, bon pour le dollar.",
    ),
    (
        &["core ppi"],
        "Les prix payés aux producteurs, sans l'alimentation ni l'énergie.",
        "Annonce l'inflation à venir : plus haut que prévu pèse sur BTC et les actions.",
    ),
    (
        &["ppi"],
        "Les prix payés aux producteurs (prix de gros).",
        "Annonce l'inflation à venir : plus haut que prévu pèse sur BTC et les actions.",
    ),
    (
        &["non-farm"],
        "Les emplois créés le mois passé aux États-Unis, hors agriculture.",
        "Le chiffre qui fait le plus bouger les marchés : un écart au prévu donne souvent une forte bougie dans les minutes qui suivent.",
    ),
    (
        &["unemployment claims"],
        "Les nouvelles inscriptions au chômage de la semaine.",
        "Plus haut que prévu : l'économie ralentit, la Fed peut baisser ses taux plus tôt.",
    ),
    (
        &["unemployment rate"],
        "La part des actifs sans emploi.",
        "Plus haut que prévu : économie plus faible, baisse des taux plus probable ; le dollar baisse souvent.",
    ),
    (
        &["average hourly earnings"],
        "La hausse du salaire horaire moyen sur un mois.",
        "Des salaires qui montent vite nourrissent l'inflation : plus haut que prévu, taux plus hauts plus longtemps.",
    ),
    (
        &["gdp"],
        "La croissance de l'économie américaine sur un trimestre (PIB).",
        "Plus haut que prévu : économie solide, bon pour le dollar ; l'effet sur BTC dépend de la Fed.",
    ),
    (
        &["core pce"],
        "L'inflation que la Fed regarde en premier : les prix payés par les ménages, sans l'alimentation ni l'énergie.",
        "Plus haut que prévu : la Fed baisse ses taux moins vite. Souvent mauvais pour BTC et les actions, bon pour le dollar.",
    ),
    (
        &["pce"],
        "L'inflation des dépenses des ménages, la mesure préférée de la Fed.",
        "Plus haut que prévu : la Fed baisse ses taux moins vite ; mauvais pour BTC et les actions.",
    ),
    (
        &["retail sales"],
        "Les ventes des magasins sur un mois : la santé de la consommation.",
        "Plus haut que prévu : consommation solide, bon pour le dollar ; moins de baisses de taux.",
    ),
    (
        &["ism"],
        "L'enquête ISM auprès des directeurs d'achat : au-dessus de 50, l'activité grandit.",
        "Un écart au prévu change l'idée qu'on se fait de la croissance et des taux.",
    ),
    (
        &["jolts"],
        "Les offres d'emploi ouvertes (enquête JOLTS).",
        "Beaucoup d'offres : marché du travail tendu, salaires et inflation plus hauts.",
    ),
    (
        &["fomc meeting minutes", "fomc minutes"],
        "Le compte rendu de la dernière réunion de la Fed.",
        "Montre si la Fed penche vers une hausse ou une baisse des taux : le marché réagit au ton.",
    ),
    (
        &["fomc statement"],
        "Le communiqué de la Fed après sa réunion.",
        "Chaque mot compte : le ton (prudent ou ferme) fait bouger le dollar, l'or et BTC.",
    ),
    (
        &["fomc press conference"],
        "La conférence de presse du président de la Fed après la décision.",
        "Ses réponses font souvent plus bouger le marché que la décision elle-même.",
    ),
    (
        &["federal funds rate"],
        "La décision de taux de la Fed (borne haute de la fourchette).",
        "Une surprise fait bouger tous les marchés ; plus haut que prévu : mauvais pour BTC et les actions.",
    ),
    (
        &["main refinancing rate", "ecb"],
        "La décision de taux de la Banque centrale européenne.",
        "Fait bouger l'euro face au dollar, et par lui l'indice dollar.",
    ),
    (
        &["official bank rate", "boe"],
        "La décision de taux de la Banque d'Angleterre.",
        "Fait bouger la livre ; effet plus faible sur BTC.",
    ),
    (
        &["boj", "policy rate"],
        "La décision de taux de la Banque du Japon.",
        "Une hausse du yen peut forcer des ventes sur les marchés (fin du « carry trade »).",
    ),
    (
        &["overnight rate", "boc"],
        "La décision de taux de la Banque du Canada.",
        "Fait bouger le dollar canadien.",
    ),
];

/// The card's texts of a release, `None` if unknown.
pub fn fiche(title: &str) -> Option<(&'static str, &'static str)> {
    let t = title.to_lowercase();
    FICHES.iter().find(|(k, _, _)| k.iter().any(|w| t.contains(w))).map(|(_, a, b)| (*a, *b))
}

/// Where a release comes from (« BLS », « Fed »…), shown on its card.
pub fn source_of(e: &Event) -> &'static str {
    let t = e.title.to_lowercase();
    if e.country == "USD"
        && ["cpi", "ppi", "non-farm", "unemployment rate", "hourly earnings", "jolts"]
            .iter()
            .any(|k| t.contains(k))
    {
        "BLS"
    } else if e.country == "USD" && (t.contains("fomc") || t.contains("federal funds") || t.contains("fed ")) {
        "Fed"
    } else if e.country == "USD" && (t.contains("gdp") || t.contains("pce")) {
        "BEA"
    } else {
        "Forex Factory"
    }
}

/// The colour of a « Réel »: green better, red worse, white equal or unknown.
pub fn couleur_reel(e: &Event) -> Rgb {
    match reel::better(e) {
        Some(true) => crate::palette::up(),
        Some(false) => crate::palette::down(),
        None => j::TEXTE,
    }
}

const JOURS: [&str; 7] = ["Lun.", "Mar.", "Mer.", "Jeu.", "Ven.", "Sam.", "Dim."];
const MOIS: [&str; 12] = [
    "janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc.",
];

fn ny(ts: i64) -> chrono::DateTime<chrono_tz::Tz> {
    use chrono::TimeZone;
    chrono_tz::America::New_York
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| chrono_tz::America::New_York.timestamp_opt(0, 0).unwrap())
}

/// « Mer. 25 nov. »
pub fn jour(ts: i64) -> String {
    use chrono::Datelike;
    let d = ny(ts);
    format!(
        "{} {} {}",
        JOURS[d.weekday().num_days_from_monday() as usize],
        d.day(),
        MOIS[d.month0() as usize]
    )
}

/// « dans 2 h 48 », « dans 12 min ».
pub fn dans(s: i64) -> String {
    let s = s.max(0);
    if s < 3600 {
        format!("dans {} min", (s / 60).max(1))
    } else if s < 86_400 {
        format!("dans {} h {:02}", s / 3600, s / 60 % 60)
    } else {
        format!("dans {} j {} h", s / 86_400, s / 3600 % 24)
    }
}

/// The right edges of the columns Réel, Prévu, Préc.
fn colonnes(pw: f64) -> (f64, f64, f64) {
    (pw - PAD - 104.0, pw - PAD - 52.0, pw - PAD)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CZone {
    Periode(bool),
    Impact(usize),
    Devise(usize),
    /// A release (its index in `agenda`).
    Annonce(usize),
    /// 📖 The Academy's news sheet at this row (US dollar releases it explains), under the release's time.
    Learn(usize),
}

/// 📖 Where a release's 📖 sits in its line `r` (under the time).
fn livre_rect(r: &Rect) -> Rect {
    Rect::new(6.0, r.y + 24.0, 44.0, 24.0)
}

/// What is drawn, top to bottom: a day, the « now » line, a release.
#[derive(Debug, Clone, PartialEq)]
enum Item {
    Jour(String),
    Maintenant,
    Annonce(usize),
}

impl Eco {
    /// The releases kept by the impact and currency chips and the period (today in New York, or the whole week).
    fn cal_shown(&self, now: i64) -> Vec<usize> {
        let today = ny(now).date_naive();
        self.agenda
            .iter()
            .enumerate()
            .filter(|(_, e)| match self.impact {
                0 => e.impact == "High",
                1 => e.impact == "High" || e.impact == "Medium",
                _ => true,
            })
            .filter(|(_, e)| self.currency == 0 || e.country == CURRENCIES[self.currency])
            .filter(|(_, e)| self.semaine || ny(e.ts).date_naive() == today)
            .map(|(i, _)| i)
            .collect()
    }

    fn cal_items(&self, now: i64) -> Vec<Item> {
        let mut out = vec![];
        let mut day = String::new();
        let mut now_done = false;
        let today = jour(now);
        for i in self.cal_shown(now) {
            let e = &self.agenda[i];
            let d = jour(e.ts);
            // (the first coming release opens a later day: the line goes between the past and that day's header)
            if !now_done && e.ts > now && d != day && d != today {
                out.push(Item::Maintenant);
                now_done = true;
            }
            if d != day {
                out.push(Item::Jour(d.clone()));
                day = d;
            }
            if !now_done && e.ts > now {
                out.push(Item::Maintenant);
                now_done = true;
            }
            out.push(Item::Annonce(i));
        }
        out
    }

    /// The past high-impact releases that get the chart's reaction (10 at most, the latest).
    pub fn cal_reactions(&self, now: i64) -> Vec<i64> {
        let mut v: Vec<i64> = self.agenda.iter().filter(|e| high_impact(e) && e.ts <= now).map(|e| e.ts).collect();
        v.sort_unstable();
        v.dedup();
        let k = v.len().saturating_sub(10);
        v.split_off(k)
    }

    fn puces(&self, text: &Text, pw: f64) -> (Vec<(Rect, CZone)>, f64) {
        let mut z = vec![];
        let half = (pw - 2.0 * PAD) / 2.0;
        z.push((Rect::new(PAD, HEAD + 2.0, half, 32.0), CZone::Periode(false)));
        z.push((Rect::new(PAD + half, HEAD + 2.0, half, 32.0), CZone::Periode(true)));
        let (mut x, mut y) = (PAD, HEAD + 44.0);
        let labels: Vec<(String, CZone)> = IMPACTS
            .iter()
            .enumerate()
            .map(|(k, l)| (l.to_string(), CZone::Impact(k)))
            .chain(CURRENCIES.iter().enumerate().map(|(k, l)| (l.to_string(), CZone::Devise(k))))
            .collect();
        for (l, c) in labels {
            let w = text.width(&l, 12.5) as f64 + 22.0;
            if x + w > pw - PAD && x > PAD {
                (x, y) = (PAD, y + PUCE_H + 6.0);
            }
            z.push((Rect::new(x, y, w, PUCE_H), c));
            x += w + 6.0;
        }
        (z, y + PUCE_H + 8.0)
    }

    /// 🚫 The PropFirm banner's place (under the chips, when there is one) and the top of the column header.
    fn cal_haut(&self, text: &Text, pw: f64, now: i64) -> (Option<Rect>, f64) {
        let (_, y) = self.puces(text, pw);
        if self.news_banner(now).0.is_empty() {
            (None, y)
        } else {
            (Some(Rect::new(PAD, y, pw - 2.0 * PAD, 34.0)), y + 42.0)
        }
    }

    /// A release's title on 2 lines at most (between the impact dots and the « Réel » column).
    fn titre_lignes(title: &str, pw: f64) -> Vec<String> {
        let gras = Text::graisse(Graisse::Bold);
        let w = colonnes(pw).0 - 44.0 - (PAD + 108.0);
        let mut l = crate::assistant::wrap(&gras, title, 13.5, w);
        if l.len() > 2 {
            l.truncate(2);
            l[1] = gras.fit(&format!("{}…", l[1]), 13.5, w);
        }
        l
    }

    fn cal_lines(&self, text: &Text, pw: f64, now: i64) -> Vec<(Item, Rect)> {
        let (_, mut y) = self.cal_haut(text, pw, now);
        y += COLS;
        let reactions = self.cal_reactions(now);
        let mut out = vec![];
        for it in self.cal_items(now) {
            let h = match &it {
                Item::Jour(_) => JOUR,
                Item::Maintenant => MAINTENANT,
                Item::Annonce(i) => {
                    let e = &self.agenda[*i];
                    let r = reactions.contains(&e.ts) && self.reactions.get(&e.ts).is_some_and(|r| r[0].is_some());
                    let extra = (Self::titre_lignes(&e.title, pw).len().max(1) - 1) as f64 * 18.0;
                    LIGNE + extra + if r { REACTION } else { 0.0 } + if e.ts > now && high_impact(e) { 4.0 } else { 0.0 }
                }
            };
            out.push((it, Rect::new(0.0, y, pw, h)));
            y += h;
        }
        out
    }

    pub(crate) fn cal_zones(&self, text: &Text, pw: f64, now: i64) -> Vec<(Rect, CZone)> {
        let (mut z, _) = self.puces(text, pw);
        for (it, r) in self.cal_lines(text, pw, now) {
            if let Item::Annonce(i) = it {
                // (📖 first: a touch on it opens the Academy, the rest of the line the release's card)
                if let Some(row) = crate::eco::learn_row(&self.agenda[i]) {
                    z.push((livre_rect(&r), CZone::Learn(row)));
                }
                z.push((Rect::new(6.0, r.y, pw - 12.0, r.h), CZone::Annonce(i)));
            }
        }
        z
    }

    /// The Calendrier's height (it scrolls).
    pub fn cal_h(&self, text: &Text, pw: f64, now: i64) -> f64 {
        let (_, y) = self.cal_haut(text, pw, now);
        self.cal_lines(text, pw, now).last().map_or(y + COLS + 60.0, |(_, r)| r.y + r.h) + 20.0 + PIED + crate::eco::marge_ai()
    }

    pub(crate) fn cal_click(&mut self, text: &Text, pw: f64, x: f64, y: f64, now: i64) -> EcoClick {
        match self.cal_zones(text, pw, now).into_iter().find(|(r, _)| r.contains(x, y)).map(|(_, z)| z) {
            Some(CZone::Periode(s)) => self.semaine = s,
            Some(CZone::Impact(k)) => self.impact = k,
            Some(CZone::Devise(k)) => self.currency = k,
            Some(CZone::Annonce(i)) => {
                self.carte = (self.carte != Some(i)).then_some(i);
                return EcoClick::Carte;
            }
            Some(CZone::Learn(row)) => return EcoClick::Learn(row),
            None => return EcoClick::None,
        }
        EcoClick::Changed
    }

    pub(crate) fn cal_draw(&self, pm: &mut Pixmap, text: &Text, pw: f64, now: i64, mouse: Option<(f64, f64)>) {
        let gras = Text::graisse(Graisse::Bold);
        let survol = |r: &Rect| mouse.is_some_and(|(x, y)| r.contains(x, y));
        gras.draw(pm, "📅 Calendrier", (PAD as f32, 31.0), 17.0, j::TEXTE, Align::Left);
        let (puces, _) = self.puces(text, pw);
        let (banniere, bas) = self.cal_haut(text, pw, now);
        // 🚫 the next PropFirm release and its window without orders (red when on or in less than 15 min)
        if let Some(r) = banniere {
            let (msg, hot) = self.news_banner(now);
            let red = crate::palette::down();
            round_fill(pm, r, r.h / 2.0, if hot { red } else { j::BLANC }, if hot { 0.25 } else { 0.05 });
            round_stroke(pm, r, r.h / 2.0, if hot { red } else { j::sur_fond(j::LISERE) }, 1.0);
            text.draw(
                pm,
                &text.fit(&msg, 12.5, r.w - 24.0),
                ((r.x + 12.0) as f32, (r.cy() + 4.5) as f32),
                12.5,
                j::TEXTE,
                Align::Left,
            );
        }
        let seg = Rect::new(PAD, HEAD + 2.0, pw - 2.0 * PAD, 32.0);
        round_fill(pm, seg, 16.0, j::BLANC, 0.05);
        for (r, z) in &puces {
            match z {
                CZone::Periode(s) => {
                    let on = *s == self.semaine;
                    if on {
                        round_fill(pm, Rect::new(r.x + 3.0, r.y + 3.0, r.w - 6.0, r.h - 6.0), 13.0, j::ACCENT, 0.35);
                    }
                    let l = if *s { "Semaine" } else { "Aujourd'hui" };
                    (if on { &gras } else { text }).draw(pm, l, (r.cx() as f32, (r.cy() + 5.0) as f32), 13.5, j::TEXTE, Align::Center);
                }
                CZone::Impact(k) | CZone::Devise(k) => {
                    let (l, on) = match z {
                        CZone::Impact(_) => (IMPACTS[*k], *k == self.impact),
                        _ => (CURRENCIES[*k], *k == self.currency),
                    };
                    let red = matches!(z, CZone::Impact(0)) && on;
                    let c = if red { crate::palette::down() } else { j::ACCENT };
                    round_fill(
                        pm,
                        *r,
                        r.h / 2.0,
                        if on { c } else { j::BLANC },
                        if on {
                            0.25
                        } else if survol(r) {
                            0.1
                        } else {
                            0.05
                        },
                    );
                    round_stroke(pm, *r, r.h / 2.0, if on { c } else { j::sur_fond(j::LISERE) }, 1.0);
                    text.draw(pm, l, (r.cx() as f32, (r.cy() + 4.5) as f32), 12.5, j::TEXTE, Align::Center);
                }
                CZone::Annonce(_) | CZone::Learn(_) => {}
            }
        }
        // columns
        let (xr, xp, xc) = colonnes(pw);
        let yc = (bas + 18.0) as f32;
        text.draw(pm, "Heure", (PAD as f32, yc), 11.5, j::DIM, Align::Left);
        text.draw(pm, "Annonce", ((PAD + 64.0) as f32, yc), 11.5, j::DIM, Align::Left);
        for (x, l) in [(xr, "Réel"), (xp, "Prévu"), (xc, "Préc.")] {
            text.draw(pm, l, (x as f32, yc), 11.5, j::DIM, Align::Right);
        }
        let lines = self.cal_lines(text, pw, now);
        if lines.is_empty() {
            let msg = if !self.note.is_empty() {
                self.note.as_str()
            } else if self.agenda.is_empty() {
                "Chargement du calendrier…"
            } else if self.semaine {
                "Rien avec ces filtres cette semaine."
            } else {
                "Rien avec ces filtres aujourd'hui."
            };
            text.draw(
                pm,
                &text.fit(msg, 13.0, pw - 2.0 * PAD),
                (PAD as f32, (bas + COLS + 24.0) as f32),
                13.0,
                j::DIM,
                Align::Left,
            );
        }
        for (it, r) in lines {
            match it {
                Item::Jour(d) => {
                    gras.draw(pm, &d.to_uppercase(), (PAD as f32, (r.y + 20.0) as f32), 11.5, j::DIM, Align::Left);
                }
                Item::Maintenant => {
                    let s = (now + crate::settings::tz_offset("America/New_York", now).unwrap_or(0)).rem_euclid(86_400);
                    let l = format!("{:02}:{:02} · maintenant", s / 3600, s % 3600 / 60);
                    let w = text.width(&l, 12.0) as f64;
                    text.draw(pm, &l, (PAD as f32, (r.y + 16.0) as f32), 12.0, j::ACCENT, Align::Left);
                    crate::fill(pm, PAD + w + 8.0, r.y + 12.0, pw - 2.0 * PAD - w - 8.0, 1.0, j::ACCENT, 0.8);
                }
                Item::Annonce(i) => {
                    let e = &self.agenda[i];
                    let past = e.ts <= now;
                    let choisi = self.carte == Some(i);
                    let zr = Rect::new(6.0, r.y + 1.0, pw - 12.0, r.h - 2.0);
                    if choisi {
                        round_fill(pm, zr, j::R_LIGNE, j::ACCENT, 0.16);
                        round_stroke(pm, zr, j::R_LIGNE, j::ACCENT, 0.9);
                    } else if survol(&zr) {
                        round_fill(pm, zr, j::R_LIGNE, j::BLANC, 0.05);
                    }
                    let y = r.y + 21.0;
                    let t = if e.impact == "Holiday" {
                        "Jour".to_string()
                    } else {
                        ny(e.ts).format("%H:%M").to_string()
                    };
                    text.draw(pm, &t, (PAD as f32, y as f32), 13.5, if past { j::DIM } else { j::TEXTE }, Align::Left);
                    // 📖 the Academy's sheet (as before the Calendrier: US dollar releases it explains)
                    if crate::eco::learn_row(e).is_some() {
                        let b = livre_rect(&r);
                        if survol(&b) {
                            round_fill(pm, Rect::new(b.x + 2.0, b.y + 1.0, 30.0, b.h - 2.0), 8.0, j::BLANC, 0.10);
                        }
                        text.draw(pm, "📖", ((PAD + 1.0) as f32, (y + 19.0) as f32), 13.0, j::TEXTE, Align::Left);
                    }
                    // impact dots
                    let (n, c) = match e.impact.as_str() {
                        "High" => (3, crate::palette::down()),
                        "Medium" => (2, Rgb(0xf5, 0x9e, 0x0b)),
                        _ => (1, Rgb(0xea, 0xb3, 0x08)),
                    };
                    for k in 0..3 {
                        let on = k < n;
                        round_fill(
                            pm,
                            Rect::new(PAD + 52.0 + k as f64 * 7.0, y - 8.0, 5.5, 5.5),
                            2.75,
                            if on { c } else { j::DIM },
                            if on { 1.0 } else { 0.4 },
                        );
                    }
                    gras.draw(pm, &e.country, ((PAD + 76.0) as f32, y as f32), 11.5, j::DIM, Align::Left);
                    let lignes = Self::titre_lignes(&e.title, pw);
                    for (k, l) in lignes.iter().enumerate() {
                        gras.draw(pm, l, ((PAD + 108.0) as f32, (y + k as f64 * 18.0) as f32), 13.5, j::TEXTE, Align::Left);
                    }
                    let y2 = y + (lignes.len().max(1) - 1) as f64 * 18.0;
                    let reel_s = reel::french(&e.actual);
                    let reel_s = if reel_s.is_empty() && past && reel::known(e) {
                        "—".to_string()
                    } else {
                        reel_s
                    };
                    gras.draw(pm, &reel_s, (xr as f32, y as f32), 13.5, couleur_reel(e), Align::Right);
                    text.draw(pm, &reel::french(&e.forecast), (xp as f32, y as f32), 13.0, j::DIM, Align::Right);
                    text.draw(pm, &reel::french(&e.previous), (xc as f32, y as f32), 13.0, j::DIM, Align::Right);
                    if !past && high_impact(e) {
                        text.draw(pm, &dans(e.ts - now), ((PAD + 108.0) as f32, (y2 + 19.0) as f32), 12.0, j::DIM, Align::Left);
                    }
                    if let Some(p) = self.reactions.get(&e.ts).and_then(|r| r[0]).filter(|_| past && high_impact(e)) {
                        let s = reel::reaction_text(&self.reaction_coin, p, "5 min");
                        let w = text.width(&s, 12.5) as f64 + 22.0;
                        let b = Rect::new(PAD + 52.0, y2 + 10.0, w, 22.0);
                        round_fill(pm, b, 11.0, j::BLANC, 0.06);
                        round_stroke(pm, b, 11.0, j::sur_fond(j::LISERE), 1.0);
                        let c = if p >= 0.0 { crate::palette::up() } else { crate::palette::down() };
                        text.draw(pm, &s, ((b.x + 11.0) as f32, (b.cy() + 4.5) as f32), 12.5, c, Align::Left);
                    }
                }
            }
        }
        // (📱 above the room of the round Argos AI button)
        let h = crate::px::lh(pm) - crate::eco::marge_ai();
        crate::fill(pm, PAD, h - PIED, pw - 2.0 * PAD, 1.0, j::BLANC, 0.08);
        text.draw(pm, "Heure de New York", (PAD as f32, (h - PIED / 2.0 + 5.0) as f32), 12.5, j::DIM, Align::Left);
        text.draw(
            pm,
            "Forex Factory · Réel : BLS / Fed",
            ((pw - PAD) as f32, (h - PIED / 2.0 + 5.0) as f32),
            12.5,
            j::DIM,
            Align::Right,
        );
    }
}

/// What a click on the card did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CarteClick {
    None,
    Fermer,
    /// The chart centred on this time.
    VoirChart(i64),
}

/// 📅 The card of a release (PC: glass card ≈ 360 px left of the panel; phone: a sheet).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Carte {
    pub ev: Event,
    /// The chart's crypto (« BTC », empty: not a crypto chart) and its symbol (« BTCUSDT.P »).
    pub coin: String,
    pub symbole: String,
    /// % at 5 min, 15 min, 1 h, and the curve (time, close) from −30 to +60 min.
    pub reaction: Option<[Option<f64>; 3]>,
    pub courbe: Vec<(i64, f64)>,
    pub now: i64,
}

pub const CARTE_W: f64 = 360.0;

impl Carte {
    fn lignes(text: &Text, s: &str, w: f64) -> Vec<String> {
        crate::assistant::wrap(text, s, 13.5, w)
    }

    /// Its parts top to bottom: (height of each) and the button's place.
    fn hauteur(&self, text: &Text, w: f64) -> (f64, Rect) {
        let mut y = 92.0 + 76.0;
        if let Some((a, b)) = fiche(&self.ev.title) {
            y += 22.0 + Self::lignes(text, a, w - 2.0 * PAD).len() as f64 * 19.0 + 10.0;
            y += 22.0 + Self::lignes(text, b, w - 2.0 * PAD).len() as f64 * 19.0 + 10.0;
        }
        if self.ev.ts <= self.now {
            if !self.coin.is_empty() {
                y += 22.0 + 110.0 + 8.0 + 52.0 + 10.0;
            }
        } else {
            y += 30.0;
        }
        let b = Rect::new(PAD, y + 4.0, w - 2.0 * PAD, 42.0);
        (y + 4.0 + 42.0 + PAD, b)
    }

    /// The card's size at width `w`.
    pub fn taille(&self, text: &Text, w: f64) -> (f64, f64) {
        (w, self.hauteur(text, w).0)
    }

    /// A click at (`x`, `y`) in the card (its own points).
    pub fn click(&self, text: &Text, w: f64, x: f64, y: f64) -> CarteClick {
        let (h, b) = self.hauteur(text, w);
        if Rect::new(w - PAD - 32.0, 14.0, 32.0, 32.0).contains(x, y) || !(0.0..=w).contains(&x) || !(0.0..=h).contains(&y) {
            CarteClick::Fermer
        } else if b.contains(x, y) {
            CarteClick::VoirChart(self.ev.ts)
        } else {
            CarteClick::None
        }
    }

    /// Draws the card `w` wide.
    pub fn draw(&self, text: &Text, w: f64, mouse: Option<(f64, f64)>) -> Option<Pixmap> {
        let (h, bouton) = self.hauteur(text, w);
        let mut pm = crate::px::new(w.ceil() as u32, h.ceil() as u32)?;
        let gras = Text::graisse(Graisse::Bold);
        let carte = Rect::new(0.5, 0.5, w - 1.0, h - 1.0);
        round_fill(&mut pm, carte, j::R_VOLET, Rgb(0x12, 0x15, 0x22), 1.0);
        round_stroke(&mut pm, carte, j::R_VOLET, j::sur_fond(j::LISERE), 1.0);
        let e = &self.ev;
        gras.draw(
            &mut pm,
            &gras.fit(&e.title, 18.0, w - 2.0 * PAD - 80.0),
            (PAD as f32, 38.0),
            18.0,
            j::TEXTE,
            Align::Left,
        );
        let (n, c) = match e.impact.as_str() {
            "High" => (3, crate::palette::down()),
            "Medium" => (2, Rgb(0xf5, 0x9e, 0x0b)),
            _ => (1, Rgb(0xea, 0xb3, 0x08)),
        };
        for k in 0..3 {
            round_fill(
                &mut pm,
                Rect::new(w - PAD - 70.0 + k as f64 * 9.0, 27.0, 7.0, 7.0),
                3.5,
                if k < n { c } else { j::DIM },
                if k < n { 1.0 } else { 0.4 },
            );
        }
        let f = Rect::new(w - PAD - 32.0, 14.0, 32.0, 32.0);
        round_fill(
            &mut pm,
            f,
            16.0,
            j::BLANC,
            if mouse.is_some_and(|(x, y)| f.contains(x, y)) { 0.14 } else { 0.07 },
        );
        text.draw(&mut pm, "✕", (f.cx() as f32, (f.cy() + 5.0) as f32), 14.0, j::TEXTE, Align::Center);
        let pays = match e.country.as_str() {
            "USD" => "États-Unis",
            "EUR" => "Zone euro",
            "GBP" => "Royaume-Uni",
            "JPY" => "Japon",
            "CAD" => "Canada",
            "AUD" => "Australie",
            "NZD" => "Nouvelle-Zélande",
            "CHF" => "Suisse",
            "CNY" => "Chine",
            _ => "",
        };
        let quand = format!("{}, {}", jour(e.ts), ny(e.ts).format("%H:%M"));
        let sous = [e.country.as_str(), pays, quand.as_str(), &format!("source {}", source_of(e))]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · ");
        text.draw(&mut pm, &text.fit(&sous, 13.0, w - 2.0 * PAD), (PAD as f32, 66.0), 13.0, j::DIM, Align::Left);
        // tiles
        let tw = (w - 2.0 * PAD - 16.0) / 3.0;
        for (k, (l, v, col)) in [
            ("Réel", reel::french(&e.actual), couleur_reel(e)),
            ("Prévu", reel::french(&e.forecast), j::TEXTE),
            ("Précédent", reel::french(&e.previous), j::TEXTE),
        ]
        .into_iter()
        .enumerate()
        {
            let r = Rect::new(PAD + k as f64 * (tw + 8.0), 84.0, tw, 64.0);
            round_fill(&mut pm, r, j::R_TUILE * 0.75, j::BLANC, 0.05);
            round_stroke(&mut pm, r, j::R_TUILE * 0.75, j::sur_fond(j::LISERE), 1.0);
            text.draw(&mut pm, l, ((r.x + 12.0) as f32, (r.y + 22.0) as f32), 12.0, j::DIM, Align::Left);
            let v = if v.is_empty() { "—".to_string() } else { v };
            gras.draw(&mut pm, &v, ((r.x + 12.0) as f32, (r.y + 50.0) as f32), 20.0, col, Align::Left);
        }
        let mut y = 168.0;
        if let Some((a, b)) = fiche(&e.title) {
            for (t, s) in [("CE QUE C'EST", a), ("POURQUOI ÇA COMPTE", b)] {
                gras.draw(&mut pm, t, (PAD as f32, (y + 14.0) as f32), 11.5, j::DIM, Align::Left);
                y += 22.0;
                for l in Self::lignes(text, s, w - 2.0 * PAD) {
                    text.draw(&mut pm, &l, (PAD as f32, (y + 14.0) as f32), 13.5, j::TEXTE, Align::Left);
                    y += 19.0;
                }
                y += 10.0;
            }
        }
        if e.ts <= self.now {
            if !self.coin.is_empty() {
                gras.draw(&mut pm, "RÉACTION DU PRIX", (PAD as f32, (y + 14.0) as f32), 11.5, j::DIM, Align::Left);
                gras.draw(&mut pm, &self.symbole, ((w - PAD) as f32, (y + 14.0) as f32), 11.5, j::TEXTE, Align::Right);
                y += 22.0;
                let g = Rect::new(PAD, y, w - 2.0 * PAD, 110.0);
                round_fill(&mut pm, g, 12.0, j::BLANC, 0.04);
                self.courbe_draw(&mut pm, text, g);
                y += 118.0;
                for (k, l) in ["5 min", "15 min", "1 h"].iter().enumerate() {
                    let r = Rect::new(PAD + k as f64 * (tw + 8.0), y, tw, 52.0);
                    round_fill(&mut pm, r, 12.0, j::BLANC, 0.05);
                    text.draw(&mut pm, l, (r.cx() as f32, (r.y + 19.0) as f32), 12.0, j::DIM, Align::Center);
                    let p = self.reaction.and_then(|v| v[k]);
                    let (s, c) = match p {
                        Some(p) => (
                            reel::reaction_text("", p, "").trim().trim_end_matches(" en").to_string(),
                            if p >= 0.0 { crate::palette::up() } else { crate::palette::down() },
                        ),
                        None => ("—".to_string(), j::DIM),
                    };
                    gras.draw(&mut pm, &s, (r.cx() as f32, (r.y + 41.0) as f32), 16.0, c, Align::Center);
                }
            }
        } else {
            text.draw(&mut pm, &dans(e.ts - self.now), (PAD as f32, (y + 18.0) as f32), 14.0, j::ACCENT, Align::Left);
        }
        let hb = mouse.is_some_and(|(x, yy)| bouton.contains(x, yy));
        round_fill(&mut pm, bouton, bouton.h / 2.0, j::ACCENT, if hb { 0.35 } else { 0.2 });
        round_stroke(&mut pm, bouton, bouton.h / 2.0, j::ACCENT, 0.9);
        gras.draw(
            &mut pm,
            "Voir sur la chart ›",
            (bouton.cx() as f32, (bouton.cy() + 5.0) as f32),
            14.5,
            j::TEXTE,
            Align::Center,
        );
        Some(pm)
    }

    /// The small ±30 min curve (then to +60) with the release's time dotted.
    fn courbe_draw(&self, pm: &mut Pixmap, text: &Text, g: Rect) {
        let pts: Vec<&(i64, f64)> = self
            .courbe
            .iter()
            .filter(|(t, _)| (self.ev.ts - 1800..=self.ev.ts + 1800).contains(t))
            .collect();
        let (t0, t1) = (self.ev.ts - 1800, self.ev.ts + 1800);
        let x = |t: i64| g.x + 8.0 + (t - t0) as f64 / (t1 - t0) as f64 * (g.w - 16.0);
        // (n° 24: no 1-minute candles yet, the line of the hour crossed the words saying so: drawn with a curve only)
        if pts.len() >= 2 {
            crate::fill(pm, x(self.ev.ts), g.y + 6.0, 1.0, g.h - 26.0, j::VIOLET, 0.7);
        }
        let hm = |t: i64| ny(t).format("%H:%M").to_string();
        for (t, a) in [(t0, Align::Left), (self.ev.ts, Align::Center), (t1, Align::Right)] {
            let xx = match a {
                Align::Left => g.x + 8.0,
                Align::Right => g.x + g.w - 8.0,
                _ => x(t),
            };
            text.draw(pm, &hm(t), (xx as f32, (g.y + g.h - 6.0) as f32), 11.0, j::DIM, a);
        }
        if pts.len() < 2 {
            text.draw(pm, "Bougies 1 min pas encore là", (g.cx() as f32, g.cy() as f32), 12.0, j::DIM, Align::Center);
            return;
        }
        let (lo, hi) = pts.iter().fold((f64::MAX, f64::MIN), |a, p| (a.0.min(p.1), a.1.max(p.1)));
        let y = |v: f64| g.y + 10.0 + (1.0 - (v - lo) / (hi - lo).max(1e-12)) * (g.h - 40.0);
        let mut pb = tiny_skia::PathBuilder::new();
        for (k, (t, v)) in pts.iter().enumerate() {
            if k == 0 {
                pb.move_to(x(*t) as f32, y(*v) as f32);
            } else {
                pb.line_to(x(*t) as f32, y(*v) as f32);
            }
        }
        if let Some(p) = pb.finish() {
            let mut paint = tiny_skia::Paint::default();
            paint.set_color_rgba8(0x5b, 0x9c, 0xff, 255);
            paint.anti_alias = true;
            let st = tiny_skia::Stroke {
                width: 2.0,
                ..Default::default()
            };
            pm.stroke_path(&p, &paint, &st, crate::px::t(), None);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(title: &str, impact: &str, ts: i64) -> Event {
        Event {
            title: title.into(),
            country: "USD".into(),
            impact: impact.into(),
            ts,
            forecast: "0.3%".into(),
            previous: "0.2%".into(),
            ..Default::default()
        }
    }

    // Wed 25 Nov 2026 10:00 and 14:00 New York, Thu 26 Nov 2026 08:30
    const MER10: i64 = 1_795_618_800;

    fn eco() -> Eco {
        Eco {
            agenda: vec![
                Event {
                    actual: "0.4%".into(),
                    ..ev("Core PCE Price Index m/m", "High", MER10)
                },
                ev("FOMC Meeting Minutes", "High", MER10 + 4 * 3600),
                ev("Unemployment Claims", "High", MER10 + 86_400 - 5400),
                ev("Crude Oil Inventories", "Low", MER10 + 1800),
            ],
            semaine: true,
            ..Default::default()
        }
    }

    /// 📅 Today / Week, the « now » line between past and coming, the columns, the click → card.
    #[test]
    fn calendrier_panneau() {
        let t = Text::default();
        let now = MER10 + 4320; // 11:12
        let mut e = eco();
        let items = e.cal_items(now);
        assert_eq!(items[0], Item::Jour("Mer. 25 nov.".into()));
        assert_eq!(items[1], Item::Annonce(0));
        assert_eq!(items[2], Item::Maintenant, "between the past and the coming releases");
        assert_eq!(items[3], Item::Annonce(1));
        assert_eq!(items[4], Item::Jour("Jeu. 26 nov.".into()));
        // nothing left today: the line sits above the next day's header, not under it (captures 10, 11)
        let items = e.cal_items(MER10 + 5 * 3600);
        let k = items.iter().position(|i| *i == Item::Maintenant).unwrap();
        assert_eq!(items[k + 1], Item::Jour("Jeu. 26 nov.".into()), "{items:?}");
        assert_eq!(items[k - 1], Item::Annonce(1));
        // today with only coming releases: under today's header
        let items = e.cal_items(MER10 - 3600);
        assert_eq!(items[..2], [Item::Jour("Mer. 25 nov.".into()), Item::Maintenant]);
        e.semaine = false;
        assert_eq!(e.cal_shown(now), [0, 1], "today only (the low one is not shown with « Fort »)");
        let (r, _) = e.cal_zones(&t, 390.0, now).into_iter().find(|(_, z)| *z == CZone::Periode(true)).unwrap();
        assert_eq!(e.cal_click(&t, 390.0, r.cx(), r.cy(), now), EcoClick::Changed);
        assert!(e.semaine);
        let (r, _) = e.cal_zones(&t, 390.0, now).into_iter().find(|(_, z)| *z == CZone::Annonce(1)).unwrap();
        assert_eq!(e.cal_click(&t, 390.0, r.cx(), r.cy(), now), EcoClick::Carte);
        assert_eq!(e.carte, Some(1));
        // 📖 under the time: the Academy's sheet; the rest of the line: the card
        let zs = e.cal_zones(&t, 390.0, now);
        let (b, z) = zs.iter().find(|(_, z)| matches!(z, CZone::Learn(_))).cloned().unwrap();
        let CZone::Learn(row) = z else { unreachable!() };
        assert_eq!(Some(row), crate::eco::learn_row(&e.agenda[0]), "Core PCE: the PCE row");
        assert_eq!(e.cal_click(&t, 390.0, b.cx(), b.cy(), now), EcoClick::Learn(row));
        assert_eq!(e.carte, Some(1), "📖 does not open the card");
        let (a, _) = zs.iter().find(|(_, z)| *z == CZone::Annonce(0)).unwrap();
        assert!(a.contains(b.cx(), b.cy()), "the 📖 lies in its line");
        assert_eq!(e.cal_click(&t, 390.0, a.x + a.w - 20.0, b.cy(), now), EcoClick::Carte);
        assert_eq!(e.carte, Some(0));
        e.carte = Some(1);
        assert_eq!(e.cal_reactions(now), [MER10], "past high impact only");
        e.reactions.insert(MER10, [Some(-1.23), Some(-1.6), None]);
        e.reaction_coin = "BTC".into();
        for pw in [320.0, 390.0, 480.0] {
            for (r, _) in e.cal_zones(&t, pw, now) {
                assert!(r.x + r.w <= pw + 0.1);
            }
            let mut pm = crate::px::new(pw as u32, e.cal_h(&t, pw, now) as u32).unwrap();
            e.cal_draw(&mut pm, &t, pw, now, None);
        }
        assert_eq!(dans(2 * 3600 + 48 * 60 + 5), "dans 2 h 48");
        assert_eq!(dans(2 * 86_400 + 5 * 3600 + 60), "dans 2 j 5 h");
        assert_eq!(jour(MER10), "Mer. 25 nov.");
    }

    /// 📅 The card: what it is and why it matters for a known release, nothing for an unknown one; « Voir sur la
    /// chart › » gives the release's time; ✕ closes.
    #[test]
    fn carte_d_une_annonce() {
        let t = Text::default();
        assert!(fiche("Core PCE Price Index m/m").is_some_and(|f| f.0.contains("Fed")));
        assert!(fiche("CPI m/m").is_some());
        assert!(fiche("Non-Farm Employment Change").is_some());
        assert!(fiche("Belgian Consumer Confidence").is_none());
        assert_eq!(source_of(&ev("CPI m/m", "High", 0)), "BLS");
        assert_eq!(source_of(&ev("Federal Funds Rate", "High", 0)), "Fed");
        let mut c = Carte {
            ev: eco().agenda[0].clone(),
            coin: "BTC".into(),
            symbole: "BTCUSDT.P".into(),
            reaction: Some([Some(-1.2), Some(-1.6), Some(-0.9)]),
            courbe: (-30..=60).map(|k| (MER10 + k * 60, 100.0 - if k > 0 { 1.0 } else { 0.0 })).collect(),
            now: MER10 + 4320,
        };
        let (w, h) = c.taille(&t, CARTE_W);
        assert!(c.draw(&t, w, None).is_some());
        let (_, b) = c.hauteur(&t, w);
        assert_eq!(c.click(&t, w, b.cx(), b.cy()), CarteClick::VoirChart(MER10));
        assert_eq!(c.click(&t, w, w - PAD - 16.0, 30.0), CarteClick::Fermer);
        assert_eq!(c.click(&t, w, w / 2.0, h / 2.0 - 100.0), CarteClick::None);
        c.ev = ev("Belgian Consumer Confidence", "Medium", MER10 + 9000);
        let (_, h2) = c.taille(&t, CARTE_W);
        assert!(h2 < h, "unknown and coming: no texts, no reaction");
        assert!(c.draw(&t, w, None).is_some());
    }
}
