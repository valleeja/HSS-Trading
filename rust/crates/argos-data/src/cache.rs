//! 💾 Cache des bougies entre deux lancements, **par exchange, symbole et unité de temps**.
//! Au démarrage, la chart montre tout de suite les bougies de la dernière session, puis on télécharge seulement ce qui
//! manque depuis. Règles :
//! - le cache ne garde **que des bougies fermées** : la bougie en cours (et le direct) reste seulement en mémoire. Elle
//!   n'est jamais écrite, et une bougie pas encore fermée trouvée dans un vieux fichier est retirée à la lecture ;
//! - les bougies fraîches **remplacent** celles du cache à partir de leur première heure ; s'il y a un trou entre le
//!   cache et les bougies fraîches, on garde seulement les fraîches.
//!
//! - fichier protégé : lisible seulement par toi (droits 0600) et une **empreinte** SHA-256 en tête ; un fichier
//!   abîmé, modifié ou d'un ancien format est **jeté** (puis refait au prochain téléchargement), sans plantage ;
//! - seulement des bougies cohérentes (`bar_ok`) et des nombres : jamais de clé, de jeton ni de texte libre.
//!
//! Une bougie est fermée quand sa fin est passée : début + unité (lundi 00:00 UTC + 7 jours pour une semaine, 1er du
//! mois suivant pour un mois). Même règle pour la chart, les écrans partagés et la coque Tauri (même module).
//! Format **2** (binaire compact, depuis l'étape 5) : `ARGB`, version (1 octet), 3 octets à zéro, nombre de bougies
//! (u64), empreinte SHA-256 du corps (32 octets), puis 48 octets par bougie (temps i64, ouverture, haut, bas, clôture,
//! volume en f64, petit-boutiste). Fichier `…-1h.bin`. L'ancien format 1 (texte : `argos-bougies 1 <SHA-256>` puis du
//! JSON, fichier `…-1h.json`) est relu une fois, puis remplacé au prochain enregistrement ; une autre version est jetée.
//!
//! **Morceaux** (revue R&D, point 3) : chaque série (`hl-btc-1h.bin`) est rangée en morceaux de 10 000 bougies selon
//! l'heure (`hl-btc-1h.m<n>.bin`), chacun avec son en-tête, son empreinte et ses droits 0600. Un enregistrement ne
//! réécrit que les morceaux qui ont changé (en pratique le dernier, quand des bougies s'ajoutent) ; les autres ne sont
//! même pas relus en entier (seulement leurs 48 octets d'en-tête). Un morceau abîmé est supprimé seul (le trou est
//! ensuite redemandé, étape 3). Un ancien fichier d'un seul tenant (format 2 ou 1) est relu une fois puis remplacé.
//!
//! Taille : au plus 200 000 bougies par série (les morceaux plus anciens sont retirés) et 200 Mio pour tout le cache ;
//! au-delà, les fichiers **le moins utilisés** (date de dernière lecture ou écriture) sont supprimés.

use argos_core::{Bar, Timeframe};

/// ⏱️ Marge pour l'horloge du PC (revue R&D, point 2) : une bougie n'est « fermée » que 2 min après sa fin, au cas où
/// l'horloge du PC serait en avance sur celle de l'exchange.
pub const MARGE_HORLOGE_S: i64 = 120;
/// Au lancement suivant, les dernières bougies du cache sont **redemandées** (l'exchange a pu les corriger).
pub const RELUES: usize = 3;

/// Fermée : la fin de la bougie (début + `sec`, ou le 1er du mois suivant pour un mois), plus la marge de l'horloge,
/// est passée à `now_s`.
pub fn is_closed(b: &Bar, sec: i64, now_s: i64) -> bool {
    argos_core::bucket_end(b.time, sec) + MARGE_HORLOGE_S <= now_s
}

/// Les bougies sans celles qui ne sont pas encore fermées (à la fin : la bougie en cours, parfois une d'avance).
pub fn closed(bars: &[Bar], sec: i64, now_s: i64) -> &[Bar] {
    let n = bars.iter().rposition(|b| is_closed(b, sec, now_s)).map_or(0, |i| i + 1);
    &bars[..n]
}

/// Combien de bougies demander pour rattraper le cache : celles écoulées depuis la dernière bougie du cache, plus les
/// `RELUES` dernières du cache (redemandées : l'exchange a pu les corriger) et la bougie en cours. Au plus `max`
/// (= rechargement complet).
pub fn missing(cached: &[Bar], now_s: i64, tf: Timeframe, max: usize) -> usize {
    match cached.last() {
        Some(last) => (((now_s - last.time).max(0) / tf.seconds()) as usize + 1 + RELUES).min(max),
        None => max,
    }
}

/// Cache + bougies fraîches : les fraîches gagnent à partir de leur première heure. Garde les `keep` plus récentes.
pub fn merge(cached: &[Bar], fresh: Vec<Bar>, tf: Timeframe, keep: usize) -> Vec<Bar> {
    let Some(first) = fresh.first().map(|b| b.time) else {
        return cached.to_vec();
    };
    let older: Vec<Bar> = cached.iter().copied().filter(|b| b.time < first).collect();
    // trou entre le cache et les fraîches (ou cache vide) : seulement les fraîches
    let joined = older.last().is_some_and(|b| first - b.time <= tf.seconds());
    let mut out = if joined { older } else { vec![] };
    out.extend(fresh);
    let cut = out.len().saturating_sub(keep);
    out.drain(..cut);
    out
}

pub fn encode(bars: &[Bar]) -> String {
    let rows: Vec<serde_json::Value> = bars
        .iter()
        .map(|b| serde_json::json!([b.time, b.open, b.high, b.low, b.close, b.volume]))
        .collect();
    serde_json::Value::Array(rows).to_string()
}

/// `None` si illisible (on repart d'un téléchargement complet).
pub fn decode(raw: &str) -> Option<Vec<Bar>> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    v.as_array()?
        .iter()
        .map(|r| {
            let n = |i: usize| r.get(i).and_then(|x| x.as_f64());
            Some(Bar {
                time: r.get(0)?.as_i64()?,
                open: n(1)?,
                high: n(2)?,
                low: n(3)?,
                close: n(4)?,
                volume: n(5)?,
            })
        })
        .collect()
}

