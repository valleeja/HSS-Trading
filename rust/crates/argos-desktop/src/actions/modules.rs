//! 🧩 Page Modules, avec le **kit Argos 1.0.2** (interface v2, copie figée dans `rust/vendor/`) :
//! la liste des modules installés, l'installation d'un module (`.wasm` + son manifeste `argos-module.json`), ses
//! droits (aucun par défaut ; seulement son dossier privé, redemandé quand une nouvelle version en ajoute ; réseau et IA
//! refusés : l'app fournit les données), ses réglages (générés depuis le manifeste, entiers à leur échelle), et
//! démarrer / arrêter / retirer. Un module démarré dessine ses courbes sur la chart comme un script (légende, 👁).
//!
//! - **Desktop** : la bulle du kit (`argos_sandbox`, wasmtime sans WASI), carburant, mémoire et temps limités.
//! - **Navigateur** : l'entrée au format du kit est faite ici (`Input::encode`), un Web Worker de la page appelle les
//!   exports v2 (`app/module-worker.js`), arrêté s'il dépasse son temps ; la sortie est décodée ici (`Output::decode`).
//! - L'empreinte SHA-384 du `.wasm` est l'accord : un autre fichier est refusé. Jamais de clé.
//! - Le `.wasm` est rangé à part (`modules/<id>.wasm.b64`) ; le dossier privé du module est gardé aux mises à jour et
//!   même quand il est retiré.

use crate::*;
use argos_module_host::{cards_of, lines_of, scan, sha384, Capability, Manifest, Output as ModOut, SettingKind, SettingSpec};
use argos_render::listmenu::{MenuGroup, MenuItem};
use std::collections::HashMap;

/// 📦 Module d'exemple embarqué : la Moyenne du kit module Argos (construite par `rust/tools/argos_module_exemple.sh`).
pub(crate) const EXAMPLE_WASM: &[u8] = include_bytes!("../../../../../app/modules/moyenne.wasm");
pub(crate) const EXAMPLE_JSON: &str = include_str!("../../../../../app/modules/moyenne.json");
/// 📈 Premier indicateur d'Argos passé en module : l'EMA (`rust/modules/ema`), égale à l'EMA intégrée.
pub(crate) const EMA_WASM: &[u8] = include_bytes!("../../../../../app/modules/ema.wasm");
pub(crate) const EMA_JSON: &str = include_str!("../../../../../app/modules/ema.json");
/// 🎓 Module de l'Académie (`rust/modules/academie`) : le haut et le bas des fenêtres horaires des leçons (New York).
pub(crate) const ACADEMIE_WASM: &[u8] = include_bytes!("../../../../../app/modules/academie.wasm");
pub(crate) const ACADEMIE_JSON: &str = include_str!("../../../../../app/modules/academie.json");
/// 🖌 Pinceau de zone (`rust/modules/pinceau-zone`) : volume par prix, POC, zone de valeur, VWAP et delta estimé d'une
/// zone (rectangle) ; lancé par le clic droit sur un rectangle.
pub(crate) const PINCEAU_WASM: &[u8] = include_bytes!("../../../../../app/modules/pinceau-zone.wasm");
pub(crate) const PINCEAU_JSON: &str = include_str!("../../../../../app/modules/pinceau-zone.json");

/// Un module installé (gardé dans `modules.json`, sans le `.wasm`).
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct ModEntry {
    /// Chemin du kit (`trading/moyenne`).
    pub(crate) id: String,
    pub(crate) version: String,
    /// Le manifeste `argos-module.json` tel qu'installé (un ancien `module.toml` n'est plus lu : réinstaller).
    pub(crate) manifest: String,
    /// Empreinte SHA-384 (hexadécimal) du `.wasm` accepté à l'installation : un autre fichier est refusé.
    pub(crate) sha: String,
    pub(crate) on: bool,
    pub(crate) hidden: bool,
    /// Dossier privé (lire / écrire) accordé ; tout le reste est refusé.
    pub(crate) fichiers: bool,
    /// Droits déjà présentés à l'utilisateur (textes de `rights_text`) : un nouveau texte = redemander.
    pub(crate) asked: Vec<String>,
    /// ⚙ Valeurs choisies des réglages (entiers à leur échelle), par identifiant ; gardées à la mise à jour.
    pub(crate) values: std::collections::BTreeMap<String, f64>,
}

impl ModEntry {
    pub(crate) fn name(&self) -> &str {
        self.id.strip_prefix(argos_module_host::PREFIX).unwrap_or(&self.id)
    }

    pub(crate) fn manifest(&self) -> Option<Manifest> {
        Manifest::parse(&self.manifest).ok()
    }

    /// Droits accordés, à donner à la bulle (seulement ceux que le manifeste demande).
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(crate) fn granted(&self) -> Vec<Capability> {
        if self.fichiers {
            vec![Capability::ReadFiles, Capability::WriteFiles]
        } else {
            vec![]
        }
    }

