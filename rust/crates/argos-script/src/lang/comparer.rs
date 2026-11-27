//! ⚖️ A backtest against TradingView's « List of trades » (its CSV export): the same strategy on the same bars should
//! give the same trades. The file is read whatever its language and columns (TradingView renamed them over the
//! years), its time zone is found from the trades themselves, and each trade gets what differs: an entry or an exit
//! some bars earlier or later, another price, another result, or a trade only one side has. Only the period both
//! sides cover is compared (a list that starts earlier has trades the chart cannot have).

use super::rapport::Ligne;
use argos_core::Side;

/// A closed trade of TradingView's list.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeTv {
    pub numero: usize,
    pub side: Side,
    /// Its « Signal » column: the names of its entry and of its exit.
    pub signal_in: String,
    pub signal_out: String,
    /// Its times as the file writes them (in the time zone of the TradingView chart), in seconds as if they were UTC.
    pub t_in: i64,
    pub t_out: i64,
    pub prix_in: f64,
    pub prix_out: f64,
    pub qty: Option<f64>,
    /// Its result (« Profit », « Net P&L »), when the file has it.
    pub net: Option<f64>,
}

/// TradingView's list, as read.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListeTv {
    /// Oldest first.
    pub trades: Vec<TradeTv>,
    /// Trades still open at the end (« Open »): not compared, as the report leaves them out.
    pub ouverts: usize,
}

/// What differs between the two sides of a trade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Ecart {
    /// Only in the backtest (TradingView has no trade there).
    PasChezTv,
    /// Only in TradingView's list.
    PasChezArgos,
    /// TradingView's entry (exit) is this many bars later (> 0) or earlier (< 0).
    Entree(i64),
    Sortie(i64),
    /// TradingView's price minus the backtest's.
    PrixEntree(f64),
    PrixSortie(f64),
    /// TradingView's result minus the backtest's ($).
    Resultat(f64),
}

/// A row of the comparison: the backtest's trade (its index in the report's list) and / or TradingView's.
#[derive(Debug, Clone, PartialEq)]
pub struct Rangee {
    pub argos: Option<usize>,
    pub tv: Option<TradeTv>,
    /// Empty: the same trade.
    pub ecarts: Vec<Ecart>,
}

impl Rangee {
    pub fn pareil(&self) -> bool {
        self.ecarts.is_empty()
    }
}

/// The comparison.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Comparaison {
    /// Oldest first.
    pub rangees: Vec<Rangee>,
    pub pareils: usize,
    pub differents: usize,
    /// Trades of either side outside the period both cover: not compared.
    pub hors_periode: usize,
    /// TradingView's trades still open at the end: not compared.
    pub ouverts_tv: usize,
    /// The file's time zone found from the trades: `None` the chart's (with its summer time), else this many seconds
    /// from UTC all year.
    pub fuseau: Option<i64>,
}

/// The difference of two results still called the same (the mock-up's « à 0,01 $ près »).
pub const RESULTAT_PRES: f64 = 0.01;

/// Lower case without accents, for the column names and the types (« Entrée » = « entree »).
fn simple(s: &str) -> String {
    s.trim()
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// The fields of a line (`sep`), with TradingView's quotes (`"a, b"`, `""` for a quote).
fn champs(ligne: &str, sep: char) -> Vec<String> {
    let mut out = vec![];
    let mut cur = String::new();
    let mut dedans = false;
    let mut it = ligne.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '"' if dedans && it.peek() == Some(&'"') => {
                cur.push('"');
                it.next();
            }
            '"' => dedans = !dedans,
            c if c == sep && !dedans => out.push(std::mem::take(&mut cur).trim().to_string()),
            c => cur.push(c),
        }
    }
    out.push(cur.trim().to_string());
    out
}

/// A number as the file writes it: « 61111.5 », « 61 111,5 », « 61,111.5 », « −9.03 », « $6.15 », « 12 % ».
fn nombre(s: &str) -> Option<f64> {
    let s: String = s
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '\u{a0}' | '\u{202f}' | '$' | '%' | '€'))
        .map(|c| if c == '−' { '-' } else { c })
        .collect();
    let s = s.trim_end_matches(|c: char| c.is_alphabetic());
    let s = if s.contains('.') && s.contains(',') {
        s.replace(',', "")
    } else {
        s.replace(',', ".")
    };
    s.parse::<f64>().ok().filter(|v| v.is_finite())
}

