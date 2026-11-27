//! 📅 Agenda et 📰 Éco / News, **comme les panneaux d'Argos v1** :
//! - Agenda : le calendrier économique de la semaine (Forex Factory), par jour, heure de New York, pastille d'impact,
//!   devise, résultat / prévision / précédent ; filtres impact (Fort, Moyen +, Tout) et devise ;
//!   🚫 news PropFirm : les annonces à fort impact sont marquées, et un bandeau donne la prochaine (compte à rebours) avec
//!   sa fenêtre sans ordre (2 min avant / 2 min après, règle prop firm standard) ;
//! - News : les nouvelles crypto / marchés (flux RSS), la plus récente en haut ; toucher = ouvrir l'article.
//!
//! Le dessin et les clics : le Calendrier dans `calendrier`, les Actualités dans `actualites` (maquettes de Jee du
//! 22 nov. 2026).

use crate::text::Text;
use crate::{fill, Theme};
use argos_data::eco::{Event, News};
use chrono::TimeZone;
use tiny_skia::Pixmap;

/// Filtres d'impact et de devise.
pub const IMPACTS: [&str; 3] = ["🔴 Fort", "🟠 Moyen +", "Tout"];
pub const CURRENCIES: [&str; 6] = ["Toutes", "USD", "EUR", "GBP", "CAD", "JPY"];

/// Annonce autour de laquelle les prop firms interdisent de trader (fort impact).
pub fn high_impact(e: &Event) -> bool {
    e.impact == "High"
}

/// Ligne de la fiche news de l'Académie pour une annonce (seulement le dollar US : la fiche parle du DXY).
pub fn learn_row(e: &Event) -> Option<usize> {
    (e.country == "USD").then(|| crate::academy::news_row(&e.title)).flatten()
}

/// 🛡️ Fenêtre sans ordre du gardien autour d'une news à fort impact (secondes avant et après).
pub const NEWS_GAP: i64 = 120;

/// 🛡️ Devises touchées par une news pour le symbole `sym` (comme v1) : forex et métaux OANDA (« XAUUSD » → XAU, USD ;
/// pétrole → + OIL), marchés HIP-3 (« xyz:GOLD » → USD ; xyz:EUR → USD, EUR) ; rien pour la crypto.
pub fn news_ccys(sym: &str) -> Vec<String> {
    let s = sym.to_uppercase();
    if let Some((_, k)) = s.split_once(':') {
        return match k {
            "CL" | "BRENTOIL" | "HO" => vec!["USD".into(), "OIL".into()],
            "EUR" | "JPY" | "GBP" => vec!["USD".into(), k.into()],
            _ => vec!["USD".into()],
        };
    }
    let Some(o) = argos_data::oanda::OANDA.iter().find(|o| o.0.replace('_', "") == s || o.0 == s) else {
        return vec![];
    };
    let (b, q) = o.0.split_once('_').unwrap_or((o.0, ""));
    let mut out = vec![b.to_string(), q.to_string()];
    if b.contains("WTICO") || b.contains("BCO") {
        out.push("OIL".into());
    }
    out
}

/// 🛡️ News à fort impact en cours pour le symbole `sym` à `now` (secondes) : de 2 min avant à 2 min après, sur une
/// devise du symbole. `Some("USD CPI m/m")`. Sans calendrier chargé, rien ne bloque.
pub fn news_live(agenda: &[Event], sym: &str, now: i64) -> Option<String> {
    let ccys = news_ccys(sym);
    agenda
        .iter()
        .filter(|e| high_impact(e) && ccys.contains(&e.country))
        .find(|e| now >= e.ts - NEWS_GAP && now <= e.ts + NEWS_GAP)
        .map(|e| format!("{} {}", e.country, e.title))
}

/// « 2 h 05 min », « 12 min », « 40 s ».
fn span(s: i64) -> String {
    let s = s.max(0);
    if s < 60 {
        format!("{s} s")
    } else if s < 3600 {
        format!("{} min", s / 60)
    } else if s < 86_400 {
        format!("{} h {:02} min", s / 3600, s / 60 % 60)
    } else {
        format!("{} j {} h", s / 86_400, s / 3600 % 24)
    }
}

/// 📱 Room kept at the bottom of the Calendrier and the Actualités by touch, under the round Argos AI button laid over
/// the sheet's bottom right: their bottom line ends above it (iPhone capture 11: it covered « BLS / Fed »).
pub fn marge_ai() -> f64 {
    if crate::actions::tactile() {
        70.0
    } else {
        0.0
    }
}

/// Ce que montre le panneau.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EcoKind {
    Agenda,
    News,
}