    /// ⚙ Valeurs passées au module (`Input.settings`), dans l'ordre du manifeste : la valeur choisie si le manifeste
    /// la permet, sinon celle par défaut.
    pub(crate) fn settings(&self) -> Vec<i64> {
        self.manifest()
            .map(|m| {
                m.settings
                    .iter()
                    .map(|s| match self.values.get(&s.id).map(|v| v.round() as i64) {
                        Some(v) if s.allows(v) => v,
                        _ => s.default_value(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Des droits jamais présentés (nouveau module, ou nouvelle version qui en ajoute) ?
    pub(crate) fn pending(&self) -> bool {
        self.manifest().is_some_and(|m| m.rights_text().iter().any(|r| !self.asked.contains(r)))
    }
}

/// État de la page et des modules qui tournent.
#[derive(Default)]
pub(crate) struct ModState {
    pub(crate) list: Vec<ModEntry>,
    /// Détail ouvert, fenêtre des droits ouverte, « retirer » armé (identifiants).
    pub(crate) open: Option<String>,
    pub(crate) rights_for: Option<String>,
    pub(crate) remove_armed: bool,
    /// Dernier résultat par module : (clé de l'entrée, sortie ou erreur), et la dernière sortie réussie.
    pub(crate) out: HashMap<String, (u64, Result<ModOut, String>)>,
    pub(crate) good: HashMap<String, ModOut>,
    /// 🌐 Navigateur : requêtes en cours (requête → module, clé, nombre de bougies), modules déjà envoyés à la page,
    /// choix de fichiers.
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) pending: HashMap<String, (String, u64, usize)>,
    pub(crate) sent: std::collections::HashSet<String>,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) picking: bool,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
    pub(crate) seq: u64,
    /// ⚙ Fenêtre des réglages ouverte (module), et le réglage en cours de saisie (module, réglage).
    pub(crate) settings_for: Option<String>,
    pub(crate) editing: Option<(String, String)>,
    /// 🛒 Modules importés d'un catalogue local (gardés dans `modules-catalogue.json`), et celui dont la fiche est
    /// ouverte (index dans `App::catalog`).
    pub(crate) catalog: Vec<crate::actions::catalog::CatalogItem>,
    pub(crate) cat_open: Option<usize>,
    /// 🖥️ La bulle du kit (créée au premier appel).
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) engine: Option<argos_module_host::engine::Engine>,
}

const EXAMPLE: i64 = 1;
const FROM_FILE: i64 = 2;
const START: i64 = 10;
const STOP: i64 = 11;
const RIGHTS: i64 = 12;
const REMOVE: i64 = 13;
const BACK: i64 = 14;
const R_FILES: i64 = 31;
const R_OK: i64 = 32;
const SETTINGS: i64 = 15;
const SET_RESET: i64 = 40;
const SET_CLOSE: i64 = 41;
const SET0: i64 = 50;
/// Couleurs proposées (touche = la suivante) : bleu, orange, sarcelle, rose, violet, or, blanc.
const PRESETS: [i64; 7] = [0x2962ff, 0xff9800, 0x26a69a, 0xe91e63, 0xab47bc, 0xf0b90b, 0xffffff];
const MOD0: i64 = 1000;
const CAT_INSTALL: i64 = 70;
const CAT_BACK: i64 = 71;
const CAT_IMPORT: i64 = 72;
const CAT_EXPORT: i64 = 73;
const CAT0: i64 = 500;

/// Nom sûr tiré de l'identifiant d'un module.
fn safe(id: &str) -> String {
    id.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' })
        .take(60)
        .collect()
}

fn wasm_key(id: &str) -> String {
    format!("modules/{}.wasm.b64", safe(id))
}

fn b64() -> base64::engine::GeneralPurpose {
    base64::engine::general_purpose::STANDARD
}

fn to_hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[cfg_attr(target_arch = "wasm32", allow(dead_code))]
fn from_hex48(s: &str) -> Option<[u8; 48]> {
    let v: Vec<u8> = (0..s.len()).step_by(2).filter_map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok()).collect();
    v.try_into().ok()
}

/// Couleurs des courbes d'un module (une par courbe, en boucle) : bleu, orange, violet, sarcelle.
const COLORS: [u32; 4] = [0x2962ff, 0xff9800, 0xab47bc, 0x26a69a];

/// Ce qu'un module a rendu → un dessin de script sur la chart (sur le prix, une couleur par courbe).
fn view(e: &ModEntry, out: &ModOut, n: usize, error: Option<String>) -> argos_render::scriptview::ScriptView {
    use argos_script::lang::{Plot, PlotStyle};
    let set = e.settings();
    // ⚙ k-ième réglage « couleur » du module pour la k-ième courbe, sinon les couleurs d'Argos
    let colors: Vec<u32> = e
        .manifest()
        .map(|m| {
            m.settings
                .iter()
                .zip(&set)
                .filter(|(s, _)| matches!(s.kind, SettingKind::Color { .. }))
                .map(|(_, v)| *v as u32)
                .collect()
        })
        .unwrap_or_default();
    let plots = lines_of(out)
        .into_iter()
        .enumerate()
        .map(|(k, (name, mut values))| {
            values.resize(n, f64::NAN); // une valeur par bougie, ni plus ni moins
            Plot {
                title: name,
                values,
                colors: vec![Some(colors.get(k).copied().unwrap_or(COLORS[k % COLORS.len()])); n],
                width: 2.0,
                style: PlotStyle::Line,
                offset: 0,
                visible: true,
            }
        })
        .collect();
    argos_render::scriptview::ScriptView {
        name: format!("🧩 {}", e.name()),
        out: argos_script::lang::Output {
            title: format!("🧩 {}", e.name()),
            short: format!("🧩 {}", e.name()),
            overlay: true,
            plots,
            ..Default::default()
        },
        hidden: e.hidden,
        error,
        style: Default::default(),
        id: Default::default(),
    }
}

/// Texte d'une valeur de réglage.
fn setting_text(s: &SettingSpec, v: i64) -> String {
    match &s.kind {
        SettingKind::Number { scale, .. } => argos_module_host::fixed(v, *scale),
        SettingKind::Bool { .. } => (if v != 0 { "oui" } else { "non" }).into(),
        SettingKind::Color { .. } => format!("#{:06x}", v),
        SettingKind::Choice { choices, .. } => choices.get(v as usize).cloned().unwrap_or_default(),
    }
}

impl App {
    pub(crate) fn mods_save(&self) {
        Self::write_prefs("modules.json", &self.mods.list);
    }

    /// Le `.wasm` rangé d'un module.
    pub(crate) fn mod_wasm(id: &str) -> Option<Vec<u8>> {
        use base64::Engine;
        let raw = platform::read(&wasm_key(id)).filter(|s| !s.is_empty())?;
        b64().decode(raw.trim()).ok()
    }

    /// 🖌 « Pinceau de zone » (clic droit sur un rectangle) : le module Pinceau de zone (installé s'il ne l'est pas
    /// encore) reçoit le début, la fin, le haut et le bas du rectangle en réglages et démarre sur la chart ; ses courbes
    /// (POC, VAH, VAL, VWAP) se dessinent sur les bougies de la zone, ses chiffres sont dans 🧩 Modules.
    pub(crate) fn pinceau_zone(&mut self, i: usize) {
        let Some(&[a, b]) = self.chart.drawings.get(i).filter(|d| d.tool == "rect").map(|d| d.pts.as_slice()) else {
            return;
        };
        let id = format!("{}pinceau-zone", argos_module_host::PREFIX);
        if !self.mods.list.iter().any(|e| e.id == id) {
            if let Err(e) = self.mod_install(PINCEAU_JSON, PINCEAU_WASM) {
                self.banner = Some(format!("🖌 Pinceau de zone indisponible : {e}"));
                return self.redraw();
            }
        }
        let Some(e) = self.mods.list.iter_mut().find(|e| e.id == id) else { return };
        // les prix des réglages ont 6 décimales (argos-module.json)
        let prix = |p: f64| (p * 1e6).round();
        for (k, v) in [
            ("debut", a.t.min(b.t) as f64),
            ("fin", a.t.max(b.t) as f64),
            ("haut", prix(a.p.max(b.p))),
            ("bas", prix(a.p.min(b.p))),
        ] {
            e.values.insert(k.into(), v);
        }
        e.on = true;
        self.mod_forget(&id);
        self.mods_save();
        self.banner = Some("🖌 Pinceau de zone : POC, VAH, VAL et VWAP dans la zone ; volume et delta estimé dans 🧩 Modules".into());
        self.redraw();
    }

    /// Oublie ce qui tourne pour `id` (recalculé au prochain tour).
    fn mod_forget(&mut self, id: &str) {
        self.mods.out.remove(id);
        self.mods.sent.retain(|k| !k.starts_with(&format!("{id}@")));
        self.ind.key = Default::default(); // les courbes sont recalculées
    }

    /// Installe (ou met à jour) un module du kit : manifeste vérifié (refusé par défaut, réseau / IA refusés),
    /// `.wasm` vérifié (interface v2) ; son empreinte SHA-384 est l'accord. Une mise à jour garde réglages, état et
    /// dossier privé ; un droit nouveau est refusé jusqu'à l'accord.
    pub(crate) fn mod_install(&mut self, json: &str, wasm: &[u8]) -> Result<String, String> {
        use base64::Engine;
        let m = Manifest::parse(json)?;
        m.check()?;
        scan(wasm)?;
        platform::write(&wasm_key(&m.path), &b64().encode(wasm)).map_err(|e| format!("rangement impossible ({e})"))?;
        let (rights, sha, version) = (m.rights_text(), to_hex(&sha384(wasm)), m.version.to_string());
        let msg = match self.mods.list.iter_mut().find(|e| e.id == m.path) {
            Some(e) => {
                let old = e.version.clone();
                // 🔐 un droit nouveau : le dossier privé redevient refusé jusqu'à l'accord
                if rights.iter().any(|r| !e.asked.contains(r)) {
                    e.fichiers = false;
                }
                (e.version, e.manifest, e.sha) = (version.clone(), json.to_string(), sha);
                format!("🧩 {} mis à jour : v{old} → v{version} (réglages et dossier privé gardés)", e.name())
            }
            None => {
                let e = ModEntry {
                    id: m.path.clone(),
                    version: version.clone(),
                    manifest: json.to_string(),
                    sha,
                    ..Default::default()
                };
                let msg = format!("🧩 {} v{version} installé (arrêté) : ouvre-le pour le démarrer", e.name());
                self.mods.list.push(e);
                msg
            }
        };
        self.mod_forget(&m.path);
        self.mods.open = Some(m.path.clone());
        if self.mods.list.iter().any(|e| e.id == m.path && e.pending()) {
            self.mods.rights_for = Some(m.path.clone());
        }
        self.mods_save();
        Ok(msg)
    }

    /// 📂 Installe les paires `nom.wasm` + `nom.json` (ou `argos-module.json`) trouvées.
    pub(crate) fn mod_install_files(&mut self, files: &[(String, Vec<u8>)]) -> String {
        let mut msgs = vec![];
        // 🛒 un fichier catalogue choisi : importé (ses modules sont vérifiés un par un)
        for (_, b) in files.iter().filter(|(n, _)| n.ends_with(".json")) {
            let text = String::from_utf8_lossy(b);
            if text.contains(crate::actions::catalog::FORMAT) {
                msgs.push(self.catalog_import(&text));
            }
        }
        for (name, wasm) in files.iter().filter(|(n, _)| n.ends_with(".wasm")) {
            let stem = name.trim_end_matches(".wasm");
            let json = files
                .iter()
                .find(|(n, _)| n == &format!("{stem}.json") || n == "argos-module.json")
                .and_then(|(_, b)| String::from_utf8(b.clone()).ok());
            msgs.push(match json {
                None => format!("{name} : il manque son manifeste {stem}.json (argos-module.json)"),
                Some(t) => self.mod_install(&t, wasm).unwrap_or_else(|e| format!("{name} refusé : {e}")),
            });
        }
        if msgs.is_empty() {
            "🧩 Aucun module trouvé : il faut le fichier .wasm et son manifeste .json (même nom)".into()
        } else {
            msgs.join(" · ")
        }
    }

    /// Menu 🧩 Modules.
    pub(crate) fn modules_menu(&self) -> Vec<MenuGroup> {
        let item = |id: i64, label: String| MenuItem {
            id,
            label,
            enabled: true,
            ..Default::default()
        };
        let info = |label: String| MenuItem {
            id: -1,
            label,
            enabled: false,
            ..Default::default()
        };
        let mut groups = vec![];
        let entry = |id: &Option<String>| id.as_ref().and_then(|id| self.mods.list.iter().find(|e| &e.id == id));
        // 🔐 fenêtre des droits : chaque catégorie acceptée ou refusée ; ce qui est refusé le reste
        if let Some(e) = entry(&self.mods.rights_for) {
            let m = e.manifest();
            let mut items = vec![];
            if let Some(m) = &m {
                if !m.rights_text().is_empty() {
                    items.push(item(
                        R_FILES,
                        format!("{} 📁 Lire et écrire dans son dossier privé", if e.fichiers { "✅" } else { "⛔" }),
                    ));
                }
                items.push(info("⛔ 🌐 Réseau, ✨ IA : jamais (l'app fournit les données au module)".into()));
                if !m.settings.is_empty() {
                    items.push(info(format!("⚙ {} réglage(s) : ce ne sont pas des droits", m.settings.len())));
                }
            }
            items.push(item(R_OK, "✔️ Valider mes choix".into()));
            groups.push(MenuGroup {
                title: format!("🔐 DROITS DEMANDÉS PAR {} · touche pour accepter / refuser", e.name().to_uppercase()),
                items,
                strip: false,
            });
        }
        // ⚙ réglages du module : générés depuis son manifeste (kit : entiers à leur échelle)
        if let Some(e) = entry(&self.mods.settings_for) {
            let m = e.manifest();
            let vals = e.settings();
            let mut items: Vec<MenuItem> = m
                .iter()
                .flat_map(|m| m.settings.iter())
                .zip(&vals)
                .enumerate()
                .map(|(k, (s, v))| {
                    let (label, color) = match &s.kind {
                        SettingKind::Number { .. } => (format!("{} : {}  ✎", s.name, setting_text(s, *v)), None),
                        SettingKind::Bool { .. } => (format!("{} {}", if *v != 0 { "☑" } else { "☐" }, s.name), None),
                        SettingKind::Color { .. } => (
                            format!("● {} : {}  (touche = la suivante)", s.name, setting_text(s, *v)),
                            Some(argos_render::text::Rgb((*v >> 16) as u8, (*v >> 8) as u8, *v as u8)),
                        ),
                        SettingKind::Choice { .. } => (format!("{} : {}  ▸", s.name, setting_text(s, *v)), None),
                    };
                    MenuItem {
                        id: SET0 + k as i64,
                        label,
                        enabled: true,
                        color,
                        ..Default::default()
                    }
                })
                .collect();
            items.push(item(SET_RESET, "↺ Valeurs par défaut".into()));
            items.push(item(SET_CLOSE, "✔ Fermer les réglages".into()));
            groups.push(MenuGroup {
                title: format!("⚙ RÉGLAGES DE {} · gardés à la mise à jour", e.name().to_uppercase()),
                items,
                strip: false,
            });
        }
        // 🔎 détail d'un module
        if let Some(e) = entry(&self.mods.open) {
            let m = e.manifest();
            let rights = m.as_ref().map(|m| m.rights_text()).unwrap_or_default();
            let mut items = vec![info(format!(
                "{} v{} · {}",
                e.id,
                e.version,
                m.as_ref()
                    .map_or("ancien format : réinstalle le module".into(), |m| format!("par {}", m.author))
            ))];
            if rights.is_empty() {
                items.push(info("🔐 Aucun droit demandé : ni réseau, ni fichiers, ni IA".into()));
            } else {
                for r in &rights {
                    items.push(info(format!("{} {r}", if e.fichiers { "✅" } else { "⛔" })));
                }
            }
            match self.mods.out.get(&e.id).map(|(_, r)| r) {
                Some(Err(err)) => items.push(info(format!("⚠️ {err}"))),
                Some(Ok(o)) => items.extend(cards_of(o).into_iter().map(|(k, v)| info(format!("📈 {k} · {v}")))),
                None => {}
            }
            items.push(if e.on {
                item(STOP, "⏸️ Arrêter".into())
            } else {
                item(START, "▶ Démarrer sur la chart".into())
            });
            if !rights.is_empty() {
                items.push(item(RIGHTS, "🔐 Droits…".into()));
            }
            if m.as_ref().is_some_and(|m| !m.settings.is_empty()) {
                items.push(item(SETTINGS, "⚙ Réglages…".into()));
            }
            items.push(item(
                REMOVE,
                if self.mods.remove_armed {
                    format!("🗑 Confirmer : retirer {} (son dossier privé est gardé)", e.name())
                } else {
                    "🗑 Retirer".into()
                },
            ));
            items.push(item(BACK, "↩ Retour à la liste".into()));
            groups.push(MenuGroup {
                title: format!("🔎 {}", e.name().to_uppercase()),
                items,
                strip: false,
            });
        }
        // 🛒 catalogue local : la fiche d'un module (droits montrés avant l'installation), puis la liste
        let cat = self.catalog();
        if let Some((it, m)) = self.mods.cat_open.and_then(|k| cat.get(k)) {
            use crate::actions::catalog::Status;
            let mut items = vec![info(it.description.clone()), info(format!("{} v{} · par {}", m.path, m.version, m.author))];
            let rights = m.rights_text();
            if rights.is_empty() {
                items.push(info("🔐 Aucun droit : ni réseau, ni fichiers, ni IA".into()));
            } else {
                items.extend(rights.iter().map(|r| info(format!("🔐 Demande : {r} (refusé tant que tu ne l'acceptes pas)"))));
            }
            if !m.settings.is_empty() {
                items.push(info(format!("⚙ {} réglage(s) (pas des droits)", m.settings.len())));
            }
            let short = format!("{}…{}", &it.sha384[..8.min(it.sha384.len())], &it.sha384[it.sha384.len().saturating_sub(8)..]);
            items.push(info(match it.verify() {
                Ok(_) => format!("🔏 Empreinte SHA-384 vérifiée ({short}) · kit 1.0.0 · pas encore de signature"),
                Err(e) => format!("⛔ {e}"),
            }));
            items.push(match self.catalog_status(m) {
                Status::New => item(CAT_INSTALL, "+ Installer".into()),
                Status::Update(a, b) => item(CAT_INSTALL, format!("⬆ Mettre à jour : v{a} → v{b} (réglages gardés)")),
                Status::Installed(v) => info(format!("✅ Déjà installé (v{v}) : retire-le depuis sa fiche dans 🧩 MODULES")),
            });
            items.push(item(CAT_BACK, "↩ Retour au catalogue".into()));
            groups.push(MenuGroup {
                title: format!("🛒 {}", it.title.to_uppercase()),
                items,
                strip: false,
            });
        }
        groups.push(MenuGroup {
            title: format!("🛒 CATALOGUE LOCAL ({}) · toucher = fiche et droits", cat.len()),
            items: cat
                .iter()
                .enumerate()
                .map(|(k, (it, m))| {
                    use crate::actions::catalog::Status;
                    let st = match self.catalog_status(m) {
                        Status::New => "+".to_string(),
                        Status::Update(_, b) => format!("⬆ v{b}"),
                        Status::Installed(_) => "✅".to_string(),
                    };
                    MenuItem {
                        id: CAT0 + k as i64,
                        label: format!("{st} {} · v{}", it.title, m.version),
                        hint: if m.rights_text().is_empty() {
                            "aucun droit".into()
                        } else {
                            format!("{} droit(s)", m.rights_text().len())
                        },
                        enabled: true,
                        ..Default::default()
                    }
                })
                .collect(),
            strip: false,
        });
        let mut list: Vec<MenuItem> = self
            .mods
            .list
            .iter()
            .enumerate()
            .map(|(k, e)| {
                let n = e.manifest().map_or(0, |m| m.rights_text().len());
                MenuItem {
                    id: MOD0 + k as i64,
                    label: format!(
                        "{} {} v{}{}",
                        if e.on { "▶" } else { "⏸️" },
                        e.name(),
                        e.version,
                        if e.pending() { " · 🔐 droits à valider" } else { "" }
                    ),
                    hint: if n == 0 { "aucun droit".into() } else { format!("{n} droit(s) demandé(s)") },
                    enabled: true,
                    ..Default::default()
                }
            })
            .collect();
        if list.is_empty() {
            list.push(info("Aucun module installé : essaie le module d'exemple ci-dessous".into()));
        }
        groups.push(MenuGroup {
            title: format!("🧩 MODULES ({}) · bac à sable, aucun droit par défaut", self.mods.list.len()),
            items: list,
            strip: false,
        });
        groups.push(MenuGroup {
            title: String::new(),
            items: vec![
                item(EXAMPLE, "📦 Installer le module d'exemple (Moyenne mobile)".into()),
                item(
                    FROM_FILE,
                    if cfg!(target_arch = "wasm32") {
                        "📂 Installer un module : choisir le .wasm et son .json".into()
                    } else {
                        "📂 Installer un module (.wasm + .json dans Téléchargements)".into()
                    },
                ),
                item(
                    CAT_IMPORT,
                    if cfg!(target_arch = "wasm32") {
                        "📂 Importer un catalogue (choisir argos-catalogue.json)".into()
                    } else {
                        "📂 Importer un catalogue (argos-catalogue.json dans Téléchargements)".into()
                    },
                ),
                item(CAT_EXPORT, "💾 Exporter mes modules en catalogue (pour les donner)".into()),
                info("🔑 Kit module Argos 1.0.0 · jamais de clé · aucun réseau · empreinte SHA-384 vérifiée".into()),
            ],
            strip: false,
        });
        groups
    }

    /// Ligne choisie : `true` = le menu reste ouvert (refait).
    pub(crate) fn modules_pick(&mut self, id: i64) -> bool {
        let open = self.mods.open.clone();
        let idx = |s: &Self, id: &Option<String>| id.as_ref().and_then(|id| s.mods.list.iter().position(|e| &e.id == id));
        if id != REMOVE {
            self.mods.remove_armed = false;
        }
        match id {
            EXAMPLE => {
                self.banner = Some(self.mod_install(EXAMPLE_JSON, EXAMPLE_WASM).unwrap_or_else(|e| format!("🧩 {e}")));
            }
            FROM_FILE => {
                #[cfg(target_arch = "wasm32")]
                {
                    self.mods.picking = true;
                    platform::web::call_page("argosPick", &[".wasm,.json"]);
                    self.banner = Some("📂 Choisis le fichier .wasm du module et son manifeste .json (les deux)".into());
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let names: Vec<String> = [".wasm", ".json"].iter().flat_map(|x| platform::list_downloads(x)).collect();
                    let files: Vec<(String, Vec<u8>)> = names.into_iter().filter_map(|n| platform::read_download_bytes(&n).map(|b| (n, b))).collect();
                    self.banner = Some(self.mod_install_files(&files));
                }
            }
            START | STOP => {
                if let Some(k) = idx(self, &open) {
                    if id == START && self.mods.list[k].pending() {
                        self.mods.rights_for = open;
                        self.banner = Some("🔐 Valide d'abord ses droits (accepte ou refuse chacun)".into());
                        return true;
                    }
                    self.mods.list[k].on = id == START;
                    let e = self.mods.list[k].clone();
                    self.mod_forget(&e.id);
                    self.banner = Some(format!(
                        "🧩 {} {}",
                        e.name(),
                        if e.on { "démarré : ses courbes sont sur la chart" } else { "arrêté" }
                    ));
                    self.mods_save();
                }
            }
            RIGHTS => self.mods.rights_for = open,
            R_FILES => {
                if let Some(k) = idx(self, &self.mods.rights_for.clone()) {
                    self.mods.list[k].fichiers = !self.mods.list[k].fichiers;
                }
            }
            R_OK => {
                let rf = self.mods.rights_for.take();
                if let Some(k) = idx(self, &rf) {
                    let e = &mut self.mods.list[k];
                    e.asked = e.manifest().map(|m| m.rights_text()).unwrap_or_default();
                    let id = e.id.clone();
                    let web = if cfg!(target_arch = "wasm32") && e.fichiers {
                        " (dans le navigateur, le dossier privé reste refusé pour l'instant)"
                    } else {
                        ""
                    };
                    self.banner = Some(format!("🔐 Droits enregistrés : ce que tu as refusé reste refusé{web}"));
                    self.mod_forget(&id);
                    self.mods_save();
                }
            }
            REMOVE => {
                if !self.mods.remove_armed {
                    self.mods.remove_armed = true;
                    return true;
                }
                self.mods.remove_armed = false;
                if let Some(k) = idx(self, &open) {
                    let e = self.mods.list.remove(k);
                    let _ = platform::write(&wasm_key(&e.id), ""); // le .wasm part ; le dossier privé reste
                    self.mod_forget(&e.id);
                    self.mods.open = None;
                    self.banner = Some(format!("🗑 {} retiré (son dossier privé est gardé)", e.name()));
                    self.mods_save();
                }
            }
            BACK => {
                self.mods.open = None;
                self.mods.rights_for = None;
                self.mods.settings_for = None;
            }
            SETTINGS => self.mods.settings_for = open,
            SET_CLOSE => self.mods.settings_for = None,
            SET_RESET => {
                if let Some(k) = idx(self, &self.mods.settings_for.clone()) {
                    self.mods.list[k].values.clear();
                    self.mod_settings_changed(k, "⚙ Valeurs par défaut remises".into());
                }
            }
            s if (SET0..SET0 + 16).contains(&s) => return self.mod_setting_pick((s - SET0) as usize),
            CAT_BACK => self.mods.cat_open = None,
            CAT_INSTALL => {
                if let Some((it, _)) = self.mods.cat_open.and_then(|k| self.catalog().into_iter().nth(k)) {
                    self.banner = Some(match it.verify().and_then(|(_, wasm)| self.mod_install(&it.manifest, &wasm)) {
                        Ok(msg) => msg,
                        Err(e) => format!("🛒 Installation refusée : {e}"),
                    });
                    self.mods.cat_open = None;
                }
            }
            CAT_IMPORT => {
                #[cfg(target_arch = "wasm32")]
                {
                    self.mods.picking = true;
                    platform::web::call_page("argosPick", &[".json"]);
                    self.banner = Some(format!("📂 Choisis le fichier {} du catalogue", crate::actions::catalog::FILE));
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let file = crate::actions::catalog::FILE;
                    self.banner = Some(match platform::read_download(file) {
                        Some(text) => self.catalog_import(&text),
                        None => format!("🛒 Mets {file} dans tes Téléchargements, puis réessaie"),
                    });
                }
            }
            CAT_EXPORT => {
                let file = self.catalog_file();
                let body = serde_json::to_string_pretty(&file).unwrap_or_default();
                let name = crate::actions::catalog::FILE;
                self.banner = Some(match platform::save_download(name, body.as_bytes()) {
                    Ok(dir) => format!(
                        "💾 {} module(s) dans {dir}/{name} : donne ce fichier, il s'importe dans 🧩 Modules",
                        file.modules.len()
                    ),
                    Err(e) => format!("Écriture impossible ({e})"),
                });
            }
            k if (CAT0..MOD0).contains(&k) => {
                self.mods.cat_open = Some((k - CAT0) as usize);
                self.mods.open = None;
                self.mods.rights_for = None;
            }
            k if k >= MOD0 => {
                self.mods.open = self.mods.list.get((k - MOD0) as usize).map(|e| e.id.clone());
                self.mods.rights_for = None;
            }
            _ => {}
        }
        true
    }

    /// ⚙ Touche sur un réglage : case = cocher, couleur = la suivante, choix = le suivant, nombre = boîte de saisie.
    /// `true` = le menu reste ouvert.
    fn mod_setting_pick(&mut self, k: usize) -> bool {
        let Some(i) = self.mods.settings_for.as_ref().and_then(|id| self.mods.list.iter().position(|e| &e.id == id)) else {
            return true;
        };
        let Some(s) = self.mods.list[i].manifest().and_then(|m| m.settings.get(k).cloned()) else {
            return true;
        };
        let cur = self.mods.list[i].settings().get(k).copied().unwrap_or(0);
        let next = match &s.kind {
            SettingKind::Bool { .. } => i64::from(cur == 0),
            SettingKind::Choice { choices, .. } => (cur + 1) % choices.len().max(1) as i64,
            SettingKind::Color { .. } => {
                let at = PRESETS.iter().position(|c| *c == cur).map_or(0, |p| p + 1);
                PRESETS[at % PRESETS.len()]
            }
            SettingKind::Number { .. } => {
                // la boîte de saisie s'ouvre ; la valeur arrive dans `mod_number_typed`
                self.mods.editing = Some((self.mods.list[i].id.clone(), s.id.clone()));
                let anchor = Rect::new(self.rw() + 60.0, self.top_h() + 60.0, 40.0, 24.0);
                self.ui.name_box = Some(NameBox::new(NameFor::ModNumber, &setting_text(&s, cur), anchor, self.win_w()));
                self.ui.list = None;
                return false;
            }
        };
        self.mods.list[i].values.insert(s.id.clone(), next as f64);
        self.mod_settings_changed(i, format!("⚙ {} : {}", s.name, setting_text(&s, next)));
        true
    }

    /// ⚙ Valeur tapée pour un réglage « nombre » : mise à son échelle, bornée par le manifeste.
    pub(crate) fn mod_number_typed(&mut self, value: &str) {
        let Some((id, sid)) = self.mods.editing.take() else { return };
        let Some(i) = self.mods.list.iter().position(|e| e.id == id) else { return };
        let Some(s) = self.mods.list[i].manifest().and_then(|m| m.settings.into_iter().find(|s| s.id == sid)) else {
            return;
        };
        let SettingKind::Number { min, max, scale, .. } = s.kind else { return };
        let Ok(x) = value.trim().replace(',', ".").replace(' ', "").parse::<f64>() else {
            self.banner = Some(format!("⚙ {} : écris un nombre", s.name));
            return;
        };
        let v = ((x * 10f64.powi(scale as i32)).round() as i64).clamp(min, max);
        self.mods.list[i].values.insert(sid, v as f64);
        self.mod_settings_changed(i, format!("⚙ {} = {}", s.name, setting_text(&s, v)));
    }

    /// ⚙ Après un changement de réglage : gardé, et le module tourne de nouveau (courbes recalculées).
    fn mod_settings_changed(&mut self, i: usize, msg: String) {
        let id = self.mods.list[i].id.clone();
        self.mods.out.remove(&id);
        self.ind.key = Default::default();
        self.banner = Some(msg);
        self.mods_save();
        self.redraw();
    }

    /// Légende de la chart, sur la courbe d'un module (`k` : parmi les modules démarrés).
    pub(crate) fn mod_legend(&mut self, c: argos_render::indleg::LegClick, k: usize) {
        use argos_render::indleg::LegClick;
        let Some(i) = self.mods.list.iter().enumerate().filter(|(_, e)| e.on).nth(k).map(|(i, _)| i) else {
            return;
        };
        match c {
            LegClick::ScriptEye(_) => self.mods.list[i].hidden = !self.mods.list[i].hidden,
            LegClick::ScriptRemove(_) => self.mods.list[i].on = false,
            _ => {
                self.mods.open = Some(self.mods.list[i].id.clone());
                self.open_list(ListKind::Modules, Rect::new(self.rw() + 40.0, self.top_h() + 8.0, 40.0, 24.0));
            }
        }
        self.ind.key = Default::default();
        self.mods_save();
        self.redraw();
    }

    /// Fait tourner les modules démarrés sur les bougies de la chart ; un module en erreur garde son dernier dessin.
    pub(crate) fn run_modules(&mut self) -> Vec<argos_render::scriptview::ScriptView> {
        use std::hash::{Hash, Hasher};
        let on: Vec<ModEntry> = self.mods.list.iter().filter(|e| e.on).cloned().collect();
        if on.is_empty() {
            return vec![];
        }
        let raw: Vec<argos_module_host::RawBar> = self.chart.bars.iter().map(|b| (b.time, b.open, b.high, b.low, b.close, b.volume)).collect();
        let n = raw.len();
        let (symbol, tf) = (self.symbol_label(), self.chart.interval.secs());
        let mut views = vec![];
        for e in on {
            // 📐 entiers à l'échelle, arrondis une seule fois ; 🪟 fenêtre : les bougies les plus récentes qui tiennent
            // dans les limites du kit (50 000 bougies, 4 Mio une fois encodées, taille réelle mesurée)
            let argos_module_host::Fenetre { input, laissees, .. } = argos_module_host::fenetre(&symbol, tf, &raw, e.settings());
            let mut h = std::collections::hash_map::DefaultHasher::new();
            (&e.id, &e.version, &e.sha, e.fichiers, &symbol, tf, n, &input.settings).hash(&mut h);
            if let Some(b) = input.bars.last() {
                (b.t, b.c, input.price_scale).hash(&mut h);
            }
            let key = h.finish();
            if self.mods.out.get(&e.id).is_none_or(|(k, _)| *k != key) {
                self.mod_exec(&e, key, &input);
            }
            let res = self.mods.out.get(&e.id).map(|(_, r)| r.clone());
            if let Some(Ok(o)) = &res {
                self.mods.good.insert(e.id.clone(), o.clone());
            }
            let err = match &res {
                Some(Err(x)) => Some(x.clone()),
                _ => None,
            };
            // la sortie couvre les dernières bougies (au plus 50 000) : alignée à droite sur la chart
            let shown = self.mods.good.get(&e.id).cloned().unwrap_or_default();
            let mut v = view(&e, &shown, input.bars.len(), err);
            // 🪟 l'historique ne tient pas en entier : la légende le dit
            if laissees > 0 {
                let note = format!(" · {} dernières bougies", input.bars.len());
                let t = if !v.out.short.is_empty() {
                    &mut v.out.short
                } else if !v.out.title.is_empty() {
                    &mut v.out.title
                } else {
                    &mut v.name
                };
                t.push_str(&note);
            }
            let pad = n.saturating_sub(input.bars.len());
            for p in &mut v.out.plots {
                let mut full = vec![f64::NAN; pad];
                full.append(&mut p.values);
                p.values = full;
                p.colors = vec![p.colors.first().copied().flatten(); n];
            }
            views.push(v);
        }
        views
    }

    /// 🖥️ Un appel du module, tout de suite, dans la bulle du kit.
    #[cfg(not(target_arch = "wasm32"))]
    fn mod_exec(&mut self, e: &ModEntry, key: u64, input: &argos_module_host::Input) {
        let res = (|| -> Result<ModOut, String> {
            if self.mods.engine.is_none() {
                self.mods.engine = Some(argos_module_host::engine::Engine::new()?);
            }
            let m = e.manifest().ok_or("ancien format de manifeste : réinstalle le module")?;
            let wasm = Self::mod_wasm(&e.id).ok_or("fichier du module absent : réinstalle-le")?;
            let approved = from_hex48(&e.sha).ok_or("accord du module absent : réinstalle-le")?;
            let root = crate::persist::saved::dir().map(|d| d.join("modules")).ok_or("pas de dossier de réglages")?;
            // 🔑 aucun module ne touche jamais au dossier des clés de signature
            let protected: Vec<std::path::PathBuf> = std::env::var_os("HOME")
                .map(|h| std::path::PathBuf::from(h).join(".argos-signature"))
                .into_iter()
                .collect();
            let engine = self.mods.engine.as_ref().ok_or("bulle indisponible")?;
            engine.run(&m, &wasm, &approved, &e.granted(), &root, &protected, input)
        })();
        self.mods.out.insert(e.id.clone(), (key, res));
    }

    /// 🌐 Un appel du module : l'entrée au format du kit va au Worker de la page ; la réponse arrive dans `mods_tick`.
    #[cfg(target_arch = "wasm32")]
    fn mod_exec(&mut self, e: &ModEntry, key: u64, input: &argos_module_host::Input) {
        use base64::Engine;
        if self.mods.pending.values().any(|(id, k, _)| id == &e.id && *k == key) {
            return;
        }
        let fail = |s: &mut Self, msg: &str| {
            s.mods.out.insert(e.id.clone(), (key, Err(msg.to_string())));
        };
        let Some(m) = e.manifest() else {
            return fail(self, "ancien format de manifeste : réinstalle le module");
        };
        let mkey = format!("{}@{}", e.id, e.sha);
        let wasm = if self.mods.sent.contains(&mkey) {
            String::new()
        } else {
            match Self::mod_wasm(&e.id) {
                // l'empreinte acceptée à l'installation : un autre fichier ne part pas
                Some(w) if to_hex(&sha384(&w)) == e.sha => b64().encode(w),
                Some(_) => return fail(self, "le fichier du module a changé depuis ton accord : réinstalle-le"),
                None => return fail(self, "fichier du module absent : réinstalle-le"),
            }
        };
        if let Err(x) = m.policy().check_values(&input.settings) {
            return fail(self, &format!("réglage refusé : {x}"));
        }
        let data = input.encode();
        self.mods.seq += 1;
        let req = format!("m{}", self.mods.seq);
        self.mods.pending.insert(req.clone(), (e.id.clone(), key, input.bars.len()));
        self.mods.sent.insert(mkey.clone());
        let ms = (m.budget_secs as u64 * 1000).to_string();
        // 📱 coque Tauri : le module tourne dans la bulle du kit Argos du cœur Rust (mêmes vérifications que le
        // desktop : manifeste, .wasm, empreinte acceptée ; aucun droit avant l'étape T2), pas dans le Web Worker
        if argos_data::coque::active() {
            let args = argos_data::coque::objet(&[
                ("cle", mkey.as_str().into()),
                ("wasm", wasm.as_str().into()),
                ("manifeste", e.manifest.as_str().into()),
                ("entree", b64().encode(data).into()),
            ]);
            wasm_bindgen_futures::spawn_local(async move {
                let (ok, v) = match argos_data::coque::invoke("module_lancer", &args).await {
                    Ok(v) => (true, v.as_string().unwrap_or_default()),
                    Err(e) => (false, e),
                };
                platform::web::push_mod_result(&req, ok, &v);
            });
            return;
        }
        platform::web::call_page("argosModRun", &[&req, &mkey, &wasm, &b64().encode(data), &ms]);
    }

    /// 🌐 Réponses de la page (modules du Worker, fichiers choisis) ; `true` = encore en attente (réveil bientôt).
    pub(crate) fn mods_tick(&mut self) -> bool {
        #[cfg(target_arch = "wasm32")]
        {
            use base64::Engine;
            let mut changed = false;
            for (req, ok, v) in platform::web::take_mod_results() {
                let Some((id, key, n)) = self.mods.pending.remove(&req) else { continue };
                let res = if ok {
                    // la sortie est vérifiée par le décodeur du kit (rien de partiellement lu)
                    b64()
                        .decode(v)
                        .map_err(|_| "sortie du module illisible".to_string())
                        .and_then(|b| ModOut::decode(&b, n).map_err(|e| format!("sortie du module refusée ({e:?})")))
                } else {
                    if v.contains("recharger") {
                        self.mods.sent.retain(|k| !k.starts_with(&format!("{id}@")));
                    }
                    Err(v)
                };
                self.mods.out.insert(id, (key, res));
                changed = true;
            }
            if self.mods.picking {
                let picked = platform::web::take_picked();
                if !picked.is_empty() {
                    let files: Vec<(String, Vec<u8>)> = picked.into_iter().filter_map(|(n, b)| b64().decode(b).ok().map(|b| (n, b))).collect();
                    self.mods.picking = false;
                    self.banner = Some(self.mod_install_files(&files));
                    if let Some((kind, menu)) = &self.ui.list {
                        if *kind == ListKind::Modules {
                            let anchor = menu.anchor;
                            let (ww, wh) = (self.win_w(), self.area.1 as f64);
                            self.ui.list = Some((
                                ListKind::Modules,
                                argos_render::listmenu::ListMenu::new(self.modules_menu(), vec![], anchor, ww, wh),
                            ));
                        }
                    }
                    changed = true;
                }
            }
            if changed {
                self.ind.key = Default::default();
                self.redraw();
            }
            !self.mods.pending.is_empty() || self.mods.picking
        }
        #[cfg(not(target_arch = "wasm32"))]
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_exemple_du_kit_valide() {
        let m = Manifest::parse(EXAMPLE_JSON).expect("manifeste d'exemple lisible");
        m.check().expect("exemple valide");
        assert!(m.rights_text().is_empty(), "le module d'exemple ne demande aucun droit");
        scan(EXAMPLE_WASM).expect("interface v2");
        assert_eq!(from_hex48(&to_hex(&sha384(EXAMPLE_WASM))), Some(sha384(EXAMPLE_WASM)));
    }

    #[test]
    fn droits_redemandes_et_reseau_refuse() {
        let files = EXAMPLE_JSON.replace("\"capabilities\": []", "\"capabilities\": [\"ReadFiles\"]");
        let e = ModEntry {
            id: "trading/moyenne".into(),
            manifest: files.clone(),
            fichiers: true,
            asked: vec!["📁 Lire des fichiers de son dossier privé".into()],
            ..Default::default()
        };
        assert!(!e.pending(), "droit déjà présenté");
        assert_eq!(e.granted(), vec![Capability::ReadFiles, Capability::WriteFiles]);
        // une version qui ajoute l'écriture : redemandé
        let e2 = ModEntry {
            manifest: files.replace("[\"ReadFiles\"]", "[\"ReadFiles\", \"WriteFiles\"]"),
            ..e.clone()
        };
        assert!(e2.pending());
        // 🌐 réseau et 🔑 champ de clé : refusés
        let net = EXAMPLE_JSON.replace(
            "\"capabilities\": []",
            "\"capabilities\": [\"Network\"], \"domains\": [\"api.hyperliquid.xyz\"]",
        );
        assert!(Manifest::parse(&net).unwrap().check().is_err());
        assert!(Manifest::parse(&EXAMPLE_JSON.replacen('{', "{ \"cles\": \"x\",", 1)).is_err());
        assert!(ModEntry::default().granted().is_empty(), "tout refusé par défaut");
    }
}