/// A date and time as the file writes it, in seconds as if it were UTC.
fn date(s: &str) -> Option<i64> {
    use chrono::{NaiveDate, NaiveDateTime};
    let s = s.trim();
    const AVEC_HEURE: [&str; 12] = [
        "%Y-%m-%d %H:%M",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%dT%H:%M:%S",
        "%Y/%m/%d %H:%M",
        "%d/%m/%Y %H:%M",
        "%d/%m/%Y %H:%M:%S",
        "%m/%d/%Y %H:%M",
        "%d.%m.%Y %H:%M",
        "%b %d, %Y, %H:%M",
        "%b %d, %Y %H:%M",
        "%d %b %Y %H:%M",
    ];
    for f in AVEC_HEURE {
        if let Ok(d) = NaiveDateTime::parse_from_str(s, f) {
            return Some(d.and_utc().timestamp());
        }
    }
    for f in ["%Y-%m-%d", "%d/%m/%Y", "%Y/%m/%d"] {
        if let Ok(d) = NaiveDate::parse_from_str(s, f) {
            return Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
        }
    }
    None
}

/// The columns of the file, found by their names (English, French, Spanish; old and new names of TradingView).
#[derive(Debug, Default)]
struct Colonnes {
    numero: Option<usize>,
    type_: Option<usize>,
    signal: Option<usize>,
    date: Option<usize>,
    prix: Option<usize>,
    qty: Option<usize>,
    net: Option<usize>,
}

fn colonnes(tetes: &[String]) -> Colonnes {
    let mut c = Colonnes::default();
    for (k, h) in tetes.iter().map(|h| simple(h)).enumerate() {
        let a = |m: &str| h.contains(m);
        let pct = a("%");
        if c.numero.is_none() && (a("#") || h.starts_with("trade") || h.starts_with("n°") || h == "no" || a("numero")) {
            c.numero = Some(k);
        } else if c.type_.is_none() && h == "type" {
            c.type_ = Some(k);
        } else if c.signal.is_none() && (a("signal") || a("senal")) {
            c.signal = Some(k);
        } else if c.date.is_none() && (a("date") || a("heure") || a("time") || a("fecha")) {
            c.date = Some(k);
        } else if c.prix.is_none() && (h.starts_with("price") || h.starts_with("prix") || h.starts_with("precio")) {
            c.prix = Some(k);
        } else if c.qty.is_none()
            && !a("value")
            && !a("valeur")
            && !a("valor")
            && (a("contract") || a("contrat") || a("qty") || a("qte") || a("quanti") || a("cantidad"))
        {
            c.qty = Some(k);
        } else if c.net.is_none()
            && !pct
            && !a("cumul")
            && !a("acumul")
            && !a("run")
            && !a("draw")
            && !a("excursion")
            && (a("profit") || a("p&l") || a("p/l") || a("benefice") || a("resultat") || a("ganancia"))
        {
            c.net = Some(k);
        }
    }
    c
}

/// Entry or exit, long or short, from the « Type » column (« Entry long », « Sortie short », « Sortir du short »,
/// « Exit Short »…).
fn type_de(s: &str) -> Option<(bool, Side)> {
    let s = simple(s);
    let entree = if s.contains("entr") {
        true
    } else if s.contains("exit") || s.contains("sorti") || s.contains("quitt") || s.contains("salida") || s.contains("salir") {
        false
    } else {
        return None;
    };
    let side = if s.contains("short") || s.contains("court") || s.contains("vente") || s.contains("sell") || s.contains("corto") {
        Side::Short
    } else if s.contains("long") || s.contains("achat") || s.contains("buy") || s.contains("largo") {
        Side::Long
    } else {
        return None;
    };
    Some((entree, side))
}

