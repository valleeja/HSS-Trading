//! 🧮 Orderflow réel : les trades d'un marché rangés **par bougie de 1 min et par rangée de prix** (achats et ventes au
//! marché, delta = achats − ventes). Logique pure : le flux, le stockage et l'écran sont ailleurs.
//!
//! - Le **pas de prix** est fixé une fois par marché (1, 2 ou 5 × 10ⁿ, environ 1/10 000 du prix) puis gardé avec les
//!   données : les jours restent comparables. L'écran peut regrouper plusieurs rangées (`elargir`).
//! - Une bougie est **incomplète** si l'app a démarré, changé de marché ou perdu la connexion pendant elle
//!   (`coupure`) : des trades ont pu manquer.
//! - Les unités plus grandes (5 min, 1 h…) sont la **somme** des bougies 1 min (`regrouper`).

use std::collections::BTreeMap;

/// Unité de base : 1 min, en secondes.
pub const MINUTE_S: i64 = 60;
/// ⏱️ Une minute n'est rangée comme fermée que 2 s après sa fin (les derniers trades arrivent avec un peu de retard).
pub const RETARD_MS: i64 = 2_000;
/// Pas de prix visé : environ un dix-millième du prix (BTC à 90 000 → 10 $).
pub const PAS_RELATIF: f64 = 1e-4;

/// Un marché suivi : exchange + symbole (« hl » + « BTC »). Toujours cette clé, quel que soit le marché.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct Cle {
    pub exchange: String,
    pub symbole: String,
}

/// Un trade reçu : heure (ms UTC), prix, taille, `achat` = le preneur achète au marché, et le numéro du trade quand
/// l'exchange en donne un toujours croissant (Binance) : un trade déjà compté n'est jamais recompté.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TradeFlux {
    pub time_ms: i64,
    pub prix: f64,
    pub taille: f64,
    pub achat: bool,
    pub id: Option<u64>,
}

/// Une rangée de prix : (numéro de rangée, achats, ventes) ; prix du bas = rangée × pas.
pub type Rangee = (i64, f64, f64);

/// Une bougie d'orderflow réel.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlowBar {
    /// Début de la bougie (secondes UTC, comme `Bar::time`).
    pub time: i64,
    /// Pas de prix des rangées.
    pub pas: f64,
    /// Rangées triées par numéro.
    pub rangees: Vec<Rangee>,
    /// Aucun trade n'a pu manquer pendant cette bougie.
    pub complete: bool,
}

impl FlowBar {
    /// Delta : achats − ventes.
    pub fn delta(&self) -> f64 {
        self.rangees.iter().map(|r| r.1 - r.2).sum()
    }

    /// Les rangées regroupées par paquets de `k` (pas × `k`) pour un affichage plus large.
    pub fn elargir(&self, k: i64) -> FlowBar {
        let k = k.max(1);
        let mut m: BTreeMap<i64, (f64, f64)> = BTreeMap::new();
        for &(r, a, v) in &self.rangees {
            let e = m.entry(r.div_euclid(k)).or_default();
            (e.0, e.1) = (e.0 + a, e.1 + v);
        }
        FlowBar {
            time: self.time,
            pas: self.pas * k as f64,
            rangees: m.into_iter().map(|(r, (a, v))| (r, a, v)).collect(),
            complete: self.complete,
        }
    }
}

/// Pas « rond » le plus proche de `v` (1, 2, 5 × 10ⁿ ; 1 si `v` n'est pas un nombre positif), comme `nice` de v1.
pub fn nice(v: f64) -> f64 {
    if !(v > 0.0) {
        return 1.0;
    }
    let e = 10f64.powf(v.log10().floor());
    let f = v / e;
    (if f < 1.5 {
        1.0
    } else if f < 3.5 {
        2.0
    } else if f < 7.5 {
        5.0
    } else {
        10.0
    }) * e
}

/// Pas de prix fixe d'un marché d'après un prix : environ `PAS_RELATIF` du prix, arrondi (`nice`).
pub fn pas_pour(prix: f64) -> f64 {
    nice(prix.abs() * PAS_RELATIF)
}

