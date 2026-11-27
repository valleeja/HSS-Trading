//! Rights and service settings of an Argos module, shared by the sandbox and
//! the governance core. Neutral: no key, no ledger, no signature here.
//!
//! - [`Capability`]: what a module may ask for; anything else is denied.
//! - [`ServicePolicy`]: domains, sub-folders, AI model, time budget,
//!   interface version and settings ([`SettingSpec`]), with
//!   [`ServicePolicy::check`].
//! - [`Approval`]: what the sandbox needs to run a module: the SHA-384 of the
//!   approved WASM, its rights and its policy. Whoever approved it (a vote,
//!   or the user in their own account) checked the signatures before.
//!
//! Licence: `PolyForm` Noncommercial 1.0.0 (see `LICENSE.md`); commercial use needs
//! an Argos commercial licence (see `COMMERCIAL.md`).

#![forbid(unsafe_code)]

mod settings;

pub use settings::{
    fixed, parse_color, SettingKind, SettingSpec, MAX_CHOICES, MAX_SETTINGS, MAX_SETTING_SCALE,
};

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha384};

/// Default time budget of a module run, in seconds.
pub const DEFAULT_BUDGET_SECS: u32 = 2;
/// Largest time budget a module may be granted, in seconds.
pub const MAX_BUDGET_SECS: u32 = 120;
/// Interface versions a module may declare (`docs/abi-v2.md`).
pub const ABI_VERSIONS: [u32; 2] = [1, 2];

/// Language of the rights sheet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    /// Français.
    French,
    /// English.
    English,
}

/// A capability a module may ask for. Anything not listed is denied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Capability {
    /// Read files in the module's own data folder.
    ReadFiles,
    /// Write files in the module's own data folder.
    WriteFiles,
    /// Network access (allow-listed hosts, enforced by the sandbox).
    Network,
    /// Call an AI model through the host.
    Ai,
    /// Run one approved model module (signed weights) through the host engine.
    Model,
}

/// Why a policy does not fit its rights.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum PolicyError {
    /// The time budget is 0 or above [`MAX_BUDGET_SECS`].
    #[error("time budget must be 1 to {MAX_BUDGET_SECS} seconds, got {0}")]
    Budget(u32),
    /// `Ai` without a pinned model.
    #[error("the Ai right needs a pinned model")]
    MissingModel,
    /// A model name with forbidden characters.
    #[error("invalid AI model name {0:?}")]
    InvalidModel(String),
    /// A domain that is not a plain public DNS name.
    #[error("invalid domain {0:?}")]
    InvalidDomain(String),
    /// `Network` without any domain.
    #[error("the Network right needs at least one domain")]
    MissingDomains,
    /// A sub-folder name with forbidden characters.
    #[error("invalid sub-folder name {0:?}")]
    InvalidFolder(String),
    /// Too many domains or sub-folders.
    #[error("too many domains or sub-folders")]
    TooManyItems,
    /// `Model` without the SHA-384 of the model module it may use.
    #[error("the Model right needs the SHA-384 of its model module")]
    MissingModelModule,
    /// Not a SHA-384 in lowercase hex.
    #[error("invalid model module SHA-384 {0:?}")]
    InvalidModelModule(String),
    /// An interface version other than 1 or 2.
    #[error("unknown interface version {0}")]
    Abi(u32),
    /// Settings on an interface v1 module (only v2 receives them).
    #[error("settings need interface version 2")]
    SettingsNeedAbi2,
    /// More than [`MAX_SETTINGS`] settings.
    #[error("more than {MAX_SETTINGS} settings")]
    TooManySettings,
    /// A setting with a bad identifier, name, bounds or default, or a
    /// repeated identifier.
    #[error("invalid setting {0:?}")]
    InvalidSetting(String),
    /// A chosen value outside its setting's bounds, or the wrong count.
    #[error("value not allowed for setting {0:?}")]
    SettingValue(String),
}

