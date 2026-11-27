// 🕯️ Essai réel des bougies (make essai-bougies) : pour chaque connecteur d'Argos, ~50 bougies 1 h de la paire
// principale, par la couche de données d'Argos (argos-data), endpoints publics, sans clé, en lecture seule.
// Ce fichier est inclus tel quel par deux petits programmes : l'app native (crates/argos-data/examples) et la
// coque Tauri (argos-tauri/examples), pour essayer le même code avec les dépendances de chacune.

use argos_core::feed::Feed;
use argos_core::{Bar, Timeframe};
use argos_data::{Exchange, ExchangeFeed};
use std::time::Instant;

/// Une ligne du tableau : (source, ok, bougies, dernière bougie UTC, latence ms, cache, erreur).
type Ligne = (String, bool, usize, String, u128, String, String);

/// Passe les bougies par le cache d'Argos (dossier temporaire) et vérifie le fichier : droits 600, empreinte bonne,
/// dernière bougie fermée :
/// « fermée (−1 en cours) » = la bougie en cours n'a pas été écrite.
fn essai_cache(nom: &str, bars: &[Bar]) -> Result<String, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let dir = std::env::temp_dir().join(format!("argos-essai-cache-{}", std::process::id()));
    let f = dir.join(format!("{}.json", nom.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>()));
    argos_data::cache::write(&f, bars, Timeframe::H1, now).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // le cache est rangé en morceaux (revue R&D, point 3) : on regarde le morceau écrit
        let c = argos_data::cache::morceaux(&f).first().map(|m| m.1.clone()).ok_or("aucun morceau écrit")?;
        let m = std::fs::metadata(&c).map_err(|e| e.to_string())?.permissions().mode() & 0o777;
        if m != 0o600 {
            return Err(format!("droits du fichier {m:o} au lieu de 600"));
        }
    }
    // empreinte vérifiée à la lecture : un fichier abîmé serait refusé
    let disque = argos_data::cache::read_raw(&f).ok_or("empreinte du fichier refusée")?;
    let _ = std::fs::remove_dir_all(&dir);
    match disque.last() {
        Some(b) if argos_data::cache::is_closed(b, Timeframe::H1.seconds(), now) => {
            let retirees = bars.len() - disque.len();
            Ok(if retirees == 0 {
                "fermée".into()
            } else {
                format!("fermée (−{retirees} en cours)")
            })
        }
        Some(_) => Err("bougie en cours écrite dans le cache !".into()),
        None if bars.len() <= 1 => Ok("vide (seulement la bougie en cours)".into()),
        None => Err("cache vide".into()),
    }
}

/// Heure UTC « AAAA-MM-JJ HH:MM » sans dépendance de plus.
fn heure(t: i64) -> String {
    let (d, s) = (t.div_euclid(86_400), t.rem_euclid(86_400));
    // jours depuis 1970 → date (algorithme civil de Howard Hinnant)
    let z = d + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}", s / 3600, s % 3600 / 60)
}

fn ligne(nom: &str, debut: Instant, r: Result<Vec<Bar>, String>) -> Ligne {
    let ms = debut.elapsed().as_millis();
    match r {
        Ok(b) if !b.is_empty() => {
            let last = heure(b.last().map_or(0, |x| x.time));
            match essai_cache(nom, &b) {
                Ok(c) => (nom.into(), true, b.len(), last, ms, c, String::new()),
                Err(e) => (nom.into(), false, b.len(), last, ms, "ÉCHEC".into(), e),
            }
        }
        Ok(_) => (nom.into(), false, 0, "—".into(), ms, "—".into(), "aucune bougie".into()),
        Err(e) => (nom.into(), false, 0, "—".into(), ms, "—".into(), e.chars().take(110).collect()),
    }
}

async fn bougies(ex: Exchange, base: &str) -> Result<Vec<Bar>, String> {
    ExchangeFeed::new(ex, argos_data::rt::Client::default())
        .candles(base, Timeframe::H1, 50)
        .await
        .map_err(|e| e.to_string())
}

