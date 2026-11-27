//! 🛟 Support: the tickets (problem, idea, question) sent from the app, stored in
//! `Argos HSS Trading/retours/<numéro>-<date>-<type>/` (`billet.json`, the screenshot, `note.txt`).
//!
//! This module is **pure**: `billet.json` format, validation, numbering, folder name, states, replies,
//! image signatures and base64. No file reading or writing here (it also compiles for the web page):
//! the local server (`argos-serveur`, `/api/retours…` routes) and the desktop app write the files.
//!
//! ```
//! use argos_core::retours::{Billet, Genre, Nouveau};
//! let n = Nouveau { genre: Genre::Idee, objet: "Un thème clair".into(), ..Default::default() };
//! let mut b = Billet::nouveau(n, "id".into(), 7, "2026-11-17T08:00:00Z".into()).unwrap();
//! assert_eq!(b.dossier(), "0007-2026-11-17-idee");
//! b.repondre("argos", "Bonne idée, c'est noté.", "2026-11-17T09:00:00Z").unwrap();
//! assert_eq!(b.non_lues(), 1);
//! ```

use serde::{Deserialize, Serialize};

/// Version of the `billet.json` format.
pub const FORMAT: u32 = 1;
/// Subject: at most this many characters.
pub const OBJET_MAX: usize = 120;
/// Description, reply: at most this many characters.
pub const TEXTE_MAX: usize = 10_000;
/// The screenshot: at most 8 MB (the request body is limited by the server anyway).
pub const IMAGE_MAX: usize = 8 << 20;
/// File name of the ticket inside its folder.
pub const FICHIER: &str = "billet.json";
/// The human-readable note next to the ticket.
pub const NOTE: &str = "note.txt";

/// The type of a ticket.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Genre {
    /// 🐞 Problem
    #[default]
    Probleme,
    /// 💡 Idea / Suggestion
    Idee,
    /// ❓ Question
    Question,
}

impl Genre {
    pub const TOUS: [Genre; 3] = [Genre::Probleme, Genre::Idee, Genre::Question];

    /// The word used in the folder name and in `billet.json` (`probleme`, `idee`, `question`).
    pub fn code(self) -> &'static str {
        match self {
            Genre::Probleme => "probleme",
            Genre::Idee => "idee",
            Genre::Question => "question",
        }
    }

    /// The displayed name, with its emoji.
    pub fn nom(self) -> &'static str {
        match self {
            Genre::Probleme => "🐞 Problème",
            Genre::Idee => "💡 Idée / Suggestion",
            Genre::Question => "❓ Question",
        }
    }
}

/// The state of a ticket.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Etat {
    #[default]
    Ouvert,
    #[serde(rename = "En cours")]
    EnCours,
    #[serde(rename = "Résolu")]
    Resolu,
}

impl Etat {
    pub const TOUS: [Etat; 3] = [Etat::Ouvert, Etat::EnCours, Etat::Resolu];

    pub fn nom(self) -> &'static str {
        match self {
            Etat::Ouvert => "Ouvert",
            Etat::EnCours => "En cours",
            Etat::Resolu => "Résolu",
        }
    }
}

/// The context recorded automatically when sending.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Contexte {
    /// App version.
    pub version: String,
    /// The open screen (chart, Accueil, Marketplace…).
    pub ecran: String,
    pub symbole: String,
    /// Timeframe (`30m`…).
    pub ut: String,
    /// Window size (`1440×900`).
    pub taille: String,
    /// Send date as seen by the app (ISO 8601).
    pub date: String,
}

/// A state change.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Changement {
    pub date: String,
    pub etat: Etat,
    /// `admin` (you) or `argos` (Claude).
    pub auteur: String,
}

/// A reply in the ticket's thread.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reponse {
    pub date: String,
    /// `argos` (Claude) or `admin` (you).
    pub auteur: String,
    pub texte: String,
}

/// The two allowed authors.
pub const AUTEURS: [&str; 2] = ["argos", "admin"];