pub(crate) fn push(out: &mut Vec<u8>, bytes: &[u8]) {
    let len = u32::try_from(bytes.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(bytes);
}

/// Settings of the services behind the rights.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServicePolicy {
    /// Domains reachable with `Network` (exact names, HTTPS only).
    pub domains: Vec<String>,
    /// Top-level folders allowed with `ReadFiles` / `WriteFiles` (all if `None`).
    pub subfolders: Option<Vec<String>>,
    /// Pinned local AI model for `Ai`.
    pub ai_model: Option<String>,
    /// Time budget of one run, WASM and host calls together, in seconds
    /// (from 1 to [`MAX_BUDGET_SECS`]).
    pub budget_secs: u32,
    /// SHA-384 (lowercase hex) of the one model module `Model` may run.
    #[serde(default)]
    pub model: Option<String>,
    /// Interface version of the module: 1 (`run`) or 2 (`run_data`).
    #[serde(default = "default_abi")]
    pub abi: u32,
    /// Settings of an interface v2 module, in the order given to it.
    #[serde(default)]
    pub settings: Vec<SettingSpec>,
}

fn default_abi() -> u32 {
    1
}

impl Default for ServicePolicy {
    /// No domain, no sub-folder limit, no AI model, the default budget.
    fn default() -> Self {
        Self {
            domains: Vec::new(),
            subfolders: None,
            ai_model: None,
            budget_secs: DEFAULT_BUDGET_SECS,
            model: None,
            abi: 1,
            settings: Vec::new(),
        }
    }
}

/// Most domains or sub-folders a policy may list.
const MAX_POLICY_ITEMS: usize = 32;

/// True for a local AI model name: letters, digits, `.` `:` `-` `_` `/`, no
/// `..`, no leading `/`, at most 128 characters.
pub fn valid_model(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.contains("..")
        && !name.starts_with('/')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '_' | '/'))
}

/// True for a domain a module may reach: lowercase DNS name with a dot, no
/// IP literal, no `localhost`, no wildcard.
pub fn valid_domain(name: &str) -> bool {
    let labels: Vec<&str> = name.split('.').collect();
    labels.len() >= 2
        && name.len() <= 253
        && name != "localhost"
        && !name.ends_with(".localhost")
        && name.parse::<std::net::IpAddr>().is_err()
        && !labels
            .last()
            .is_some_and(|l| l.bytes().all(|b| b.is_ascii_digit()))
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        })
}

fn valid_folder(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 255
        && name != "."
        && name != ".."
        && !name
            .chars()
            .any(|c| c.is_control() || matches!(c, '/' | '\\' | ':'))
}