/// 🦶 Diagonal imbalance of a footprint row, as the footprint draws it: `rows` are a bar's price rows from bottom to
/// top, `(row index, buy volume, sell volume)`; row `k` is a buy imbalance when its buys are at least `ratio` × the
/// sells of the row just below (index − 1), a sell imbalance when its sells are at least `ratio` × the buys of the
/// row just above (index + 1); a missing neighbour row: no imbalance. `ratio`: 3.0 for 300 %. Returns (buy, sell).
pub fn desequilibre_diagonal(rows: &[(i64, f64, f64)], k: usize, ratio: f64) -> (bool, bool) {
    let Some(&(i, achats, ventes)) = rows.get(k) else {
        return (false, false);
    };
    let dessous = k.checked_sub(1).and_then(|j| rows.get(j)).filter(|x| x.0 == i - 1);
    let dessus = rows.get(k + 1).filter(|x| x.0 == i + 1);
    (
        dessous.is_some_and(|x| achats >= ratio * x.2.max(1e-12)),
        dessus.is_some_and(|x| ventes >= ratio * x.1.max(1e-12)),
    )
}

/// Bougies de 1 min regroupées en bougies de `tf_s` secondes (multiple de 60) : rangées additionnées. Une bougie
/// regroupée est complète seulement si toutes ses minutes sont là et complètes. Toutes doivent avoir le même pas
/// (celles d'un autre pas sont laissées de côté).
pub fn regrouper(minutes: &[FlowBar], tf_s: i64) -> Vec<FlowBar> {
    let tf = tf_s.max(MINUTE_S);
    let Some(pas) = minutes.first().map(|b| b.pas) else { return vec![] };
    let mut groupes: BTreeMap<i64, (BTreeMap<i64, (f64, f64)>, i64, bool)> = BTreeMap::new();
    for b in minutes.iter().filter(|b| b.pas == pas) {
        let g = groupes.entry(b.time.div_euclid(tf) * tf).or_insert_with(|| (BTreeMap::new(), 0, true));
        for &(r, a, v) in &b.rangees {
            let e = g.0.entry(r).or_default();
            (e.0, e.1) = (e.0 + a, e.1 + v);
        }
        (g.1, g.2) = (g.1 + 1, g.2 && b.complete);
    }
    groupes
        .into_iter()
        .map(|(time, (m, n, ok))| FlowBar {
            time,
            pas,
            rangees: m.into_iter().map(|(r, (a, v))| (r, a, v)).collect(),
            complete: ok && n == tf / MINUTE_S,
        })
        .collect()
}

/// 🏦 Prix moyen d'une bougie d'orderflow, pondéré par le volume (le milieu de chaque rangée) ; `None` sans volume.
pub fn vwap(b: &FlowBar) -> Option<f64> {
    let (mut pv, mut v) = (0.0, 0.0);
    for &(r, a, s) in &b.rangees {
        pv += (r as f64 + 0.5) * b.pas * (a + s);
        v += a + s;
    }
    (v > 0.0 && b.pas > 0.0).then(|| pv / v)
}

/// 🏦 Écart de prix d'une bourse avec la référence (la bourse de la chart), à ajouter à ses prix : la médiane, sur les
/// `n` dernières bougies présentes des deux côtés, de (prix moyen de la référence − prix moyen de la bourse). Les deux
/// listes sont triées par heure ; 0 sans bougie commune.
pub fn ecart(reference: &[FlowBar], autre: &[FlowBar], n: usize) -> f64 {
    let mut d: Vec<f64> = autre
        .iter()
        .rev()
        .filter_map(|b| {
            let r = reference.binary_search_by_key(&b.time, |x| x.time).ok().map(|k| &reference[k])?;
            Some(vwap(r)? - vwap(b)?)
        })
        .take(n.max(1))
        .collect();
    if d.is_empty() {
        return 0.0;
    }
    d.sort_by(f64::total_cmp);
    let m = d.len() / 2;
    if d.len() % 2 == 1 {
        d[m]
    } else {
        (d[m - 1] + d[m]) / 2.0
    }
}