/// A ticket, as written in `billet.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Billet {
    pub format: u32,
    /// Unique identifier (UUID v4).
    pub id: String,
    /// Local number (#N), in the order of sends on this computer.
    #[serde(rename = "numéro")]
    pub numero: u32,
    #[serde(rename = "type")]
    pub genre: Genre,
    pub objet: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub contexte: Contexte,
    /// The device that sent it (its sync identifier).
    #[serde(default)]
    pub appareil: String,
    #[serde(default)]
    pub etat: Etat,
    /// The successive states (the first one: Ouvert, at send time).
    #[serde(default)]
    pub historique: Vec<Changement>,
    #[serde(default, rename = "réponses")]
    pub reponses: Vec<Reponse>,
    /// How many Argos replies have already been seen (red badge as long as there are more).
    #[serde(default)]
    pub vues: usize,
    /// The screenshot file in the folder (`capture.png` or `capture.jpg`), if there is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capture: Option<String>,
}

/// What the app sends to create a ticket (`POST /api/retours`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Nouveau {
    #[serde(rename = "type")]
    pub genre: Genre,
    pub objet: String,
    pub description: String,
    pub contexte: Contexte,
    pub appareil: String,
    /// The screenshot in base64 (PNG or JPEG), empty when there is no screenshot.
    pub image: String,
}

/// A cleaned text: leading/trailing spaces trimmed, control characters removed (except newline and tab).
fn propre(s: &str) -> String {
    s.trim().chars().filter(|c| !c.is_control() || *c == '\n' || *c == '\t').collect()
}

impl Nouveau {
    /// Checks the ticket to create: subject of 1 to `OBJET_MAX` characters, description of at most `TEXTE_MAX`.
    pub fn valider(&self) -> Result<(), String> {
        let objet = propre(&self.objet);
        if objet.is_empty() {
            return Err("l'objet est vide".into());
        }
        if objet.chars().count() > OBJET_MAX {
            return Err(format!("objet trop long (au plus {OBJET_MAX} caractères)"));
        }
        if self.description.chars().count() > TEXTE_MAX {
            return Err(format!("description trop longue (au plus {TEXTE_MAX} caractères)"));
        }
        Ok(())
    }

    /// The screenshot bytes and their extension (`png` or `jpg`), from the byte signature; `Ok(None)` without a
    /// screenshot.
    pub fn capture(&self) -> Result<Option<(Vec<u8>, &'static str)>, String> {
        if self.image.trim().is_empty() {
            return Ok(None);
        }
        let o = base64_lire(self.image.trim()).ok_or("capture illisible (base64)")?;
        if o.len() > IMAGE_MAX {
            return Err(format!("capture trop grosse (au plus {} Mo)", IMAGE_MAX >> 20));
        }
        let ext = genre_image(&o).ok_or("seulement une image PNG ou JPEG")?;
        Ok(Some((o, ext)))
    }
}

impl Billet {
    /// The ticket created from `n` (validated), with its identifier, its local number and the date received.
    pub fn nouveau(n: Nouveau, id: String, numero: u32, date: String) -> Result<Billet, String> {
        n.valider()?;
        Ok(Billet {
            format: FORMAT,
            id,
            numero,
            genre: n.genre,
            objet: propre(&n.objet),
            description: propre(&n.description),
            contexte: n.contexte,
            appareil: propre(&n.appareil),
            etat: Etat::Ouvert,
            historique: vec![Changement {
                date,
                etat: Etat::Ouvert,
                auteur: "admin".into(),
            }],
            reponses: vec![],
            vues: 0,
            capture: None,
        })
    }

    /// The send date (that of the first state).
    pub fn date(&self) -> &str {
        self.historique.first().map_or("", |c| c.date.as_str())
    }

    /// The folder name: `0007-2026-11-17-probleme`.
    pub fn dossier(&self) -> String {
        nom_dossier(self.numero, self.date(), self.genre)
    }

    /// Adds a reply (`argos` or `admin`, non-empty text).
    pub fn repondre(&mut self, auteur: &str, texte: &str, date: &str) -> Result<(), String> {
        if !AUTEURS.contains(&auteur) {
            return Err(format!("auteur inconnu : {auteur}"));
        }
        let t = propre(texte);
        if t.is_empty() {
            return Err("la réponse est vide".into());
        }
        if t.chars().count() > TEXTE_MAX {
            return Err(format!("réponse trop longue (au plus {TEXTE_MAX} caractères)"));
        }
        self.reponses.push(Reponse {
            date: date.into(),
            auteur: auteur.into(),
            texte: t,
        });
        // one's own reply is seen right away
        if auteur == "admin" {
            self.vues = self.argos();
        }
        Ok(())
    }

