//! 🛒 Catalogue local de modules (Marketplace, étape locale : ni serveur, ni paiement, ni compte). Un catalogue est un
//! fichier `argos-catalogue.json` : pour chaque module, son manifeste `argos-module.json`, son `.wasm` (base64), son
//! empreinte SHA-384, un titre et une description. Il s'importe (Téléchargements sur le desktop, choix de fichier dans
//! le navigateur) et s'exporte (mes modules installés, pour les donner à quelqu'un).
//!
//! Chaque module du catalogue est vérifié avec le kit module Argos avant d'être montré : empreinte SHA-384 du `.wasm`,
//! manifeste (refus par défaut, aucun réseau), interface v2. Ses droits sont montrés **avant** l'installation ;
//! l'installation, la mise à jour et le retrait passent par la page 🧩 Modules (mêmes règles). Pas encore de signature :
//! le kit ne vérifie pas de signature lui-même (c'est l'affaire d'Argos, niveau marketplace).

use crate::*;
use argos_module_host::{scan, sha384, Manifest};

/// Nom et format d'un fichier catalogue.
pub(crate) const FILE: &str = "argos-catalogue.json";
pub(crate) const FORMAT: &str = "argos-catalogue";

/// Un module du catalogue.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct CatalogItem {
    pub(crate) title: String,
    pub(crate) description: String,
    /// Le manifeste `argos-module.json` (texte).
    pub(crate) manifest: String,
    /// Le `.wasm` en base64.
    pub(crate) wasm: String,
    /// Empreinte SHA-384 (hexadécimal) annoncée du `.wasm`.
    pub(crate) sha384: String,
}

/// Un fichier catalogue.
#[derive(Debug, Clone, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub(crate) struct CatalogFile {
    pub(crate) format: String,
    pub(crate) version: u32,
    pub(crate) modules: Vec<CatalogItem>,
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

impl CatalogItem {
    /// Vérifie le module avec le kit : `.wasm` lisible, empreinte annoncée = empreinte réelle, manifeste valide,
    /// interface v2. Rend le manifeste et le `.wasm`.
    pub(crate) fn verify(&self) -> Result<(Manifest, Vec<u8>), String> {
        use base64::Engine;
        let wasm = base64::engine::general_purpose::STANDARD
            .decode(self.wasm.trim())
            .map_err(|_| "fichier du module illisible".to_string())?;
        if hex(&sha384(&wasm)) != self.sha384.trim().to_lowercase() {
            return Err("empreinte SHA-384 différente : fichier modifié ou abîmé".into());
        }
        let m = Manifest::parse(&self.manifest)?;
        m.check()?;
        scan(&wasm)?;
        Ok((m, wasm))
    }

    fn embedded(title: &str, description: &str, manifest: &str, wasm: &[u8]) -> Self {
        use base64::Engine;
        Self {
            title: title.into(),
            description: description.into(),
            manifest: manifest.into(),
            wasm: base64::engine::general_purpose::STANDARD.encode(wasm),
            sha384: hex(&sha384(wasm)),
        }
    }

    /// Le module d'exemple embarqué (la Moyenne du kit).
    pub(crate) fn builtin() -> Self {
        use crate::actions::modules::{EXAMPLE_JSON, EXAMPLE_WASM};
        Self::embedded(
            "Moyenne mobile (exemple du kit)",
            "Moyenne mobile simple des bougies : longueur et source (clôture, ouverture, haut, bas). Aucun droit.",
            EXAMPLE_JSON,
            EXAMPLE_WASM,
        )
    }

    /// Les modules embarqués : l'exemple du kit, puis les indicateurs d'Argos déjà passés en modules.
    pub(crate) fn builtins() -> Vec<Self> {
        use crate::actions::modules::{ACADEMIE_JSON, ACADEMIE_WASM, EMA_JSON, EMA_WASM, PINCEAU_JSON, PINCEAU_WASM};
        vec![
            Self::builtin(),
            Self::embedded(
                "EMA (module d'Argos)",
                "Moyenne mobile exponentielle, même calcul que l'EMA intégrée d'Argos : longueur et source. Aucun droit.",
                EMA_JSON,
                EMA_WASM,
            ),
            Self::embedded(
                "Sessions de l'Académie (module d'Argos)",
                "Haut et bas d'une fenêtre des leçons, à l'heure de New York : range asiatique, Londres, ORB ou Silver Bullet. Aucun droit.",
                ACADEMIE_JSON,
                ACADEMIE_WASM,
            ),
            Self::embedded(
                "Pinceau de zone (module d'Argos)",
                "Pour une zone (clic droit sur un rectangle → 🖌 Pinceau de zone) : volume par prix, POC, zone de valeur, VWAP et delta estimé. Aucun droit.",
                PINCEAU_JSON,
                PINCEAU_WASM,
            ),
        ]
    }
}

