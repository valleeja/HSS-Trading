use super::*;
use crate::num::{fix, to_dec};
use rust_decimal::Decimal;

/// Minuit UTC (le 14 nov. 2023), pour que les jours tombent juste.
const T0: i64 = 1_699_920_000;
const H1: i64 = 3_600;

/// Bougies 1 h déterministes : `f(i)` donne la clôture, l'ouverture est la clôture d'avant.
fn bars(n: usize, f: impl Fn(usize) -> f64) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let c = f(i);
            let o = if i == 0 { c } else { f(i - 1) };
            Bar {
                time: T0 + i as i64 * H1,
                open: o,
                high: o.max(c) + 0.4,
                low: o.min(c) - 0.4,
                close: c,
                volume: 100.0 + (i % 7) as f64 * 15.0,
            }
        })
        .collect()
}

fn hausse(n: usize) -> Vec<Bar> {
    bars(n, |i| 100.0 + i as f64 * 0.5 + (i as f64 / 3.0).sin() * 2.0)
}

/// Monte puis descend : plusieurs régimes.
fn vague(n: usize) -> Vec<Bar> {
    bars(n, |i| 200.0 + (i as f64 / 40.0).sin() * 30.0 + (i as f64 / 3.0).sin() * 1.5)
}

/// `as_of` où toutes les bougies sont clôturées.
fn fin(b: &[Bar]) -> i64 {
    b.last().map_or(T0, |x| x.time + H1)
}

fn lens(config: Config) -> Lens {
    Lens::new(Symbol::XauUsd, Timeframe::H1, config).unwrap()
}

fn run(b: &[Bar], config: Config) -> Vec<Packet> {
    lens(config)
        .update(&Input {
            bars: b,
            htf_bars: None,
            as_of: fin(b),
        })
        .unwrap()
}

fn json(ps: &[Packet]) -> String {
    ps.iter().map(|p| p.to_json() + "\n").collect()
}

#[test]
fn determinisme_octet_pour_octet() {
    let b = vague(600);
    let config = Config {
        bar_close: true,
        ..Config::default()
    };
    let (a, c) = (run(&b, config), run(&b, config));
    assert!(!a.is_empty());
    assert_eq!(json(&a), json(&c));

    // même historique donné bougie par bougie : mêmes paquets (toutes les séries sont causales)
    let mut l = lens(config);
    let mut pas_a_pas = Vec::new();
    for k in 1..=b.len() {
        pas_a_pas.extend(
            l.update(&Input {
                bars: &b[..k],
                htf_bars: None,
                as_of: fin(&b[..k]),
            })
            .unwrap(),
        );
    }
    assert_eq!(json(&a), json(&pas_a_pas));
}

#[test]
fn flux_valide_et_evenements() {
    let b = vague(600);
    let ps = run(&b, Config::default());
    let mut v = Verifier::new();
    for p in &ps {
        v.check(p).unwrap();
    }
    assert!(ps.iter().all(|p| p.event != EventKind::BarClose), "BAR_CLOSE éteint par défaut");
    let changes: Vec<_> = ps.iter().filter(|p| p.event == EventKind::RegimeChange).collect();
    assert!(changes.len() >= 2);
    assert_eq!(changes[0].prev_regime, None);
    for w in changes.windows(2) {
        assert_eq!(w[1].prev_regime, w[0].data.regime);
        assert_ne!(w[1].prev_regime, w[1].data.regime);
    }
    assert!(ps.iter().any(|p| p.event == EventKind::Setup));
    assert!(ps.windows(2).all(|w| w[0].seq + 1 == w[1].seq && w[1].prev == w[0].hash));
    assert_eq!((ps[0].seq, ps[0].prev), (1, GENESIS));

    let with_close = run(
        &b,
        Config {
            bar_close: true,
            ..Config::default()
        },
    );
    assert_eq!(with_close.iter().filter(|p| p.event == EventKind::BarClose).count(), b.len());
}

