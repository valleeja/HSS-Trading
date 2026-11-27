//! `ai_request`: by default only the local Ollama (`127.0.0.1:11434`), with
//! the model pinned by the module's manifest.
//!
//! The module sends only the prompt text (UTF-8); the host builds the Ollama
//! request with the pinned model, so a module can never pick another model
//! or reach another server. The answer is the model's text, limited to the
//! 64 KiB buffer. This is the one exception to the anti-localhost rule, and
//! only for this fixed address. No key is involved: the local model needs
//! none, and a module never sees any.
//!
//! Online AI is not available yet: it will need the manifest to declare it
//! and the right to be granted (by the user in their own account, or by the
//! maintainers' vote for the marketplace), with the API key held by the host.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::time::Duration;

use argos_module_policy::Capability;
use ureq::config::Config;
use ureq::Agent;

use crate::{
    CallToken, HostServices, HOST_DENIED, HOST_IO, HOST_MALFORMED, HOST_TOO_LARGE,
    MAX_REQUEST_BYTES,
};

/// The local Ollama address.
pub const OLLAMA: SocketAddr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 11434));
/// Longest time for one AI request.
const AI_TIMEOUT: Duration = Duration::from_secs(60);
/// Largest raw reply read from Ollama (JSON around the text).
const MAX_REPLY_BYTES: u64 = 1024 * 1024;

/// The AI service of one module.
#[derive(Debug, Clone)]
pub struct AiService {
    address: SocketAddr,
    model: String,
    agent: Agent,
}

/// Why an AI service could not be set up.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AiSetupError {
    /// Only a loopback address may serve the local model.
    #[error("the local AI must be on a loopback address: {0}")]
    NotLoopback(SocketAddr),
    /// The model name is empty or has unexpected characters.
    #[error("invalid model name: {0:?}")]
    InvalidModel(String),
}

impl AiService {
    /// The local Ollama with the model pinned by the manifest.
    ///
    /// # Errors
    /// [`AiSetupError::InvalidModel`].
    pub fn local(model: &str) -> Result<Self, AiSetupError> {
        Self::local_at(OLLAMA, model)
    }

    /// Same, at another loopback address (tests).
    ///
    /// # Errors
    /// [`AiSetupError`].
    pub fn local_at(address: SocketAddr, model: &str) -> Result<Self, AiSetupError> {
        if !address.ip().is_loopback() {
            return Err(AiSetupError::NotLoopback(address));
        }
        if !argos_module_policy::valid_model(model) {
            return Err(AiSetupError::InvalidModel(model.to_owned()));
        }
        let config = Config::builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .proxy(None)
            .user_agent("argos-module")
            .timeout_global(Some(AI_TIMEOUT))
            .build();
        Ok(Self {
            address,
            model: model.to_owned(),
            agent: Agent::new_with_config(config),
        })
    }

    fn generate(&self, request: &[u8], token: &CallToken) -> Result<Vec<u8>, i32> {
        let prompt = std::str::from_utf8(request).map_err(|_| HOST_MALFORMED)?;
        let body = serde_json::json!({ "model": self.model, "prompt": prompt, "stream": false });
        let body = serde_json::to_vec(&body).map_err(|_| HOST_IO)?;
        if token.is_cancelled() {
            return Err(HOST_IO);
        }
        let url = format!("http://{}/api/generate", self.address);
        let mut response = self
            .agent
            .post(&url)
            .header("content-type", "application/json")
            .send(&body[..])
            .map_err(|_| HOST_IO)?;
        if !response.status().is_success() {
            return Err(HOST_IO);
        }
        let reply = response
            .body_mut()
            .with_config()
            .limit(MAX_REPLY_BYTES)
            .read_to_vec()
            .map_err(|e| match e {
                ureq::Error::BodyExceedsLimit(_) => HOST_TOO_LARGE,
                _ => HOST_IO,
            })?;
        let reply: serde_json::Value = serde_json::from_slice(&reply).map_err(|_| HOST_IO)?;
        let text = reply["response"].as_str().ok_or(HOST_IO)?;
        if text.len() > MAX_REQUEST_BYTES {
            return Err(HOST_TOO_LARGE);
        }
        Ok(text.as_bytes().to_vec())
    }
}

impl HostServices for AiService {
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        match capability {
            Capability::Ai => self.generate(request, token),
            _ => Err(HOST_DENIED),
        }
    }
}
