//! 🕳️ Trous dans les bougies (étape 3 du plan de collecte) : les trouver, distinguer un vrai trou d'un **marché
//! fermé**, et les réparer en redemandant seulement la plage manquante à l'exchange.
//!
//! - Un trou = des bougies qui manquent entre deux bougies reçues, ou une bougie refusée par la vérification (étape 2).
//! - **Marché fermé** : la crypto ne ferme jamais ; le forex, les métaux et les indices / actions (marchés HIP-3
//!   `xyz:` d'Hyperliquid) ferment le week-end (du vendredi 20:00 au dimanche 23:00 UTC, assez large pour l'heure
//!   d'été et d'hiver), le 25 décembre et le 1er janvier ; en plus, **jours fériés par marché** (`Calendrier`) :
//!   - actions et indices américains : jours fériés de la bourse de New York (1er janvier, Martin Luther King,
//!     Presidents Day, Vendredi saint, Memorial Day, Juneteenth, 4 juillet, Labor Day, Thanksgiving, Noël), avec la
//!     règle du samedi → vendredi d'avant et du dimanche → lundi d'après (sauf le 1er janvier un samedi) ;
//!   - métaux, énergie et forex : le Vendredi saint en plus (marchés à terme américains fermés) ;
//!   - autres bourses (Japon…) : seulement le week-end, Noël et le 1er janvier pour l'instant.
//!     Un jour férié ferme toute la journée UTC, et la veille dès 23:00 (comme la réouverture du dimanche). Pâques et
//!     Thanksgiving sont **calculés** (pas une liste d'années). Un trou entièrement dans une fermeture n'est **pas**
//!     réparé.
//! - Réparation : une demande par trou, vers le passé à partir de la fin du trou (mêmes attentes que le remplissage :
//!   Retry-After de l'exchange et petit délai au hasard). Un trou que l'exchange ne peut pas remplir est noté **une
//!   fois** dans le journal (sans rien de sensible) et devient « connu » (`Connu`, avec l'heure de l'essai) : il est
//!   **réessayé une fois par semaine** (`REESSAI_S`), au cas où l'exchange n'avait qu'une panne ce jour-là. Les trous
//!   connus sont gardés à côté du cache de la série (`….trous`, 0600, 1000 au plus), pour survivre à un redémarrage.
//! - 🔒 A false price taken out of the chart (bug B-025) is never put back by a repair: a very long candle found in a
//!   hole is asked to the next exchanges first (`reparer`).

use crate::supervisor::Supervisor;
use crate::{Exchange, ExchangeFeed, Http};
use argos_core::{Bar, Timeframe};

/// Horaires d'un marché.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Calendrier {
    /// Crypto : ouvert tout le temps, chaque trou est un vrai trou.
    Continu,
    /// Crypto on an exchange that sends no candle for an interval without a trade (Bitstamp, Bitfinex): open all the
    /// time, but a gap between two of its candles is a quiet time, not a hole (asked again, it stays empty); only a
    /// candle refused by the checks is repaired.
    Quiet,
    /// Autres bourses (Japon…) : fermé le week-end, le 25 décembre et le 1er janvier.
    TradFi,
    /// Métaux, énergie, forex : comme `TradFi`, plus le Vendredi saint.
    Metaux,
    /// Actions et indices américains : comme `TradFi`, plus les jours fériés de la bourse de New York.
    BourseUs,
}

/// Les marchés HIP-3 qui suivent les marchés à terme et le forex (le reste est une action ou un indice).
const METAUX: [&str; 13] = [
    "GOLD",
    "SILVER",
    "PLATINUM",
    "PALLADIUM",
    "COPPER",
    "CL",
    "BRENTOIL",
    "NATGAS",
    "EUR",
    "GBP",
    "JPY",
    "CHF",
    "AUD",
];
/// Les indices d'autres bourses que New York (leurs jours fériés ne sont pas encore dans Argos).
const AUTRES_BOURSES: [&str; 1] = ["JP225"];

/// Horaires du marché `base` sur `ex` : les marchés HIP-3 (`xyz:…`, or, indices, actions) suivent leur bourse.
pub fn calendrier(ex: Exchange, base: &str) -> Calendrier {
    let Some((_, nom)) = base.split_once(':') else {
        return if ex.skips_quiet_candles() { Calendrier::Quiet } else { Calendrier::Continu };
    };
    let nom = nom.to_ascii_uppercase();
    if METAUX.contains(&nom.as_str()) {
        Calendrier::Metaux
    } else if AUTRES_BOURSES.contains(&nom.as_str()) {
        Calendrier::TradFi
    } else {
        Calendrier::BourseUs
    }
}

