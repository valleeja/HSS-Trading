//! 🎓 Académie d'Argos HSS Trading : la logique pure, sans dessin. Les leçons sont des fichiers Markdown embarqués
//! (`contenu/`, copiés de `croquis/academie/contenu/`, sans `brut/`), lus une fois par un mini-lecteur maison
//! ([`markdown`]). On y trouve les parcours (0 Bien commencer, 1 Figures, 2 SMC, 5 News éco, 6 Stratégies ; les
//! autres sont « bientôt »), la fiche de chaque leçon (sections emoji, check-list, quiz de 5 questions), la
//! progression gardée par l'app, la recherche et les horaires (New York par défaut, réglable).
//! C'est de la formation, pas un conseil financier.

mod fichiers;
pub mod horaires;
pub mod markdown;

use markdown::{blocs, element_numerote, sans_gras, texte_bloc, Bloc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::sync::OnceLock;

pub use horaires::Fuseau;

/// Nombre de questions d'un quiz.
pub const QUESTIONS: usize = 5;
/// Bonnes réponses pour qu'une leçon soit terminée (4 sur 5, soit 80 %).
pub const REUSSITE: u8 = 4;

/// Un parcours.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parcours {
    pub num: u8,
    pub icone: &'static str,
    pub nom: &'static str,
    /// `false` : affiché « bientôt », sans leçon.
    pub ouvert: bool,
}

/// Les parcours de la maquette validée par Jee, dans l'ordre.
pub const PARCOURS: [Parcours; 9] = [
    Parcours {
        num: 0,
        icone: "🚀",
        nom: "Bien commencer",
        ouvert: true,
    },
    Parcours {
        num: 1,
        icone: "📐",
        nom: "Figures de retournement et continuation",
        ouvert: true,
    },
    Parcours {
        num: 2,
        icone: "🧱",
        nom: "Structure SMC",
        ouvert: true,
    },
    Parcours {
        num: 3,
        icone: "🛡️",
        nom: "Risque, journal, psychologie",
        ouvert: false,
    },
    Parcours {
        num: 4,
        icone: "🏁",
        nom: "PropFirm",
        ouvert: false,
    },
    Parcours {
        num: 5,
        icone: "📰",
        nom: "News éco",
        ouvert: true,
    },
    Parcours {
        num: 6,
        icone: "🎯",
        nom: "Stratégies",
        ouvert: true,
    },
    Parcours {
        num: 7,
        icone: "🧪",
        nom: "Indicateurs Argos",
        ouvert: false,
    },
    Parcours {
        num: 8,
        icone: "🏦",
        nom: "Plateformes",
        ouvert: false,
    },
];

/// Une section d'une leçon (`## 🔍 Anatomie`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    /// Titre avec son emoji.
    pub titre: String,
    pub blocs: Vec<Bloc>,
}

/// Une question de quiz : 3 choix, une seule bonne réponse.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub q: String,
    pub choix: Vec<String>,
    pub bonne: usize,
}

/// Une leçon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lecon {
    /// `<parcours>-<fichier>` (ex. `2-02-bos`).
    pub id: String,
    pub parcours: u8,
    pub titre: String,
    /// Groupe dans le parcours (« Figures de retournement », « Astuces »…).
    pub groupe: String,
    pub minutes: u32,
    /// La ligne d'en-tête (source, signal…), sans « Parcours N · groupe · N min ».
    pub note: String,
    pub sections: Vec<Section>,
    pub checklist: Vec<String>,
    /// Le quiz : 5 questions (vide pour une fiche de référence).
    pub quiz: Vec<Question>,
    /// La leçon parle d'heures (heure de New York) : l'app affiche le décalage du fuseau choisi.
    pub horaires: bool,
}

/// Toutes les leçons, dans l'ordre des parcours (lues une fois).
pub fn lecons() -> &'static [Lecon] {
    static L: OnceLock<Vec<Lecon>> = OnceLock::new();
    L.get_or_init(|| fichiers::FICHIERS.iter().map(|(p, nom, src)| lire(*p, nom, src)).collect())
}

/// L'index d'une leçon par son id.
pub fn index(id: &str) -> Option<usize> {
    lecons().iter().position(|l| l.id == id)
}

/// Les leçons d'un parcours (index), dans l'ordre.
pub fn lecons_du(parcours: u8) -> Vec<usize> {
    (0..lecons().len()).filter(|&i| lecons()[i].parcours == parcours).collect()
}

/// Le parcours complet : toutes les leçons dans l'ordre (pour « Reprendre », précédente / suivante).
pub fn chemin() -> Vec<usize> {
    PARCOURS.iter().flat_map(|p| lecons_du(p.num)).collect()
}

