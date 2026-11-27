//! 🕒 Textes du temps : dates (calendrier civil), étiquettes de l'axe, réticule, horloge, compte à rebours de la bougie.

use crate::*;

/// Horloge dans le fuseau affiché : « 18:20:15 UTC-4 ».
pub(crate) fn clock_text(now_s: i64, off: i64) -> String {
    let t = (now_s + off).rem_euclid(86_400);
    let zone = match (off / 3600, off.abs() % 3600 / 60) {
        (0, 0) => "UTC".to_string(),
        (h, 0) => format!("UTC{h:+}"),
        (h, m) => format!("UTC{h:+}:{m:02}"),
    };
    format!("{:02}:{:02}:{:02} {zone}", t / 3600, t % 3600 / 60, t % 60)
}

/// Date / heure courte d'une étiquette de temps : le jour du mois à minuit, sinon « HH:MM ».
/// (année, mois 1–12, jour) d'un jour compté depuis 1970 (algorithme de Hinnant).
pub(crate) fn civil(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// « 12 nov » : jour et mois d'une heure (secondes, déjà dans le fuseau affiché).
pub fn civil_day(t: i64) -> String {
    let (_, m, d) = civil(t.div_euclid(86_400));
    format!("{d} {}", MONTH_NAMES[(m - 1).clamp(0, 11) as usize])
}

pub(crate) const MONTH_NAMES: [&str; 12] = ["janv", "févr", "mars", "avr", "mai", "juin", "juil", "août", "sept", "oct", "nov", "déc"];

/// Case de l'échelle de temps d'une heure `t` et son étiquette : en mois et en années (très dézoomé), les mois du
/// calendrier (« mars », l'année en janvier ou par années : « 2024 ») ; sinon des blocs de `interval` secondes.
pub(crate) fn time_bucket(t: i64, interval: i64) -> (i64, String) {
    if interval >= viewport::MONTH {
        let (y, m, _) = civil(t.div_euclid(86_400));
        let step = (interval / viewport::MONTH).max(1);
        let b = (y * 12 + m - 1).div_euclid(step);
        let (by, bm) = ((b * step).div_euclid(12), (b * step).rem_euclid(12) + 1);
        let label = if step >= 12 || bm == 1 {
            by.to_string()
        } else {
            MONTH_NAMES[(bm - 1) as usize].to_string()
        };
        return (b, label);
    }
    let b = t.div_euclid(interval);
    (b, time_label(b * interval, interval))
}

pub(crate) fn time_label(t: i64, interval: i64) -> String {
    let days = t.div_euclid(86_400);
    let sec = t.rem_euclid(86_400);
    if interval >= 86_400 || sec == 0 {
        let (_, m, d) = civil(days);
        const MONTHS: [&str; 12] = MONTH_NAMES;
        if d == 1 {
            MONTHS[(m - 1) as usize].to_string()
        } else {
            d.to_string()
        }
    } else {
        format!("{:02}:{:02}", sec / 3600, sec % 3600 / 60)
    }
}

/// Étiquette du réticule sur l'échelle de temps (comme TradingView) : « sam. 24 Oct '26  06:00 » ; en 1 jour et plus,
/// la date seulement. `t` est déjà dans le fuseau affiché.
pub fn crosshair_label(t: i64, tf_seconds: i64) -> String {
    use chrono::{DateTime, Datelike, Timelike};
    const DAYS: [&str; 7] = ["lun.", "mar.", "mer.", "jeu.", "ven.", "sam.", "dim."];
    const MONTHS: [&str; 12] = ["Jan", "Fév", "Mar", "Avr", "Mai", "Juin", "Juil", "Aoû", "Sep", "Oct", "Nov", "Déc"];
    let Some(d) = DateTime::from_timestamp(t, 0) else { return String::new() };
    let date = format!(
        "{} {} {} '{:02}",
        DAYS[d.weekday().num_days_from_monday() as usize],
        d.day(),
        MONTHS[d.month0() as usize],
        d.year() % 100
    );
    if tf_seconds >= 86_400 {
        date
    } else {
        format!("{date}  {:02}:{:02}", d.hour(), d.minute())
    }
}

/// ⏱️ Fermeture (secondes UTC) de la bougie d'unité de temps `tf` ouverte à `start` : le 1er du mois suivant pour les
/// mois (28 à 31 jours), `start + tf` sinon (la bougie garde l'heure d'ouverture que l'exchange lui a donnée).
pub(crate) fn close_of(start: i64, tf: i64) -> i64 {
    if tf >= argos_core::MONTH_SECONDS && tf % argos_core::MONTH_SECONDS == 0 {
        argos_core::bucket_end(start, tf)
    } else {
        start + tf
    }
}

/// ⏱️ Texte du compte à rebours : « 4j 04:37 » à partir d'un jour, « 04:37:52 » à partir d'une heure, « 37:52 » sinon.
pub(crate) fn countdown_text(left: i64) -> String {
    if left >= 86_400 {
        format!("{}j {:02}:{:02}", left / 86_400, left % 86_400 / 3600, left % 3600 / 60)
    } else if left >= 3600 {
        format!("{:02}:{:02}:{:02}", left / 3600, left % 3600 / 60, left % 60)
    } else {
        format!("{:02}:{:02}", left / 60, left % 60)
    }
}