/// 🏦 Une bourse du footprint multi-bourses : ses bougies (triées par heure, à l'unité de la chart, tailles en pièces)
/// et l'écart de prix ajouté à chacun de ses trades (0 : ses prix tels quels).
#[derive(Debug, Clone, Copy)]
pub struct Bourse<'a> {
    pub bars: &'a [FlowBar],
    pub ecart: f64,
}

/// 🏦 La rangée au pas `pas` d'une rangée de `b` décalée de `ecart`, et sa taille dans l'unité voulue (`dollars` :
/// taille × prix du milieu de la rangée).
fn recaler(b: &FlowBar, r: i64, ecart: f64, pas: f64, dollars: bool) -> (i64, f64) {
    let milieu = (r as f64 + 0.5) * b.pas + ecart;
    ((milieu / pas).floor() as i64, if dollars { milieu } else { 1.0 })
}

/// 🏦 Le footprint multi-bourses : à chaque heure, les rangées de toutes les bourses additionnées au pas `pas` (prix
/// décalés de leur écart ; `dollars` : tailles × prix) ; puis, à la même heure, le volume de chaque bourse (même
/// unité, dans l'ordre de `bourses`). Une bougie est complète si chaque bourse présente à cette heure l'est.
pub fn fusionner(bourses: &[Bourse], pas: f64, dollars: bool) -> (Vec<FlowBar>, Vec<(i64, Vec<f64>)>) {
    if !(pas > 0.0) {
        return (vec![], vec![]);
    }
    let mut heures: BTreeMap<i64, (BTreeMap<i64, (f64, f64)>, bool, Vec<f64>)> = BTreeMap::new();
    for (k, src) in bourses.iter().enumerate() {
        for b in src.bars.iter().filter(|b| b.pas > 0.0) {
            let h = heures.entry(b.time).or_insert_with(|| (BTreeMap::new(), true, vec![0.0; bourses.len()]));
            h.1 &= b.complete;
            for &(r, a, v) in &b.rangees {
                let (j, x) = recaler(b, r, src.ecart, pas, dollars);
                let e = h.0.entry(j).or_default();
                (e.0, e.1) = (e.0 + a * x, e.1 + v * x);
                h.2[k] += (a + v) * x;
            }
        }
    }
    let mut parts = Vec::with_capacity(heures.len());
    let bars = heures
        .into_iter()
        .map(|(time, (m, complete, p))| {
            parts.push((time, p));
            FlowBar {
                time,
                pas,
                rangees: m.into_iter().map(|(r, (a, v))| (r, a, v)).collect(),
                complete,
            }
        })
        .collect();
    (bars, parts)
}

/// 🏦 Le détail d'une rangée de l'écran (rangée `j` au pas `step` de l'écran) de la bougie de `time` : (achats, ventes)
/// de chaque bourse, dans l'ordre de `bourses`, comptés comme [`fusionner`] puis le footprint les rangent (pas commun
/// `pas`, puis le milieu de chaque rangée commune au pas de l'écran).
pub fn detail(bourses: &[Bourse], pas: f64, dollars: bool, time: i64, step: f64, j: i64) -> Vec<(f64, f64)> {
    bourses
        .iter()
        .map(|src| {
            let Ok(k) = src.bars.binary_search_by_key(&time, |b| b.time) else {
                return (0.0, 0.0);
            };
            let b = &src.bars[k];
            let (mut a_, mut v_) = (0.0, 0.0);
            if pas > 0.0 && step > 0.0 && b.pas > 0.0 {
                for &(r, a, v) in &b.rangees {
                    let (c, x) = recaler(b, r, src.ecart, pas, dollars);
                    if (((c as f64 + 0.5) * pas) / step).floor() as i64 == j {
                        (a_, v_) = (a_ + a * x, v_ + v * x);
                    }
                }
            }
            (a_, v_)
        })
        .collect()
}