#[test]
fn chaine_cassee() {
    let ps = run(&vague(600), Config::default());
    assert!(ps.len() >= 3);

    // contenu modifié
    let mut v = Verifier::new();
    v.check(&ps[0]).unwrap();
    let mut x = ps[1].clone();
    x.data.score_long = x.data.score_long.wrapping_add(1);
    assert_eq!(v.check(&x), Err(ChainError::BadHash { seq: 2 }));
    assert_eq!(v.check(&ps[1]), Err(ChainError::Stopped), "flux arrêté");

    // prev qui ne suit pas (même recalculé)
    let mut v = Verifier::new();
    v.check(&ps[0]).unwrap();
    let mut x = ps[1].clone();
    x.prev[0] ^= 1;
    x.hash = x.digest();
    assert_eq!(v.check(&x), Err(ChainError::Broken { seq: 2 }));

    // premier paquet qui ne part pas de zéro
    let mut v = Verifier::new();
    assert_eq!(v.check(&ps[1]), Err(ChainError::Gap { expected: 1, got: 2 }));
}

#[test]
fn trou_de_seq_et_doublon() {
    let ps = run(&vague(600), Config::default());
    let mut v = Verifier::new();
    v.check(&ps[0]).unwrap();
    assert_eq!(v.check(&ps[2]), Err(ChainError::Gap { expected: 2, got: 3 }));
    assert_eq!(v.check(&ps[1]), Err(ChainError::Stopped));

    let mut v = Verifier::new();
    v.check(&ps[0]).unwrap();
    v.check(&ps[1]).unwrap();
    assert_eq!(v.check(&ps[1]), Err(ChainError::Duplicate { seq: 2 }));
    assert!(ChainError::Duplicate { seq: 2 }.to_string().contains("arrêté"));
}

#[test]
fn symbole_hors_liste() {
    assert_eq!(Symbol::parse("XAUUSD"), Ok(Symbol::XauUsd));
    for s in ["xauusd", "XAUUSD ", "XAU/USD", "", "BTCUSDT\"}", "ABCDEF"] {
        assert_eq!(Symbol::parse(s), Err(LensError::SymbolNotAllowed), "{s:?}");
    }
    assert!(Symbol::ALL.iter().all(|&s| Symbol::parse(s.id()) == Ok(s)));
}

#[test]
fn donnees_vides_ou_absurdes() {
    let mut l = lens(Config::default());
    let input = |b: &[Bar]| -> Result<Vec<Packet>, LensError> {
        let mut l = lens(Config::default());
        l.update(&Input {
            bars: b,
            htf_bars: None,
            as_of: fin(b),
        })
    };
    assert_eq!(input(&[]), Err(LensError::NoClosedBar));

    let ok = hausse(50);
    let fault = |i: usize, f: fn(&mut Bar), kind: FaultKind| {
        let mut b = ok.clone();
        f(&mut b[i]);
        assert_eq!(input(&b), Err(LensError::Bars(BarFault { index: i, kind })));
    };
    fault(3, |b| b.close = f64::NAN, FaultKind::NotFinite);
    fault(4, |b| b.volume = f64::INFINITY, FaultKind::NotFinite);
    fault(5, |b| b.low = -1.0, FaultKind::NotPositive);
    fault(6, |b| b.volume = -5.0, FaultKind::NegativeVolume);
    fault(7, |b| b.high = b.low - 1.0, FaultKind::Inconsistent);
    fault(8, |b| b.time -= 2 * H1, FaultKind::NotSorted);
    fault(9, |b| b.time -= H1, FaultKind::NotSorted);

    // bougies de l'unité supérieure aussi contrôlées
    let mut htf = argos_core::aggregate(&ok, 14_400);
    htf[1].open = 0.0;
    let r = l.update(&Input {
        bars: &ok,
        htf_bars: Some(&htf),
        as_of: fin(&ok),
    });
    assert_eq!(
        r,
        Err(LensError::HtfBars(BarFault {
            index: 1,
            kind: FaultKind::NotPositive
        }))
    );

    // une erreur ne change rien au flux : la suite repart de seq 1
    let ps = l
        .update(&Input {
            bars: &ok,
            htf_bars: None,
            as_of: fin(&ok),
        })
        .unwrap();
    let mut v = Verifier::new();
    ps.iter().for_each(|p| v.check(p).unwrap());

    // peu de bougies : indicateurs absents (null), aucun setup, pas de panique
    let p = run(
        &hausse(3),
        Config {
            bar_close: true,
            ..Config::default()
        },
    );
    assert_eq!(p.len(), 3);
    assert!(p[2].data.ema200.is_none() && p[2].data.regime.is_none() && p[2].data.setup.is_none());
    assert!(p[2].to_json().contains("\"ema200\":null"));
}