/// Lit une leçon Markdown.
pub fn lire(parcours: u8, nom: &str, src: &str) -> Lecon {
    let lignes: Vec<&str> = src.lines().collect();
    let titre = lignes
        .iter()
        .find_map(|l| l.strip_prefix("# "))
        .map(sans_gras)
        .unwrap_or_else(|| nom.to_string());
    let tete = lignes.iter().find_map(|l| l.strip_prefix("> ")).unwrap_or("");
    let (groupe, minutes, note) = entete(tete);
    let mut sections = Vec::new();
    let (mut checklist, mut reponses) = (Vec::new(), Vec::new());
    let debuts: Vec<usize> = (0..lignes.len()).filter(|&i| lignes[i].starts_with("## ")).collect();
    for (k, &d) in debuts.iter().enumerate() {
        let fin = debuts.get(k + 1).copied().unwrap_or(lignes.len());
        let titre = lignes[d][3..].trim().to_string();
        let corps = &lignes[d + 1..fin];
        if titre.contains("Quiz") {
            reponses = corps.iter().filter_map(|l| question(l)).collect();
        } else if titre.starts_with("✅ Check-list") && corps.iter().all(|l| l.trim().is_empty() || l.starts_with("- [ ] ")) {
            checklist = corps.iter().filter_map(|l| l.strip_prefix("- [ ] ")).map(sans_gras).collect();
        } else {
            sections.push(Section { titre, blocs: blocs(corps) });
        }
    }
    let horaires = ["heure de l'Est", "heures de l'Est", "New York", "(ET)", "Heures de l'Est"]
        .iter()
        .any(|m| src.contains(m));
    Lecon {
        id: format!("{parcours}-{nom}"),
        parcours,
        titre,
        groupe,
        minutes,
        note,
        sections,
        checklist,
        quiz: quiz(&reponses),
        horaires,
    }
}

/// `Parcours 1 · Figures de retournement · 5 min · Signal : **baissier**.` → (groupe, minutes, reste).
fn entete(t: &str) -> (String, u32, String) {
    let parts: Vec<&str> = t.split(" · ").collect();
    let mut groupe = String::new();
    let mut minutes = 0;
    let mut reste = Vec::new();
    for (k, p) in parts.iter().enumerate() {
        if k == 0 && p.starts_with("Parcours") {
            continue;
        }
        if let Some(m) = p.strip_suffix(" min").and_then(|m| m.trim().parse().ok()) {
            minutes = m;
        } else if k == 1 {
            // « Stratégies de la série (2/10) » → « Stratégies de la série »
            groupe = p.split(" (").next().unwrap_or(p).trim().to_string();
        } else {
            reste.push(sans_gras(p));
        }
    }
    (groupe, minutes, reste.join(" · "))
}

/// Une ligne de quiz `1. Question → **réponse**` (avec des pièges facultatifs : `→ **réponse** ~ piège ~ piège`).
fn question(l: &str) -> Option<(String, String, Vec<String>)> {
    let l = element_numerote(l.trim())?;
    let (q, r) = l.split_once(" → ")?;
    let mut parts = r.split(" ~ ");
    let bonne = sans_gras(parts.next()?.trim());
    let pieges = parts.map(|p| sans_gras(p.trim())).filter(|p| !p.is_empty()).collect();
    Some((sans_gras(q.trim()), bonne, pieges))
}

/// Le quiz d'une leçon : les 5 premières questions, 3 choix chacune. Les mauvais choix sont les pièges écrits
/// (`→ **bonne** ~ piège ~ piège`) ; sans pièges, les réponses d'autres questions de la même leçon (jamais égales
/// à la bonne). L'ordre des 3 choix est mélangé de façon stable (toujours le même pour une question donnée).
fn quiz(rep: &[(String, String, Vec<String>)]) -> Vec<Question> {
    rep.iter()
        .take(QUESTIONS)
        .enumerate()
        .map(|(i, (q, bonne, pieges))| {
            let mut faux: Vec<String> = Vec::new();
            for p in pieges {
                if p != bonne && !faux.contains(p) {
                    faux.push(p.clone());
                }
            }
            for k in 1..rep.len() {
                let autre = &rep[(i + k) % rep.len()].1;
                if faux.len() < 2 && autre != bonne && !faux.contains(autre) {
                    faux.push(autre.clone());
                }
            }
            faux.truncate(2);
            let mut choix = faux;
            choix.insert(0, bonne.clone());
            melange_stable(&mut choix, &format!("{i}{q}"));
            let bonne = choix.iter().position(|c| c == bonne).unwrap_or(0);
            Question { q: q.clone(), choix, bonne }
        })
        .collect()
}

/// Mélange (Fisher-Yates) tiré du texte `graine` (hachage FNV-1a) : même texte, même ordre, sur toutes les machines.
fn melange_stable<T>(v: &mut [T], graine: &str) {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in graine.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x0100_0000_01b3);
    }
    for k in (1..v.len()).rev() {
        h = h.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        v.swap(k, ((h >> 33) % (k as u64 + 1)) as usize);
    }
}