/// L'agrégateur d'un marché : range les trades reçus dans les minutes ouvertes, rend les minutes fermées.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Agregateur {
    pas: f64,
    /// Minutes ouvertes : début (s) → (rangée → (achats, ventes)), complète.
    ouvertes: BTreeMap<i64, (BTreeMap<i64, (f64, f64)>, bool)>,
    /// Avant cette minute (s), tout est déjà rangé comme fermé : un trade plus vieux est ignoré.
    ferme_avant: i64,
    /// Heure du dernier trade compté (ms).
    dernier_ms: i64,
    /// Plus grand numéro de trade compté (exchanges qui en donnent un).
    dernier_id: Option<u64>,
    /// Juste après une (re)connexion : le premier lot peut redonner des trades déjà comptés.
    apres_connexion: bool,
}

impl Agregateur {
    /// Un agrégateur ; `pas` : celui déjà gardé pour ce marché (sinon fixé au premier trade).
    pub fn new(pas: Option<f64>) -> Self {
        Self {
            pas: pas.filter(|p| p.is_finite() && *p > 0.0).unwrap_or(0.0),
            ..Self::default()
        }
    }

    /// Le pas de prix (0 tant qu'aucun trade n'est arrivé).
    pub fn pas(&self) -> f64 {
        self.pas
    }

    /// Range des trades. Ignorés : taille ou prix nuls, négatifs ou pas des nombres ; trade d'une minute déjà fermée ;
    /// trade avec un numéro pas plus grand que le dernier compté ; sans numéro, juste après une (re)connexion, les
    /// trades pas plus récents que le dernier compté (lot de trades récents que l'exchange renvoie à l'abonnement).
    /// Un lot numéroté est compté dans l'ordre de ses numéros (Bitget les envoie du plus récent au plus ancien).
    pub fn ajouter(&mut self, trades: &[TradeFlux]) {
        let deja = std::mem::take(&mut self.apres_connexion).then_some(self.dernier_ms);
        let mut trades = trades.to_vec();
        trades.sort_by_key(|t| t.id);
        for t in &trades {
            let ok = t.prix.is_finite() && t.prix > 0.0 && t.taille.is_finite() && t.taille > 0.0;
            let deja_compte = match t.id {
                Some(id) => self.dernier_id.is_some_and(|d| id <= d),
                None => deja.is_some_and(|d| t.time_ms <= d),
            };
            if !ok || deja_compte {
                continue;
            }
            let minute = t.time_ms.div_euclid(1000 * MINUTE_S) * MINUTE_S;
            if minute < self.ferme_avant {
                continue;
            }
            if self.pas <= 0.0 {
                self.pas = pas_pour(t.prix);
            }
            let r = (t.prix / self.pas).floor() as i64;
            let m = self.ouvertes.entry(minute).or_insert_with(|| (BTreeMap::new(), true));
            let e = m.0.entry(r).or_default();
            if t.achat {
                e.0 += t.taille;
            } else {
                e.1 += t.taille;
            }
            self.dernier_ms = self.dernier_ms.max(t.time_ms);
            self.dernier_id = t.id.max(self.dernier_id);
        }
    }

    /// Démarrage, changement de marché, connexion perdue ou retrouvée à `now_ms` : la minute en cours est incomplète,
    /// et le prochain lot de trades est filtré (voir `ajouter`).
    pub fn coupure(&mut self, now_ms: i64) {
        let minute = now_ms.div_euclid(1000 * MINUTE_S) * MINUTE_S;
        if minute >= self.ferme_avant {
            self.ouvertes.entry(minute).or_insert_with(|| (BTreeMap::new(), true)).1 = false;
        }
        self.apres_connexion = true;
    }

    /// Les minutes fermées à `now_ms` (fin + `RETARD_MS` passée), retirées de l'agrégateur, dans l'ordre.
    pub fn fermees(&mut self, now_ms: i64) -> Vec<FlowBar> {
        let limite = (now_ms - RETARD_MS).div_euclid(1000 * MINUTE_S) * MINUTE_S;
        let garde = self.ouvertes.split_off(&limite);
        let parties = std::mem::replace(&mut self.ouvertes, garde);
        self.ferme_avant = self.ferme_avant.max(limite);
        parties.into_iter().map(|(t, m)| self.bougie(t, m)).collect()
    }