#[test]
fn bougie_non_cloturee_ignoree() {
    let b = hausse(300);
    let config = Config {
        bar_close: true,
        ..Config::default()
    };
    let mut l = lens(config);
    let last = b[b.len() - 1].time;
    // la dernière bougie n'est pas finie (1 s avant sa clôture)
    let ps = l
        .update(&Input {
            bars: &b,
            htf_bars: None,
            as_of: last + H1 - 1,
        })
        .unwrap();
    assert_eq!(ps.iter().filter(|p| p.event == EventKind::BarClose).count(), b.len() - 1);
    assert!(ps.iter().all(|p| p.bar_time < last));
    // même appel : rien de nouveau
    let again = l.update(&Input {
        bars: &b,
        htf_bars: None,
        as_of: last + H1 - 1,
    });
    assert_eq!(again, Ok(vec![]));
    // une fois clôturée, elle sort, et ses valeurs sont celles du calcul complet
    let ps2 = l
        .update(&Input {
            bars: &b,
            htf_bars: None,
            as_of: last + H1,
        })
        .unwrap();
    assert!(!ps2.is_empty() && ps2.iter().all(|p| p.bar_time == last));
    assert_eq!(ps2[0].event, EventKind::BarClose);
    assert_eq!(ps2[0].data, run(&b, config).last().unwrap().data);
}

#[test]
fn decimales_fixes() {
    let s = |x: f64, dp: u32| to_dec(x, dp).map(|d| d.to_string());
    assert_eq!(s(1.5, 2).as_deref(), Some("1.50"));
    assert_eq!(s(2000.0, 8).as_deref(), Some("2000.00000000"));
    assert_eq!(s(0.125, 2).as_deref(), Some("0.12"), "arrondi au pair");
    assert_eq!(s(0.375, 2).as_deref(), Some("0.38"));
    assert_eq!(s(-0.001, 2).as_deref(), Some("0.00"), "jamais -0");
    assert_eq!(s(f64::NAN, 2), None);
    assert_eq!(s(f64::INFINITY, 8), None);
    assert_eq!(s(1e40, 8), None, "hors des bornes d'un décimal");

    let ps = run(
        &hausse(400),
        Config {
            bar_close: true,
            ..Config::default()
        },
    );
    let d = &ps.last().unwrap().data;
    let dp = |x: Option<Decimal>| x.unwrap().scale();
    for x in [
        d.close,
        d.ema20,
        d.ema50,
        d.ema200,
        d.macd,
        d.macd_signal,
        d.macd_hist,
        d.atr14,
        d.vwap,
        d.prev_day_high,
        d.prev_day_low,
    ] {
        assert_eq!(dp(x), 8);
    }
    for x in [d.rsi14, d.plus_di, d.minus_di, d.adx14, d.atr_pct] {
        assert_eq!(dp(x), 2);
    }
    assert_eq!(dp(d.rel_volume), 4);
    let j = ps.last().unwrap().to_json();
    assert!(j.starts_with("{\"v\":\"argos-packet/v1\",\"seq\":"));
    assert!(j.contains(&format!("\"close\":\"{}\"", d.close.unwrap())));
    assert!(!j.contains("e+") && !j.contains("e-") && !j.contains("NaN"));
}

#[test]
fn setup_stop_et_cibles() {
    let ps = run(&hausse(400), Config::default());
    let p = ps.iter().find(|p| p.event == EventKind::Setup).expect("un setup en tendance haussière");
    let s = p.data.setup.unwrap();
    assert_eq!(s.side, Direction::Long);
    assert!(p.data.score_long >= 70);
    assert_eq!(Some(s.entry), p.data.close);
    let risk = fix(p.data.atr14.unwrap() * Decimal::new(15, 1), 8);
    assert_eq!(s.stop, fix(s.entry - risk, 8));
    assert_eq!(s.tp1, fix(s.entry + risk * Decimal::new(15, 1), 8));
    assert_eq!(s.tp2, fix(s.entry + risk * Decimal::from(3), 8));
    for x in [s.entry, s.stop, s.tp1, s.tp2] {
        assert_eq!(x.scale(), 8);
    }
    assert!(p.to_json().contains("\"setup\":{\"side\":\"LONG\",\"entry\":"));
    let [e, st, t1, t2] = s.niveaux_f64();
    assert!(st < e && e < t1 && t1 < t2 && ((t2 - e) - 2.0 * (t1 - e)).abs() < 1e-6);
}