async fn essai() -> Vec<Ligne> {
    let mut out = vec![];
    for ex in Exchange::ALL {
        let t = Instant::now();
        out.push(ligne(ex.name(), t, bougies(ex, "BTC").await));
    }
    let t = Instant::now();
    out.push(ligne("Hyperliquid HIP-3 (xyz:GOLD)", t, bougies(Exchange::Hyperliquid, "xyz:GOLD").await));
    for sym in ["XAUGBP", "XAUEUR", "XAGEUR"] {
        let t = Instant::now();
        let r = match argos_data::synth::legs(sym) {
            Some((a, b)) => match (bougies(Exchange::Hyperliquid, a).await, bougies(Exchange::Hyperliquid, b).await) {
                (Ok(x), Ok(y)) => Ok(argos_data::tv::ratio_bars(&x, &y)),
                (Err(e), _) => Err(format!("{a} : {e}")),
                (_, Err(e)) => Err(format!("{b} : {e}")),
            },
            None => Err("symbole inconnu".into()),
        };
        out.push(ligne(&format!("{sym} (calculé, HIP-3)"), t, r));
    }
    let t = Instant::now();
    out.push(ligne("DXY (Yahoo)", t, argos_data::synth::dxy_yahoo(3600).await.map_err(|e| e.to_string())));
    let t = Instant::now();
    out.push(ligne(
        "DXY (calculé, Kraken)",
        t,
        argos_data::synth::dxy_calc(3600).await.map_err(|e| e.to_string()),
    ));
    out
}

