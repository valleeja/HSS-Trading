//! 🕘 Horaires des leçons : elles sont écrites à l'heure de New York (heure de l'Est). L'élève choisit son fuseau
//! (New York par défaut) et l'app lui dit combien d'heures ajouter, heure d'été comprise (règles américaines et
//! européennes calculées ici, sans dépendance).

use serde::{Deserialize, Serialize};

/// Un fuseau proposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Fuseau {
    /// New York (heure de l'Est), comme les leçons.
    #[default]
    NewYork,
    /// Montréal / Toronto : même heure que New York.
    Montreal,
    /// Vancouver (heure du Pacifique).
    Vancouver,
    Londres,
    Paris,
    Utc,
}

/// Lit un fuseau gardé dans `academy.json` ; un nom inconnu (ou un autre type) revient à New York.
pub fn lire_fuseau<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Fuseau, D::Error> {
    let v = serde_json::Value::deserialize(d)?;
    Ok(Fuseau::TOUS.into_iter().find(|f| v.as_str() == Some(&format!("{f:?}"))).unwrap_or_default())
}

impl Fuseau {
    pub const TOUS: [Fuseau; 6] = [
        Fuseau::NewYork,
        Fuseau::Montreal,
        Fuseau::Vancouver,
        Fuseau::Londres,
        Fuseau::Paris,
        Fuseau::Utc,
    ];

    pub fn nom(self) -> &'static str {
        match self {
            Fuseau::NewYork => "New York",
            Fuseau::Montreal => "Montréal",
            Fuseau::Vancouver => "Vancouver",
            Fuseau::Londres => "Londres",
            Fuseau::Paris => "Paris",
            Fuseau::Utc => "UTC",
        }
    }

    /// Le fuseau suivant (le bouton du haut de l'Académie fait le tour).
    pub fn suivant(self) -> Fuseau {
        let i = Self::TOUS.iter().position(|f| *f == self).unwrap_or(0);
        Self::TOUS[(i + 1) % Self::TOUS.len()]
    }

    /// Décalage avec UTC, en minutes, à l'instant `t` (secondes Unix).
    pub fn decalage(self, t: i64) -> i64 {
        match self {
            Fuseau::NewYork | Fuseau::Montreal => 60 * if ete_usa(t) { -4 } else { -5 },
            Fuseau::Vancouver => 60 * if ete_usa(t) { -7 } else { -8 },
            Fuseau::Londres => 60 * if ete_europe(t) { 1 } else { 0 },
            Fuseau::Paris => 60 * if ete_europe(t) { 2 } else { 1 },
            Fuseau::Utc => 0,
        }
    }

    /// Heures à ajouter à une heure de New York pour l'avoir dans ce fuseau, à l'instant `t`.
    pub fn ecart_new_york(self, t: i64) -> i64 {
        (self.decalage(t) - Fuseau::NewYork.decalage(t)) / 60
    }

    /// La phrase affichée en haut d'une leçon qui parle d'heures.
    pub fn phrase(self, t: i64) -> String {
        match self.ecart_new_york(t) {
            0 => format!("🕘 Heures de New York (heure de l'Est) : c'est aussi ton heure ({}).", self.nom()),
            e => format!(
                "🕘 Heures de New York (heure de l'Est). Chez toi ({}) : {} {} h.",
                self.nom(),
                if e > 0 { "ajoute" } else { "enlève" },
                e.abs()
            ),
        }
    }

    /// Une heure de New York (`h`:`m`) convertie dans ce fuseau à l'instant `t`, sur 24 h.
    #[cfg(test)]
    pub fn depuis_new_york(self, h: u32, m: u32, t: i64) -> (u32, u32) {
        let min = (h * 60 + m) as i64 + self.ecart_new_york(t) * 60;
        let min = min.rem_euclid(24 * 60);
        ((min / 60) as u32, (min % 60) as u32)
    }
}

/// Jours depuis le 1er janvier 1970 d'une date (calendrier grégorien).
pub fn jours(an: i64, mois: u32, jour: u32) -> i64 {
    let a = if mois <= 2 { an - 1 } else { an };
    let ere = a.div_euclid(400);
    let ya = a - ere * 400;
    let mp = (mois as i64 + 9) % 12;
    let jda = (153 * mp + 2) / 5 + jour as i64 - 1;
    let jde = ya * 365 + ya / 4 - ya / 100 + jda;
    ere * 146_097 + jde - 719_468
}