/// Nom de fichier : « hl-btc-30m.bin ».
pub fn file_name(ex: crate::Exchange, base: &str, tf: Timeframe) -> String {
    let base: String = base.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    format!("{}-{}-{}{}.bin", ex.id(), base.to_lowercase(), tf.id(), revision(ex, tf))
}

/// 🔒 A series whose candles changed meaning gets a new name: what was kept under the old one is never read again
/// (it goes with the least used files), and the candles are asked anew. 24 Nov. 2026 (`-r2`): MEXC, Gate.io, OKX
/// and BloFin give the volume of their perps in coins (it was in contracts), and the 12 hours, days and weeks of
/// Bitget and the 12 hours of OKX begin at 00:00 UTC (they began at 00:00 in Asia).
fn revision(ex: crate::Exchange, tf: Timeframe) -> &'static str {
    use crate::Exchange::*;
    match (ex, tf) {
        (Mexc | Gate | Okx | Blofin, _) => "-r2",
        (Bitget | BitgetSpot, Timeframe::H12 | Timeframe::D1 | Timeframe::W1) => "-r2",
        (OkxSpot, Timeframe::H12) => "-r2",
        _ => "",
    }
}

/// Au plus autant de bougies par série (les plus récentes).
pub const MAX_BOUGIES_FICHIER: usize = 200_000;
/// Bougies par morceau de fichier (un morceau couvre 10 000 unités de temps).
pub const MORCEAU: i64 = 10_000;

/// Les morceaux d'une série (`…/hl-btc-1h.bin` → `…/hl-btc-1h.m<n>.bin`), triés : (numéro, chemin).
pub fn morceaux(path: &std::path::Path) -> Vec<(i64, std::path::PathBuf)> {
    let (Some(dir), Some(stem)) = (path.parent(), path.file_stem().and_then(|s| s.to_str())) else {
        return vec![];
    };
    let debut = format!("{stem}.m");
    let Ok(rd) = std::fs::read_dir(dir) else { return vec![] };
    let mut out: Vec<(i64, std::path::PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().into_string().ok()?;
            let k = n.strip_prefix(&debut)?.strip_suffix(".bin")?.parse::<i64>().ok()?;
            Some((k, e.path()))
        })
        .collect();
    out.sort_by_key(|m| m.0);
    out
}

/// Le chemin du morceau `k` d'une série (`…/hl-btc-1h.bin` → `…/hl-btc-1h.m<k>.bin`).
pub(crate) fn chemin_morceau(path: &std::path::Path, k: i64) -> std::path::PathBuf {
    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("serie");
    path.with_file_name(format!("{stem}.m{k}.bin"))
}
/// Au plus autant d'octets pour tout le cache des bougies.
pub const MAX_OCTETS_CACHE: u64 = 200 * 1024 * 1024;

/// Format 2 : signature, version, taille d'une bougie, taille de l'en-tête.
const MAGIC: &[u8; 4] = b"ARGB";
pub const VERSION: u8 = 2;
const BOUGIE: usize = 48;
const ENTETE: usize = 4 + 4 + 8 + 32;

/// En-tête commun des fichiers binaires d'Argos (bougies, orderflow) : signature, version, 3 octets à zéro, nombre
/// d'éléments (u64), empreinte SHA-256 du corps, puis le corps.
pub(crate) fn emballer(magic: &[u8; 4], version: u8, n: usize, body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(ENTETE + body.len());
    out.extend_from_slice(magic);
    out.extend_from_slice(&[version, 0, 0, 0]);
    out.extend_from_slice(&(n as u64).to_le_bytes());
    out.extend_from_slice(&argos_core::sha256(body));
    out.extend_from_slice(body);
    out
}

/// Le nombre d'éléments et le corps d'un fichier `emballer`, si la signature, la version et l'empreinte sont bonnes.
pub(crate) fn deballer<'a>(data: &'a [u8], magic: &[u8; 4], version: u8) -> Option<(usize, &'a [u8])> {
    if data.len() < ENTETE || &data[..4] != magic || data[4] != version || data[5..8] != [0, 0, 0] {
        return None;
    }
    let n = usize::try_from(u64::from_le_bytes(data[8..16].try_into().ok()?)).ok()?;
    let body = &data[ENTETE..];
    (data[16..48] == argos_core::sha256(body)).then_some((n, body))
}

/// Le contenu binaire (format 2) : en-tête avec l'empreinte, puis les bougies cohérentes, dans l'ordre.
pub fn file_bytes(bars: &[Bar]) -> Vec<u8> {
    let bars = argos_core::feed::clean_bars(bars.to_vec());
    let mut body = Vec::with_capacity(bars.len() * BOUGIE);
    for b in &bars {
        body.extend_from_slice(&b.time.to_le_bytes());
        for v in [b.open, b.high, b.low, b.close, b.volume] {
            body.extend_from_slice(&v.to_le_bytes());
        }
    }
    emballer(MAGIC, VERSION, bars.len(), &body)
}

/// Les bougies d'un fichier binaire (format 2), si la signature, la version, la taille et l'empreinte sont bonnes ;
/// `None` sinon. Les bougies incohérentes sont retirées.
pub fn parse_bytes(data: &[u8]) -> Option<Vec<Bar>> {
    let (n, body) = deballer(data, MAGIC, VERSION)?;
    if body.len() != n.checked_mul(BOUGIE)? {
        return None;
    }
    let f = |c: &[u8; BOUGIE], i: usize| f64::from_le_bytes(c[8 + 8 * i..16 + 8 * i].try_into().unwrap_or_default());
    let bars = body
        .as_chunks::<BOUGIE>()
        .0
        .iter()
        .map(|c| Bar {
            time: i64::from_le_bytes(c[..8].try_into().unwrap_or_default()),
            open: f(c, 0),
            high: f(c, 1),
            low: f(c, 2),
            close: f(c, 3),
            volume: f(c, 4),
        })
        .collect();
    Some(argos_core::feed::clean_bars(bars))
}

