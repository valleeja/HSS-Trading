//! 🌏 Sessions de marché, **comme Argos v1** : Sydney, Tokyo, Londres, New York. Les sessions ouvertes s'affichent en
//! pastilles en bas à droite (à côté de l'horloge), avec ⚡ Chevauchement quand deux sont ouvertes en même temps. Clic :
//! le menu Fuseau horaire, avec les heures des sessions dans le fuseau choisi.
//!
//! Chaque session est écrite **dans l'heure de sa ville** (fuseau IANA) : les changements d'heure (été / hiver) de
//! chaque pays sont donc toujours justes.

use crate::settings::tz_offset;
use crate::text::Rgb;

/// Une session : nom, couleur, fuseau de la ville, ouverture et fermeture (minutes depuis minuit, heure locale).
pub struct Session {
    pub name: &'static str,
    pub color: Rgb,
    pub tz: &'static str,
    pub open: i64,
    pub close: i64,
}

pub const SESSIONS: [Session; 4] = [
    Session {
        name: "Sydney",
        color: Rgb(0xf0, 0xb9, 0x0b),
        tz: "Australia/Sydney",
        open: 7 * 60,
        close: 16 * 60,
    },
    Session {
        name: "Tokyo",
        color: Rgb(0xf2, 0x5c, 0x7a),
        tz: "Asia/Tokyo",
        open: 9 * 60,
        close: 18 * 60,
    },
    Session {
        name: "Londres",
        color: Rgb(0x5b, 0x9c, 0xff),
        tz: "Europe/London",
        open: 8 * 60,
        close: 17 * 60,
    },
    Session {
        name: "New York",
        color: Rgb(0x6f, 0xc2, 0x8b),
        tz: "America/New_York",
        open: 8 * 60,
        close: 17 * 60,
    },
];

/// Couleur de la pastille ⚡ Chevauchement.
pub const OVERLAP: Rgb = Rgb(0xf0, 0xb9, 0x0b);

/// Fuseaux du menu (comme Argos v1) : (nom affiché, fuseau IANA).
pub const ZONES: [(&str, &str); 9] = [
    ("New York", "America/New_York"),
    ("Québec / Toronto", "America/Toronto"),
    ("UTC", "UTC"),
    ("Londres", "Europe/London"),
    ("Paris", "Europe/Paris"),
    ("Tokyo", "Asia/Tokyo"),
    ("Sydney", "Australia/Sydney"),
    ("Chicago", "America/Chicago"),
    ("Los Angeles", "America/Los_Angeles"),
];

const DAY: i64 = 86_400;

impl Session {
    /// Ouverte au moment `now` (secondes UTC) ? En semaine seulement (lundi → vendredi, jour de la ville).
    pub fn is_open(&self, now: i64) -> bool {
        let Some(off) = tz_offset(self.tz, now) else { return false };
        let local = now + off;
        let minute = local.rem_euclid(DAY) / 60;
        // 1970-01-01 était un jeudi : 0 = lundi … 6 = dimanche
        let weekday = (local.div_euclid(DAY) + 3).rem_euclid(7);
        weekday < 5 && minute >= self.open && minute < self.close
    }

    /// Heures d'ouverture et de fermeture dans le fuseau `zone` (« 16:00 », « 01:00 »), pour la journée de `now`.
    #[cfg(test)]
    pub fn hours_in(&self, zone: &str, now: i64) -> (String, String) {
        match tz_offset(zone, now) {
            Some(dst) => self.hours_at(dst, now),
            None => ("—".into(), "—".into()),
        }
    }

    /// Mêmes heures, pour un décalage UTC donné (secondes ; ex. l'heure de l'ordinateur).
    pub fn hours_at(&self, dst: i64, now: i64) -> (String, String) {
        let Some(src) = tz_offset(self.tz, now) else {
            return ("—".into(), "—".into());
        };
        let hm = |m: i64| {
            let m = (m - src / 60 + dst / 60).rem_euclid(24 * 60);
            format!("{:02}:{:02}", m / 60, m % 60)
        };
        (hm(self.open), hm(self.close))
    }
}

/// Sessions ouvertes en ce moment (index dans `SESSIONS`).
pub fn open_now(now: i64) -> Vec<usize> {
    (0..SESSIONS.len()).filter(|&i| SESSIONS[i].is_open(now)).collect()
}

/// Pastilles du bas : les sessions ouvertes, puis ⚡ Chevauchement s'il y en a au moins deux.
pub fn chips(now: i64) -> Vec<(String, Rgb)> {
    let open = open_now(now);
    let mut out: Vec<(String, Rgb)> = open.iter().map(|&i| (SESSIONS[i].name.to_string(), SESSIONS[i].color)).collect();
    if open.len() >= 2 {
        out.push(("⚡ Chevauchement".into(), OVERLAP));
    }
    out
}

/// Nom affiché d'un fuseau (« New York ») ; un fuseau hors de la liste : son nom IANA.
pub fn zone_name(tz: &str) -> String {
    ZONES
        .iter()
        .find(|(_, z)| *z == tz)
        .map_or_else(|| tz.rsplit('/').next().unwrap_or(tz).replace('_', " "), |(n, _)| n.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mardi 20 octobre 2026, `h` heures UTC.
    fn tue(h: i64) -> i64 {
        // 2026-10-20 00:00 UTC
        1_792_454_400 + h * 3600
    }

    #[test]
    fn heures_comme_argos_v1() {
        // en octobre (heure d'été à New York, à Sydney aussi) : les heures de New York de la capture d'Argos v1
        let ny = "America/New_York";
        let at = tue(12);
        assert_eq!(SESSIONS[0].hours_in(ny, at), ("16:00".into(), "01:00".into()), "Sydney");
        assert_eq!(SESSIONS[1].hours_in(ny, at), ("20:00".into(), "05:00".into()), "Tokyo");
        assert_eq!(
            SESSIONS[2].hours_in(ny, at),
            ("03:00".into(), "12:00".into()),
            "Londres : 8 h – 17 h heure de Londres (encore à l'heure d'été le 20 oct.)"
        );
        assert_eq!(SESSIONS[3].hours_in(ny, at), ("08:00".into(), "17:00".into()), "New York");
    }

    #[test]
    fn ouvertes_et_chevauchement() {
        // 13 h UTC un mardi : Londres (14 h BST) et New York (9 h) ouverts → chevauchement
        let c = chips(tue(13));
        let names: Vec<&str> = c.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, vec!["Londres", "New York", "⚡ Chevauchement"]);
        // 2 h UTC : Tokyo (11 h) seulement… et Sydney (13 h AEDT) : chevauchement Sydney + Tokyo
        assert_eq!(open_now(tue(2)), vec![0, 1]);
        // samedi midi UTC : tout fermé
        assert!(open_now(tue(4 * 24 + 12)).is_empty());
        assert_eq!(zone_name("America/Toronto"), "Québec / Toronto");
        assert_eq!(zone_name("Asia/Dubai"), "Dubai");
    }
}