/// « Open », « Ouverture »…: the exit of a trade still open.
fn ouvert(s: &str) -> bool {
    matches!(
        simple(s).as_str(),
        "open" | "ouvert" | "ouverte" | "ouverture" | "abierta" | "abierto" | "apertura"
    )
}

/// One row of the file: entry or exit of trade `numero`.
struct RangTv {
    numero: Option<usize>,
    entree: bool,
    side: Side,
    signal: String,
    t: i64,
    prix: f64,
    qty: Option<f64>,
    net: Option<f64>,
}

/// Reads TradingView's « List of trades » exported as CSV (comma, semicolon or tab). Its error says what is wrong in
/// words for the banner.
pub fn lire_csv(texte: &str) -> Result<ListeTv, String> {
    if texte.starts_with("PK") {
        return Err("c'est un fichier Excel : à l'export, choisis CSV".into());
    }
    let texte = texte.trim_start_matches('\u{feff}');
    let mut lignes = texte.lines().filter(|l| !l.trim().is_empty());
    let tete = lignes.next().ok_or("le fichier est vide")?;
    let sep = [',', ';', '\t'].into_iter().max_by_key(|s| champs(tete, *s).len()).unwrap_or(',');
    let c = colonnes(&champs(tete, sep));
    let (Some(ct), Some(cd), Some(cp)) = (c.type_, c.date, c.prix) else {
        return Err("ce n'est pas la liste des trades de TradingView (colonnes Type, Date et Prix introuvables)".into());
    };
    let mut rangs = vec![];
    // (the trades still open: their exit row says « Open » / « Ouverture » instead of a date or as its signal)
    let mut ouverts = std::collections::BTreeSet::new();
    for (k, l) in lignes.enumerate() {
        let f = champs(l, sep);
        let get = |i: Option<usize>| i.and_then(|i| f.get(i)).map(String::as_str).unwrap_or("");
        let Some((entree, side)) = type_de(get(Some(ct))) else { continue };
        if !entree && (ouvert(get(Some(cd))) || ouvert(get(c.signal))) {
            ouverts.insert(nombre(get(c.numero)).map_or(usize::MAX - k / 2, |n| n as usize));
            continue;
        }
        let (Some(t), Some(prix)) = (date(get(Some(cd))), nombre(get(Some(cp)))) else {
            return Err(format!("ligne {} : date ou prix illisible", k + 2));
        };
        rangs.push(RangTv {
            numero: nombre(get(c.numero)).map(|n| n as usize),
            entree,
            side,
            signal: get(c.signal).to_string(),
            t,
            prix,
            qty: nombre(get(c.qty)),
            net: nombre(get(c.net)),
        });
    }
    if rangs.is_empty() {
        return Err("aucun trade dans le fichier".into());
    }
    // each trade: its entry and its exit (by their number; without numbers, two rows after one another)
    let mut liste = ListeTv::default();
    let mut par_numero: std::collections::BTreeMap<usize, (Option<RangTv>, Option<RangTv>)> = Default::default();
    for (k, r) in rangs.into_iter().enumerate() {
        let n = r.numero.unwrap_or(usize::MAX - k / 2);
        let e = par_numero.entry(n).or_default();
        if r.entree {
            e.0 = Some(r);
        } else {
            e.1 = Some(r);
        }
    }
    for (n, (e, s)) in par_numero {
        let (Some(e), Some(s)) = (e, s) else {
            liste.ouverts += usize::from(ouverts.contains(&n));
            continue;
        };
        liste.trades.push(TradeTv {
            numero: if n > usize::MAX / 2 { 0 } else { n },
            side: e.side,
            signal_in: e.signal,
            signal_out: s.signal,
            t_in: e.t,
            t_out: s.t,
            prix_in: e.prix,
            prix_out: s.prix,
            qty: s.qty.or(e.qty),
            net: s.net.or(e.net),
        });
    }
    liste.trades.sort_by_key(|t| (t.t_in, t.t_out));
    Ok(liste)
}