/// Les bougies d'un fichier, quel que soit son format (2 binaire, ou 1 texte relu une fois) ; `None` s'il est abîmé,
/// modifié ou d'une version inconnue.
pub fn parse_any(data: &[u8]) -> Option<Vec<Bar>> {
    if data.starts_with(MAGIC) {
        parse_bytes(data)
    } else {
        parse_file(std::str::from_utf8(data).ok()?)
    }
}

/// Dossier propre à Argos HSS Trading sous le cache du système (jamais `argos/`, qui peut servir à une autre app).
pub const DOSSIER_APP: &str = "argos-hss-trading";

/// Dossier du cache : `$XDG_CACHE_HOME/argos-hss-trading/candles`, sinon `~/.cache/argos-hss-trading/candles`. Ce
/// dossier est **à Argos HSS Trading seul** : rien n'est lu, écrit, converti ni supprimé ailleurs (l'ancien
/// `~/.cache/argos/` n'est plus touché du tout ; les bougies sont retéléchargées une fois). Dans les tests : un
/// dossier de test (`rust/target/tmp`), jamais le vrai `~/.cache`.
pub fn dir() -> Option<std::path::PathBuf> {
    if cfg!(test) {
        return Some(argos_core::dossiers::de_test("cache"));
    }
    dir_from(std::env::var_os("XDG_CACHE_HOME"), std::env::var_os("HOME"))
}

/// Le dossier du cache d'après `XDG_CACHE_HOME` et `HOME`.
pub fn dir_from(xdg: Option<std::ffi::OsString>, home: Option<std::ffi::OsString>) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let base = xdg
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.map(|h| PathBuf::from(h).join(".cache")))?;
    Some(base.join(DOSSIER_APP).join("candles"))
}

/// En-tête des fichiers du cache au format 1 (texte, relu une fois pour passer au format 2).
const HEAD: &str = "argos-bougies 1 ";

/// Le contenu d'un fichier du cache : l'en-tête avec l'empreinte, puis les bougies cohérentes, dans l'ordre.
pub fn file_text(bars: &[Bar]) -> String {
    let body = encode(&argos_core::feed::clean_bars(bars.to_vec()));
    format!("{HEAD}{}\n{body}", argos_core::sha256_hex(body.as_bytes()))
}

/// Les bougies d'un fichier du cache, si l'en-tête et l'empreinte sont bons ; `None` sinon (abîmé, modifié, ancien
/// format). Les bougies incohérentes sont retirées.
pub fn parse_file(text: &str) -> Option<Vec<Bar>> {
    let (head, body) = text.split_once('\n')?;
    let sum = head.strip_prefix(HEAD)?;
    if sum != argos_core::sha256_hex(body.as_bytes()) {
        return None;
    }
    Some(argos_core::feed::clean_bars(decode(body)?))
}

/// Lit un fichier du cache, empreinte vérifiée, sans trier fermées / en cours. Format 2, ou format 1 (le même nom en
/// `.json`, relu une fois). Un fichier abîmé, modifié ou d'une version inconnue est **supprimé** (il sera refait) et
/// `None` est rendu. Une lecture compte comme une utilisation (le fichier ne sera pas retiré le premier).
pub fn read_raw(path: &std::path::Path) -> Option<Vec<Bar>> {
    let m = morceaux(path);
    if !m.is_empty() {
        let mut all = vec![];
        for (_, p) in &m {
            match std::fs::read(p).ok().and_then(|d| parse_bytes(&d)) {
                Some(b) => {
                    utilise(p);
                    all.extend(b);
                }
                // morceau abîmé ou modifié : supprimé seul (ses bougies seront redemandées comme un trou)
                None => {
                    let _ = std::fs::remove_file(p);
                }
            }
        }
        let all = argos_core::feed::clean_bars(all);
        return (!all.is_empty()).then_some(all);
    }
    let ancien = path.with_extension("json");
    let lu = if path.exists() { path } else { ancien.as_path() };
    let data = std::fs::read(lu).ok()?;
    let bars = parse_any(&data);
    match bars {
        Some(_) => utilise(lu),
        None => {
            let _ = std::fs::remove_file(lu);
        }
    }
    bars
}

/// Note l'utilisation d'un fichier (date de modification = maintenant) pour le retrait des moins utilisés.
pub(crate) fn utilise(path: &std::path::Path) {
    if let Ok(f) = std::fs::OpenOptions::new().write(true).open(path) {
        let _ = f.set_modified(std::time::SystemTime::now());
    }
}

/// Retire les fichiers du cache **le moins utilisés** jusqu'à ce que le dossier tienne dans `max` octets (sauf
/// `garder`, la série qu'on vient d'écrire, avec ses morceaux). Rend combien de fichiers ont été retirés.
pub fn evict(dir: &std::path::Path, max: u64, garder: &std::path::Path) -> usize {
    let serie = garder.file_stem().and_then(|s| s.to_str()).map(|s| format!("{s}.m"));
    let protege = |p: &std::path::Path| {
        p == garder
            || serie
                .as_deref()
                .is_some_and(|s| p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with(s)))
    };
    let Ok(rd) = std::fs::read_dir(dir) else { return 0 };
    let mut files: Vec<(std::time::SystemTime, u64, std::path::PathBuf)> = rd
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let ext = p.extension().and_then(|x| x.to_str()).unwrap_or_default();
            let m = e.metadata().ok()?;
            (m.is_file() && (ext == "bin" || ext == "json")).then(|| (m.modified().unwrap_or(std::time::UNIX_EPOCH), m.len(), p))
        })
        .collect();
    let mut total: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.0);
    let mut n = 0;
    for (_, len, p) in files {
        if total <= max {
            break;
        }
        if !protege(&p) && std::fs::remove_file(&p).is_ok() {
            total -= len;
            n += 1;
        }
    }
    n
}

/// Lit le cache d'unité `tf` à `now_s` : seulement les bougies fermées (une bougie pas encore fermée est retirée).
/// `None` si absent, abîmé, modifié, ancien format ou vide.
pub fn read(path: &std::path::Path, tf: Timeframe, now_s: i64) -> Option<Vec<Bar>> {
    let bars = read_raw(path)?;
    Some(closed(&bars, tf.seconds(), now_s).to_vec()).filter(|b| !b.is_empty())
}

