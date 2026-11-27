//! ⏱️ Intervalles de la chart (comme TradingView : 1 minute … 12 mois). Un intervalle que l'exchange donne tel quel
//! (`Timeframe`) se télécharge directement ; les autres (2m, 50m, 5h, 2 jours, 3 mois…) sont construits en regroupant
//! le plus grand intervalle de l'exchange qui tombe juste dedans (`source`).

use crate::types::MONTH_SECONDS;
use crate::Timeframe;

const MIN: i64 = 60;
const HOUR: i64 = 3600;
const DAY: i64 = 86_400;
const WEEK: i64 = 604_800;

/// Un intervalle, en secondes (les mois : multiples de `MONTH_SECONDS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Interval(pub i64);

/// Sections du menu, dans l'ordre, avec leurs intervalles.
pub const SECTIONS: [(&str, &[i64]); 4] = [
    ("SECONDES", &[1, 5, 10, 15, 30, 45]),
    (
        "MINUTES",
        &[
            MIN,
            2 * MIN,
            3 * MIN,
            5 * MIN,
            6 * MIN,
            7 * MIN,
            8 * MIN,
            10 * MIN,
            12 * MIN,
            15 * MIN,
            30 * MIN,
            45 * MIN,
            50 * MIN,
            55 * MIN,
        ],
    ),
    (
        "HEURES",
        &[HOUR, 2 * HOUR, 3 * HOUR, 4 * HOUR, 5 * HOUR, 6 * HOUR, 7 * HOUR, 8 * HOUR, 12 * HOUR],
    ),
    (
        "JOURS",
        &[
            DAY,
            2 * DAY,
            3 * DAY,
            4 * DAY,
            5 * DAY,
            WEEK,
            2 * WEEK,
            MONTH_SECONDS,
            3 * MONTH_SECONDS,
            6 * MONTH_SECONDS,
            12 * MONTH_SECONDS,
        ],
    ),
];

/// Favoris par défaut (comme TradingView) : 1m, 5m, 15m, 30m, 1h, 4h, 1 jour.
pub const DEFAULT_FAVORITES: [i64; 7] = [MIN, 5 * MIN, 15 * MIN, 30 * MIN, HOUR, 4 * HOUR, DAY];

impl Interval {
    pub fn secs(self) -> i64 {
        self.0
    }

    /// (nombre, unité) : (3, mois), (2, semaines)…
    fn parts(self) -> (i64, &'static str, &'static str, &'static str) {
        let s = self.0;
        if s % MONTH_SECONDS == 0 && s >= MONTH_SECONDS {
            (s / MONTH_SECONDS, "mois", "mois", "M")
        } else if s % WEEK == 0 {
            (s / WEEK, "semaine", "semaines", "W")
        } else if s % DAY == 0 {
            (s / DAY, "jour", "jours", "D")
        } else if s % HOUR == 0 {
            (s / HOUR, "heure", "heures", "h")
        } else if s % MIN == 0 {
            (s / MIN, "minute", "minutes", "m")
        } else {
            (s, "seconde", "secondes", "s")
        }
    }

    /// Nom du menu : « 1 minute », « 5 heures », « 3 mois ».
    pub fn label(self) -> String {
        let (n, one, many, _) = self.parts();
        format!("{n} {}", if n == 1 { one } else { many })
    }

    /// Nom court : « 30m », « 4h », « 1D », « 1W », « 3M », « 15s ».
    pub fn short(self) -> String {
        let (n, _, _, u) = self.parts();
        format!("{n}{u}")
    }

    /// Lit un nom court (« 2m », « 4h », « 1d », « 1D », « 2w », « 3M », « 30s »). Les noms de `Timeframe` marchent aussi.
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();
        let split = s.find(|c: char| !c.is_ascii_digit())?;
        let (n, u) = s.split_at(split);
        let n: i64 = n.parse().ok().filter(|n| *n > 0)?;
        let unit = match u {
            "s" => 1,
            "m" => MIN,
            "h" | "H" => HOUR,
            "d" | "D" => DAY,
            "w" | "W" => WEEK,
            "M" => MONTH_SECONDS,
            _ => return None,
        };
        Some(Interval(n * unit))
    }

    /// L'exchange le donne tel quel ?
    pub fn native(self) -> Option<Timeframe> {
        Timeframe::ALL.into_iter().find(|t| t.seconds() == self.0)
    }

    /// Intervalle de l'exchange crypto à télécharger : lui-même, sinon le plus grand qui tombe juste dedans. Mois et
    /// semaines : le jour, regroupé ici (les semaines des exchanges ne commencent pas toutes le lundi : jeudi chez
    /// Hyperliquid). `None` pour les secondes (les exchanges ne donnent pas de bougies en secondes).
    pub fn source(self) -> Option<Timeframe> {
        if self.0 % WEEK == 0 || (self.0 % MONTH_SECONDS == 0 && self.0 >= MONTH_SECONDS) {
            return Some(Timeframe::D1);
        }
        if let Some(t) = self.native() {
            return Some(t);
        }
        Timeframe::ALL.into_iter().rev().find(|t| t.seconds() <= self.0 && self.0 % t.seconds() == 0)
    }

    /// Intervalle à télécharger chez OANDA (forex, or, indices) : la semaine telle quelle (semaine forex, du dimanche
    /// soir au vendredi 17:00 heure de New York), le reste comme `source`.
    pub fn source_fx(self) -> Option<Timeframe> {
        if self.0 == WEEK {
            Some(Timeframe::W1)
        } else {
            self.source()
        }
    }

    /// Combien de bougies de la source font une bougie de cet intervalle (environ, pour les mois).
    pub fn ratio(self) -> usize {
        self.source().map_or(1, |t| (self.0 / t.seconds()).max(1) as usize)
    }
}