/// Compares the backtest's closed trades (`argos`, played on bars at `temps`, `tf` seconds each) with TradingView's
/// `liste`. `zone`: the chart's time zone (its offset from UTC at a UTC time, summer time included), tried first for
/// the file's; `tick`: the smallest price step shown (two prices nearer than half of it are the same).
pub fn comparer(argos: &[Ligne], liste: &ListeTv, temps: &[i64], tf: i64, zone: &dyn Fn(i64) -> i64, tick: f64) -> Comparaison {
    let tf = tf.max(1);
    let meme_t = |a: i64, b: i64| (a - b).abs() * 2 < tf;
    // a time of the file in UTC: in the chart's zone (its summer time too), or a fixed offset
    let en_utc = |l: i64, f: Option<i64>| match f {
        Some(d) => l - d,
        None => l - zone(l - zone(l)),
    };
    // the file's time zone: the one that puts the most entries on the backtest's (the chart's first, then UTC, then
    // every quarter of an hour from −12 h to +14 h all year)
    let essais = [None, Some(0)].into_iter().chain((-48..=56).map(|q| Some(q * 900)));
    // (the most entries on one, then the nearest: a quarter of an hour off still falls in an hour bar)
    let score = |f: Option<i64>| {
        liste.trades.iter().fold((0usize, 0i64), |(n, ecart), t| {
            let u = en_utc(t.t_in, f);
            match argos.iter().filter(|a| a.side == t.side && meme_t(a.t_in, u)).map(|a| (a.t_in - u).abs()).min() {
                Some(e) => (n + 1, ecart + e),
                None => (n, ecart),
            }
        })
    };
    let mut fuseau = None;
    let mut meilleur = (0usize, 0i64);
    for f in essais {
        let s = score(f);
        if s.0 > meilleur.0 || (s.0 == meilleur.0 && s.0 > 0 && s.1 < meilleur.1) {
            (fuseau, meilleur) = (f, s);
        }
    }
    let tv: Vec<TradeTv> = liste
        .trades
        .iter()
        .map(|t| TradeTv {
            t_in: en_utc(t.t_in, fuseau),
            t_out: en_utc(t.t_out, fuseau),
            ..t.clone()
        })
        .collect();
    // the period both cover: the chart's bars and the file's trades
    let (Some(&b0), Some(&b1)) = (temps.first(), temps.last()) else {
        return Comparaison {
            fuseau,
            ouverts_tv: liste.ouverts,
            ..Default::default()
        };
    };
    let b1 = b1 + tf.max(1) - 1; // (the last bar lasts until the next one)
    let debut = tv.iter().map(|t| t.t_in).min().map_or(b0, |t| t.max(b0));
    let fin = tv.iter().map(|t| t.t_out).max().map_or(b1, |t| t.min(b1));
    let dedans = |t_in: i64, t_out: i64| t_in >= debut && t_out <= fin;
    let mut out = Comparaison {
        fuseau,
        ouverts_tv: liste.ouverts,
        ..Default::default()
    };
    let a_voir: Vec<usize> = (0..argos.len()).filter(|&k| dedans(argos[k].t_in, argos[k].t_out)).collect();
    let tv_voir: Vec<usize> = (0..tv.len()).filter(|&k| dedans(tv[k].t_in, tv[k].t_out)).collect();
    out.hors_periode = argos.len() - a_voir.len() + tv.len() - tv_voir.len();
    // pairs: the same side and the same entry bar first, then the nearest entry within 3 bars
    let mut pris_a = vec![false; argos.len()];
    let mut paire: Vec<Option<usize>> = vec![None; tv.len()];
    for &j in &tv_voir {
        if let Some(&k) = a_voir
            .iter()
            .find(|&&k| !pris_a[k] && argos[k].side == tv[j].side && meme_t(argos[k].t_in, tv[j].t_in))
        {
            (pris_a[k], paire[j]) = (true, Some(k));
        }
    }
    for &j in &tv_voir {
        if paire[j].is_some() {
            continue;
        }
        let proche = a_voir
            .iter()
            .filter(|&&k| !pris_a[k] && argos[k].side == tv[j].side && (argos[k].t_in - tv[j].t_in).abs() <= 3 * tf)
            .min_by_key(|&&k| (argos[k].t_in - tv[j].t_in).abs());
        if let Some(&k) = proche {
            (pris_a[k], paire[j]) = (true, Some(k));
        }
    }
    let barres = |d: i64| (d as f64 / tf as f64).round() as i64;
    let prix_pareil = |a: f64, b: f64| (a - b).abs() <= (tick / 2.0).max(a.abs() * 1e-9) + 1e-12;
    for &j in &tv_voir {
        let t = &tv[j];
        let Some(k) = paire[j] else {
            out.rangees.push(Rangee {
                argos: None,
                tv: Some(t.clone()),
                ecarts: vec![Ecart::PasChezArgos],
            });
            continue;
        };
        let a = &argos[k];
        let mut e = vec![];
        let (de, ds) = (barres(t.t_in - a.t_in), barres(t.t_out - a.t_out));
        if de != 0 {
            e.push(Ecart::Entree(de));
        }
        if ds != 0 {
            e.push(Ecart::Sortie(ds));
        }
        if de == 0 && !prix_pareil(a.prix_in, t.prix_in) {
            e.push(Ecart::PrixEntree(t.prix_in - a.prix_in));
        }
        if ds == 0 && !prix_pareil(a.prix_out, t.prix_out) {
            e.push(Ecart::PrixSortie(t.prix_out - a.prix_out));
        }
        if let Some(n) = t.net {
            let d = n - a.net.to_f64();
            if d.abs() > RESULTAT_PRES + 1e-6 {
                e.push(Ecart::Resultat(d));
            }
        }
        out.rangees.push(Rangee {
            argos: Some(k),
            tv: Some(t.clone()),
            ecarts: e,
        });
    }
    for &k in a_voir.iter().filter(|&&k| !pris_a[k]) {
        out.rangees.push(Rangee {
            argos: Some(k),
            tv: None,
            ecarts: vec![Ecart::PasChezTv],
        });
    }
    let quand = |r: &Rangee| r.argos.map(|k| argos[k].t_in).or(r.tv.as_ref().map(|t| t.t_in)).unwrap_or(0);
    out.rangees.sort_by_key(quand);
    out.pareils = out.rangees.iter().filter(|r| r.pareil()).count();
    out.differents = out.rangees.len() - out.pareils;
    out
}

