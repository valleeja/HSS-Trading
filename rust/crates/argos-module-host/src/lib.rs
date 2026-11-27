//! 🧩 Modules d'Argos HSS Trading avec le **kit Argos 1.0.2** (interface v2 : données d'entrée,
//! sorties, réglages). Copie figée du kit dans `rust/vendor/kit-module-argos-1.0.2/` (empreintes vérifiées) : ce crate
//! n'est qu'un adaptateur.
//! - **Manifeste** `argos-module.json` du kit (`path = "trading/…"`, `abi: 2`, droits, réglages), lu et vérifié ici ;
//!   tout champ inconnu (une clé, un secret…) le fait refuser.
//! - **Droits** : refus par défaut ; seulement le dossier privé (lire / écrire). Réseau, IA et modèle sont refusés :
//!   l'app fournit les données (adaptation du kit v2 pour Trading).
//! - **Données** : bougies en entiers à l'échelle (`price_scale`, `volume_scale`), `EMPTY` = vide, au plus 4 Mio,
//!   au format binaire du kit (`Input::encode`, `Output::decode`) — le même code sur le desktop et dans le navigateur.
//! - **Desktop** : la bulle du kit (`argos_sandbox::Sandbox::run_data`, wasmtime sans WASI, carburant, mémoire, temps).
//! - **Navigateur** : un Web Worker de la page /app/ appelle les mêmes exports v2 ; `scan` vérifie le `.wasm` avant.

#![forbid(unsafe_code)]

pub use argos_module_policy::{fixed, sha384, Approval, Capability, ServicePolicy, SettingKind, SettingSpec};
pub use argos_module_sdk::{Bar, Card, Input, Label, Line, Output, EMPTY};

use serde::Deserialize;

/// Projet de ce dépôt : un module de Trading a un chemin `trading/…`.
pub const PREFIX: &str = "trading/";
/// Interface des modules qui tournent dans Trading (données d'entrée et sorties).
pub const ABI: u32 = 2;
/// Fonctions d'hôte du kit (module d'import `argos`) ; la bulle ne lie que celles des droits accordés.
pub const HOST_FNS: [&str; 5] = ["read_file", "write_file", "network_request", "ai_request", "model_request"];
/// Exports obligatoires d'un module v2.
pub const V2_EXPORTS: [&str; 5] = ["memory", "argos_abi", "alloc", "free", "run_data"];
/// Au plus 50 000 bougies et 4 Mio en entrée (limites du kit).
pub const MAX_BARS: usize = argos_module_sdk::data::MAX_BARS;

fn default_abi() -> u32 {
    1
}
fn default_budget() -> u32 {
    argos_module_policy::DEFAULT_BUDGET_SECS
}

/// Le manifeste `argos-module.json` du kit.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    /// `trading/nom`.
    pub path: String,
    /// Version entière qui monte toujours.
    pub version: u64,
    #[serde(default)]
    pub capabilities: Vec<Capability>,
    /// Argos ID de l'auteur.
    pub author: String,
    #[serde(default = "default_abi")]
    pub abi: u32,
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub subfolders: Option<Vec<String>>,
    #[serde(default)]
    pub ai_model: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default = "default_budget")]
    pub budget_secs: u32,
}

impl Manifest {
    /// Lit un `argos-module.json` (un champ inconnu : refusé).
    pub fn parse(text: &str) -> Result<Manifest, String> {
        serde_json::from_str(text).map_err(|e| format!("manifeste refusé : {e}"))
    }

    /// Nom court (après `trading/`).
    pub fn name(&self) -> &str {
        self.path.strip_prefix(PREFIX).unwrap_or(&self.path)
    }

    /// La politique du kit (réglages compris) : ce que la bulle applique.
    pub fn policy(&self) -> ServicePolicy {
        ServicePolicy {
            domains: self.domains.clone(),
            subfolders: self.subfolders.clone(),
            ai_model: self.ai_model.clone(),
            budget_secs: self.budget_secs,
            model: self.model.clone(),
            abi: self.abi,
            settings: self.settings.clone(),
        }
    }

    /// Vérifie les règles de Trading et celles du kit.
    pub fn check(&self) -> Result<(), String> {
        let name = self
            .path
            .strip_prefix(PREFIX)
            .ok_or(format!("chemin « {} » : doit commencer par « {PREFIX} »", self.path))?;
        if name.is_empty() || name.len() > 64 || !name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '/')) {
            return Err(format!("chemin « {} » : lettres, chiffres, - _ / seulement", self.path));
        }
        if self.abi != ABI {
            return Err(format!(
                "interface {} : Trading fait tourner les modules d'interface v2 (données d'entrée)",
                self.abi
            ));
        }
        for c in &self.capabilities {
            match c {
                Capability::ReadFiles | Capability::WriteFiles => {}
                Capability::Network => return Err("réseau refusé : dans Trading, l'app fournit les données au module".into()),
                Capability::Ai | Capability::Model => return Err("IA refusée : désactivée dans Trading pour l'instant".into()),
            }
        }
        self.policy().check(&self.capabilities).map_err(|e| format!("politique refusée : {e}"))
    }

    /// Droits demandés, écrits pour l'utilisateur (fenêtre d'accord).
    pub fn rights_text(&self) -> Vec<String> {
        self.capabilities
            .iter()
            .filter_map(|c| match c {
                Capability::ReadFiles => Some("📁 Lire des fichiers de son dossier privé".to_string()),
                Capability::WriteFiles => Some("📁 Écrire des fichiers dans son dossier privé".to_string()),
                _ => None,
            })
            .collect()
    }

    /// Valeurs par défaut des réglages, dans l'ordre.
    pub fn default_settings(&self) -> Vec<i64> {
        self.policy().default_settings()
    }
}