impl From<Timeframe> for Interval {
    fn from(t: Timeframe) -> Self {
        Interval(t.seconds())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{aggregate, bucket_start, Bar};

    #[test]
    fn noms_et_sources() {
        assert_eq!(
            (Interval(60).label(), Interval(300).label(), Interval(3 * MONTH_SECONDS).label()),
            ("1 minute".into(), "5 minutes".into(), "3 mois".into())
        );
        assert_eq!(
            (Interval(1800).short(), Interval(DAY).short(), Interval(2 * WEEK).short(), Interval(15).short()),
            ("30m".into(), "1D".into(), "2W".into(), "15s".into())
        );
        assert_eq!(Interval::parse("50m"), Some(Interval(3000)));
        assert_eq!(Interval::parse("3M"), Some(Interval(3 * MONTH_SECONDS)));
        assert_eq!(Interval::parse("1d"), Some(Interval(DAY)));
        assert_eq!(Interval::parse("x"), None);
        assert_eq!(Interval(1800).native(), Some(Timeframe::M30));
        assert_eq!(Interval(50 * MIN).source(), Some(Timeframe::M5), "50m = 10 × 5m");
        assert_eq!(Interval(45 * MIN).source(), Some(Timeframe::M15));
        assert_eq!(Interval(6 * HOUR).source(), Some(Timeframe::H2));
        assert_eq!(Interval(8 * HOUR).source(), Some(Timeframe::H4));
        assert_eq!(Interval(2 * WEEK).source(), Some(Timeframe::D1), "semaines : regroupées depuis le jour");
        assert_eq!(Interval(WEEK).source(), Some(Timeframe::D1));
        assert_eq!(Interval(WEEK).source_fx(), Some(Timeframe::W1), "OANDA : la semaine forex telle quelle");
        assert_eq!(Interval(MONTH_SECONDS).source(), Some(Timeframe::D1));
        assert_eq!(Interval(30).source(), None, "secondes : pas encore");
        assert_eq!(Interval(50 * MIN).ratio(), 10);
        // tous les intervalles du menu (sauf secondes) ont une source
        assert!(SECTIONS[1..].iter().flat_map(|(_, l)| l.iter()).all(|s| Interval(*s).source().is_some()));
    }

    #[test]
    fn trimestres_et_annees() {
        // 15 mai 2026 → trimestre : 1er avril 2026 ; année : 1er janv. 2026
        let may15 = 1_778_803_200;
        assert_eq!(bucket_start(may15, 3 * MONTH_SECONDS), 1_775_001_600);
        assert_eq!(bucket_start(may15, 12 * MONTH_SECONDS), 1_767_225_600);
        let bars: Vec<Bar> = (0..120)
            .map(|d| Bar {
                time: 1_767_225_600 + d * DAY,
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 1.0,
            })
            .collect();
        let q = aggregate(&bars, 3 * MONTH_SECONDS);
        assert_eq!(q.len(), 2, "120 jours depuis le 1er janv. : T1 + début de T2");
        assert_eq!(q[0].volume, 90.0, "janv. + févr. + mars 2026 = 90 jours");
    }

    #[test]
    fn bornes_des_semaines_et_des_mois() {
        use crate::bucket_end;
        // mercredi 4 nov. 2026, 19:23 UTC : la semaine va du lundi 2 nov. au lundi 9 nov., 00:00 UTC
        let wed = 1_793_820_180;
        assert_eq!(bucket_start(wed, WEEK), 1_793_577_600, "lundi 2 nov. 00:00 UTC (pas le jeudi)");
        assert_eq!(bucket_end(bucket_start(wed, WEEK), WEEK), 1_794_182_400, "lundi 9 nov. 00:00 UTC");
        assert_eq!(bucket_start(1_793_577_600, WEEK), 1_793_577_600, "lundi 00:00 : début de sa propre semaine");
        assert_eq!(bucket_start(1_793_577_599, WEEK), 1_793_577_600 - WEEK, "dimanche 23:59:59 : semaine d'avant");
        // mois du calendrier : novembre (30 j), décembre → janvier, février normal (28 j) et bissextile (29 j)
        assert_eq!(bucket_start(wed, MONTH_SECONDS), 1_793_491_200, "1er nov. 2026");
        assert_eq!(bucket_end(1_793_491_200, MONTH_SECONDS), 1_796_083_200, "1er déc. 2026");
        assert_eq!(
            bucket_end(bucket_start(1_798_718_400, MONTH_SECONDS), MONTH_SECONDS),
            1_798_761_600,
            "31 déc. → 1er janv. 2027"
        );
        assert_eq!(bucket_end(1_801_440_000, MONTH_SECONDS) - 1_801_440_000, 28 * DAY, "février 2027 : 28 jours");
        assert_eq!(bucket_end(1_832_976_000, MONTH_SECONDS) - 1_832_976_000, 29 * DAY, "février 2028 : 29 jours");
        assert_eq!(bucket_start(1_835_438_400, MONTH_SECONDS), 1_832_976_000, "29 févr. 2028 : dans février");
        // une semaine regroupée depuis le jour : du lundi au dimanche
        let days: Vec<Bar> = (0..14)
            .map(|d| Bar {
                time: 1_793_491_200 + d * DAY, // dimanche 1er nov. → samedi 14 nov.
                open: 1.0,
                high: 2.0,
                low: 0.5,
                close: 1.5,
                volume: 1.0,
            })
            .collect();
        let w = aggregate(&days, WEEK);
        assert_eq!(
            w.iter().map(|b| (b.time, b.volume)).collect::<Vec<_>>(),
            vec![(1_793_577_600 - WEEK, 1.0), (1_793_577_600, 7.0), (1_794_182_400, 6.0)]
        );
    }
}