/// Écrit le cache d'unité `tf` à `now_s` : seulement les bougies fermées, jamais la bougie en cours, en morceaux de
/// 10 000 bougies. Seuls les morceaux qui ont changé sont réécrits (fichier temporaire puis renommage : un fichier à
/// moitié écrit ne remplace jamais le bon) ; les morceaux sortis de la série (plus anciens que 200 000 bougies) sont
/// retirés.
pub fn write(path: &std::path::Path, bars: &[Bar], tf: Timeframe, now_s: i64) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        creer_dossier(d)?;
    }
    let fermees = closed(bars, tf.seconds(), now_s);
    let garde = argos_core::feed::clean_bars(fermees[fermees.len().saturating_sub(MAX_BOUGIES_FICHIER)..].to_vec());
    let pas = tf.seconds() * MORCEAU;
    let mut faits: Vec<i64> = vec![];
    for groupe in garde.chunk_by(|a, b| a.time.div_euclid(pas) == b.time.div_euclid(pas)) {
        let k = groupe[0].time.div_euclid(pas);
        faits.push(k);
        ecrire_morceau(&chemin_morceau(path, k), &file_bytes(groupe))?;
    }
    // morceaux sortis de la série (plus anciens, ou plus là) : retirés
    for (k, p) in morceaux(path) {
        if !faits.contains(&k) {
            let _ = std::fs::remove_file(p);
        }
    }
    // l'ancien fichier d'un seul tenant (format 2) et l'ancien format 1 (.json) ne sont plus utiles
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(path.with_extension("json"));
    if let Some(d) = path.parent() {
        evict(d, MAX_OCTETS_CACHE, path);
    }
    Ok(())
}

/// Crée un dossier du cache, lisible seulement par toi (0700), comme les fichiers (0600).
pub(crate) fn creer_dossier(d: &std::path::Path) -> std::io::Result<()> {
    let mut b = std::fs::DirBuilder::new();
    b.recursive(true);
    #[cfg(unix)]
    std::os::unix::fs::DirBuilderExt::mode(&mut b, 0o700);
    b.create(d)?;
    #[cfg(unix)]
    std::fs::set_permissions(d, std::os::unix::fs::PermissionsExt::from_mode(0o700))?;
    Ok(())
}