/// ⚡ Étape 2b : temps avant le premier écran et avant la 1re bougie du direct, sans cache. Avant : le direct
/// attendait les 5000 bougies ; après : il attend seulement le premier écran (500 bougies).
fn premier_ecran(rt: &tokio::runtime::Runtime) {
    use std::sync::mpsc;
    println!("\n| Premier écran sans cache | 5000 bougies (avant) | 500 bougies (après) | 1re bougie du direct | Direct visible : avant → après |");
    println!("|---|---|---|---|---|");
    for ex in [Exchange::Hyperliquid, Exchange::Binance, Exchange::Okx, Exchange::Bybit, Exchange::Bitget] {
        let t = Instant::now();
        let n5000 = rt.block_on(bougies_n(ex, "BTC", 5000)).map(|b| b.len()).unwrap_or(0);
        let avant = t.elapsed().as_millis();
        let t = Instant::now();
        let n500 = rt.block_on(bougies_n(ex, "BTC", 500)).map(|b| b.len()).unwrap_or(0);
        let apres = t.elapsed().as_millis();
        // le direct : temps jusqu'à la 1re bougie reçue (15 s au plus)
        let direct = argos_data::live::subscription(ex, "BTC", Timeframe::H1).and_then(|sub| {
            let (tx, rx) = mpsc::channel();
            let t = Instant::now();
            let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
            let s2 = stop.clone();
            argos_data::live::spawn(
                sub,
                move |m| argos_data::live::parse_event(ex, m),
                move |ev| {
                    if let argos_data::live::LiveEvent::Bar(_) = ev {
                        let _ = tx.send(t.elapsed().as_millis());
                        s2.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                    !s2.load(std::sync::atomic::Ordering::SeqCst)
                },
            );
            rx.recv_timeout(std::time::Duration::from_secs(15)).ok()
        });
        let d = direct.map_or("—".to_string(), |ms| format!("{ms} ms"));
        let (va, vb) = (direct.map_or(avant, |ms| ms.max(avant)), direct.map_or(apres, |ms| ms.max(apres)));
        println!("| {} | {avant} ms ({n5000}) | {apres} ms ({n500}) | {d} | {va} ms → {vb} ms |", ex.name());
    }
}

async fn bougies_n(ex: Exchange, base: &str, n: usize) -> Result<Vec<Bar>, String> {
    ExchangeFeed::new(ex, argos_data::rt::Client::default())
        .candles(base, Timeframe::H1, n)
        .await
        .map_err(|e| e.to_string())
}

/// 🕳️ Étape 3 : un trou créé exprès dans un cache temporaire (5 bougies retirées au milieu), puis réparé par
/// l'exchange ; les bougies retrouvées doivent être les mêmes.
fn trou_expres(rt: &tokio::runtime::Runtime, ex: Exchange) -> String {
    use argos_data::trous;
    let Ok(vraies) = rt.block_on(bougies(ex, "BTC")) else {
        return "ÉCHEC (pas de bougies)".into();
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let fermees = argos_data::cache::closed(&vraies, 3600, now).to_vec();
    let troue: Vec<Bar> = fermees.iter().enumerate().filter(|(k, _)| !(20..25).contains(k)).map(|(_, b)| *b).collect();
    let dir = std::env::temp_dir().join(format!("argos-essai-trou-{}", std::process::id()));
    let f = dir.join("trou.json");
    let _ = argos_data::cache::write(&f, &troue, Timeframe::H1, now);
    let relu = argos_data::cache::read(&f, Timeframe::H1, now).unwrap_or_default();
    let _ = std::fs::remove_dir_all(&dir);
    let g = trous::trouver(&relu, Timeframe::H1, &[], trous::Calendrier::Continu, &[]);
    let feed = ExchangeFeed::new(ex, argos_data::rt::Client::default());
    let r = rt.block_on(trous::reparer(&feed, "BTC", Timeframe::H1, &g, 10));
    let tout = trous::combler(&relu, &r.bars);
    let pareil = tout.len() == fermees.len()
        && tout
            .iter()
            .zip(&fermees)
            .all(|(a, b)| a.time == b.time && (a.close - b.close).abs() <= b.close * 1e-9);
    format!(
        "{} : {} trou(s) trouvé(s), {} bougie(s) retrouvée(s) sur 5, identiques : {}",
        if pareil { "OK" } else { "ÉCHEC" },
        g.len(),
        r.bars.len(),
        if pareil { "oui" } else { "non" }
    )
}

/// 💾 Étape 5 : temps de chargement depuis le cache et taille sur disque, format 1 (texte JSON) → format 2 (binaire).
/// Lecture complète comme dans l'app : fichier lu, empreinte vérifiée, bougies vérifiées (médiane de 7 lectures).
fn mesure_format(rt: &tokio::runtime::Runtime) {
    let mut cas: Vec<(String, Vec<Bar>)> = vec![];
    for (ex, base, tf) in [
        (Exchange::Binance, "BTC", Timeframe::H1),
        (Exchange::Bybit, "ETH", Timeframe::M15),
        (Exchange::Hyperliquid, "SOL", Timeframe::M1),
    ] {
        let b = rt.block_on(async {
            ExchangeFeed::new(ex, argos_data::rt::Client::default())
                .candles(base, tf, 5000)
                .await
                .unwrap_or_default()
        });
        cas.push((format!("{} {base} {tf}", ex.name()), b));
    }
    // le plus gros fichier possible : 200 000 bougies d'une minute (prix inventés, seulement pour la mesure)
    let gros: Vec<Bar> = (1..=200_000i64)
        .map(|k| {
            let p = 60_000.0 + (k % 977) as f64 * 1.37;
            Bar {
                time: 1_700_000_000 + k * 60,
                open: p,
                high: p + 5.5,
                low: p - 4.25,
                close: p + 1.5,
                volume: 12.345 + k as f64,
            }
        })
        .collect();
    cas.push(("200 000 bougies 1m (taille maximale)".into(), gros));
    let dir = std::env::temp_dir().join(format!("argos-essai-format-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let mediane = |f: &std::path::Path| {
        let mut t: Vec<u128> = (0..7)
            .map(|_| {
                let t0 = Instant::now();
                let n = std::fs::read(f).ok().and_then(|d| argos_data::cache::parse_any(&d)).map_or(0, |b| b.len());
                assert!(n > 0);
                t0.elapsed().as_micros()
            })
            .collect();
        t.sort_unstable();
        t[3]
    };
    println!("\n| Cache (lecture complète) | Bougies | Format 1 (texte) | Format 2 (binaire) | Taille 1 → 2 |");
    println!("|---|---|---|---|---|");
    for (nom, bars) in &cas {
        if bars.is_empty() {
            println!("| {nom} | 0 | — | — | — |");
            continue;
        }
        let (f1, f2) = (dir.join("v1.json"), dir.join("v2.bin"));
        let _ = std::fs::write(&f1, argos_data::cache::file_text(bars));
        let _ = std::fs::write(&f2, argos_data::cache::file_bytes(bars));
        let (t1, t2) = (mediane(&f1), mediane(&f2));
        let (s1, s2) = (std::fs::metadata(&f1).map_or(0, |m| m.len()), std::fs::metadata(&f2).map_or(0, |m| m.len()));
        let ms = |us: u128| format!("{:.1} ms", us as f64 / 1000.0);
        println!(
            "| {nom} | {} | {} | {} ({:.0}× plus vite) | {} Ko → {} Ko |",
            bars.len(),
            ms(t1),
            ms(t2),
            t1 as f64 / t2.max(1) as f64,
            s1 / 1024,
            s2 / 1024
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// 🧺 Étape 8 : la collecte complète (celle du cœur de la coque), dans un cache temporaire : premier écran, passé et
/// trous en arrière-plan, puis un 2e lancement qui part du cache.
fn collecte_complete(rt: &tokio::runtime::Runtime) -> String {
    use argos_data::collecte;
    let (ex, base, tf) = (Exchange::Hyperliquid, "BTC", Timeframe::H1);
    let dir = std::env::temp_dir().join(format!("argos-essai-collecte-{}", std::process::id()));
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let http = argos_data::rt::Client::default;
    let t = Instant::now();
    let c = rt.block_on(collecte::premier_ecran(http(), ex, base, tf, Some(&dir), now));
    let t1 = t.elapsed().as_millis();
    let t = Instant::now();
    let tout = rt.block_on(collecte::completer(http(), ex, base, tf, Some(&dir), c.bars.clone(), &c.a_reparer, now));
    let t2 = t.elapsed().as_millis();
    let t = Instant::now();
    let c2 = rt.block_on(collecte::premier_ecran(http(), ex, base, tf, Some(&dir), now));
    let t3 = t.elapsed().as_millis();
    let _ = std::fs::remove_dir_all(&dir);
    let ok = c.bars.len() >= 400 && tout.len() >= 4000 && c2.depuis_cache + 1 >= tout.len() && !c2.hors_ligne;
    format!(
        "{} : 1er écran {} bougies en {t1} ms · passé + trous → {} bougies en {t2} ms · 2e lancement {} bougies ({} du cache) en {t3} ms",
        if ok { "OK" } else { "ÉCHEC" },
        c.bars.len(),
        tout.len(),
        c2.bars.len(),
        c2.depuis_cache
    )
}

/// 🔁 Étape 9 : pour chaque exchange, un 2e passage depuis le cache (temps et bougies), une remontée de 3 pages vers
/// le passé (bougies reçues, pauses de la fiche respectées) et un trou créé exprès, réparé.
fn essai_par_exchange(rt: &tokio::runtime::Runtime) -> (usize, usize) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64);
    let dir = std::env::temp_dir().join(format!("argos-essai-9-{}", std::process::id()));
    println!("\n| Exchange | 2e passage (cache) | 3 pages vers le passé | Pauses | Trou créé exprès |");
    println!("|---|---|---|---|---|");
    let (mut ok, mut total) = (0, 0);
    for ex in Exchange::ALL {
        total += 1;
        let f = ex.fiche();
        // 2e passage : les bougies du 1er passage relues depuis le cache (empreinte, bougies fermées)
        let cache = match rt.block_on(bougies(ex, "BTC")) {
            Ok(b) => {
                let p = dir.join(argos_data::cache::file_name(ex, "BTC", Timeframe::H1));
                let _ = argos_data::cache::write(&p, &b, Timeframe::H1, now);
                let t = Instant::now();
                let lu = argos_data::cache::read(&p, Timeframe::H1, now).map_or(0, |x| x.len());
                let us = t.elapsed().as_micros();
                (lu + 1 >= b.len() && lu > 0, format!("{lu} bougies en {:.2} ms", us as f64 / 1000.0))
            }
            Err(e) => (false, format!("ÉCHEC ({})", e.chars().take(40).collect::<String>())),
        };
        // 3 pages vers le passé, à partir d'il y a 30 jours (sous la limite de chaque API)
        let avant = now - 30 * 86_400;
        let t = Instant::now();
        let r = rt.block_on(async {
            ExchangeFeed::new(ex, argos_data::rt::Client::default())
                .older("BTC", Timeframe::H1, avant, 3 * f.page.min(500))
                .await
        });
        let ms = t.elapsed().as_millis() as u64;
        let (pages_ok, pages, pauses) = match &r {
            Ok(b) if b.is_empty() => (true, "rien de plus ancien (l'API ne remonte pas)".to_string(), "—".to_string()),
            Ok(b) => {
                let n = b.len().div_ceil(f.page.max(1)).max(1);
                let attendu = (n as u64 - 1) * f.pause_ms;
                let bien = b.windows(2).all(|w| w[0].time < w[1].time) && b.last().is_some_and(|x| x.time < avant);
                (
                    bien,
                    format!("{} bougies, {n} page(s), en {ms} ms", b.len()),
                    if ms >= attendu {
                        format!("OK (≥ {attendu} ms)")
                    } else {
                        format!("TROP VITE ({ms} < {attendu} ms)")
                    },
                )
            }
            Err(e) => (false, format!("ÉCHEC ({})", e.to_string().chars().take(40).collect::<String>()), "—".into()),
        };
        let trou = trou_expres(rt, ex);
        let tout = cache.0 && pages_ok && trou.starts_with("OK") && !pauses.starts_with("TROP");
        ok += usize::from(tout);
        println!(
            "| {} | {} | {pages} | {pauses} | {} |",
            ex.name(),
            cache.1,
            trou.split(" : ").next().unwrap_or_default()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
    (ok, total)
}

fn main() {
    let ou = std::env::args().nth(1).unwrap_or_else(|| "?".into());
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
    let lignes = rt.block_on(essai());
    println!("| Source ({ou}) | Résultat | Bougies | Dernière bougie (UTC) | Latence | Cache (dernière) | Erreur |");
    println!("|---|---|---|---|---|---|---|");
    for (n, ok, c, h, ms, k, e) in &lignes {
        println!("| {n} | {} | {c} | {h} | {ms} ms | {k} | {e} |", if *ok { "OK" } else { "ÉCHEC" });
    }
    println!("| OANDA | ignoré | — | — | — | — | sans jeton, ignoré (l'essai ne lit jamais les clés) |");
    let ko = lignes.iter().filter(|l| !l.1).count();
    println!("\n{} OK, {ko} échec(s) sur {}", lignes.len() - ko, lignes.len());
    premier_ecran(&rt);
    println!(
        "\n🕳️ Trou créé exprès (Binance BTC 1 h, cache temporaire) : {}",
        trou_expres(&rt, Exchange::Binance)
    );
    println!("🕳️ Trou créé exprès (OKX BTC 1 h, cache temporaire) : {}", trou_expres(&rt, Exchange::Okx));
    // l'essai par exchange (étape 9) une seule fois : depuis l'app native (la coque utilise le même code)
    if ou == "app native" {
        let (ok, total) = essai_par_exchange(&rt);
        println!("\n🔁 Essai par exchange : {ok} OK sur {total}");
    }
    mesure_format(&rt);
    println!("\n🧺 Collecte complète (Hyperliquid BTC 1 h, cache temporaire) : {}", collecte_complete(&rt));
}