#[cfg(test)]
mod tests {
    use super::super::rapport::{csv, lignes};
    use super::*;
    use argos_core::Bar;
    use std::collections::HashMap;

    /// 100 bars of one hour from 2026-11-02 00:00 UTC that go up and down (a long, a short, a long… on the crossings).
    fn barres() -> Vec<Bar> {
        (0..100)
            .map(|i| {
                let c = 100.0 + 10.0 * ((i as f64) / 3.0).sin();
                Bar {
                    time: 1_793_577_600 + i * 3600,
                    open: c - 0.5,
                    high: c + 1.0,
                    low: c - 1.0,
                    close: c,
                    volume: 1.0,
                }
            })
            .collect()
    }

    fn rapport_de(b: &[Bar]) -> Vec<Ligne> {
        let src = r#"//@version=6
strategy("Croise", overlay = true)
m = ta.sma(close, 5)
if ta.crossover(close, m)
    strategy.entry("L", strategy.long)
if ta.crossunder(close, m)
    strategy.entry("S", strategy.short)
"#;
        let o = super::super::run(src, b, 3600, &HashMap::new()).unwrap();
        lignes(&o.strategie.unwrap().trades, b)
    }

    /// The backtest's own export (TradingView's columns) read back: every trade the same, in any time zone of the file;
    /// one exit a bar earlier, one trade missing on each side, one result changed: each says so.
    #[test]
    fn la_liste_de_tradingview_contre_le_backtest() {
        let b = barres();
        let l = rapport_de(&b);
        assert!(l.len() >= 4, "{}", l.len());
        let temps: Vec<i64> = b.iter().map(|x| x.time).collect();
        // the same list, written in New York time (UTC − 5) while the chart is in UTC: found from the trades
        let ny = -5 * 3600;
        let liste = lire_csv(&csv(&l, 1_000_000.0, "USD", ny)).unwrap();
        assert_eq!(liste.trades.len(), l.len());
        assert_eq!(liste.trades[0].signal_in, l[0].entree);
        let utc = |_| 0;
        let c = comparer(&l, &liste, &temps, 3600, &utc, 0.01);
        assert_eq!(c.fuseau, Some(ny));
        assert_eq!((c.pareils, c.differents, c.hors_periode), (l.len(), 0, 0));
        // the file in the chart's zone, with a summer time that ends in the middle of the trades: found as the chart's
        // (between two trades: the hour after it is written twice, as in autumn, and no trade is there)
        let bascule = l[2].t_in - 4 * 3600;
        assert!(l[1].t_in < bascule);
        let zone = move |t: i64| if t < bascule { -4 * 3600 } else { -5 * 3600 };
        let mut ete = liste.clone();
        for t in ete.trades.iter_mut() {
            t.t_in += 3600 * i64::from(t.t_in - ny < bascule);
            t.t_out += 3600 * i64::from(t.t_out - ny < bascule);
        }
        let c = comparer(&l, &ete, &temps, 3600, &zone, 0.01);
        assert_eq!((c.fuseau, c.differents), (None, 0), "{:?}", c.rangees.iter().find(|r| !r.pareil()));
        // TradingView: the 2nd trade leaves a bar earlier (its result changes with it), the 3rd is missing
        let mut t2 = liste.clone();
        t2.trades[1].t_out -= 3600;
        t2.trades[1].prix_out = b[l[1].bar_out - 1].open;
        let net = t2.trades[1].net.unwrap();
        t2.trades[1].net = Some(net + 5.0);
        t2.trades.remove(2);
        let c = comparer(&l, &t2, &temps, 3600, &utc, 0.01);
        assert_eq!(c.fuseau, Some(ny));
        assert_eq!(c.differents, 2, "{:?}", c.rangees);
        let r1 = c.rangees.iter().find(|r| r.argos == Some(1)).unwrap();
        assert_eq!(r1.ecarts[0], Ecart::Sortie(-1));
        assert!(
            matches!(r1.ecarts.last(), Some(Ecart::Resultat(d)) if (d - 5.0).abs() < 1e-9),
            "{:?}",
            r1.ecarts
        );
        let r2 = c.rangees.iter().find(|r| r.argos == Some(2)).unwrap();
        assert_eq!((r2.tv.as_ref(), r2.ecarts.as_slice()), (None, [Ecart::PasChezTv].as_slice()));
        // a trade only TradingView has, inside the period
        let mut t3 = liste.clone();
        let mut x = t3.trades[1].clone();
        (x.t_in, x.t_out) = (x.t_in + 1800 * 3, x.t_out);
        x.side = if x.side == Side::Long { Side::Short } else { Side::Long };
        t3.trades.push(x);
        let c = comparer(&l, &t3, &temps, 3600, &utc, 0.01);
        assert!(c.rangees.iter().any(|r| r.argos.is_none() && r.ecarts == [Ecart::PasChezArgos]));
        // a price off by less than half a tick is the same; by more, it is said
        let mut t4 = liste.clone();
        t4.trades[0].prix_in += 0.004;
        t4.trades[0].net = None;
        assert_eq!(comparer(&l, &t4, &temps, 3600, &utc, 0.01).differents, 0);
        t4.trades[0].prix_in += 0.5;
        let c = comparer(&l, &t4, &temps, 3600, &utc, 0.01);
        assert!(
            matches!(c.rangees[0].ecarts[0], Ecart::PrixEntree(d) if (d - 0.504).abs() < 1e-6),
            "{:?}",
            c.rangees[0]
        );
        // the chart starts later than the file: the trades before it are not compared
        let c = comparer(&l, &liste, &temps[l[1].bar_in..], 3600, &utc, 0.01);
        assert!(c.hors_periode >= 2, "{}", c.hors_periode);
        assert_eq!(c.differents, 0);
    }