/// Pourquoi un module est refusé ou arrêté.
pub type Error = String;

/// Vérifie le `.wasm` lui-même, sans le faire tourner : imports seulement `argos::*` du kit, exports v2 présents,
/// pas d'export v1 `run`, une seule mémoire.
pub fn scan(wasm: &[u8]) -> Result<(), Error> {
    use wasmparser::{Parser, Payload, TypeRef};
    let bad = |e: wasmparser::BinaryReaderError| format!("WASM illisible : {}", e.message());
    let (mut exports, mut mems) = (Vec::<String>::new(), 0usize);
    for payload in Parser::new(0).parse_all(wasm) {
        match payload.map_err(bad)? {
            Payload::ImportSection(r) => {
                for imp in r.into_imports() {
                    let imp = imp.map_err(bad)?;
                    if imp.module != "argos" || !HOST_FNS.contains(&imp.name) || !matches!(imp.ty, TypeRef::Func(_)) {
                        return Err(format!("import refusé : {}::{}", imp.module, imp.name));
                    }
                }
            }
            Payload::MemorySection(r) => mems += r.count() as usize,
            Payload::ExportSection(r) => {
                for e in r {
                    exports.push(e.map_err(bad)?.name.to_string());
                }
            }
            _ => {}
        }
    }
    if let Some(miss) = V2_EXPORTS.iter().find(|n| !exports.iter().any(|e| e == *n)) {
        return Err(format!("interface : export « {miss} » absent"));
    }
    if exports.iter().any(|e| e == "run") {
        return Err("interface : un module v2 n'exporte pas « run »".into());
    }
    if mems != 1 {
        return Err("interface : une seule mémoire".into());
    }
    Ok(())
}

/// Décimales nécessaires pour écrire tous ces nombres en entiers (au plus `max`).
fn scale_of(values: impl Iterator<Item = f64> + Clone, max: u8) -> u8 {
    (0..=max)
        .find(|&s| {
            let k = 10f64.powi(s as i32);
            values.clone().all(|v| ((v * k).round() - v * k).abs() < 1e-6 * k.max(1.0))
        })
        .unwrap_or(max)
}

/// Une bougie de la chart en nombres à virgule (heure, ouverture, haut, bas, clôture, volume).
pub type RawBar = (i64, f64, f64, f64, f64, f64);

/// L'entrée du module : les dernières bougies (au plus 50 000, heures strictement croissantes) en entiers à
/// l'échelle, arrondis une seule fois ; `settings` : les valeurs des réglages, dans l'ordre du manifeste.
pub fn input_of(symbol: &str, timeframe_s: i64, bars: &[RawBar], settings: Vec<i64>) -> Input {
    let bars = &bars[bars.len().saturating_sub(MAX_BARS)..];
    let price_scale = scale_of(bars.iter().flat_map(|b| [b.1, b.2, b.3, b.4]), 8);
    let volume_scale = scale_of(bars.iter().map(|b| b.5), 6);
    let (pk, vk) = (10f64.powi(price_scale as i32), 10f64.powi(volume_scale as i32));
    let int = |x: f64, k: f64| {
        if x.is_finite() {
            (x * k).round().clamp(i64::MIN as f64 + 1.0, i64::MAX as f64) as i64
        } else {
            EMPTY
        }
    };
    let mut last = i64::MIN;
    let out: Vec<Bar> = bars
        .iter()
        .filter(|b| {
            let ok = b.0 > last;
            last = last.max(b.0);
            ok
        })
        .map(|b| Bar {
            t: b.0,
            o: int(b.1, pk),
            h: int(b.2, pk),
            l: int(b.3, pk),
            c: int(b.4, pk),
            v: int(b.5, vk),
        })
        .collect();
    let mut sym: String = symbol.chars().filter(|c| !c.is_control()).collect();
    while sym.len() > 32 {
        sym.pop();
    }
    if sym.is_empty() {
        sym = "?".into();
    }
    Input {
        timeframe: timeframe_s.clamp(1, u32::MAX as i64) as u32,
        price_scale,
        volume_scale,
        symbol: sym,
        bars: out,
        settings,
    }
}

/// Au plus autant d'octets dans l'entrée d'un module (limite du kit : 4 Mio).
pub const MAX_BYTES: usize = argos_module_sdk::data::MAX_BYTES;