/// Données et filtres des deux panneaux.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Eco {
    pub agenda: Vec<Event>,
    pub news: Vec<News>,
    /// Index dans `IMPACTS` et `CURRENCIES`.
    pub impact: usize,
    pub currency: usize,
    /// Message de l'Agenda et des News (chargement, erreur), chacun le sien.
    pub note: String,
    pub news_note: String,
    /// 📰 Actualités : la crypto filtrée (« BTC », ouvert par « Voir tout › » ou un ⚡), l'onglet (0 = Toutes),
    /// la recherche (`Some` : le champ reçoit le clavier), ⚙ ouvert, les sources décochées, la langue (index de
    /// `actualites::LANGUES`), ⚡ sur la chart, et l'heure des nouvelles reçues (s).
    pub filtre_coin: Option<String>,
    pub onglet: usize,
    pub recherche: Option<String>,
    pub reglages_news: bool,
    pub sources_off: Vec<String>,
    pub langue: usize,
    pub eclairs: bool,
    pub news_at: Option<i64>,
    /// 📰 How far the row of tabs is slid to the left (points; 0 when the tabs fit on the line).
    pub onglets_x: f64,
    /// 📅 Calendrier : la semaine (sinon aujourd'hui), l'annonce dont la carte est ouverte (index dans `agenda`), les
    /// réactions de la chart (heure de l'annonce → % à 5 min, 15 min, 1 h) et leur crypto.
    pub semaine: bool,
    pub carte: Option<usize>,
    pub reactions: std::collections::HashMap<i64, [Option<f64>; 3]>,
    pub reaction_coin: String,
}

/// Résultat d'un clic.
#[derive(Debug, Clone, PartialEq)]
pub enum EcoClick {
    None,
    Changed,
    /// Ouvrir cet article.
    Open(String),
    /// ✕ : fermer le panneau.
    Close,
    /// 📅 La carte d'une annonce s'ouvre ou se ferme (`carte`).
    Carte,
    /// 📖 Ouvrir la fiche « News éco → DXY / XAU » de l'Académie à cette ligne du tableau.
    Learn(usize),
}

/// Heure de New York d'un instant (secondes Unix).
fn ny(ts: i64) -> chrono::DateTime<chrono_tz::Tz> {
    chrono_tz::America::New_York
        .timestamp_opt(ts, 0)
        .single()
        .unwrap_or_else(|| chrono_tz::America::New_York.timestamp_opt(0, 0).unwrap())
}

const JOURS: [&str; 7] = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];

impl Eco {
    /// 🚫 Prochaine news PropFirm (toutes devises) dont la fenêtre sans ordre n'est pas finie à `now`.
    pub fn next_news(&self, now: i64) -> Option<&Event> {
        self.agenda.iter().filter(|e| high_impact(e) && e.ts + NEWS_GAP >= now).min_by_key(|e| e.ts)
    }

    /// Texte du bandeau PropFirm et s'il faut alerter (fenêtre en cours ou dans moins de 15 min).
    pub(crate) fn news_banner(&self, now: i64) -> (String, bool) {
        let Some(e) = self.next_news(now) else {
            return (
                if self.agenda.is_empty() {
                    String::new()
                } else {
                    "✅ Plus de news PropFirm cette semaine".into()
                },
                false,
            );
        };
        let hm = |ts: i64| ny(ts).format("%H:%M").to_string();
        if now >= e.ts - NEWS_GAP {
            (
                format!("🚫 PropFirm : pas d'ordre jusqu'à {} · {} {}", hm(e.ts + NEWS_GAP), e.country, e.title),
                true,
            )
        } else {
            let d = ny(e.ts);
            use chrono::Datelike;
            (
                format!(
                    "🚫 News PropFirm dans {} · pas d'ordre {}–{} · {} {} · {} {}",
                    span(e.ts - now),
                    hm(e.ts - NEWS_GAP),
                    hm(e.ts + NEWS_GAP),
                    e.country,
                    e.title,
                    JOURS[d.weekday().num_days_from_monday() as usize],
                    hm(e.ts)
                ),
                e.ts - NEWS_GAP - now < 900,
            )
        }
    }

    /// Hauteur du contenu (le panneau défile) à la largeur `pw` et à l'heure `now`.
    pub fn content_h(&self, kind: EcoKind, pw: f64, now: i64) -> f64 {
        let t = Text::default();
        match kind {
            EcoKind::Agenda => self.cal_h(&t, pw, now),
            EcoKind::News => self.news_h(&t, pw),
        }
    }

    /// Clic (coordonnées du contenu, défilement déjà compté) ; `now` : heure (secondes Unix).
    pub fn click(&mut self, kind: EcoKind, text: &Text, pw: f64, x: f64, y: f64, now: i64) -> EcoClick {
        match kind {
            EcoKind::Agenda => self.cal_click(text, pw, x, y, now),
            EcoKind::News => self.news_click(text, pw, x, y),
        }
    }

