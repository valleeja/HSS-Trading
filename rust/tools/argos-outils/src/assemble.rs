//! 🧩 Assemble `src/*` (dans l'ordre des numéros) en `index.html` + `app.js` (site v1, à la racine du dépôt).
//! - Le gros script de l'app (morceaux `.js`) devient `/app.js` : le navigateur le garde en cache ET garde sa version
//!   compilée (ouverture plus rapide).
//! - `index.html` le charge avec un numéro de version (`?v=…`, 10 premiers chiffres du SHA-256 de `app.js`) : chaque
//!   mise à jour est prise tout de suite.
//!
//! ```text
//! argos-outils assemble [--racine rust/]          écrit index.html et app.js
//! argos-outils assemble --check [--racine rust/]  vérifie qu'ils sont à jour
//! argos-outils assemble --test [--racine rust/]   écrit aussi _test.html, tout-en-un, pour les tests
//! ```
//! Remplace `tools/assemble.py` : mêmes fichiers, octet pour octet.

use crate::Args;
use std::path::{Path, PathBuf};

const DEBUT: &str = "<script src=\"/vendor/lightweight-charts";
const FIN: &str = "\n})();\n</script>\n";

/// Texte lu comme `read_text` de Python (fins de ligne `\r\n` et `\r` → `\n`).
fn lire(p: &Path) -> Result<String, String> {
    let t = std::fs::read_to_string(p).map_err(|e| format!("{} : {e}", p.display()))?;
    Ok(if t.contains('\r') { t.replace("\r\n", "\n").replace('\r', "\n") } else { t })
}

/// Le gros script : (fin de la ligne lightweight-charts, début du script, fin du script, fin de `</script>\n`).
fn gros_script(full: &str) -> Option<(usize, usize, usize, usize)> {
    let mut depuis = 0;
    while let Some(i) = full[depuis..].find(DEBUT).map(|i| i + depuis) {
        let fin_ligne = full[i..].find('\n').map(|j| i + j + 1)?;
        if full[fin_ligne..].starts_with("<script>\n") {
            let corps = fin_ligne + "<script>\n".len();
            if let Some(k) = full[corps..].find(FIN) {
                let fin_script = corps + k + "\n})();\n".len();
                return Some((i, fin_ligne, fin_script, fin_script + "</script>\n".len()));
            }
        }
        depuis = i + 1;
    }
    None
}

/// (index.html, app.js, version, nombre de morceaux, tout-en-un)
pub fn assembler(depot: &Path) -> Result<(String, String, String, usize, String), String> {
    let mut parts: Vec<PathBuf> = std::fs::read_dir(depot.join("src"))
        .map_err(|e| format!("src/ : {e}"))?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("").as_bytes();
            n.len() >= 3 && n[0].is_ascii_digit() && n[1].is_ascii_digit() && n[2] == b'-'
        })
        .collect();
    parts.sort();
    if parts.is_empty() {
        return Err("src/ est vide".into());
    }
    let mut full = String::new();
    for p in &parts {
        full.push_str(&lire(p)?);
    }
    let (debut, fin_ligne, fin_script, fin) = gros_script(&full).ok_or("gros script introuvable")?;
    let app = full[fin_ligne + "<script>\n".len()..fin_script].to_string();
    let ver: String = sha256(app.as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..10].to_string();
    let html = format!(
        "{}{}<script src=\"/app.js?v={ver}\"></script>\n{}",
        &full[..debut],
        &full[debut..fin_ligne],
        &full[fin..]
    );
    Ok((html, app, ver, parts.len(), full))
}

pub fn main(a: &Args) -> Result<(), String> {
    let depot = a.racine.join("..");
    let (html, app, ver, n, full) = assembler(&depot)?;
    let sorties = [(depot.join("index.html"), &html), (depot.join("app.js"), &app)];
    if a.pos.iter().any(|x| x == "--check") {
        let mauvais: Vec<String> = sorties
            .iter()
            .filter(|(p, t)| lire(p).map_or(true, |x| &x != *t))
            .map(|(p, _)| p.file_name().unwrap_or_default().to_string_lossy().into_owned())
            .collect();
        if mauvais.is_empty() {
            println!("✅ index.html et app.js à jour");
            return Ok(());
        }
        println!("❌ pas à jour : {} → argos-outils assemble", mauvais.join(", "));
        return Err("index.html / app.js pas à jour".into());
    }
    for (p, t) in &sorties {
        std::fs::write(p, t.as_bytes()).map_err(|e| format!("{} : {e}", p.display()))?;
    }
    if a.pos.iter().any(|x| x == "--test") {
        let p = depot.join("_test.html");
        std::fs::write(&p, full.as_bytes()).map_err(|e| format!("{} : {e}", p.display()))?;
    }
    println!(
        "assemblé : {n} morceaux · index.html {} Ko · app.js {} Ko (v={ver})",
        html.chars().count() / 1024,
        app.chars().count() / 1024
    );
    Ok(())
}

/// SHA-256 (FIPS 180-4), sans dépendance.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut m = data.to_vec();
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&((data.len() as u64).wrapping_mul(8)).to_be_bytes());
    for bloc in m.chunks(64) {
        let mut w = [0u32; 64];
        for (i, c) in bloc.chunks(4).enumerate() {
            w[i] = u32::from_be_bytes([c[0], c[1], c[2], c[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7].wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [t1.wrapping_add(t2), v[0], v[1], v[2], v[3].wrapping_add(t1), v[4], v[5], v[6]];
        }
        for (x, y) in h.iter_mut().zip(v) {
            *x = x.wrapping_add(y);
        }
    }
    let mut o = [0u8; 32];
    for (i, x) in h.iter().enumerate() {
        o[4 * i..4 * i + 4].copy_from_slice(&x.to_be_bytes());
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha256_vecteurs_connus() {
        assert_eq!(hex(&sha256(b"")), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        assert_eq!(hex(&sha256(b"abc")), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        let long = vec![b'a'; 1000];
        assert_eq!(hex(&sha256(&long)), "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3");
    }

    #[test]
    fn decoupe_du_gros_script() {
        let full = "<head>\n<script src=\"/vendor/lightweight-charts.js\"></script>\n<script>\nlet a=1;\n})();\n</script>\n</html>\n";
        let (d, l, f, e) = gros_script(full).unwrap();
        assert_eq!(&full[l + 9..f], "let a=1;\n})();\n");
        assert_eq!(&full[e..], "</html>\n");
        assert_eq!(&full[d..l], "<script src=\"/vendor/lightweight-charts.js\"></script>\n");
        assert!(gros_script("<script>\nx\n</script>\n").is_none());
    }
}