impl ServicePolicy {
    /// Checks the policy against the rights it serves: budget from 1 to
    /// [`MAX_BUDGET_SECS`], valid domains (and at least one with `Network`),
    /// valid sub-folder names, a valid model with `Ai`.
    ///
    /// # Errors
    /// The [`PolicyError`] of the first problem.
    pub fn check(&self, capabilities: &[Capability]) -> Result<(), PolicyError> {
        if self.budget_secs == 0 || self.budget_secs > MAX_BUDGET_SECS {
            return Err(PolicyError::Budget(self.budget_secs));
        }
        let folders = self.subfolders.as_deref().unwrap_or(&[]);
        if self.domains.len() > MAX_POLICY_ITEMS || folders.len() > MAX_POLICY_ITEMS {
            return Err(PolicyError::TooManyItems);
        }
        if let Some(bad) = self.domains.iter().find(|d| !valid_domain(d)) {
            return Err(PolicyError::InvalidDomain(bad.clone()));
        }
        if capabilities.contains(&Capability::Network) && self.domains.is_empty() {
            return Err(PolicyError::MissingDomains);
        }
        if let Some(bad) = folders.iter().find(|f| !valid_folder(f)) {
            return Err(PolicyError::InvalidFolder(bad.clone()));
        }
        match (&self.model, capabilities.contains(&Capability::Model)) {
            (None, true) => return Err(PolicyError::MissingModelModule),
            (Some(digest), _)
                if digest.len() != 96
                    || !digest
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) =>
            {
                return Err(PolicyError::InvalidModelModule(digest.clone()));
            }
            _ => {}
        }
        match (&self.ai_model, capabilities.contains(&Capability::Ai)) {
            (None, true) => return Err(PolicyError::MissingModel),
            (Some(model), _) if !valid_model(model) => {
                return Err(PolicyError::InvalidModel(model.clone()));
            }
            _ => {}
        }
        self.check_settings()
    }

    fn check_settings(&self) -> Result<(), PolicyError> {
        if !ABI_VERSIONS.contains(&self.abi) {
            return Err(PolicyError::Abi(self.abi));
        }
        if self.abi == 1 && !self.settings.is_empty() {
            return Err(PolicyError::SettingsNeedAbi2);
        }
        if self.settings.len() > MAX_SETTINGS {
            return Err(PolicyError::TooManySettings);
        }
        for (i, spec) in self.settings.iter().enumerate() {
            spec.check()?;
            if self.settings[..i].iter().any(|s| s.id == spec.id) {
                return Err(PolicyError::InvalidSetting(spec.id.clone()));
            }
        }
        Ok(())
    }

    /// The default value of every setting, in order.
    pub fn default_settings(&self) -> Vec<i64> {
        self.settings
            .iter()
            .map(SettingSpec::default_value)
            .collect()
    }

    /// Checks chosen values (one per setting, in order) before a run.
    ///
    /// # Errors
    /// [`PolicyError::SettingValue`] with the identifier of the first value
    /// not allowed, or an empty identifier if the count is wrong.
    pub fn check_values(&self, values: &[i64]) -> Result<(), PolicyError> {
        if values.len() != self.settings.len() {
            return Err(PolicyError::SettingValue(String::new()));
        }
        match self
            .settings
            .iter()
            .zip(values)
            .find(|(s, v)| !s.allows(**v))
        {
            Some((spec, _)) => Err(PolicyError::SettingValue(spec.id.clone())),
            None => Ok(()),
        }
    }

    /// Fixed binary encoding, part of the signed bytes of grants and releases.
    pub fn encode(&self, out: &mut Vec<u8>) {
        let domains = u32::try_from(self.domains.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&domains.to_be_bytes());
        for domain in &self.domains {
            push(out, domain.as_bytes());
        }
        match &self.subfolders {
            None => out.push(0),
            Some(folders) => {
                out.push(1);
                let count = u32::try_from(folders.len()).unwrap_or(u32::MAX);
                out.extend_from_slice(&count.to_be_bytes());
                for folder in folders {
                    push(out, folder.as_bytes());
                }
            }
        }
        push(out, self.ai_model.as_deref().unwrap_or("").as_bytes());
        out.extend_from_slice(&self.budget_secs.to_be_bytes());
        push(out, self.model.as_deref().unwrap_or("").as_bytes());
        out.extend_from_slice(&self.abi.to_be_bytes());
        let count = u32::try_from(self.settings.len()).unwrap_or(u32::MAX);
        out.extend_from_slice(&count.to_be_bytes());
        for spec in &self.settings {
            spec.encode(out);
        }
    }
}

/// What the sandbox needs to run a module, once someone approved it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    /// SHA-384 of the approved WASM: any other bytes are refused.
    pub wasm_sha384: [u8; 48],
    /// Granted rights (only their host functions are linked).
    pub capabilities: Vec<Capability>,
    /// Service settings, including the time budget.
    pub policy: ServicePolicy,
}

impl Approval {
    /// An approval of exactly these WASM bytes.
    pub fn of(wasm: &[u8], capabilities: Vec<Capability>, policy: ServicePolicy) -> Self {
        Self {
            wasm_sha384: sha384(wasm),
            capabilities,
            policy,
        }
    }
}

/// SHA-384 of `bytes`.
pub fn sha384(bytes: &[u8]) -> [u8; 48] {
    Sha384::digest(bytes).into()
}