/// Des bougies manquantes : de `debut` (1re bougie absente) à `fin` (exclue : la bougie présente suivante).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Trou {
    pub debut: i64,
    pub fin: i64,
}

impl Trou {
    /// Combien de bougies manquent.
    pub fn bougies(&self, tf: Timeframe) -> usize {
        ((self.fin - self.debut) / tf.seconds()).max(1) as usize
    }
}

const JOUR: i64 = 86_400;

/// Le marché est-il fermé à l'instant `t` (secondes UTC) ?
pub fn ferme_a(cal: Calendrier, t: i64) -> bool {
    if matches!(cal, Calendrier::Continu | Calendrier::Quiet) {
        return false;
    }
    let jours = t.div_euclid(JOUR);
    let sec = t.rem_euclid(JOUR);
    // 1er janvier 1970 = jeudi : 0 = lundi … 6 = dimanche
    let jour_semaine = (jours + 3).rem_euclid(7);
    let week_end = match jour_semaine {
        4 => sec >= 20 * 3600, // vendredi dès 20:00
        5 => true,             // samedi
        6 => sec < 23 * 3600,  // dimanche jusqu'à 23:00
        _ => false,
    };
    week_end || ferie(cal, jours) || (sec >= 23 * 3600 && ferie(cal, jours + 1))
}

/// Le jour `jours` (depuis 1970) est-il férié pour ce marché ?
pub fn ferie(cal: Calendrier, jours: i64) -> bool {
    let (a, m, d) = civil(jours);
    let toujours = (m == 12 && d == 25) || (m == 1 && d == 1);
    match cal {
        Calendrier::Continu | Calendrier::Quiet => false,
        Calendrier::TradFi => toujours,
        Calendrier::Metaux => toujours || jours == paques(a) - 2,
        Calendrier::BourseUs => toujours || feries_us(a).contains(&jours) || feries_us(a + 1).contains(&jours),
    }
}