#[test]
fn poids_reglables() {
    let w = |trend, momentum, strength, volume, structure| Config {
        weights: Weights {
            trend,
            momentum,
            strength,
            volume,
            structure,
        },
        bar_close: true,
        ..Config::default()
    };
    assert_eq!(
        Lens::new(Symbol::XauUsd, Timeframe::H1, w(20, 20, 20, 20, 21)).err(),
        Some(LensError::Weights { sum: 101 })
    );
    assert_eq!(
        Lens::new(Symbol::XauUsd, Timeframe::H1, w(0, 0, 0, 0, 0)).err(),
        Some(LensError::Weights { sum: 0 })
    );

    let b = hausse(400);
    let base = run(&b, w(20, 20, 20, 20, 20));
    let trend_only = run(&b, w(100, 0, 0, 0, 0));
    let momentum_only = run(&b, w(0, 100, 0, 0, 0));
    let last = |ps: &[Packet]| ps.last().unwrap().data.clone();
    // tendance haussière nette : tout le facteur tendance pour le long, rien pour le short
    assert_eq!((last(&trend_only).score_long, last(&trend_only).score_short), (100, 0));
    assert!(momentum_only.iter().any(|p| p.data.score_long % 50 == 0));
    // les mêmes indicateurs, d'autres scores
    assert_eq!(last(&base).close, last(&trend_only).close);
    assert_ne!(json(&base), json(&trend_only));
    // les scores sont des multiples du poids sur 4
    assert!(trend_only.iter().all(|p| p.data.score_long % 25 == 0 && p.data.score_short % 25 == 0));
}

#[test]
fn aucun_droit() {
    // le code de Lens (hors tests) n'appelle ni le réseau, ni les fichiers, ni l'horloge, ni de code unsafe
    let sources = [
        include_str!("lib.rs"),
        include_str!("affichage.rs"),
        include_str!("reglages.rs"),
        include_str!("analyse.rs"),
        include_str!("market.rs"),
        include_str!("num.rs"),
        include_str!("packet.rs"),
        include_str!("verify.rs"),
    ];
    let forbidden = [
        ["std::", "fs"],
        ["std::", "net"],
        ["std::", "time"],
        ["std::", "env"],
        ["std::", "process"],
        ["Sys", "temTime"],
        ["Inst", "ant::"],
        ["unsafe", " {"],
    ];
    for src in sources {
        for f in forbidden {
            assert!(!src.contains(&f.concat()), "{}", f.concat());
        }
    }
}

#[test]
fn photos_comme_le_flux() {
    let b = vague(600);
    let config = Config {
        bar_close: true,
        ..Config::default()
    };
    let ph = photos(&b, H1, &config, None).unwrap();
    assert_eq!(ph.len(), b.len());
    let ps = run(&b, config);
    // la photo de chaque bougie = les données de son paquet BAR_CLOSE
    let closes: Vec<_> = ps.iter().filter(|p| p.event == EventKind::BarClose).collect();
    assert!(closes.iter().zip(&ph).all(|(p, s)| p.data == *s));
    // un setup nouveau par paquet SETUP, sur la même bougie
    let setups: Vec<i64> = ps.iter().filter(|p| p.event == EventKind::Setup).map(|p| p.bar_time).collect();
    let nouveaux: Vec<i64> = nouveaux_setups(&ph).iter().zip(&b).filter(|(s, _)| s.is_some()).map(|(_, x)| x.time).collect();
    assert!(!setups.is_empty());
    assert_eq!(setups, nouveaux);
    // n'importe quelle unité en secondes (ici 2 h, absente de la liste fermée des paquets)
    assert_eq!(photos(&b, 7_200, &config, None).unwrap().len(), b.len());
    assert!(photos(&[], H1, &config, None).unwrap().is_empty());
}

#[test]
fn photos_refusees() {
    let b = hausse(50);
    assert_eq!(photos(&b, 0, &Config::default(), None), Err(LensError::Interval));
    let mut w = Config::default();
    w.weights.trend = 0;
    assert_eq!(photos(&b, H1, &w, None), Err(LensError::Weights { sum: 80 }));
    let mut bad = b.clone();
    bad[2].close = f64::NAN;
    assert_eq!(
        photos(&bad, H1, &Config::default(), None),
        Err(LensError::Bars(BarFault {
            index: 2,
            kind: FaultKind::NotFinite
        }))
    );
}

// ---- Argos AI level 1: score components and explanations ----

use crate::explain::{check, explain, Command, Explanation, Tone, Window, MAX_DRAWINGS, REMINDER};