    /// Toutes les minutes, fermées ou non (changement de marché, sortie) : la minute en cours est marquée incomplète.
    pub fn tout_fermer(&mut self, now_ms: i64) -> Vec<FlowBar> {
        self.coupure(now_ms);
        self.ferme_avant = i64::MAX;
        let parties = std::mem::take(&mut self.ouvertes);
        parties.into_iter().map(|(t, m)| self.bougie(t, m)).collect()
    }

    /// Les minutes encore ouvertes (bougie en cours), sans les retirer.
    pub fn en_cours(&self) -> Vec<FlowBar> {
        self.ouvertes.iter().map(|(&t, m)| self.bougie(t, m.clone())).collect()
    }

    fn bougie(&self, time: i64, (m, complete): (BTreeMap<i64, (f64, f64)>, bool)) -> FlowBar {
        FlowBar {
            time,
            pas: self.pas,
            rangees: m.into_iter().map(|(r, (a, v))| (r, a, v)).collect(),
            complete,
        }
    }
}

/// ● Au plus autant de marchés enregistrés en continu (en plus du marché affiché).
pub const MAX_ENREGISTRES: usize = 5;

/// ● Marchés enregistrés en continu tant que l'app est ouverte, même s'ils ne sont pas affichés.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Enregistres {
    pub liste: Vec<Cle>,
}

impl Enregistres {
    /// Ajoute `cle` (rend `Ok(true)`) ou la retire si elle y est (`Ok(false)`) ; liste pleine : `Err` avec le message.
    pub fn basculer(&mut self, cle: Cle) -> Result<bool, String> {
        if let Some(i) = self.liste.iter().position(|c| *c == cle) {
            self.liste.remove(i);
            return Ok(false);
        }
        if self.liste.len() >= MAX_ENREGISTRES {
            return Err(format!("{MAX_ENREGISTRES} symboles au plus : retire-en un"));
        }
        self.liste.push(cle);
        Ok(true)
    }

    pub fn contient(&self, cle: &Cle) -> bool {
        self.liste.contains(cle)
    }
}

/// 💾 Où ranger l'orderflow (seulement des minutes fermées). Une seule mise en œuvre aujourd'hui (le cache binaire
/// d'`argos-data`) ; l'agrégateur et le footprint n'en dépendent pas, pour pouvoir changer de moteur plus tard.
pub trait FlowStore {
    /// Les minutes gardées de `cle` dont le début est dans `[de_s, a_s)`, dans l'ordre.
    fn lire(&self, cle: &Cle, de_s: i64, a_s: i64) -> Vec<FlowBar>;
    /// Ajoute (ou remplace, même heure) des minutes fermées.
    fn ecrire(&mut self, cle: &Cle, minutes: &[FlowBar]) -> Result<(), String>;
    /// Applique les limites de garde (durée et taille) à `now_s`.
    fn limiter(&mut self, now_s: i64);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 🏦 Multi-exchange footprint: two exchanges at the same minute add up price by price once the second one is
    /// moved by its gap; each one's volume is kept; the bar is complete only if both are; a minute of only one
    /// exchange stays; in dollars, size × price.
    #[test]
    fn fusionner_deux_bourses() {
        let b = |time, pas, rangees: Vec<Rangee>, complete| FlowBar { time, pas, rangees, complete };
        // reference at 100 $ steps of 10: rows 10 (100-110) and 11 (110-120)
        let a = vec![
            b(60, 10.0, vec![(10, 2.0, 1.0), (11, 1.0, 3.0)], true),
            b(120, 10.0, vec![(10, 1.0, 0.0)], true),
        ];
        // the other one trades 20 $ lower, steps of 5: 80-85 → +20 → 100-105 (row 10), 95-100 → 115-120 (row 11)
        let o = vec![b(60, 5.0, vec![(16, 4.0, 0.0), (19, 0.0, 2.0)], false)];
        // one minute in common: its gap (volume-weighted prices 110,71 and 87,5)
        assert!((ecart(&a, &o, 30) - (775.0 / 7.0 - 87.5)).abs() < 1e-9);
        let src = [Bourse { bars: &a, ecart: 0.0 }, Bourse { bars: &o, ecart: 20.0 }];
        let (m, parts) = fusionner(&src, 10.0, false);
        assert_eq!(m.len(), 2);
        assert_eq!(m[0].rangees, vec![(10, 6.0, 1.0), (11, 1.0, 5.0)]);
        assert!(!m[0].complete, "the second exchange's minute is incomplete");
        assert!(m[1].complete, "only the reference at 120: complete");
        assert_eq!(parts, vec![(60, vec![7.0, 6.0]), (120, vec![1.0, 0.0])]);
        // the screen's row of 100-120 (step 20, row 5) holds everything of minute 60, exchange by exchange
        assert_eq!(detail(&src, 10.0, false, 60, 20.0, 5), vec![(3.0, 4.0), (4.0, 2.0)]);
        assert_eq!(detail(&src, 10.0, false, 999, 20.0, 5), vec![(0.0, 0.0), (0.0, 0.0)]);
        // in dollars: size × the row's middle price (105 for row 10 of the reference)
        let (d, _) = fusionner(&src[..1], 10.0, true);
        assert_eq!(d[1].rangees, vec![(10, 105.0, 0.0)]);
        assert!(fusionner(&src, 0.0, false).0.is_empty());
    }

