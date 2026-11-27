//! All services of one module, built from its rights and policy.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use argos_module_policy::{Capability, ServicePolicy};

use crate::ai::{AiService, AiSetupError};
use crate::files::{FileService, FileSetupError};
use crate::net::NetService;
use crate::{CallToken, HostServices, HOST_DENIED};

/// Why the services could not be built.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ServicesError {
    /// File service setup failed.
    #[error(transparent)]
    Files(#[from] FileSetupError),
    /// AI service setup failed.
    #[error(transparent)]
    Ai(#[from] AiSetupError),
}

/// The file, network and AI services a module was granted; the others are absent.
#[derive(Debug, Clone)]
pub struct Services {
    files: Option<FileService>,
    net: Option<NetService>,
    ai: Option<AiService>,
    model: Option<Arc<dyn HostServices>>,
}

impl std::fmt::Debug for dyn HostServices {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HostServices")
    }
}

impl Services {
    /// Builds only the services behind `capabilities`, configured by `policy`.
    ///
    /// # Errors
    /// [`ServicesError`].
    pub fn build(
        module: &str,
        capabilities: &[Capability],
        policy: &ServicePolicy,
        data_root: &Path,
        quota: u64,
        protected: &[PathBuf],
    ) -> Result<Self, ServicesError> {
        let has = |c| capabilities.contains(&c);
        let files = if has(Capability::ReadFiles) || has(Capability::WriteFiles) {
            Some(FileService::new(
                data_root,
                module,
                policy.subfolders.clone(),
                quota,
                protected,
            )?)
        } else {
            None
        };
        let net = has(Capability::Network).then(|| NetService::new(policy.domains.clone()));
        let ai = match (&policy.ai_model, has(Capability::Ai)) {
            (Some(model), true) => Some(AiService::local(model)?),
            _ => None,
        };
        Ok(Self {
            files,
            net,
            ai,
            model: None,
        })
    }

    /// Adds the model service (see [`crate::models::ModelService`]), used only
    /// if the `Model` right was granted.
    #[must_use]
    pub fn with_model(mut self, service: Arc<dyn HostServices>) -> Self {
        self.model = Some(service);
        self
    }
}

impl HostServices for Services {
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        let denied = || Err(HOST_DENIED);
        match capability {
            Capability::ReadFiles | Capability::WriteFiles => self
                .files
                .as_ref()
                .map_or_else(denied, |s| s.call(capability, request, token)),
            Capability::Network => self
                .net
                .as_ref()
                .map_or_else(denied, |s| s.call(capability, request, token)),
            Capability::Ai => self
                .ai
                .as_ref()
                .map_or_else(denied, |s| s.call(capability, request, token)),
            Capability::Model => self
                .model
                .as_ref()
                .map_or_else(denied, |s| s.call(capability, request, token)),
        }
    }
}