/// The score comes from its components, and only from them, whatever the weights.
#[test]
fn score_components_give_the_scores() {
    let w = Config {
        weights: Weights {
            trend: 40,
            momentum: 10,
            strength: 10,
            volume: 20,
            structure: 20,
        },
        ..Config::default()
    };
    for config in [Config::default(), w] {
        for b in [vague(600), hausse(400)] {
            for p in photos(&b, H1, &config, None).unwrap() {
                assert_eq!(p.for_long.score(config.weights), p.score_long);
                assert_eq!(p.for_short.score(config.weights), p.score_short);
                assert!(p.for_long.quarters().iter().all(|q| *q <= 4));
            }
        }
    }
}

/// All the snapshots of the wave, with their bar times.
fn wave_photos() -> Vec<(i64, Snapshot)> {
    let b = vague(600);
    let p = photos(&b, H1, &Config::default(), None).unwrap();
    b.iter().map(|x| x.time).zip(p).collect()
}

fn ids(e: &Explanation) -> Vec<&'static str> {
    e.blocks.iter().flat_map(|b| &b.sentences).map(|s| s.id).collect()
}

fn whole(e: &Explanation) -> String {
    e.blocks
        .iter()
        .flat_map(|b| &b.sentences)
        .map(|s| s.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

const WIDE: Window = Window {
    time_from: i64::MIN,
    time_to: i64::MAX,
    price_low: Decimal::MIN,
    price_high: Decimal::MAX,
};

/// With a setup (Long and Short): three blocks, the setup levels in the text and drawn, every drawing tied to a
/// sentence, the reminder, and no more than the cap.
#[test]
fn explanation_with_a_setup() {
    for side in [Direction::Long, Direction::Short] {
        let (t, s) = wave_photos().into_iter().find(|(_, p)| p.setup.map(|x| x.side) == Some(side)).unwrap();
        let st = s.setup.unwrap();
        let e = explain(&s, &Config::default(), t, H1);
        let titles: Vec<_> = e.blocks.iter().map(|b| b.title).collect();
        assert_eq!(titles, ["Ce que dit la fiche", "Pourquoi", "Ce qui annulerait l'idée"]);
        assert_eq!(e.reminder, REMINDER);
        assert!(REMINDER.contains("PropFirm") && REMINDER.contains("pas un ordre"));
        let text = whole(&e);
        assert!(text.contains(&nombre_fr(st.entry, decimales_prix(st.entry))), "{text}");
        // French as written: « au-dessus de l' », « au-dessus du VWAP », never « de le » or « au-dessus l' »
        assert!(
            !text.contains("de le ") && !text.contains("au-dessus l") && !text.contains("au-dessus 5"),
            "{text}"
        );
        assert!(ids(&e).contains(&"setup") && ids(&e).contains(&"stop") && !ids(&e).contains(&"no_setup"));
        // entry, stop and both targets drawn with their tone
        let levels: Vec<(Decimal, Tone)> = e
            .drawings
            .iter()
            .filter_map(|c| match c {
                Command::Level { price, tone, .. } => Some((*price, *tone)),
                _ => None,
            })
            .collect();
        for l in [(st.entry, Tone::Entry), (st.stop, Tone::Stop), (st.tp1, Tone::Target), (st.tp2, Tone::Target)] {
            assert!(levels.contains(&l), "{l:?}");
        }
        // every drawing is tied to a sentence; context levels only when cited
        assert!(e.drawings.iter().all(|c| ids(&e).contains(&c.sentence())));
        let kept = check(&e, &s, &WIDE);
        assert_eq!(kept, e.drawings, "nothing dropped in a wide window");
        assert!(kept.len() <= MAX_DRAWINGS);
        assert!(kept.iter().any(|c| matches!(c, Command::TimeMark { time, .. } if *time == t)));
        assert!(kept.iter().any(|c| matches!(c, Command::Arrow { time2, .. } if *time2 == t + 5 * H1)));
    }
}

/// Without a setup: the text says so and why, nothing to cancel, only the cited context levels are drawn.
#[test]
fn explanation_without_a_setup() {
    let (t, s) = wave_photos().into_iter().rev().find(|(_, p)| p.setup.is_none() && p.ema200.is_some()).unwrap();
    let e = explain(&s, &Config::default(), t, H1);
    let text = whole(&e);
    assert!(ids(&e).contains(&"no_setup") && ids(&e).contains(&"nothing"), "{text}");
    assert!(text.contains("Pas de setup") && text.contains("au moins 70"));
    assert!(e.drawings.iter().all(|c| matches!(c, Command::Level { tone: Tone::Context, .. })));
    assert!(!e.drawings.is_empty(), "the cited EMA 200 is drawn");
    assert!(e.drawings.iter().all(|c| ids(&e).contains(&c.sentence())));
}

/// A missing value is said, never guessed: the first bars have no EMA, RSI or regime yet.
#[test]
fn missing_values_are_said() {
    let (t, s) = wave_photos().into_iter().next().unwrap();
    let e = explain(&s, &Config::default(), t, H1);
    let text = whole(&e);
    assert!(text.contains("Régime inconnu"), "{text}");
    assert!(text.contains("donnée absente") && text.contains("RSI absent"), "{text}");
    // no setup yet: only the context levels already computed (the VWAP starts at the first bar), each one cited
    assert!(e
        .drawings
        .iter()
        .all(|c| matches!(c, Command::Level { tone: Tone::Context, .. }) && ids(&e).contains(&c.sentence())));
    assert!(!ids(&e).contains(&"ema200"), "no EMA 200 yet, so not cited");
}

/// The check drops a price Lens did not compute, a drawing outside the window, a drawing with no sentence, and keeps
/// at most the cap.
#[test]
fn check_keeps_only_safe_drawings() {
    let (t, s) = wave_photos().into_iter().find(|(_, p)| p.setup.is_some()).unwrap();
    let mut e = explain(&s, &Config::default(), t, H1);
    let st = s.setup.unwrap();
    let invented = st.entry + Decimal::ONE;
    e.drawings.push(Command::Level {
        price: invented,
        label: "Inventé".into(),
        tone: Tone::Target,
        sentence: "setup",
    });
    e.drawings.push(Command::Level {
        price: st.entry,
        label: "Sans phrase".into(),
        tone: Tone::Entry,
        sentence: "nowhere",
    });
    let kept = check(&e, &s, &WIDE);
    assert!(kept.iter().all(|c| !matches!(c, Command::Level { price, .. } if *price == invented)));
    assert!(kept.iter().all(|c| c.sentence() != "nowhere"));
    // a window ending before the setup bar keeps only the plain levels and zones inside its prices
    let narrow = Window { time_to: t - 1, ..WIDE };
    assert!(check(&e, &s, &narrow).iter().all(|c| matches!(c, Command::Level { .. } | Command::Zone { .. })));
    let low = Window {
        price_high: st.entry.min(st.stop) - Decimal::ONE,
        ..WIDE
    };
    assert!(check(&e, &s, &low)
        .iter()
        .all(|c| !matches!(c, Command::Level { price, .. } if *price == st.entry)));
    // the cap
    let level = e.drawings[0].clone();
    e.drawings = vec![level; MAX_DRAWINGS + 5];
    assert_eq!(check(&e, &s, &WIDE).len(), MAX_DRAWINGS);
}

/// 🧪 Real examples on candles from the cache (not run unless `ARGOS_EXEMPLES_LENS` names a candle file): the last
/// Long setup, the last Short setup and the last bar, printed for Jee.
#[test]
fn real_examples_from_the_cache() {
    let Some(path) = std::env::var_os("ARGOS_EXEMPLES_LENS") else { return };
    let secs: i64 = std::env::var("ARGOS_EXEMPLES_SECS").ok().and_then(|s| s.parse().ok()).unwrap_or(H1);
    let b = argos_data::cache::read_raw(std::path::Path::new(&path)).expect("cache file");
    let p = photos(&b, secs, &Config::default(), None).unwrap();
    let all: Vec<(i64, &Snapshot)> = b.iter().map(|x| x.time).zip(&p).collect();
    let last_of = |side: Option<Direction>| all.iter().rev().find(|(_, s)| s.setup.map(|x| x.side) == side).copied();
    for (name, pick) in [
        ("SETUP LONG", last_of(Some(Direction::Long))),
        ("SETUP SHORT", last_of(Some(Direction::Short))),
        ("SANS SETUP", last_of(None)),
    ] {
        let Some((t, s)) = pick else { continue };
        let e = explain(s, &Config::default(), t, secs);
        println!("===== {name} · bougie {t} =====");
        for bl in &e.blocks {
            println!("## {}", bl.title);
            for x in &bl.sentences {
                println!("- {}", x.text);
            }
        }
        println!("{}", e.reminder);
        println!("Dessins : {}", e.drawings.len());
    }
}