/// Jours (depuis 1970) d'une date (année, mois, jour).
fn jours_de(a: i64, m: i64, d: i64) -> i64 {
    let a = if m <= 2 { a - 1 } else { a };
    let era = a.div_euclid(400);
    let yoe = a - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Le `n`-ième jour `js` (0 = lundi … 6 = dimanche) du mois ; `n = 0` : le dernier.
fn nieme(a: i64, m: i64, js: i64, n: i64) -> i64 {
    if n == 0 {
        let fin = jours_de(a + i64::from(m == 12), m % 12 + 1, 1) - 1;
        return fin - (fin + 3 - js).rem_euclid(7);
    }
    let premier = jours_de(a, m, 1);
    premier + (js - (premier + 3)).rem_euclid(7) + 7 * (n - 1)
}

/// Dimanche de Pâques (calendrier grégorien, méthode de Meeus / Jones / Butcher), en jours depuis 1970.
pub fn paques(a: i64) -> i64 {
    let (b, c) = (a / 100, a % 100);
    let g = a % 19;
    let h = (19 * g + b - b / 4 - (8 * b + 13) / 25 + 15) % 30;
    let l = (32 + 2 * (b % 4) + 2 * (c / 4) - h - c % 4) % 7;
    let m = (g + 11 * h + 22 * l) / 451;
    let mois = (h + l - 7 * m + 114) / 31;
    let jour = (h + l - 7 * m + 114) % 31 + 1;
    jours_de(a, mois, jour)
}

/// Un jour férié fixe qui tombe un samedi est pris le vendredi d'avant, un dimanche le lundi d'après.
fn observe(j: i64) -> i64 {
    match (j + 3).rem_euclid(7) {
        5 => j - 1,
        6 => j + 1,
        _ => j,
    }
}

/// Les jours de fermeture de la bourse de New York pour l'année `a` (jours depuis 1970).
pub fn feries_us(a: i64) -> Vec<i64> {
    let mut v = vec![
        nieme(a, 1, 0, 3),  // Martin Luther King : 3e lundi de janvier
        nieme(a, 2, 0, 3),  // Presidents Day : 3e lundi de février
        paques(a) - 2,      // Vendredi saint
        nieme(a, 5, 0, 0),  // Memorial Day : dernier lundi de mai
        nieme(a, 9, 0, 1),  // Labor Day : 1er lundi de septembre
        nieme(a, 11, 3, 4), // Thanksgiving : 4e jeudi de novembre
        observe(jours_de(a, 7, 4)),
        observe(jours_de(a, 12, 25)),
    ];
    if a >= 2022 {
        v.push(observe(jours_de(a, 6, 19))); // Juneteenth
    }
    // 1er janvier un samedi : pas de fermeture le 31 décembre (règle de la bourse)
    let nouvel_an = jours_de(a, 1, 1);
    if (nouvel_an + 3).rem_euclid(7) != 5 {
        v.push(observe(nouvel_an));
    }
    v
}

/// Date (année, mois, jour) d'un nombre de jours depuis 1970.
fn civil(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

/// Un trou entièrement dans une fermeture du marché (chaque bougie manquante tombe quand le marché est fermé).
pub fn marche_ferme(cal: Calendrier, trou: &Trou, tf: Timeframe) -> bool {
    let step = tf.seconds();
    cal != Calendrier::Continu && (0..trou.bougies(tf) as i64).all(|k| ferme_a(cal, trou.debut + k * step))
}

/// Les trous à réparer dans `bars` (triées) : bougies manquantes entre deux bougies reçues (sauf sur un exchange qui
/// n'envoie pas de bougie sans trade, `Calendrier::Quiet`), plus les bougies refusées (`a_reparer`, heures) qui ne sont
/// pas là. Les trous d'un marché fermé et ceux de `connus` (déjà essayés, sans
/// succès) sont laissés de côté. Rien pour les unités d'un mois et plus (pas de pas fixe).
pub fn trouver(bars: &[Bar], tf: Timeframe, a_reparer: &[i64], cal: Calendrier, connus: &[Trou]) -> Vec<Trou> {
    let step = tf.seconds();
    if step >= 2_419_200 || bars.is_empty() {
        return vec![];
    }
    let mut out: Vec<Trou> = bars
        .windows(2)
        .filter(|w| cal != Calendrier::Quiet && w[1].time - w[0].time > step)
        .map(|w| Trou {
            debut: w[0].time + step,
            fin: w[1].time,
        })
        .collect();
    let (first, last) = (bars[0].time, bars[bars.len() - 1].time);
    for &t in a_reparer {
        let dedans = out.iter().any(|g| g.debut <= t && t < g.fin);
        if t >= first && t <= last && !dedans && bars.binary_search_by_key(&t, |b| b.time).is_err() {
            out.push(Trou { debut: t, fin: t + step });
        }
    }
    out.sort_by_key(|g| g.debut);
    out.retain(|g| !marche_ferme(cal, g, tf) && !connus.contains(g));
    out
}

/// ⏳ Un trou que l'exchange n'avait pas est réessayé après ce délai (une semaine).
pub const REESSAI_S: i64 = 7 * 86_400;
/// Au plus autant de trous connus par série (les plus récents essais sont gardés).
pub const MAX_CONNUS: usize = 1000;

/// Un trou que l'exchange n'avait pas, et quand on l'a essayé (secondes UTC).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Connu {
    pub trou: Trou,
    pub essaye: i64,
}

/// Les trous à laisser de côté à `now` : ceux essayés il y a moins d'une semaine.
pub fn a_eviter(connus: &[Connu], now: i64) -> Vec<Trou> {
    connus.iter().filter(|c| now - c.essaye < REESSAI_S).map(|c| c.trou).collect()
}

/// Note les trous que l'exchange n'a pas eus à `now` (un trou déjà connu prend la nouvelle date ; 1000 au plus).
pub fn noter(connus: &mut Vec<Connu>, irreparables: &[Trou], now: i64) {
    connus.retain(|c| !irreparables.contains(&c.trou));
    connus.extend(irreparables.iter().map(|&trou| Connu { trou, essaye: now }));
    if connus.len() > MAX_CONNUS {
        connus.sort_by_key(|c| std::cmp::Reverse(c.essaye));
        connus.truncate(MAX_CONNUS);
    }
}

/// Le fichier des trous connus d'une série du cache (`…/hl-btc-1h.bin` → `…/hl-btc-1h.trous`).
pub fn chemin_connus(cache: &std::path::Path) -> std::path::PathBuf {
    cache.with_extension("trous")
}

/// Lit les trous connus (une ligne « début fin essai » par trou) ; un fichier absent, trop gros ou abîmé = rien ; une
/// ligne illisible est sautée.
pub fn lire_connus(path: &std::path::Path) -> Vec<Connu> {
    let Ok(m) = std::fs::metadata(path) else { return vec![] };
    if m.len() > 64 * 1024 {
        return vec![];
    }
    let Ok(t) = std::fs::read_to_string(path) else { return vec![] };
    t.lines()
        .filter_map(|l| {
            let mut n = l.split(' ').map(|x| x.parse::<i64>().ok());
            let (debut, fin, essaye) = (n.next()??, n.next()??, n.next()??);
            (n.next().is_none() && debut < fin).then_some(Connu {
                trou: Trou { debut, fin },
                essaye,
            })
        })
        .take(MAX_CONNUS)
        .collect()
}

/// Écrit les trous connus (fichier temporaire puis renommage, 0600) ; aucun trou = fichier retiré.
pub fn ecrire_connus(path: &std::path::Path, connus: &[Connu]) -> std::io::Result<()> {
    use std::io::Write;
    if connus.is_empty() {
        return match std::fs::remove_file(path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        };
    }
    let texte: String = connus
        .iter()
        .take(MAX_CONNUS)
        .map(|c| format!("{} {} {}\n", c.trou.debut, c.trou.fin, c.essaye))
        .collect();
    let tmp = path.with_extension(format!("trous.tmp{}", std::process::id()));
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    o.open(&tmp)?.write_all(texte.as_bytes())?;
    #[cfg(unix)]
    std::fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    std::fs::rename(&tmp, path)
}

/// Ce que la réparation a donné : les bougies retrouvées, et les trous que l'exchange n'a pas pu remplir.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Reparation {
    pub bars: Vec<Bar>,
    pub irreparables: Vec<Trou>,
}