    /// The file as TradingView writes it in other years and languages: its new columns (« Net P&L », « Position size
    /// (qty) »), a French file with semicolons and decimal commas, an open trade, an Excel file, another file.
    #[test]
    fn fichiers_de_tradingview() {
        let neuf = "\u{feff}Trade #,Type,Date/Time,Signal,Price USDT,Position size (qty),Position size (value),Net P&L USDT,Net P&L %,Cumulative P&L USDT\n\
            2,Exit short,2026-11-20 15:00,Open,61111.0,1,61111,-120.5,-0.2,300\n\
            2,Entry short,2026-11-19 10:00,RSI_BB_S,60990.5,1,60990.5,-120.5,-0.2,300\n\
            1,Exit long,2026-11-19 10:00,RSI_BB_S,60990.5,1,60990.5,\"1,420.5\",2.4,420.5\n\
            1,Entry long,2026-11-18 05:00,RSI_BB_L,59570,1,59570,\"1,420.5\",2.4,420.5\n";
        let l = lire_csv(neuf).unwrap();
        assert_eq!((l.trades.len(), l.ouverts), (1, 1));
        let t = &l.trades[0];
        assert_eq!(
            (t.numero, t.side, t.signal_in.as_str(), t.signal_out.as_str()),
            (1, Side::Long, "RSI_BB_L", "RSI_BB_S")
        );
        assert_eq!((t.prix_in, t.prix_out, t.qty, t.net), (59570.0, 60990.5, Some(1.0), Some(1420.5)));
        assert_eq!(t.t_in, 1_794_978_000); // 2026-11-18 05:00
        let fr = "N° de transaction;Type;Signal;Date/Heure;Prix USD;Contrats;Profit USD;Profit %\n\
            1;Sortie long;Vente;19/11/2026 10:00;60 990,5;2;2 841;2,4\n\
            1;Entrée long;Achat;18/11/2026 05:00;59 570;2;2 841;2,4\n";
        let l = lire_csv(fr).unwrap();
        let t = &l.trades[0];
        assert_eq!(
            (t.side, t.prix_in, t.prix_out, t.qty, t.net),
            (Side::Long, 59570.0, 60990.5, Some(2.0), Some(2841.0))
        );
        assert_eq!(t.t_in, 1_794_978_000);
        // TradingView 2026 in French: « Entrer short », « Sortir du long », the open trade « Ouverture » with no price
        let fr26 = "\u{feff}Numéro de trade,Type,Date et heure,Signal,Prix USDC,Taille (qté),Taille (valeur),P&L net USDC,Retour %,\
            Commission USDC,Excursion favorable USDC,Excursion favorable %,Excursion adverse USDC,Excursion adverse %,\
            P&L cumulé USDC,P&L cumulé %,Durée (barres)\n\
            1,Sortir du short,2026-06-23 04:00,RSI_BB_L,101883,1,93690,-8193,-8.74,0,937,1.00,-18434,-19.68,-8193,-8.19,338\n\
            1,Entrer short,2026-04-27 20:00,RSI_BB_S,93690,1,93690,-8193,-8.74,0,937,1.00,-18434,-19.68,-8193,-8.19,338\n\
            2,Sortir du long,Ouverture,Ouverture,—,1,101883,-2467,-3.90,0,24162,38.17,-8139,-12.86,-10660,-10.66,1008\n\
            2,Entrer long,2026-06-23 04:00,RSI_BB_L,101883,1,101883,-2467,-3.90,0,24162,38.17,-8139,-12.86,-10660,-10.66,1008\n";
        let l = lire_csv(fr26).unwrap();
        assert_eq!((l.trades.len(), l.ouverts), (1, 1));
        let t = &l.trades[0];
        assert_eq!(
            (t.side, t.signal_out.as_str(), t.prix_in, t.prix_out, t.qty, t.net),
            (Side::Short, "RSI_BB_L", 93690.0, 101883.0, Some(1.0), Some(-8193.0))
        );
        assert!(lire_csv("PK\u{3}\u{4}...").unwrap_err().contains("Excel"));
        assert!(lire_csv("Date,Open,High,Low,Close\n2026-11-18,1,2,0,1\n")
            .unwrap_err()
            .contains("liste des trades"));
        assert!(lire_csv("").is_err());
    }
}