    /// 🏦 The gap is the median over the last bars both have (one strange bar does not move it); none in common: 0.
    #[test]
    fn ecart_median_des_dernieres_bougies() {
        let b = |time, r: i64| FlowBar {
            time,
            pas: 1.0,
            rangees: vec![(r, 1.0, 1.0)],
            complete: true,
        };
        let reference: Vec<FlowBar> = (0..5).map(|k| b(k * 60, 100)).collect();
        let autre = vec![b(0, 95), b(60, 95), b(120, 40), b(180, 95), b(240, 96)];
        assert_eq!(ecart(&reference, &autre, 30), 5.0);
        assert_eq!(ecart(&reference, &autre, 1), 4.0, "the last bar only");
        assert_eq!(ecart(&reference, &[b(999, 1)], 30), 0.0);
    }

    /// 🦶 Diagonal imbalance: buys against the sells of the row below, sells against the buys of the row above; a gap
    /// in the rows (or the bottom / top row) gives none; exactly `ratio` × counts.
    #[test]
    fn desequilibre_en_diagonale() {
        let rows = [(10, 5.0, 2.0), (11, 6.0, 9.0), (12, 1.0, 1.0), (14, 30.0, 0.0)];
        assert_eq!(
            desequilibre_diagonal(&rows, 0, 3.0),
            (false, false),
            "bottom row: nothing below; 2 < 3 × 6 above"
        );
        assert_eq!(desequilibre_diagonal(&rows, 1, 3.0), (true, true), "6 ≥ 3 × 2 below; 9 ≥ 3 × 1 above");
        assert_eq!(desequilibre_diagonal(&rows, 2, 3.0), (false, false), "1 < 3 × 9; nothing at 13 above");
        assert_eq!(desequilibre_diagonal(&rows, 3, 3.0), (false, false), "a gap below (13 missing)");
        assert_eq!(desequilibre_diagonal(&rows, 9, 3.0), (false, false), "no such row");
        assert_eq!(
            desequilibre_diagonal(&[(0, 0.0, 1.0), (1, 2.0, 0.0)], 1, 2.0),
            (true, false),
            "2 = 2 × 1: counts"
        );
    }

    fn t(time_ms: i64, prix: f64, taille: f64, achat: bool) -> TradeFlux {
        TradeFlux {
            time_ms,
            prix,
            taille,
            achat,
            id: None,
        }
    }

    const M: i64 = 60_000;

    #[test]
    fn pas_fixe_1_2_5() {
        let proche = |a: f64, b: f64| (a - b).abs() < b * 1e-9;
        assert!(proche(pas_pour(90_000.0), 10.0), "BTC");
        assert!(proche(pas_pour(3_000.0), 0.2), "ETH");
        assert!(proche(pas_pour(200.0), 0.02));
        assert!(proche(pas_pour(0.6), 0.00005));
    }