/// L'année d'un instant (secondes Unix, UTC).
pub fn annee(t: i64) -> i64 {
    let z = t.div_euclid(86_400) + 719_468;
    let ere = z.div_euclid(146_097);
    let je = z - ere * 146_097;
    let ae = (je - je / 1460 + je / 36_524 - je / 146_096) / 365;
    let jda = je - (365 * ae + ae / 4 - ae / 100);
    let mp = (5 * jda + 2) / 153;
    let mois = if mp < 10 { mp + 3 } else { mp - 9 };
    ae + ere * 400 + i64::from(mois <= 2)
}

/// Jour de la semaine (0 = dimanche).
fn semaine(j: i64) -> i64 {
    (j + 4).rem_euclid(7)
}

/// Le `n`-ième dimanche d'un mois (jours depuis 1970).
fn dimanche(an: i64, mois: u32, n: i64) -> i64 {
    let d = jours(an, mois, 1);
    d + (7 - semaine(d)) % 7 + 7 * (n - 1)
}

/// Le dernier dimanche d'un mois.
fn dernier_dimanche(an: i64, mois: u32) -> i64 {
    let fin = jours(an, mois + 1, 1) - 1;
    fin - semaine(fin)
}

/// Heure d'été aux États-Unis : du 2e dimanche de mars (2 h, soit 7 h UTC) au 1er dimanche de novembre (2 h, 6 h UTC).
pub fn ete_usa(t: i64) -> bool {
    let a = annee(t);
    t >= dimanche(a, 3, 2) * 86_400 + 7 * 3600 && t < dimanche(a, 11, 1) * 86_400 + 6 * 3600
}

/// Heure d'été en Europe : du dernier dimanche de mars au dernier dimanche d'octobre, à 1 h UTC.
pub fn ete_europe(t: i64) -> bool {
    let a = annee(t);
    t >= dernier_dimanche(a, 3) * 86_400 + 3600 && t < dernier_dimanche(a, 10) * 86_400 + 3600
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(an: i64, mois: u32, jour: u32, h: i64) -> i64 {
        jours(an, mois, jour) * 86_400 + h * 3600
    }

    #[test]
    fn heure_d_ete_et_ecarts() {
        assert_eq!(jours(1970, 1, 1), 0);
        assert_eq!(annee(t(2026, 12, 31, 23)), 2026);
        // 2026 : É.-U. du 8 mars au 1er novembre ; Europe du 29 mars au 25 octobre
        assert!(!ete_usa(t(2026, 3, 8, 6)) && ete_usa(t(2026, 3, 8, 7)));
        assert!(ete_usa(t(2026, 11, 1, 5)) && !ete_usa(t(2026, 11, 1, 6)));
        assert!(!ete_europe(t(2026, 3, 29, 0)) && ete_europe(t(2026, 3, 29, 1)));
        assert!(ete_europe(t(2026, 10, 25, 0)) && !ete_europe(t(2026, 10, 25, 1)));
        // Paris : +6 h en temps normal, +5 h entre les deux changements (mi-mars)
        assert_eq!(Fuseau::Paris.ecart_new_york(t(2026, 1, 15, 12)), 6);
        assert_eq!(Fuseau::Paris.ecart_new_york(t(2026, 3, 15, 12)), 5);
        assert_eq!(Fuseau::Londres.ecart_new_york(t(2026, 7, 1, 12)), 5);
        assert_eq!(Fuseau::Vancouver.ecart_new_york(t(2026, 7, 1, 12)), -3);
        assert_eq!(Fuseau::Montreal.ecart_new_york(t(2026, 7, 1, 12)), 0);
        assert_eq!(Fuseau::Utc.ecart_new_york(t(2026, 1, 15, 12)), 5);
        // 20 h à New York = 2 h à Paris (hiver)
        assert_eq!(Fuseau::Paris.depuis_new_york(20, 0, t(2026, 1, 15, 12)), (2, 0));
        assert!(Fuseau::Paris.phrase(t(2026, 1, 15, 12)).contains("ajoute 6 h"));
        assert!(Fuseau::NewYork.phrase(0).contains("c'est aussi ton heure"));
        // le tour des fuseaux revient au départ
        let mut f = Fuseau::NewYork;
        for _ in 0..Fuseau::TOUS.len() {
            f = f.suivant();
        }
        assert_eq!(f, Fuseau::NewYork);
        // un nom inconnu dans le fichier : New York
        let lire = |s: &str| lire_fuseau(&mut serde_json::Deserializer::from_str(s)).unwrap();
        assert_eq!(
            (lire("\"Tokyo\""), lire("3"), lire("\"Paris\"")),
            (Fuseau::NewYork, Fuseau::NewYork, Fuseau::Paris)
        );
    }
}