/// Écrit un morceau seulement s'il a changé : l'en-tête (nombre de bougies + empreinte SHA-256) déjà sur le disque est
/// comparé à celui du nouveau contenu ; identique = rien n'est réécrit (ni relu au-delà de 48 octets).
pub(crate) fn ecrire_morceau(p: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let mut tete = [0u8; ENTETE];
    let pareil = std::fs::File::open(p).and_then(|mut f| f.read_exact(&mut tete)).is_ok() && tete[..] == data[..ENTETE];
    if pareil {
        return Ok(());
    }
    let tmp = p.with_extension(format!("tmp{}", std::process::id()));
    let mut o = std::fs::OpenOptions::new();
    o.write(true).create(true).truncate(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut o, 0o600);
    {
        let mut f = o.open(&tmp)?;
        f.write_all(data)?;
        f.sync_all()?;
    }
    // un fichier déjà là (ancienne version) peut avoir d'autres droits : 0600 dans tous les cas
    #[cfg(unix)]
    std::fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    std::fs::rename(&tmp, p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bar(t: i64, c: f64) -> Bar {
        Bar {
            time: t,
            open: c,
            high: c + 1.0,
            low: c * 0.5,
            close: c,
            volume: 2.5,
        }
    }

    #[test]
    fn combien_rattraper() {
        let tf = Timeframe::M30;
        let cached = vec![bar(0, 1.0), bar(1800, 2.0)];
        // les 3 dernières du cache sont redemandées (corrigées par l'exchange ?) + la bougie en cours
        assert_eq!(missing(&cached, 1800 + 10, tf, 5000), 1 + RELUES, "même bougie : relues + la suivante");
        assert_eq!(missing(&cached, 1800 + 3 * 1800, tf, 5000), 3 + 1 + RELUES);
        assert_eq!(missing(&cached, 1800 * 100_000, tf, 5000), 5000, "trop vieux : rechargement complet");
        assert_eq!(missing(&[], 0, tf, 5000), 5000);
    }

    #[test]
    fn fusion_les_fraiches_gagnent() {
        let tf = Timeframe::M30;
        let cached = vec![bar(0, 1.0), bar(1800, 2.0), bar(3600, 3.0)];
        // la fraîche remplace celle du cache à la même heure
        let out = merge(&cached, vec![bar(3600, 3.5), bar(5400, 4.0)], tf, 5000);
        assert_eq!(
            out.iter().map(|b| (b.time, b.close)).collect::<Vec<_>>(),
            vec![(0, 1.0), (1800, 2.0), (3600, 3.5), (5400, 4.0)]
        );
        // trou entre le cache et les fraîches : seulement les fraîches
        let out = merge(&cached, vec![bar(36_000, 9.0)], tf, 5000);
        assert_eq!(out.len(), 1);
        // rien de frais : le cache
        assert_eq!(merge(&cached, vec![], tf, 5000), cached);
        // on garde les plus récentes
        let out = merge(&cached, vec![bar(5400, 4.0)], tf, 2);
        assert_eq!(out.iter().map(|b| b.time).collect::<Vec<_>>(), vec![3600, 5400]);
    }

    #[test]
    fn aller_retour_fichier() {
        let bars = vec![bar(1_792_962_000, 80512.5), bar(1_792_963_800, 80400.25)];
        assert_eq!(decode(&encode(&bars)), Some(bars.clone()));
        assert_eq!(decode("pas du json"), None);
        assert_eq!(file_name(crate::Exchange::Hyperliquid, "BTC", Timeframe::M30), "hl-btc-30m.bin");
        // 🔒 a series whose candles changed meaning (24 Nov. 2026) is not read from its old file
        assert_eq!(file_name(crate::Exchange::Mexc, "BTC", Timeframe::M1), "mexc-btc-1m-r2.bin");
        assert_eq!(file_name(crate::Exchange::Okx, "BTC", Timeframe::H1), "okx-btc-1h-r2.bin");
        assert_eq!(file_name(crate::Exchange::Bitget, "BTC", Timeframe::D1), "bitget-btc-1d-r2.bin");
        assert_eq!(file_name(crate::Exchange::Bitget, "BTC", Timeframe::H4), "bitget-btc-4h.bin", "unchanged");
        assert_eq!(file_name(crate::Exchange::OkxSpot, "BTC", Timeframe::H12), "okx-spot-btc-12h-r2.bin");
        assert_eq!(file_name(crate::Exchange::OkxSpot, "BTC", Timeframe::D1), "okx-spot-btc-1d.bin", "unchanged");
        let p = std::env::temp_dir().join(format!("argos-cache-test-{}", std::process::id())).join("x.json");
        let now = 1_792_963_800 + 1800 + MARGE_HORLOGE_S;
        write(&p, &bars, Timeframe::M30, now).unwrap();
        assert_eq!(read(&p, Timeframe::M30, now), Some(bars));
        std::fs::remove_dir_all(p.parent().unwrap()).ok();
    }

    #[test]
    fn la_bougie_en_cours_n_est_jamais_ecrite() {
        let d = 86_400; // 2 janv. 1970 : des heures > 0
        let (tf, now) = (Timeframe::H1, d + 7200 + 600); // 02:10 : la bougie de 02:00 est en cours
        let bars = vec![bar(d, 1.0), bar(d + 3600, 2.0), bar(d + 7200, 3.0)];
        let p = std::env::temp_dir().join(format!("argos-cache-ferme-{}", std::process::id())).join("x.json");
        write(&p, &bars, tf, now).unwrap();
        let disque = read_raw(&p).unwrap();
        assert_eq!(
            disque.iter().map(|b| b.time - d).collect::<Vec<_>>(),
            vec![0, 3600],
            "02:00 en cours : pas écrite"
        );
        // une heure plus tard, la même bougie est fermée : gardée
        let plus_tard = d + 7200 + 3600 + MARGE_HORLOGE_S + 1;
        write(&p, &bars, tf, plus_tard).unwrap();
        assert_eq!(read(&p, tf, plus_tard).map(|b| b.len()), Some(3), "fermée : gardée");
        std::fs::remove_dir_all(p.parent().unwrap()).ok();
    }

    #[test]
    fn vieux_fichier_nettoye_a_la_lecture() {
        let (tf, now) = (Timeframe::M30, 3600 + MARGE_HORLOGE_S + 60);
        let p = std::env::temp_dir().join(format!("argos-cache-vieux-{}", std::process::id())).join("x.json");
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        // fichier valide qui garde une bougie pas encore fermée (horloge changée, autre machine) : elle est retirée
        std::fs::write(&p, file_text(&[bar(1800, 1.0), bar(3600, 2.0)])).unwrap();
        assert_eq!(read(&p, tf, now).unwrap().iter().map(|b| b.time).collect::<Vec<_>>(), vec![1800]);
        // seulement une bougie en cours : rien d'utilisable
        std::fs::write(&p, file_text(&[bar(3600, 2.0)])).unwrap();
        assert_eq!(read(&p, tf, now), None);
        std::fs::remove_dir_all(p.parent().unwrap()).ok();
    }

    #[test]
    fn fermee_pile_a_la_limite() {
        // fermée seulement après sa fin + la marge de l'horloge (2 min) : une horloge de PC en avance ne fait pas
        // écrire une bougie encore en cours chez l'exchange
        let (b, m) = (bar(3600, 1.0), MARGE_HORLOGE_S);
        assert!(!is_closed(&b, 3600, 7200), "pile à la fin : pas encore (marge)");
        assert!(!is_closed(&b, 3600, 7199 + m), "une seconde avant la fin + marge : en cours");
        assert!(is_closed(&b, 3600, 7200 + m), "fin + marge : fermée");
        assert_eq!(closed(&[bar(0, 1.0), b], 3600, 7200 + m).len(), 2);
        assert_eq!(closed(&[], 3600, 7200).len(), 0);
    }

    #[test]
    fn semaines_et_mois() {
        // semaine : lundi 2 nov. 2026 00:00 UTC → fermée le lundi 9 nov. 00:00 UTC
        let lundi = 1_793_577_600;
        let w = bar(lundi, 1.0);
        let m = MARGE_HORLOGE_S;
        assert!(!is_closed(&w, Timeframe::W1.seconds(), lundi + 7 * 86_400 + m - 1));
        assert!(is_closed(&w, Timeframe::W1.seconds(), lundi + 7 * 86_400 + m));
        // mois : février 2028 (29 jours) → fermé le 1er mars 2028 00:00 UTC, pas avant
        let (fev, mars) = (1_832_976_000, 1_835_481_600);
        let m = bar(fev, 1.0);
        assert!(!is_closed(&m, argos_core::MONTH_SECONDS, mars + MARGE_HORLOGE_S - 1));
        assert!(is_closed(&m, argos_core::MONTH_SECONDS, mars + MARGE_HORLOGE_S));
    }

    #[test]
    fn les_tests_n_ecrivent_jamais_dans_le_vrai_cache() {
        let d = dir().unwrap();
        assert!(d.starts_with(argos_core::dossiers::racine_des_tests()), "{}", d.display());
    }

    #[test]
    fn fichier_abime_modifie_ou_ancien_jete() {
        let (tf, now) = (Timeframe::M30, 100_000);
        let dir = std::env::temp_dir().join(format!("argos-cache-abime-{}", std::process::id()));
        let p = dir.join("x.bin");
        let bars = vec![bar(1800, 1.0), bar(3600, 2.0)];
        // bon fichier : lu
        write(&p, &bars, tf, now).unwrap();
        assert_eq!(read(&p, tf, now), Some(bars.clone()));
        // un octet modifié à la main : l'empreinte ne correspond plus → jeté (supprimé), aucun plantage
        let c = morceaux(&p)[0].1.clone();
        let mut d = std::fs::read(&c).unwrap();
        let n = d.len();
        d[n - 9] ^= 0x40;
        std::fs::write(&c, &d).unwrap();
        assert_eq!(read(&p, tf, now), None);
        assert!(!c.exists(), "morceau abîmé supprimé, il sera refait");
        // morceau coupé : jeté
        write(&p, &bars, tf, now).unwrap();
        let c = morceaux(&p)[0].1.clone();
        let d = std::fs::read(&c).unwrap();
        std::fs::write(&c, &d[..d.len() - 10]).unwrap();
        assert_eq!(read(&p, tf, now), None);
        assert!(morceaux(&p).is_empty());
        // version inconnue (3) : jetée proprement
        let mut d = file_bytes(&bars);
        d[4] = 3;
        std::fs::write(&p, &d).unwrap();
        assert_eq!(read(&p, tf, now), None);
        assert!(!p.exists());
        // JSON sans en-tête (avant l'étape 1b) : jeté aussi
        std::fs::write(&p, encode(&bars)).unwrap();
        assert_eq!(read(&p, tf, now), None);
        // n'importe quoi : jeté
        std::fs::write(&p, "\u{0}\u{1}pas un cache").unwrap();
        assert_eq!(read(&p, tf, now), None);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn bougies_incoherentes_jamais_dans_le_cache() {
        let mut folle = bar(5400, 3.0);
        folle.high = 1.0; // haut sous la clôture
        let t = file_text(&[bar(1800, 1.0), folle, bar(3600, 2.0)]);
        assert_eq!(parse_file(&t).unwrap().iter().map(|b| b.time).collect::<Vec<_>>(), vec![1800, 3600]);
    }

    #[cfg(unix)]
    #[test]
    fn fichier_lisible_seulement_par_toi() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("argos-cache-droits-{}", std::process::id()));
        let p = dir.join("x.bin");
        std::fs::create_dir_all(&dir).unwrap();
        // un morceau déjà là, avec d'autres droits (0644) : réécrit en 0600
        let c = chemin_morceau(&p, 1800i64.div_euclid(1800 * MORCEAU));
        std::fs::write(&c, "ancien").unwrap();
        std::fs::set_permissions(&c, std::fs::Permissions::from_mode(0o644)).unwrap();
        write(&p, &[bar(1800, 1.0)], Timeframe::M30, 100_000).unwrap();
        assert_eq!(morceaux(&p).len(), 1);
        assert_eq!(std::fs::metadata(&c).unwrap().permissions().mode() & 0o777, 0o600);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn aucun_texte_libre_dans_le_cache() {
        // format 2 : un en-tête fixe de 48 octets puis exactement 48 octets par bougie (6 nombres) : pas de place
        // pour un texte, une clé ou un jeton
        let bars = [bar(1_792_962_000, 80512.5), bar(1_792_963_800, 0.25)];
        let d = file_bytes(&bars);
        assert_eq!(d.len(), ENTETE + 2 * BOUGIE);
        assert_eq!(&d[..8], b"ARGB\x02\0\0\0");
        assert_eq!(parse_bytes(&d), Some(bars.to_vec()));
    }

    #[test]
    fn ancien_format_relu_une_fois_puis_remplace() {
        let (tf, now) = (Timeframe::M30, 100_000);
        let dir = std::env::temp_dir().join(format!("argos-cache-v1-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (bin, json) = (dir.join("hl-btc-30m.bin"), dir.join("hl-btc-30m.json"));
        let bars = vec![bar(1800, 1.0), bar(3600, 2.0)];
        // le cache d'avant (format 1, .json) : relu tel quel, aucun téléchargement de plus
        std::fs::write(&json, file_text(&bars)).unwrap();
        assert_eq!(read(&bin, tf, now), Some(bars.clone()));
        // au prochain enregistrement : format 2 en morceaux, l'ancien fichier disparaît
        write(&bin, &bars, tf, now).unwrap();
        assert!(!bin.exists() && !json.exists());
        let m = morceaux(&bin);
        assert_eq!(m.len(), 1);
        assert_eq!(std::fs::read(&m[0].1).unwrap()[..4], *b"ARGB");
        assert_eq!(read(&bin, tf, now), Some(bars));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn au_plus_200_000_bougies_par_fichier() {
        let dir = std::env::temp_dir().join(format!("argos-cache-max-{}", std::process::id()));
        let p = dir.join("x.bin");
        let bars: Vec<Bar> = (1..=MAX_BOUGIES_FICHIER as i64 + 10).map(|k| bar(k * 60, 5.0)).collect();
        write(&p, &bars, Timeframe::M1, i64::MAX / 2).unwrap();
        let lu = read_raw(&p).unwrap();
        assert_eq!(lu.len(), MAX_BOUGIES_FICHIER, "les plus récentes");
        assert_eq!(lu.last().unwrap().time, bars.last().unwrap().time);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn retrait_des_fichiers_le_moins_utilises() {
        let dir = std::env::temp_dir().join(format!("argos-cache-lru-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let un = |nom: &str, age_s: u64| {
            let p = dir.join(nom);
            std::fs::write(&p, vec![0u8; 1000]).unwrap();
            let f = std::fs::OpenOptions::new().write(true).open(&p).unwrap();
            f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(age_s)).unwrap();
            p
        };
        let (vieux, moyen, recent) = (un("a.bin", 3000), un("b.json", 2000), un("c.bin", 10));
        let autre = dir.join("note.txt");
        std::fs::write(&autre, "pas du cache").unwrap();
        // 3000 octets, plafond 2500 : le moins utilisé part
        assert_eq!(evict(&dir, 2500, &recent), 1);
        assert!(!vieux.exists() && moyen.exists() && recent.exists() && autre.exists());
        // le fichier qu'on vient d'écrire n'est jamais retiré, même seul au-dessus du plafond
        assert_eq!(evict(&dir, 0, &recent), 1);
        assert!(!moyen.exists() && recent.exists() && autre.exists(), "seulement les fichiers du cache");
        // une lecture compte comme une utilisation : le fichier lu redevient le plus récent
        let lu = dir.join("lu.bin");
        std::fs::write(&lu, file_bytes(&[bar(1800, 1.0)])).unwrap();
        let f = std::fs::OpenOptions::new().write(true).open(&lu).unwrap();
        f.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(9000)).unwrap();
        assert!(read_raw(&lu).is_some());
        let age = std::fs::metadata(&lu).unwrap().modified().unwrap().elapsed().unwrap_or_default();
        assert!(age.as_secs() < 60, "lecture = utilisation");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn dossier_propre_a_argos_hss_trading() {
        let d = dir_from(None, Some("/home/x".into())).unwrap();
        assert_eq!(d, std::path::PathBuf::from("/home/x/.cache/argos-hss-trading/candles"));
        let d = dir_from(Some("/var/c".into()), Some("/home/x".into())).unwrap();
        assert_eq!(d, std::path::PathBuf::from("/var/c/argos-hss-trading/candles"));
        assert!(!d.starts_with("/var/c/argos/"), "jamais le dossier argos/ d'une autre app");
        assert_eq!(dir_from(Some("".into()), None), None);
    }

    /// Rien n'est lu, écrit, converti ni supprimé hors du dossier d'Argos HSS Trading : écriture, lecture (avec la
    /// relecture de l'ancien format) et retrait des moins utilisés laissent intact le dossier `argos/` d'à côté.
    #[test]
    fn jamais_rien_hors_de_son_dossier() {
        let racine = std::env::temp_dir().join(format!("argos-cache-isole-{}", std::process::id()));
        let autre = racine.join("argos").join("candles");
        std::fs::create_dir_all(&autre).unwrap();
        // les fichiers d'une autre app, au même nom que les nôtres (anciens formats, abîmés…)
        let etrangers: Vec<(std::path::PathBuf, Vec<u8>)> = ["hl-btc-1h.json", "hl-btc-1h.bin", "okx-btc-1h.json", "x.tmp"]
            .iter()
            .map(|n| (autre.join(n), format!("autre app : {n}").into_bytes()))
            .collect();
        for (p, d) in &etrangers {
            std::fs::write(p, d).unwrap();
        }
        let logos_autre = racine.join("argos").join("logos").join("1.png");
        std::fs::create_dir_all(logos_autre.parent().unwrap()).unwrap();
        std::fs::write(&logos_autre, "logo").unwrap();
        // notre dossier, d'après XDG_CACHE_HOME = racine
        let nous = dir_from(Some(racine.clone().into_os_string()), None).unwrap();
        assert!(nous.starts_with(racine.join("argos-hss-trading")));
        let p = nous.join("hl-btc-1h.bin");
        let (tf, now) = (Timeframe::H1, 100_000);
        assert_eq!(read(&p, tf, now), None, "rien chez nous : on ne va pas chercher chez l'autre app");
        write(&p, &[bar(3600, 1.0), bar(7200, 2.0)], tf, now).unwrap();
        assert!(read(&p, tf, now).is_some());
        assert_eq!(evict(&nous, 0, &p), 0);
        std::fs::write(nous.join("vieux.bin"), b"x").unwrap();
        assert_eq!(evict(&nous, 0, &p), 1, "le retrait ne touche que notre dossier");
        // l'autre app : tout est encore là, identique
        for (f, d) in &etrangers {
            assert_eq!(std::fs::read(f).unwrap(), *d, "{} intact", f.display());
        }
        assert!(logos_autre.exists());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&nous).unwrap().permissions().mode() & 0o777, 0o700, "dossier 0700");
            let c = &morceaux(&p)[0].1;
            assert_eq!(std::fs::metadata(c).unwrap().permissions().mode() & 0o777, 0o600, "fichier 0600");
        }
        std::fs::remove_dir_all(&racine).ok();
    }

    /// 🛡️ Robustesse (demande de Jee) : des centaines de fichiers abîmés ou au hasard (coupés, mauvaise taille, en-tête
    /// faux, nombre de bougies énorme, nombres énormes, NaN, infinis) donnés au lecteur binaire. Il doit tous les
    /// refuser proprement (ou n'en garder que des bougies cohérentes), sans planter, et sans réserver plus de mémoire
    /// que la taille réelle du fichier (le nombre de bougies lu dans le fichier n'est jamais utilisé pour réserver).
    #[test]
    fn centaines_de_fichiers_abimes_ou_au_hasard() {
        let mut h = argos_core::alea::Alea::default(); // graine fixe : mêmes fichiers à chaque fois
        let bonnes: Vec<Bar> = (1..=40).map(|k| bar(k * 3600, 100.0 + k as f64)).collect();
        let bon = file_bytes(&bonnes);
        let mut refuses = 0;
        let mut verifie = |d: &[u8]| {
            // jamais de plantage ; si quelque chose est rendu, ce ne sont que des bougies cohérentes, et jamais plus
            // de bougies que le fichier n'en peut contenir
            if let Some(b) = parse_any(d) {
                assert!(b.iter().all(argos_core::feed::bar_ok));
                assert!(b.len() <= d.len() / BOUGIE + 1);
            } else {
                refuses += 1;
            }
        };
        // 1. 300 fichiers d'octets au hasard (0 à 4000 octets), certains avec la bonne signature et la bonne version
        for k in 0..300 {
            let n = h.entier(4000) as usize;
            let mut d: Vec<u8> = (0..n).map(|_| h.suivant() as u8).collect();
            if k % 3 == 0 && d.len() >= 8 {
                d[..8].copy_from_slice(b"ARGB\x02\0\0\0");
            }
            verifie(&d);
        }
        // 2. 200 bons fichiers coupés à un endroit au hasard, ou allongés de quelques octets
        for _ in 0..200 {
            let mut d = bon.clone();
            if h.suivant().is_multiple_of(2) {
                d.truncate(h.entier(bon.len() as u64) as usize);
            } else {
                d.extend((0..1 + h.entier(60)).map(|_| h.suivant() as u8));
            }
            assert_eq!(parse_any(&d), None, "taille fausse : refusé");
        }
        // 3. 200 bons fichiers avec un octet changé au hasard (en-tête ou bougies) : refusés (empreinte, version…)
        for _ in 0..200 {
            let mut d = bon.clone();
            let i = h.entier(d.len() as u64) as usize;
            d[i] ^= 1 << h.entier(8);
            if (8..16).contains(&i) {
                continue; // un octet du nombre de bougies : vérifié à part (taille)
            }
            assert_eq!(parse_any(&d), None, "octet {i} changé : refusé");
        }
        // 3b. chaque octet de l'en-tête changé (signature, version, octets réservés, nombre, empreinte) : refusé
        for i in 0..ENTETE {
            let mut d = bon.clone();
            d[i] ^= 0x10;
            assert_eq!(parse_any(&d), None, "octet {i} de l'en-tête changé : refusé");
        }
        // 4. nombre de bougies énorme dans l'en-tête (jusqu'à u64::MAX) : refusé tout de suite, sans rien réserver
        for n in [u64::MAX, u64::MAX / 48, 1 << 60, 1 << 40, 41, 39, 0] {
            let mut d = bon.clone();
            d[8..16].copy_from_slice(&n.to_le_bytes());
            assert_eq!(parse_bytes(&d), None, "nombre annoncé {n} ≠ taille réelle");
        }
        // 5. 100 fichiers à l'empreinte juste mais aux nombres fous (NaN, infinis, énormes, négatifs, heures folles) :
        //    lus sans planter, seules les bougies cohérentes restent
        let fous = [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::MAX,
            -f64::MAX,
            -1.0,
            0.0,
            1e300,
            f64::MIN_POSITIVE,
        ];
        for _ in 0..100 {
            let mut b = bonnes.clone();
            for _ in 0..1 + h.entier(10) {
                let k = h.entier(b.len() as u64) as usize;
                let v = fous[h.entier(fous.len() as u64) as usize];
                match h.entier(7) {
                    0 => b[k].open = v,
                    1 => b[k].high = v,
                    2 => b[k].low = v,
                    3 => b[k].close = v,
                    4 => b[k].volume = v,
                    5 => b[k].time = h.suivant() as i64,
                    _ => b[k].time = i64::MIN,
                }
            }
            // fichier écrit à la main (sans le nettoyage de file_bytes), empreinte juste
            let mut body = vec![];
            for x in &b {
                body.extend_from_slice(&x.time.to_le_bytes());
                for v in [x.open, x.high, x.low, x.close, x.volume] {
                    body.extend_from_slice(&v.to_le_bytes());
                }
            }
            let mut d = b"ARGB\x02\0\0\0".to_vec();
            d.extend_from_slice(&(b.len() as u64).to_le_bytes());
            d.extend_from_slice(&argos_core::sha256(&body));
            d.extend_from_slice(&body);
            verifie(&d);
        }
        // 6. texte (ancien format) au hasard ou abîmé : refusé sans planter
        for _ in 0..100 {
            let mut t = file_text(&bonnes).into_bytes();
            let i = h.entier(t.len() as u64) as usize;
            t[i] = h.suivant() as u8;
            verifie(&t);
        }
        assert!(refuses >= 300, "les fichiers au hasard sont refusés ({refuses})");
    }

    /// 🧩 Revue R&D, point 3 : ajouter des bougies ne réécrit que le dernier morceau ; les morceaux anciens ne sont
    /// ni réécrits (date et contenu inchangés) ni relus en entier. Le temps d'un ajout est mesuré, avant / après.
    #[test]
    fn ajout_sans_reecrire_les_morceaux_anciens() {
        let dir = std::env::temp_dir().join(format!("argos-cache-morceaux-{}", std::process::id()));
        let p = dir.join("hl-btc-1m.bin");
        let tf = Timeframe::M1;
        // presque la plus grosse série (199 990 bougies d'une minute, 21 morceaux) : l'ajout ne fait rien sortir
        let base = 1_700_000_000 / 60 * 60;
        let n0 = 199_990i64;
        let mut bars: Vec<Bar> = (0..n0).map(|k| bar(base + k * 60, 100.0 + (k % 97) as f64)).collect();
        let now = base + 300_000 * 60;
        write(&p, &bars, tf, now).unwrap();
        let m = morceaux(&p);
        assert!(m.len() >= 20, "{} morceaux", m.len());
        // les anciens morceaux : vieillis exprès pour voir s'ils sont touchés
        let vieux = std::time::SystemTime::now() - std::time::Duration::from_secs(86_400);
        for (_, c) in &m {
            std::fs::OpenOptions::new().write(true).open(c).unwrap().set_modified(vieux).unwrap();
        }
        let avant: Vec<Vec<u8>> = m.iter().map(|(_, c)| std::fs::read(c).unwrap()).collect();
        // ajout de 3 bougies
        for k in 0..3 {
            bars.push(bar(base + (n0 + k) * 60, 50.0));
        }
        let t = std::time::Instant::now();
        write(&p, &bars, tf, now).unwrap();
        let ajout = t.elapsed();
        let m2 = morceaux(&p);
        let derniers = m2.last().unwrap().0;
        for ((k, c), d) in m.iter().zip(&avant) {
            if *k == derniers {
                continue;
            }
            let date = std::fs::metadata(c).unwrap().modified().unwrap();
            assert_eq!(date, vieux, "morceau {k} pas réécrit");
            assert_eq!(&std::fs::read(c).unwrap(), d, "morceau {k} inchangé");
        }
        let dernier = std::fs::metadata(&m2.last().unwrap().1).unwrap().modified().unwrap();
        assert!(dernier > vieux, "le dernier morceau, lui, est réécrit");
        assert_eq!(read_raw(&p).unwrap().len(), bars.len(), "tout est là");
        // avant (un seul fichier, tout réécrit) : mesuré sur le même contenu
        let t = std::time::Instant::now();
        let seul = dir.join("seul.bin");
        std::fs::write(&seul, file_bytes(&bars)).unwrap();
        let entier = t.elapsed();
        let ecrit = std::fs::metadata(&m2.last().unwrap().1).unwrap().len();
        let tout_ecrit = std::fs::metadata(&seul).unwrap().len();
        println!(
            "ajout de 3 bougies à {} : avant (fichier entier) {tout_ecrit} octets en {entier:?} ; après (dernier morceau) {ecrit} octets en {ajout:?}",
            bars.len()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