    #[test]
    fn range_par_minute_et_par_rangee() {
        let mut a = Agregateur::new(Some(10.0));
        a.ajouter(&[t(5_000, 100.0, 1.0, true), t(6_000, 109.99, 2.0, false), t(7_000, 110.0, 3.0, true)]);
        a.ajouter(&[t(M, 95.0, 4.0, false)]);
        let f = a.fermees(2 * M + RETARD_MS);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].time, f[0].pas, f[0].complete), (0, 10.0, true));
        assert_eq!(
            f[0].rangees,
            vec![(10, 1.0, 2.0), (11, 3.0, 0.0)],
            "109,99 dans la rangée 100-110, 110 dans la suivante"
        );
        assert_eq!(f[0].delta(), 2.0);
        assert_eq!((f[1].time, f[1].rangees.clone()), (60, vec![(9, 0.0, 4.0)]));
        assert_eq!(f[1].delta(), -4.0);
    }

    #[test]
    fn bords_de_minute() {
        let mut a = Agregateur::new(Some(1.0));
        a.ajouter(&[t(M - 1, 100.0, 1.0, true), t(M, 100.0, 1.0, true)]);
        assert!(a.fermees(M + RETARD_MS - 1).is_empty(), "2 s de retard : pas encore fermée");
        let f = a.fermees(M + RETARD_MS);
        assert_eq!(f.iter().map(|b| b.time).collect::<Vec<_>>(), vec![0], "59,999 s dans la 1re minute");
        assert_eq!(a.en_cours().iter().map(|b| b.time).collect::<Vec<_>>(), vec![60]);
        // un trade en retard pour une minute déjà fermée : ignoré
        a.ajouter(&[t(30_000, 100.0, 9.0, false)]);
        assert_eq!(a.fermees(3 * M).iter().map(|b| b.delta()).collect::<Vec<_>>(), vec![1.0]);
    }

    #[test]
    fn trades_hors_ordre_et_valeurs_absurdes() {
        let mut a = Agregateur::new(None);
        a.ajouter(&[t(M + 5, 200.0, 1.0, true), t(10, 200.0, 2.0, false), t(M + 1, 200.0, 3.0, true)]);
        a.ajouter(&[
            t(20, f64::NAN, 1.0, true),
            t(20, 200.0, 0.0, true),
            t(20, 200.0, f64::NAN, true),
            t(20, -1.0, 1.0, false),
        ]);
        assert!((a.pas() - 0.02).abs() < 1e-12, "fixé au premier trade valable");
        let f = a.fermees(3 * M);
        assert_eq!(f.iter().map(|b| (b.time, b.delta())).collect::<Vec<_>>(), vec![(0, -2.0), (60, 4.0)]);
    }

    /// Binance : numéro de trade toujours croissant → un trade redonné (reconnexion) n'est jamais recompté, même à la
    /// même milliseconde qu'un autre.
    #[test]
    fn numero_de_trade_jamais_compte_deux_fois() {
        let n = |id: u64, taille: f64| TradeFlux {
            id: Some(id),
            ..t(1_000, 100.0, taille, true)
        };
        let mut a = Agregateur::new(Some(1.0));
        a.ajouter(&[n(10, 1.0), n(11, 2.0)]);
        a.coupure(2_000);
        a.ajouter(&[n(11, 2.0), n(12, 4.0)]);
        a.ajouter(&[n(9, 8.0), n(13, 16.0)]);
        // un lot du plus récent au plus ancien (Bitget) : tous comptés, dans l'ordre de leurs numéros
        a.ajouter(&[n(16, 32.0), n(15, 64.0), n(14, 128.0)]);
        assert_eq!(a.fermees(3 * M)[0].delta(), 1.0 + 2.0 + 4.0 + 16.0 + 32.0 + 64.0 + 128.0);
    }

    #[test]
    fn coupure_rend_la_minute_incomplete_et_filtre_le_lot_suivant() {
        let mut a = Agregateur::new(Some(1.0));
        a.ajouter(&[t(10_000, 100.0, 1.0, true)]);
        a.coupure(20_000);
        // à l'abonnement, l'exchange renvoie des trades récents : ceux déjà comptés (≤ 10 s) sont ignorés
        a.ajouter(&[t(10_000, 100.0, 1.0, true), t(15_000, 100.0, 2.0, true)]);
        // le lot suivant n'est plus filtré (plusieurs trades peuvent avoir la même milliseconde)
        a.ajouter(&[t(15_000, 100.0, 4.0, true)]);
        a.ajouter(&[t(M + 1, 100.0, 1.0, false)]);
        let f = a.fermees(3 * M);
        assert_eq!((f[0].complete, f[0].delta()), (false, 7.0));
        assert!(f[1].complete, "la minute suivante est complète");
        // changement de marché : tout est rendu, la minute en cours incomplète
        a.ajouter(&[t(3 * M + 1, 100.0, 1.0, true)]);
        let fin = a.tout_fermer(3 * M + 30_000);
        assert_eq!(fin.iter().map(|b| (b.time, b.complete)).collect::<Vec<_>>(), vec![(180, false)]);
    }

    #[test]
    fn regroupement_5_min_egal_somme_directe() {
        let mut a = Agregateur::new(Some(1.0));
        let mut b5 = Agregateur::new(Some(1.0));
        let trades: Vec<TradeFlux> = (0..10i64).map(|k| t(k * 37_000, 100.0 + (k % 3) as f64, 1.0 + k as f64, k % 2 == 0)).collect();
        a.ajouter(&trades);
        let minutes = a.fermees(20 * M);
        // somme directe : les mêmes trades rangés comme s'ils étaient tous dans la même minute
        let directs: Vec<TradeFlux> = trades.iter().map(|x| TradeFlux { time_ms: 0, ..*x }).collect();
        b5.ajouter(&directs);
        let direct = b5.fermees(20 * M);
        let g = regrouper(&minutes, 300);
        assert_eq!(g.len(), 2, "0-5 min et 5-10 min");
        let total: Vec<FlowBar> = regrouper(&g, 600);
        assert_eq!(total[0].rangees, direct[0].rangees);
        assert!((g.iter().map(FlowBar::delta).sum::<f64>() - direct[0].delta()).abs() < 1e-9);
        assert!(g[0].complete, "5 minutes là et complètes");
        assert!(!regrouper(&minutes[..3], 300)[0].complete, "minutes manquantes : incomplète");
    }

    #[test]
    fn enregistres_au_plus_5_ajout_et_retrait() {
        let cle = |s: &str| Cle {
            exchange: "hl".into(),
            symbole: s.into(),
        };
        let mut e = Enregistres::default();
        for s in ["BTC", "ETH", "SOL", "XRP", "HYPE"] {
            assert_eq!(e.basculer(cle(s)), Ok(true));
        }
        assert_eq!(e.basculer(cle("DOGE")).unwrap_err(), "5 symboles au plus : retire-en un");
        assert!(!e.contient(&cle("DOGE")));
        assert_eq!(e.basculer(cle("ETH")), Ok(false), "retiré");
        assert_eq!(e.basculer(cle("DOGE")), Ok(true), "une place libre");
        assert_eq!(e.liste.len(), MAX_ENREGISTRES);
        // relu d'un fichier (et d'un ancien fichier sans la liste)
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(serde_json::from_str::<Enregistres>(&json).unwrap(), e);
        assert_eq!(serde_json::from_str::<Enregistres>("{}").unwrap(), Enregistres::default());
    }

    #[test]
    fn elargir_les_rangees() {
        let b = FlowBar {
            time: 0,
            pas: 1.0,
            rangees: vec![(100, 1.0, 0.0), (101, 2.0, 1.0), (105, 0.0, 3.0)],
            complete: true,
        };
        let e = b.elargir(5);
        assert_eq!((e.pas, e.rangees.clone()), (5.0, vec![(20, 3.0, 1.0), (21, 0.0, 3.0)]));
        assert_eq!(e.delta(), b.delta());
    }
}