    /// Changes the state (recorded in the history); the same state changes nothing.
    pub fn changer_etat(&mut self, etat: Etat, auteur: &str, date: &str) -> Result<(), String> {
        if !AUTEURS.contains(&auteur) {
            return Err(format!("auteur inconnu : {auteur}"));
        }
        if etat != self.etat {
            self.etat = etat;
            self.historique.push(Changement {
                date: date.into(),
                etat,
                auteur: auteur.into(),
            });
        }
        Ok(())
    }

    /// Number of Argos replies.
    fn argos(&self) -> usize {
        self.reponses.iter().filter(|r| r.auteur == "argos").count()
    }

    /// Argos replies not seen yet.
    pub fn non_lues(&self) -> usize {
        self.argos().saturating_sub(self.vues)
    }

    /// All Argos replies are seen.
    pub fn marquer_lu(&mut self) {
        self.vues = self.argos();
    }

    /// The human-readable note (`note.txt`) next to `billet.json`.
    pub fn note(&self) -> String {
        let c = &self.contexte;
        let mut s = format!(
            "#{} · {} · {}\n{}\n\nÉtat : {}\nEnvoyé le {}\nApp {} · écran {} · {} {} · fenêtre {}\n",
            self.numero,
            self.genre.nom(),
            self.date(),
            self.objet,
            self.etat.nom(),
            self.date(),
            c.version,
            c.ecran,
            c.symbole,
            c.ut,
            c.taille
        );
        if !self.description.is_empty() {
            s.push_str(&format!("\n{}\n", self.description));
        }
        for r in &self.reponses {
            let qui = if r.auteur == "argos" { "Argos" } else { "Toi" };
            s.push_str(&format!("\n— {qui}, {} :\n{}\n", r.date, r.texte));
        }
        s
    }
}

/// The folder name of a ticket: 4-digit number, day (`AAAA-MM-JJ`, taken from the start of `date`) and type.
pub fn nom_dossier(numero: u32, date: &str, genre: Genre) -> String {
    let jour: String = date.chars().take(10).filter(|c| c.is_ascii_digit() || *c == '-').collect();
    format!("{numero:04}-{jour}-{}", genre.code())
}

/// The number from a ticket folder name (`0007-…` → 7); `None` for any other name.
pub fn numero_du_dossier(nom: &str) -> Option<u32> {
    let (n, reste) = nom.split_once('-')?;
    (n.len() >= 4 && n.chars().all(|c| c.is_ascii_digit()) && !reste.is_empty()).then(|| n.parse().ok())?
}

/// The next number based on the folder names already there (1 for the first).
pub fn prochain_numero<'a>(noms: impl IntoIterator<Item = &'a str>) -> u32 {
    noms.into_iter().filter_map(numero_du_dossier).max().unwrap_or(0) + 1
}

/// Exact name of the project folder where tickets go (`<it>/retours`); the upward search never goes above it.
pub const PROJET: &str = "Argos HSS Trading";

/// The tickets folder: `env` (`$ARGOS_RETOURS`) if given, otherwise `Argos HSS Trading/retours` searching upward
/// from `depart` (the served folder or the executable, already resolved), otherwise `<data>/argos-hss-trading/retours`
/// (`$XDG_DATA_HOME`, otherwise `~/.local/share`). No disk access: the path only.
pub fn dossier_de(
    env: Option<std::ffi::OsString>,
    depart: Option<&std::path::Path>,
    xdg_data: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(e) = env.filter(|e| !e.is_empty()) {
        return Some(PathBuf::from(e));
    }
    if let Some(p) = depart.and_then(|d| d.ancestors().find(|a| a.file_name().is_some_and(|n| n == PROJET))) {
        return Some(p.join("retours"));
    }
    let data = xdg_data
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| home.filter(|v| !v.is_empty()).map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(data.join(crate::dossiers::APP).join("retours"))
}

/// ISO 8601 date in UTC (`2026-11-17T08:00:00Z`) of an instant in Unix seconds.
pub fn date_iso(secs: i64) -> String {
    let (j, s) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    // days → (year, month, day) (Hinnant's algorithm)
    let z = j + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s % 3600 / 60, s % 60)
}

/// `png` or `jpg` from the first bytes (signature), otherwise `None`.
pub fn genre_image(o: &[u8]) -> Option<&'static str> {
    if o.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some("png")
    } else if o.len() > 3 && o.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else {
        None
    }
}