impl Lecon {
    /// Le texte simple de la leçon (recherche, référence de l'assistant) ; tableaux en lignes `| a | b |`.
    pub fn texte(&self) -> String {
        let mut s = format!("{}\n", self.titre);
        for x in &self.sections {
            s += &format!("\n## {}\n", x.titre);
            for b in &x.blocs {
                s += &texte_bloc(b);
                s += "\n";
            }
        }
        s
    }

    /// Le premier tableau d'une section (pour viser une ligne depuis l'Agenda).
    pub fn tableau(&self, section: usize) -> Option<&Vec<Vec<String>>> {
        self.sections.get(section)?.blocs.iter().find_map(|b| match b {
            Bloc::Tableau(t) => Some(t),
            _ => None,
        })
    }
}

/// Le score d'un quiz : bonnes réponses parmi celles données.
pub fn score(l: &Lecon, reponses: &[Option<usize>]) -> u8 {
    l.quiz.iter().zip(reponses).filter(|(q, r)| **r == Some(q.bonne)).count() as u8
}

/// Progression gardée entre deux lancements (`academy.json` des réglages, même format qu'avant : `read` et `done`,
/// plus les meilleurs scores et le fuseau choisi ; les anciens fichiers se lisent toujours).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub read: Vec<String>,
    pub done: Vec<String>,
    /// Meilleur score (sur 5) de chaque quiz.
    pub scores: BTreeMap<String, u8>,
    /// Fuseau des horaires (New York par défaut).
    #[serde(deserialize_with = "horaires::lire_fuseau")]
    pub fuseau: Fuseau,
}

impl Progress {
    /// Terminée : quiz réussi (4 / 5 ou mieux), ou lue (fiche sans quiz).
    pub fn is_done(&self, l: &Lecon) -> bool {
        self.done.contains(&l.id) || (l.quiz.is_empty() && self.read.contains(&l.id))
    }

    /// Marque la leçon lue ; `true` si c'est nouveau.
    pub fn lire(&mut self, l: &Lecon) -> bool {
        if self.read.contains(&l.id) {
            return false;
        }
        self.read.push(l.id.clone());
        true
    }

    /// Garde le score d'un quiz fini (le meilleur) et marque la leçon terminée s'il est réussi ; `true` si ça change.
    pub fn noter(&mut self, l: &Lecon, s: u8) -> bool {
        let avant = self.clone();
        let best = self.scores.entry(l.id.clone()).or_insert(0);
        *best = (*best).max(s);
        if s >= REUSSITE && !self.done.contains(&l.id) {
            self.done.push(l.id.clone());
        }
        *self != avant
    }

    /// Leçons terminées / total d'un parcours.
    pub fn du_parcours(&self, p: u8) -> (usize, usize) {
        let ls = lecons_du(p);
        (ls.iter().filter(|&&i| self.is_done(&lecons()[i])).count(), ls.len())
    }

    /// Leçons terminées / total de l'Académie.
    pub fn globale(&self) -> (usize, usize) {
        (lecons().iter().filter(|l| self.is_done(l)).count(), lecons().len())
    }

    /// La prochaine leçon du parcours pas encore terminée.
    pub fn prochaine(&self) -> Option<usize> {
        chemin().into_iter().find(|&i| !self.is_done(&lecons()[i]))
    }
}

/// Minuscules sans accents, pour comparer.
pub fn simple(s: &str) -> String {
    s.chars()
        .flat_map(char::to_lowercase)
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' => 'i',
            'ô' | 'ö' | 'ó' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            c => c,
        })
        .collect()
}

/// Recherche : les leçons qui contiennent tous les mots tapés (titre d'abord, puis le texte), les meilleures d'abord.
pub fn chercher(q: &str) -> Vec<usize> {
    let q = simple(q.trim());
    let mots: Vec<&str> = q.split_whitespace().collect();
    if mots.is_empty() {
        return vec![];
    }
    let mut v: Vec<(u8, usize)> = lecons()
        .iter()
        .enumerate()
        .filter_map(|(i, l)| {
            let titre = simple(&l.titre);
            let tout = simple(&format!("{} {} {}", l.groupe, l.note, l.texte()));
            if !mots.iter().all(|m| titre.contains(m) || tout.contains(m)) {
                return None;
            }
            Some((if mots.iter().all(|m| titre.contains(m)) { 0 } else { 1 }, i))
        })
        .collect();
    v.sort();
    v.into_iter().map(|(_, i)| i).collect()
}

/// Mots-clés d'une leçon pour la recherche Ctrl + K (titres de sections et texte, en minuscules sans accents).
pub fn mots_cles(l: &Lecon) -> String {
    simple(&format!("{} {} {}", l.groupe, l.note, l.texte()))
}

#[cfg(test)]
mod tests;