/// La ligne du journal pour un trou que l'exchange ne peut pas remplir (exchange, symbole, unité, heures UTC, nombre
/// de bougies ; jamais d'adresse ni de paramètre).
pub fn texte_irreparable(ex: Exchange, base: &str, tf: Timeframe, g: &Trou) -> String {
    let h = |t: i64| chrono::DateTime::from_timestamp(t, 0).map_or(t.to_string(), |d| d.format("%Y-%m-%d %H:%M UTC").to_string());
    format!(
        "⚠️ Trou non réparable · {} {base} {tf} · {} → {} · {} bougie(s) : l'exchange ne les a pas",
        ex.name(),
        h(g.debut),
        h(g.fin),
        g.bougies(tf)
    )
}

/// Répare au plus `max` trous, un à la fois (pause de l'exchange entre deux) : les bougies retrouvées dans chaque
/// trou, et les trous restés vides (notés une fois dans le journal).
///
/// 🔒 Bug B-025: a very long candle found in a hole is asked to the next exchanges first, like on the first screen
/// (`Supervisor::held_back`). A false price (taken out of the chart, which left this hole) is never put back: its hole
/// stays, noted like one the exchange does not have. One no exchange could answer for yet stays out too, and its hole
/// is asked again at the next repair (never noted). No question more when no candle found is very long.
pub async fn reparer<H: Http + Clone>(feed: &ExchangeFeed<H>, base: &str, tf: Timeframe, trous: &[Trou], max: usize) -> Reparation {
    let mut found: Vec<(Trou, Vec<Bar>)> = vec![];
    for (k, g) in trous.iter().take(max).enumerate() {
        if k > 0 {
            feed.http.wait(feed.exchange.page_pause_ms()).await;
        }
        let got: Vec<Bar> = feed
            .older(base, tf, g.fin, g.bougies(tf))
            .await
            .map(|b| b.into_iter().filter(|b| b.time >= g.debut && b.time < g.fin).collect())
            .unwrap_or_default();
        found.push((*g, got));
    }
    let all: Vec<Bar> = found.iter().flat_map(|(_, b)| b.iter().copied()).collect();
    let held = Supervisor::shared(feed.http.clone(), feed.now_ms)
        .held_back(feed.exchange, base, tf, &all)
        .await;
    let mut out = Reparation::default();
    for (g, got) in found {
        let unsure = got.iter().any(|b| held.unsure.contains(&b.time));
        let got: Vec<Bar> = got.into_iter().filter(|b| !held.holds(b.time)).collect();
        if !got.is_empty() {
            out.bars.extend(got);
        } else if !unsure {
            eprintln!("{}", texte_irreparable(feed.exchange, base, tf, &g));
            out.irreparables.push(g);
        }
    }
    out
}