/// 🪟 La fenêtre de bougies envoyée à un module (étape 7 du plan de collecte) : l'entrée, combien de bougies plus
/// anciennes n'ont pas été envoyées, et la taille réelle de l'entrée encodée (format du kit, prix entiers).
#[derive(Debug, Clone, PartialEq)]
pub struct Fenetre {
    pub input: Input,
    pub laissees: usize,
    pub octets: usize,
}

/// Les bougies **les plus récentes** qui tiennent dans les limites du kit (50 000 bougies et 4 Mio une fois encodées) :
/// jamais d'entrée refusée par le module, jamais tout l'historique. Au-delà, les plus anciennes sont laissées de côté
/// (et comptées, pour le dire dans l'interface).
pub fn fenetre(symbol: &str, timeframe_s: i64, bars: &[RawBar], settings: Vec<i64>) -> Fenetre {
    fenetre_avec(symbol, timeframe_s, bars, settings, MAX_BARS, MAX_BYTES)
}

/// `fenetre` avec des limites données (pour les tests).
pub fn fenetre_avec(symbol: &str, timeframe_s: i64, bars: &[RawBar], settings: Vec<i64>, max_bars: usize, max_bytes: usize) -> Fenetre {
    let recentes = &bars[bars.len().saturating_sub(max_bars.min(MAX_BARS))..];
    let mut input = input_of(symbol, timeframe_s, recentes, settings);
    let mut octets = input.encode().len();
    if octets > max_bytes {
        // 48 octets par bougie dans le format du kit : on retire juste ce qu'il faut, les plus anciennes d'abord
        let trop = (octets - max_bytes).div_ceil(48).min(input.bars.len());
        input.bars.drain(..trop);
        octets = input.encode().len();
    }
    let premiere = input.bars.first().map_or(i64::MAX, |b| b.t);
    let laissees = bars.iter().filter(|b| b.0 < premiere).count();
    Fenetre { input, laissees, octets }
}

/// Une valeur à l'échelle en nombre à virgule (`EMPTY` → `NaN`).
pub fn real(v: i64, scale: u8) -> f64 {
    if v == EMPTY {
        f64::NAN
    } else {
        v as f64 / 10f64.powi(scale as i32)
    }
}

/// Les courbes d'une sortie en nombres à virgule : (nom, une valeur par bougie).
pub fn lines_of(out: &Output) -> Vec<(String, Vec<f64>)> {
    out.lines
        .iter()
        .map(|l| (l.name.clone(), l.values.iter().map(|v| real(*v, l.scale)).collect()))
        .collect()
}

/// Les cartes d'une sortie : (nom, valeur écrite).
pub fn cards_of(out: &Output) -> Vec<(String, String)> {
    out.cards
        .iter()
        .map(|c| (c.name.clone(), argos_module_policy::fixed(c.value, c.scale)))
        .collect()
}

/// 🖥️ La bulle du kit (desktop seulement).
#[cfg(not(target_arch = "wasm32"))]
pub mod engine {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;

    /// Carburant d'un appel : assez pour 50 000 bougies et une longueur de 500 (mesure du kit).
    pub const FUEL: u64 = 200_000_000;
    /// Taille du dossier privé d'un module.
    pub const QUOTA: u64 = 1 << 20;

    pub struct Engine {
        sandbox: argos_sandbox::Sandbox,
    }

    impl Engine {
        pub fn new() -> Result<Self, Error> {
            argos_sandbox::Sandbox::new().map(|sandbox| Self { sandbox }).map_err(|e| e.to_string())
        }

        /// Un appel. `approved` : l'empreinte SHA-384 du `.wasm` accepté à l'installation (un autre fichier est
        /// refusé) ; `granted` : les droits accordés ; `data_root` : le dossier des dossiers privés ; `protected` :
        /// des chemins qu'aucun module ne touche jamais.
        #[allow(clippy::too_many_arguments)]
        pub fn run(
            &self,
            m: &Manifest,
            wasm: &[u8],
            approved: &[u8; 48],
            granted: &[Capability],
            data_root: &Path,
            protected: &[PathBuf],
            input: &Input,
        ) -> Result<Output, Error> {
            m.check()?;
            if &sha384(wasm) != approved {
                return Err("le fichier du module a changé depuis ton accord : réinstalle-le".into());
            }
            let caps: Vec<Capability> = m.capabilities.iter().copied().filter(|c| granted.contains(c)).collect();
            let policy = m.policy();
            let approval = Approval::of(wasm, caps.clone(), policy.clone());
            let services: Arc<dyn argos_sandbox::HostServices> = if caps.is_empty() {
                Arc::new(argos_sandbox::NoServices)
            } else {
                Arc::new(argos_sandbox::services::Services::build(&m.path, &caps, &policy, data_root, QUOTA, protected).map_err(|e| e.to_string())?)
            };
            let limits = argos_sandbox::Limits {
                fuel: FUEL,
                ..argos_sandbox::Limits::DEFAULT
            };
            self.sandbox.run_data(&approval, wasm, input, limits, services).map_err(|e| e.to_string())
        }
    }
}

#[cfg(test)]
mod tests;