/// A UUID v4 from 16 random bytes.
pub fn uuid(mut o: [u8; 16]) -> String {
    o[6] = (o[6] & 0x0F) | 0x40;
    o[8] = (o[8] & 0x3F) | 0x80;
    let h: String = o.iter().map(|b| format!("{b:02x}")).collect();
    format!("{}-{}-{}-{}-{}", &h[0..8], &h[8..12], &h[12..16], &h[16..20], &h[20..32])
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Standard base64 (with `=`).
pub fn base64_ecrire(o: &[u8]) -> String {
    let mut s = String::with_capacity(o.len().div_ceil(3) * 4);
    for c in o.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                s.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// Reads standard base64 (spaces and newlines ignored); `None` if malformed.
pub fn base64_lire(s: &str) -> Option<Vec<u8>> {
    let v: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !v.len().is_multiple_of(4) {
        return None;
    }
    let val = |b: u8| B64.iter().position(|&x| x == b).map(|p| p as u32);
    let mut o = Vec::with_capacity(v.len() / 4 * 3);
    for (k, c) in v.chunks(4).enumerate() {
        let fin = k + 1 == v.len() / 4;
        let pad = c.iter().rev().take_while(|&&b| b == b'=').count();
        if pad > 2 || (pad > 0 && !fin) {
            return None;
        }
        let mut n = 0u32;
        for (i, &b) in c.iter().enumerate() {
            let x = if i >= 4 - pad { 0 } else { val(b)? };
            n |= x << (18 - 6 * i);
        }
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        o.extend_from_slice(&bytes[..3 - pad]);
    }
    Some(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nouveau(objet: &str) -> Nouveau {
        Nouveau {
            genre: Genre::Probleme,
            objet: objet.into(),
            description: "Il cache l'axe.\n\u{7}Deux lignes.".into(),
            contexte: Contexte {
                version: "202611170000".into(),
                ecran: "chart".into(),
                symbole: "BTC".into(),
                ut: "30m".into(),
                taille: "1440×900".into(),
                date: "2026-11-17T08:00:00Z".into(),
            },
            appareil: "pc-1".into(),
            image: String::new(),
        }
    }

    /// 🛟 billet.json: the format 1 field names (with accents), and read back identically.
    #[test]
    fn billet_json_format_1() {
        let b = Billet::nouveau(nouveau("  Le bouton AI cache l'axe  "), uuid([7; 16]), 3, "2026-11-17T08:00:01Z".into()).unwrap();
        assert_eq!(b.objet, "Le bouton AI cache l'axe");
        assert_eq!(b.description, "Il cache l'axe.\nDeux lignes.", "caractère de contrôle retiré");
        assert_eq!(b.dossier(), "0003-2026-11-17-probleme");
        let j = serde_json::to_value(&b).unwrap();
        for k in [
            "format",
            "id",
            "numéro",
            "type",
            "objet",
            "description",
            "contexte",
            "appareil",
            "etat",
            "historique",
            "réponses",
        ] {
            assert!(j.get(k).is_some(), "champ {k}");
        }
        assert_eq!(j["type"], "probleme");
        assert_eq!(j["etat"], "Ouvert");
        assert_eq!(j["historique"][0]["auteur"], "admin");
        let r: Billet = serde_json::from_value(j).unwrap();
        assert_eq!(r, b);
        assert_eq!(uuid([7; 16]), "07070707-0707-4707-8707-070707070707");
    }

    /// Empty or too long subject, unknown author, empty reply: rejected.
    #[test]
    fn refus() {
        assert!(nouveau("   ").valider().is_err());
        assert!(nouveau(&"x".repeat(OBJET_MAX + 1)).valider().is_err());
        assert!(nouveau(&"é".repeat(OBJET_MAX)).valider().is_ok(), "compté en caractères, pas en octets");
        let mut b = Billet::nouveau(nouveau("a"), "i".into(), 1, "2026-11-17".into()).unwrap();
        assert!(b.repondre("pirate", "x", "d").is_err());
        assert!(b.repondre("admin", "  ", "d").is_err());
        assert!(b.changer_etat(Etat::Resolu, "x", "d").is_err());
        assert!(b.reponses.is_empty() && b.historique.len() == 1);
    }

    /// States (dated history), replies and the unread badge.
    #[test]
    fn etats_reponses_non_lues() {
        let mut b = Billet::nouveau(nouveau("a"), "i".into(), 1, "2026-11-17".into()).unwrap();
        b.changer_etat(Etat::EnCours, "argos", "d1").unwrap();
        b.changer_etat(Etat::EnCours, "argos", "d2").unwrap();
        assert_eq!(b.historique.len(), 2, "le même état ne s'ajoute pas");
        assert_eq!(serde_json::to_value(&b).unwrap()["etat"], "En cours");
        b.repondre("argos", "Je regarde.", "d3").unwrap();
        b.repondre("argos", "Corrigé.", "d4").unwrap();
        assert_eq!(b.non_lues(), 2);
        b.repondre("admin", "Merci", "d5").unwrap();
        assert_eq!(b.non_lues(), 0, "répondre, c'est avoir lu");
        b.repondre("argos", "De rien", "d6").unwrap();
        assert_eq!(b.non_lues(), 1);
        b.marquer_lu();
        assert_eq!(b.non_lues(), 0);
        assert!(b.note().contains("— Argos, d6 :\nDe rien") && b.note().contains("— Toi, d5"));
    }

    /// Numbering based on the folders already there; other names are ignored.
    #[test]
    fn numerotation() {
        assert_eq!(prochain_numero([]), 1);
        assert_eq!(
            prochain_numero(["0002-2026-11-16-idee", "0010-2026-11-17-question", "notes", "12-x", ".cache"]),
            11
        );
        assert_eq!(numero_du_dossier("0042-2026-11-17-probleme"), Some(42));
        assert_eq!(numero_du_dossier("0042"), None);
        assert_eq!(nom_dossier(12345, "2026-11-17T08:00Z", Genre::Question), "12345-2026-11-17-question");
        assert_eq!(nom_dossier(1, "../../x", Genre::Idee), "0001--idee", "jamais de chemin dans le nom");
    }

    /// Tickets folder: $ARGOS_RETOURS, then "Argos HSS Trading/retours" searching upward (never above it),
    /// then the data folder. And the ISO date.
    #[test]
    fn dossier_et_date() {
        use std::path::Path;
        let p = |s: &str| Some(std::ffi::OsString::from(s));
        let depart = Path::new("/home/j/Documents/Argos/Argos HSS Trading/servi/A");
        assert_eq!(dossier_de(p("/x/r"), Some(depart), None, p("/home/j")).unwrap(), Path::new("/x/r"));
        assert_eq!(
            dossier_de(None, Some(depart), None, p("/home/j")).unwrap(),
            Path::new("/home/j/Documents/Argos/Argos HSS Trading/retours")
        );
        let ailleurs = Path::new("/opt/Argos HSS Trading 2/servi");
        assert_eq!(
            dossier_de(p(""), Some(ailleurs), None, p("/home/j")).unwrap(),
            Path::new("/home/j/.local/share/argos-hss-trading/retours"),
            "nom exact seulement"
        );
        assert_eq!(
            dossier_de(None, None, p("/d"), p("/home/j")).unwrap(),
            Path::new("/d/argos-hss-trading/retours")
        );
        assert_eq!(date_iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(date_iso(1_794_960_000), "2026-11-18T00:00:00Z");
        assert_eq!(date_iso(951_782_400 + 3661), "2000-02-29T01:01:01Z");
    }

    /// Screenshot: base64 round trip, PNG / JPEG signature checked on the bytes.
    #[test]
    fn capture_signature_et_base64() {
        for o in [&b""[..], b"a", b"ab", b"abc", b"abcd", &[0u8, 255, 128, 7, 9]] {
            assert_eq!(base64_lire(&base64_ecrire(o)).as_deref(), Some(o));
        }
        assert_eq!(base64_ecrire(b"Man"), "TWFu");
        assert_eq!(base64_lire("TW\nE="), Some(b"Ma".to_vec()));
        assert_eq!(base64_lire("TWE"), None);
        assert_eq!(base64_lire("T=E="), None);
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0];
        let mut n = nouveau("a");
        n.image = base64_ecrire(&png);
        assert_eq!(n.capture().unwrap().unwrap().1, "png");
        n.image = base64_ecrire(&[0xFF, 0xD8, 0xFF, 0xE0]);
        assert_eq!(n.capture().unwrap().unwrap().1, "jpg");
        n.image = base64_ecrire(b"GIF89a....");
        assert!(n.capture().is_err(), "ni PNG ni JPEG");
        n.image = "%%%%".into();
        assert!(n.capture().is_err());
        n.image = String::new();
        assert_eq!(n.capture().unwrap(), None);
    }
}