/// Les bougies avec celles retrouvées, triées, sans doublon.
pub fn combler(bars: &[Bar], retrouvees: &[Bar]) -> Vec<Bar> {
    argos_core::feed::clean_bars([bars, retrouvees].concat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use argos_core::Result;

    const H: i64 = 3600;
    /// Lundi 2 novembre 2026, 00:00 UTC.
    const LUNDI: i64 = 1_793_577_600;

    fn bar(t: i64) -> Bar {
        Bar {
            time: t,
            open: 10.0,
            high: 11.0,
            low: 9.0,
            close: 10.5,
            volume: 1.0,
        }
    }

    fn serie(de: i64, n: i64, sans: &[i64]) -> Vec<Bar> {
        (0..n).filter(|k| !sans.contains(k)).map(|k| bar(de + k * H)).collect()
    }

    #[test]
    fn trouve_les_trous_et_les_bougies_refusees() {
        // 10 bougies d'une heure, la 3e et la 4e manquent ; la 7e a été refusée (absente aussi)
        let b = serie(LUNDI, 10, &[2, 3, 6]);
        let g = trouver(&b, Timeframe::H1, &[LUNDI + 6 * H], Calendrier::Continu, &[]);
        assert_eq!(
            g,
            vec![
                Trou {
                    debut: LUNDI + 2 * H,
                    fin: LUNDI + 4 * H
                },
                Trou {
                    debut: LUNDI + 6 * H,
                    fin: LUNDI + 7 * H
                }
            ]
        );
        assert_eq!(g[0].bougies(Timeframe::H1), 2);
        // une bougie refusée hors de la plage chargée : rien
        assert_eq!(trouver(&b, Timeframe::H1, &[LUNDI - 50 * H], Calendrier::Continu, &[]).len(), 2);
        // un trou déjà essayé sans succès : laissé de côté
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[g[0]]), vec![g[1]]);
        assert!(trouver(&[], Timeframe::H1, &[], Calendrier::Continu, &[]).is_empty());
    }

    /// 🔒 Reliable data: Bitstamp and Bitfinex send no candle for a time without a trade (Bitstamp's minute of 05:42
    /// on 24 Nov. 2026): a gap between two of their candles is a quiet time, never asked again (it would stay empty);
    /// a candle refused by the checks is still repaired. Elsewhere, the same gap is a hole.
    #[test]
    fn a_quiet_time_is_not_a_hole() {
        for ex in [Exchange::Bitstamp, Exchange::Bitfinex, Exchange::BitfinexSpot] {
            assert_eq!(calendrier(ex, "BTC"), Calendrier::Quiet, "{ex:?}");
        }
        for ex in [Exchange::Binance, Exchange::Coinbase, Exchange::Kraken] {
            assert_eq!(calendrier(ex, "BTC"), Calendrier::Continu, "{ex:?}");
        }
        let b = serie(LUNDI, 10, &[2, 3, 6]);
        let refused = Trou {
            debut: LUNDI + 6 * H,
            fin: LUNDI + 7 * H,
        };
        assert_eq!(trouver(&b, Timeframe::H1, &[LUNDI + 6 * H], Calendrier::Quiet, &[]), vec![refused]);
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::Quiet, &[]).is_empty());
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[]).len(), 2);
        assert!(
            !ferme_a(Calendrier::Quiet, LUNDI + 5 * 86_400) && !ferie(Calendrier::Quiet, 20_812),
            "never closed"
        );
    }

    #[test]
    fn marche_ferme_week_end_et_jours_feries() {
        let vendredi = LUNDI + 4 * 86_400;
        // or (HIP-3) : trou du vendredi 21:00 au dimanche 23:00 = fermeture du week-end, pas réparé
        let b = vec![bar(vendredi + 20 * H), bar(vendredi + 2 * 86_400 + 23 * H)];
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::TradFi, &[]).is_empty(), "week-end : marché fermé");
        // la même chose en crypto : un vrai trou
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[]).len(), 1, "crypto : jamais fermé");
        // TradFi, trou un mardi : vrai trou
        let mardi = LUNDI + 86_400;
        let b = serie(mardi, 10, &[4, 5]);
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::TradFi, &[]).len(), 1);
        // un trou qui déborde du week-end (dimanche 22:00 → lundi 02:00) : réparé (des bougies manquent marché ouvert)
        let b = vec![bar(LUNDI + 6 * 86_400 + 21 * H), bar(LUNDI + 7 * 86_400 + 2 * H)];
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::TradFi, &[]).len(), 1);
        // 25 décembre 2026 (vendredi) : fermé toute la journée
        let noel = 1_798_156_800;
        assert!(ferme_a(Calendrier::TradFi, noel + 10 * H) && !ferme_a(Calendrier::TradFi, noel - 10 * H));
        assert!(ferme_a(Calendrier::TradFi, 1_798_761_600 + 12 * H), "1er janvier 2027");
        assert!(!ferme_a(Calendrier::Continu, noel + 10 * H));
        assert_eq!(calendrier(Exchange::Hyperliquid, "xyz:GOLD"), Calendrier::Metaux);
        assert_eq!(calendrier(Exchange::Hyperliquid, "xyz:SP500"), Calendrier::BourseUs);
        assert_eq!(calendrier(Exchange::Hyperliquid, "xyz:nvda"), Calendrier::BourseUs);
        assert_eq!(calendrier(Exchange::Hyperliquid, "xyz:JP225"), Calendrier::TradFi);
        assert_eq!(calendrier(Exchange::Binance, "BTC"), Calendrier::Continu);
    }

    /// Date d'un jour (depuis 1970), pour lire les tests.
    fn date(j: i64) -> (i64, i64, i64) {
        civil(j)
    }

    /// 📅 Pâques et Thanksgiving sont calculés ; comparés ici aux dates publiées (calendrier grégorien).
    #[test]
    fn paques_et_thanksgiving_calcules() {
        assert_eq!(date(paques(2026)), (2026, 4, 5));
        assert_eq!(date(paques(2027)), (2027, 3, 28));
        assert_eq!(date(paques(2028)), (2028, 4, 16));
        // et des années plus éloignées (dates connues de Pâques)
        assert_eq!(date(paques(2000)), (2000, 4, 23));
        assert_eq!(date(paques(2038)), (2038, 4, 25));
        assert_eq!(date(paques(2285)), (2285, 3, 22));
        for (a, j) in [(2026, 26), (2027, 25), (2028, 23)] {
            assert!(feries_us(a).iter().any(|&x| date(x) == (a, 11, j)), "Thanksgiving {a}");
        }
        // le jour de la semaine est juste partout : MLK, Presidents, Memorial, Labor un lundi, Thanksgiving un jeudi
        for a in 1990..2100 {
            let f = feries_us(a);
            for x in &f[..5] {
                if *x != paques(a) - 2 {
                    assert_eq!((x + 3).rem_euclid(7), 0, "{a} {:?}", date(*x));
                }
            }
            assert_eq!((f[5] + 3).rem_euclid(7), 3, "Thanksgiving {a}");
            assert_eq!((paques(a) + 3).rem_euclid(7), 6, "Pâques un dimanche ({a})");
        }
    }

    /// 📅 Les jours fériés de la bourse de New York de 2026 à 2028 (règle samedi → vendredi, dimanche → lundi).
    #[test]
    fn feries_bourse_us_2026_2028() {
        let liste = |a: i64| {
            let mut v: Vec<(i64, i64, i64)> = feries_us(a).into_iter().map(date).collect();
            v.sort();
            v
        };
        assert_eq!(
            liste(2026),
            [
                (2026, 1, 1),
                (2026, 1, 19),
                (2026, 2, 16),
                (2026, 4, 3),
                (2026, 5, 25),
                (2026, 6, 19),
                (2026, 7, 3), // 4 juillet un samedi
                (2026, 9, 7),
                (2026, 11, 26),
                (2026, 12, 25)
            ]
        );
        assert_eq!(
            liste(2027),
            [
                (2027, 1, 1),
                (2027, 1, 18),
                (2027, 2, 15),
                (2027, 3, 26),
                (2027, 5, 31),
                (2027, 6, 18), // Juneteenth un samedi
                (2027, 7, 5),  // 4 juillet un dimanche
                (2027, 9, 6),
                (2027, 11, 25),
                (2027, 12, 24) // Noël un samedi
            ]
        );
        // 2028 : le 1er janvier est un samedi, pas de fermeture le vendredi 31 décembre 2027
        assert_eq!(
            liste(2028),
            [
                (2028, 1, 17),
                (2028, 2, 21),
                (2028, 4, 14),
                (2028, 5, 29),
                (2028, 6, 19),
                (2028, 7, 4),
                (2028, 9, 4),
                (2028, 11, 23),
                (2028, 12, 25)
            ]
        );
        assert!(!ferie(Calendrier::BourseUs, jours_de(2027, 12, 31)));
    }

    /// 🕳️ Un trou un jour férié n'est pas réparé, selon le marché : Thanksgiving ferme les actions américaines, pas
    /// l'or ; le Vendredi saint ferme les deux ; la crypto ne ferme jamais.
    #[test]
    fn trou_un_jour_ferie_pas_repare() {
        let jour = |a, m, d| jours_de(a, m, d) * 86_400;
        // Thanksgiving 2026 (jeudi 26 novembre) : de la veille 23:00 au vendredi 00:00
        let tg = jour(2026, 11, 26);
        let b = vec![bar(tg - 2 * H), bar(tg + 24 * H)];
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::BourseUs, &[]).is_empty(), "actions : fermé");
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::Metaux, &[]).len(), 1, "or : ouvert à Thanksgiving");
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[]).len(), 1, "crypto");
        // Vendredi saint 2027 (26 mars) : jeudi 23:00 → réouverture du dimanche 23:00 (avec le week-end)
        let vs = jour(2027, 3, 26);
        let b = vec![bar(vs - 2 * H), bar(vs + 2 * 86_400 + 23 * H)];
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::BourseUs, &[]).is_empty());
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::Metaux, &[]).is_empty());
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::TradFi, &[]).len(), 1, "Japon : pas ce jour-là");
        // Memorial Day 2028 (lundi 29 mai) : du vendredi 20:00 au mardi 00:00, tout fermé
        let md = jour(2028, 5, 29);
        let b = vec![bar(md - 3 * 86_400 + 19 * H), bar(md + 24 * H)];
        assert!(trouver(&b, Timeframe::H1, &[], Calendrier::BourseUs, &[]).is_empty());
        // un trou qui déborde sur le mardi 01:00 (marché ouvert) : réparé
        let b = vec![bar(md - 3 * 86_400 + 19 * H), bar(md + 26 * H)];
        assert_eq!(trouver(&b, Timeframe::H1, &[], Calendrier::BourseUs, &[]).len(), 1);
        // en bougies de 1 jour : la bougie du jour férié manque, pas réparée
        let b = vec![bar(jour(2026, 7, 2)), bar(jour(2026, 7, 6))];
        assert!(
            trouver(&b, Timeframe::D1, &[], Calendrier::BourseUs, &[]).is_empty(),
            "3 juillet 2026 + week-end"
        );
    }

    /// ⏳ Un trou que l'exchange n'avait pas est laissé de côté une semaine, puis réessayé ; sa date d'essai est
    /// remise à jour s'il manque encore.
    #[test]
    fn trou_impossible_reessaye_chaque_semaine() {
        let b = serie(LUNDI, 10, &[4]);
        let g = trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[]);
        assert_eq!(g.len(), 1);
        let mut connus = vec![];
        let t0 = LUNDI + 20 * H;
        noter(&mut connus, &g, t0);
        let reste = |connus: &[Connu], now| trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &a_eviter(connus, now));
        assert!(reste(&connus, t0).is_empty(), "tout de suite : pas redemandé");
        assert!(reste(&connus, t0 + 6 * 86_400).is_empty(), "6 jours après : toujours pas");
        assert_eq!(reste(&connus, t0 + REESSAI_S), g, "une semaine après : réessayé");
        // encore absent : noté de nouveau, pour une semaine de plus (pas en double)
        noter(&mut connus, &g, t0 + REESSAI_S);
        assert_eq!(connus.len(), 1);
        assert!(reste(&connus, t0 + REESSAI_S + 86_400).is_empty());
        // au plus 1000 trous gardés, les essais les plus récents
        let beaucoup: Vec<Trou> = (0..1500).map(|k| Trou { debut: k * H, fin: k * H + H }).collect();
        for (k, t) in beaucoup.iter().enumerate() {
            noter(&mut connus, &[*t], k as i64);
        }
        assert_eq!(connus.len(), MAX_CONNUS);
        assert!(connus.iter().all(|c| c.essaye >= 500));
    }

    /// 💾 Les trous connus survivent à un redémarrage (dossier temporaire, jamais le vrai cache) ; fichier en 0600 ;
    /// un fichier abîmé ne fait rien planter.
    #[test]
    fn trous_connus_sur_le_disque() {
        let dir = std::env::temp_dir().join(format!("argos-trous-connus-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = chemin_connus(&dir.join("hl-btc-1h.bin"));
        assert_eq!(p.file_name().unwrap(), "hl-btc-1h.trous");
        assert!(lire_connus(&p).is_empty(), "pas encore de fichier");
        let c = vec![
            Connu {
                trou: Trou { debut: 3600, fin: 7200 },
                essaye: 10,
            },
            Connu {
                trou: Trou { debut: 9000, fin: 9900 },
                essaye: 20,
            },
        ];
        ecrire_connus(&p, &c).unwrap();
        assert_eq!(lire_connus(&p), c);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&p).unwrap().permissions().mode() & 0o777, 0o600);
        }
        std::fs::write(&p, "1 2 3\nabc\n5 4 1\n7 8 9 10\n\u{0}\n11 12 13\n").unwrap();
        assert_eq!(lire_connus(&p).len(), 2, "lignes illisibles sautées");
        std::fs::write(&p, vec![b'1'; 70 * 1024]).unwrap();
        assert!(lire_connus(&p).is_empty(), "fichier trop gros : ignoré");
        ecrire_connus(&p, &[]).unwrap();
        assert!(!p.exists(), "plus aucun trou : fichier retiré");
        ecrire_connus(&p, &[]).unwrap();
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Faux exchange (Binance) : a toutes les bougies d'une heure, sauf celles de `absentes` ; compte les demandes.
    #[derive(Clone)]
    struct Ex(Vec<i64>, std::sync::Arc<std::sync::Mutex<usize>>);
    impl Http for Ex {
        async fn get(&self, url: &str) -> Result<String> {
            *self.1.lock().unwrap() += 1;
            let q = |k: &str| url.split(&format!("{k}=")).nth(1).map(|v| v.split('&').next().unwrap().parse::<i64>().unwrap());
            let (limit, end) = (q("limit").unwrap(), q("endTime").unwrap() / 1000);
            let rows: Vec<String> = (1..=limit)
                .rev()
                .map(|k| (end / H - k + 1) * H)
                .filter(|t| !self.0.contains(t))
                .map(|t| format!(r#"[{},"10","11","9","10.5","1"]"#, t * 1000))
                .collect();
            Ok(format!("[{}]", rows.join(",")))
        }
        async fn post_json(&self, _: &str, _: &str) -> Result<String> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn repare_ce_qui_existe_et_note_le_reste() {
        let b = serie(LUNDI, 20, &[3, 4, 10]);
        let g = trouver(&b, Timeframe::H1, &[], Calendrier::Continu, &[]);
        assert_eq!(g.len(), 2);
        // l'exchange a les bougies 3 et 4, mais pas la 10
        let feed = ExchangeFeed::new(Exchange::Binance, Ex(vec![LUNDI + 10 * H], Default::default()));
        let r = reparer(&feed, "BTC", Timeframe::H1, &g, 10).await;
        assert_eq!(r.bars.iter().map(|b| b.time).collect::<Vec<_>>(), vec![LUNDI + 3 * H, LUNDI + 4 * H]);
        assert_eq!(r.irreparables, vec![g[1]]);
        let tout = combler(&b, &r.bars);
        assert_eq!(tout.len(), 19, "20 moins la bougie que l'exchange n'a pas");
        // le trou irréparable n'est plus redemandé
        assert!(trouver(&tout, Timeframe::H1, &[], Calendrier::Continu, &r.irreparables).is_empty());
        // au plus `max` trous par passage
        let feed = ExchangeFeed::new(Exchange::Binance, Ex(vec![], Default::default()));
        let r = reparer(&feed, "BTC", Timeframe::H1, &g, 1).await;
        assert_eq!((r.bars.len(), *feed.http.1.lock().unwrap()), (2, 1));
    }

    #[test]
    fn journal_sans_rien_de_sensible() {
        let l = texte_irreparable(
            Exchange::Okx,
            "BTC",
            Timeframe::H1,
            &Trou {
                debut: LUNDI,
                fin: LUNDI + 2 * H,
            },
        );
        assert_eq!(
            l,
            "⚠️ Trou non réparable · OKX BTC 1h · 2026-11-02 00:00 UTC → 2026-11-02 02:00 UTC · 2 bougie(s) : l'exchange ne les a pas"
        );
        assert!(!l.contains("http") && !l.contains('?') && !l.contains('='));
    }
}