    /// Dessine le panneau (largeur `pw`) dans `pm` (sa hauteur = le contenu) ; `now` : heure (secondes Unix) ; `tz` :
    /// décalage de l'heure affichée pour les nouvelles (s).
    #[allow(clippy::too_many_arguments)]
    pub fn draw(&self, kind: EcoKind, pm: &mut Pixmap, text: &Text, pw: f64, now: i64, mouse: Option<(f64, f64)>, th: &Theme, tz: i64) {
        let h = crate::px::lh(pm);
        fill(pm, 0.0, 0.0, pw, h, th.background, 1.0);
        match kind {
            EcoKind::Agenda => self.cal_draw(pm, text, pw, now, mouse),
            EcoKind::News => {
                self.news_draw(pm, text, pw, now, mouse, tz);
                self.news_pied(pm, text, pw, h, now, th.background);
            }
        }
    }

    /// 📰 What stays at the bottom of the view while the content scrolls (the Actualités' sources and language): drawn
    /// by the host over the part in view, `bas` its bottom (points of `pm`).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_pied(&self, kind: EcoKind, pm: &mut Pixmap, text: &Text, pw: f64, bas: f64, now: i64, th: &Theme) {
        if kind == EcoKind::News {
            self.news_pied(pm, text, pw, bas, now, th.background);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(title: &str, country: &str, impact: &str, ts: i64) -> Event {
        Event {
            title: title.into(),
            country: country.into(),
            impact: impact.into(),
            ts,
            ..Default::default()
        }
    }

    /// 🚫 The PropFirm banner of the Calendrier: the next high-impact release and its window without orders.
    #[test]
    fn bandeau_propfirm() {
        let e = Eco {
            agenda: vec![
                ev("BOC", "CAD", "High", 1793022300),
                ev("PMI", "EUR", "Low", 1793030000),
                ev("CPI", "USD", "High", 1793104200),
            ],
            ..Default::default()
        };
        assert_eq!(e.next_news(1793000000).map(|x| x.title.as_str()), Some("BOC"));
        assert_eq!(
            e.next_news(1793022300 + NEWS_GAP).map(|x| x.title.as_str()),
            Some("BOC"),
            "encore dans la fenêtre"
        );
        assert_eq!(
            e.next_news(1793022300 + NEWS_GAP + 1).map(|x| x.title.as_str()),
            Some("CPI"),
            "le PMI faible ne bloque pas"
        );
        assert!(e.next_news(1793104200 + NEWS_GAP + 1).is_none());
        let (msg, hot) = e.news_banner(1793022300 - 60);
        assert!(hot && msg.contains("pas d'ordre jusqu'à 09:47"), "{msg}");
        let (msg, hot) = e.news_banner(1793022300 + 200);
        assert!(!hot && msg.contains("CPI") && msg.contains("pas d'ordre 08:28–08:32"), "{msg}");
        assert_eq!(e.news_banner(1793200000).0, "✅ Plus de news PropFirm cette semaine");
        assert_eq!(span(7500), "2 h 05 min");
        assert_eq!(ny(1793104200).format("%H:%M").to_string(), "08:30");
        let t = Text::default();
        for k in [EcoKind::Agenda, EcoKind::News] {
            let mut pm = crate::px::new(390, e.content_h(k, 390.0, 1793050000) as u32).unwrap();
            e.draw(k, &mut pm, &t, 390.0, 1793050000, None, &Theme::default(), 0);
        }
    }

    #[test]
    fn news_qui_bloquent_le_gardien() {
        assert_eq!(news_ccys("XAUUSD"), ["XAU", "USD"]);
        assert_eq!(news_ccys("xyz:GOLD"), ["USD"]);
        assert_eq!(news_ccys("xyz:EUR"), ["USD", "EUR"]);
        assert!(news_ccys("BTC").is_empty(), "crypto : pas de devise");
        let ev = |c: &str, imp: &str, ts| Event {
            title: "CPI m/m".into(),
            country: c.into(),
            impact: imp.into(),
            ts,
            ..Default::default()
        };
        let cal = [ev("CAD", "High", 1_000), ev("USD", "Medium", 1_000), ev("USD", "High", 5_000)];
        assert_eq!(news_live(&cal, "XAUUSD", 1_000), None, "CAD et impact moyen : rien pour l'or");
        assert_eq!(news_live(&cal, "XAUUSD", 4_880).as_deref(), Some("USD CPI m/m"), "2 min avant");
        assert_eq!(news_live(&cal, "XAUUSD", 5_120).as_deref(), Some("USD CPI m/m"), "2 min après");
        assert_eq!(news_live(&cal, "XAUUSD", 5_121), None);
        assert_eq!(news_live(&cal, "BTC", 5_000), None, "crypto : jamais bloquée");
        assert_eq!(news_live(&[], "XAUUSD", 5_000), None, "sans calendrier : rien ne bloque");
    }
}