/// État d'un module du catalogue par rapport à ceux installés.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Status {
    New,
    Update(String, u64),
    Installed(String),
}

impl App {
    /// Le catalogue : l'exemple embarqué, puis les modules importés (un seul par chemin, la version la plus haute).
    pub(crate) fn catalog(&self) -> Vec<(CatalogItem, Manifest)> {
        let mut out: Vec<(CatalogItem, Manifest)> = vec![];
        for it in CatalogItem::builtins().into_iter().chain(self.mods.catalog.iter().cloned()) {
            let Ok(m) = Manifest::parse(&it.manifest) else { continue };
            match out.iter_mut().find(|(_, x)| x.path == m.path) {
                Some(slot) if m.version > slot.1.version => *slot = (it, m),
                Some(_) => {}
                None => out.push((it, m)),
            }
        }
        out
    }

    /// Nouveau, mise à jour ou déjà installé ?
    pub(crate) fn catalog_status(&self, m: &Manifest) -> Status {
        match self.mods.list.iter().find(|e| e.id == m.path) {
            None => Status::New,
            Some(e) if e.version.parse::<u64>().unwrap_or(0) < m.version => Status::Update(e.version.clone(), m.version),
            Some(e) => Status::Installed(e.version.clone()),
        }
    }

    /// 📂 Importe un fichier catalogue : chaque module vérifié avec le kit ; les refusés sont nommés.
    pub(crate) fn catalog_import(&mut self, text: &str) -> String {
        let Ok(file) = serde_json::from_str::<CatalogFile>(text) else {
            return format!("🛒 {FILE} illisible");
        };
        if file.format != FORMAT {
            return format!("🛒 Ce fichier n'est pas un catalogue de modules ({FORMAT})");
        }
        let (mut added, mut refused) = (0, vec![]);
        for it in file.modules {
            match it.verify() {
                Ok((m, _)) => {
                    self.mods
                        .catalog
                        .retain(|x| Manifest::parse(&x.manifest).map_or(true, |o| !(o.path == m.path && o.version == m.version)));
                    self.mods.catalog.push(it);
                    added += 1;
                }
                Err(e) => refused.push(format!("{} : {e}", if it.title.is_empty() { "?" } else { &it.title })),
            }
        }
        Self::write_prefs("modules-catalogue.json", &self.mods.catalog);
        let mut msg = format!("🛒 Catalogue importé : {added} module(s) vérifié(s)");
        if !refused.is_empty() {
            msg.push_str(&format!(", {} refusé(s) — {}", refused.len(), refused.join(" · ")));
        }
        msg
    }

    /// 💾 Mes modules installés en fichier catalogue (pour les donner à quelqu'un).
    pub(crate) fn catalog_file(&self) -> CatalogFile {
        use base64::Engine;
        let modules = self
            .mods
            .list
            .iter()
            .filter_map(|e| {
                let wasm = Self::mod_wasm(&e.id)?;
                let m = e.manifest()?;
                Some(CatalogItem {
                    title: m.name().to_string(),
                    description: format!("{} v{} · par {}", m.path, m.version, m.author),
                    manifest: e.manifest.clone(),
                    wasm: base64::engine::general_purpose::STANDARD.encode(&wasm),
                    sha384: hex(&sha384(&wasm)),
                })
            })
            .collect();
        CatalogFile {
            format: FORMAT.into(),
            version: 1,
            modules,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exemple_du_catalogue_verifie_et_empreinte_modifiee_refusee() {
        let it = CatalogItem::builtin();
        let (m, wasm) = it.verify().expect("l'exemple passe les vérifications du kit");
        assert_eq!((m.path.as_str(), wasm.len() > 1000), ("trading/moyenne", true));
        let bad = CatalogItem {
            sha384: "00".repeat(48),
            ..it.clone()
        };
        assert!(bad.verify().unwrap_err().contains("empreinte"));
        let net = CatalogItem {
            manifest: it
                .manifest
                .replace("\"capabilities\": []", "\"capabilities\": [\"Network\"], \"domains\": [\"x.com\"]"),
            ..it
        };
        assert!(net.verify().is_err(), "réseau : refusé même dans un catalogue");
    }
}
