//! Checking an AI model file before anything loads it (step 13).
//!
//! A model is a module of its own (`argos-modele-<name>`): its file is
//! approved like a WASM module (SHA-384 in an [`Approval`] signed and voted,
//! or granted by the user), and it never has any right, network included.
//! Only two formats are accepted, recognised by their bytes, never by the
//! file name:
//!
//! - **safetensors**: 8-byte little-endian header length, a JSON header whose
//!   tensors all fit inside the file;
//! - **ONNX**: a protobuf `ModelProto` with an IR version and a graph, every
//!   top-level field well formed to the last byte.
//!
//! Python pickles (`.pkl`, and `PyTorch` `.pt`/`.pth`, which are zip files with
//! a pickle inside) are refused even when renamed: they can run code when
//! loaded.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use serde::{Deserialize, Serialize};

use crate::{
    sha384, Approval, CallToken, Capability, HostServices, HOST_DENIED, HOST_MALFORMED,
    HOST_MODEL_REFUSED,
};

/// Largest model file accepted, whatever the manifest says (512 MiB).
pub const MAX_MODEL_BYTES: u64 = 512 * 1024 * 1024;
/// Largest safetensors JSON header (16 MiB).
const MAX_HEADER_BYTES: u64 = 16 * 1024 * 1024;

/// The two accepted model formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModelFormat {
    /// Hugging Face safetensors (candle).
    Safetensors,
    /// ONNX protobuf (tract).
    Onnx,
}

/// `argos-model.json`: what the model module declares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelManifest {
    /// Module name, `argos-modele-<name>`.
    pub name: String,
    /// File format.
    pub format: ModelFormat,
    /// SHA-384 of the model file, hex.
    pub sha384: String,
    /// Largest size the file may have, in bytes.
    pub max_bytes: u64,
}

/// Why a model file is refused.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ModelError {
    /// The name is not `argos-modele-<lowercase name>`.
    #[error("model module names are argos-modele-<name>")]
    BadName,
    /// The approval or manifest grants a right (a model never has any).
    #[error("a model may not have any right, network included")]
    RightRequested,
    /// Larger than the manifest allows or than [`MAX_MODEL_BYTES`].
    #[error("model file too large: {0} bytes")]
    TooLarge(u64),
    /// The bytes are not the approved ones.
    #[error("model file does not match the approved SHA-384")]
    DigestMismatch,
    /// A Python pickle or a `PyTorch` zip, whatever its name.
    #[error("pickle or PyTorch file refused: it can run code when loaded")]
    Pickle,
    /// Not a valid file of the declared format.
    #[error("not a valid {0:?} file")]
    BadFormat(ModelFormat),
}

/// Checks a model file against its manifest and approval, before loading.
///
/// # Errors
/// The [`ModelError`] of the first problem.
pub fn verify_model(
    bytes: &[u8],
    manifest: &ModelManifest,
    approval: &Approval,
) -> Result<(), ModelError> {
    let suffix = manifest
        .name
        .strip_prefix("argos-modele-")
        .unwrap_or_default();
    if suffix.is_empty()
        || !suffix
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(ModelError::BadName);
    }
    if !approval.capabilities.is_empty() || !approval.policy.domains.is_empty() {
        return Err(ModelError::RightRequested);
    }
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if size > manifest.max_bytes.min(MAX_MODEL_BYTES) {
        return Err(ModelError::TooLarge(size));
    }
    let digest = sha384(bytes);
    if digest != approval.wasm_sha384 || hex_of(&digest) != manifest.sha384.to_ascii_lowercase() {
        return Err(ModelError::DigestMismatch);
    }
    // Positive recognition first: a pickle or a zip is never a valid
    // safetensors or ONNX file, while a real safetensors header may start
    // with the same bytes as a pickle (a 640-byte header is `80 02 00 ...`).
    let valid = match manifest.format {
        ModelFormat::Safetensors => valid_safetensors(bytes),
        ModelFormat::Onnx => valid_onnx(bytes),
    };
    if valid {
        Ok(())
    } else if is_pickle_or_zip(bytes) {
        Err(ModelError::Pickle)
    } else {
        Err(ModelError::BadFormat(manifest.format))
    }
}

fn hex_of(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Pickle protocol 2 to 5 (`0x80 <protocol>`), a protocol 0/1 pickle ending
/// in `.` after an opcode stream, or a zip (`PyTorch` `.pt`).
fn is_pickle_or_zip(bytes: &[u8]) -> bool {
    let zip = bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06");
    let pickle_v2 =
        bytes.first() == Some(&0x80) && bytes.get(1).is_some_and(|p| (2..=5).contains(p));
    let pickle_old =
        matches!(bytes.first(), Some(b'(' | b'c' | b']' | b'}')) && bytes.last() == Some(&b'.');
    zip || pickle_v2 || pickle_old
}

/// safetensors: `u64` LE header length, JSON object header; every tensor's
/// `data_offsets` lie inside the data area, which they cover exactly.
fn valid_safetensors(bytes: &[u8]) -> bool {
    let Some((len, rest)) = bytes.split_first_chunk::<8>() else {
        return false;
    };
    let header_len = u64::from_le_bytes(*len);
    let Ok(n) = usize::try_from(header_len) else {
        return false;
    };
    if header_len > MAX_HEADER_BYTES || n > rest.len() {
        return false;
    }
    let (header, data) = rest.split_at(n);
    let Ok(serde_json::Value::Object(map)) = serde_json::from_slice::<serde_json::Value>(header)
    else {
        return false;
    };
    let mut end = 0_u64;
    for (name, tensor) in &map {
        if name == "__metadata__" {
            continue;
        }
        let offsets = tensor["data_offsets"].as_array();
        let (Some(start), Some(stop)) = (
            offsets
                .and_then(|o| o.first())
                .and_then(serde_json::Value::as_u64),
            offsets
                .and_then(|o| o.get(1))
                .and_then(serde_json::Value::as_u64),
        ) else {
            return false;
        };
        if !tensor["dtype"].is_string() || !tensor["shape"].is_array() || start > stop {
            return false;
        }
        end = end.max(stop);
    }
    !map.is_empty() && u64::try_from(data.len()).is_ok_and(|d| d == end)
}

/// ONNX: every top-level protobuf field is well formed up to the last byte,
/// with an IR version (field 1, varint) and a graph (field 7, bytes).
fn valid_onnx(bytes: &[u8]) -> bool {
    let mut at = 0;
    let (mut ir_version, mut graph) = (false, false);
    while at < bytes.len() {
        let Some((key, next)) = varint(bytes, at) else {
            return false;
        };
        at = next;
        let (field, wire) = (key >> 3, key & 7);
        if field == 0 || field > 32 {
            return false;
        }
        match wire {
            0 => match varint(bytes, at) {
                Some((_, next)) => at = next,
                None => return false,
            },
            1 => at = at.saturating_add(8),
            2 => {
                let Some((len, next)) = varint(bytes, at) else {
                    return false;
                };
                let Ok(len) = usize::try_from(len) else {
                    return false;
                };
                at = next.saturating_add(len);
            }
            5 => at = at.saturating_add(4),
            _ => return false,
        }
        if at > bytes.len() {
            return false;
        }
        ir_version |= field == 1 && wire == 0;
        graph |= field == 7 && wire == 2;
    }
    ir_version && graph
}

/// A protobuf varint at `at`: its value and the next position.
fn varint(bytes: &[u8], mut at: usize) -> Option<(u64, usize)> {
    let mut value = 0_u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(at)?;
        at += 1;
        value |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some((value, at));
        }
    }
    None
}

/// The host's inference engine (for example tract, in the private core).
/// The sandbox only gives it verified bytes.
pub trait ModelEngine: Send + Sync + 'static {
    /// A loaded, ready-to-run model.
    type Loaded: Send + Sync + 'static;

    /// Loads verified model bytes.
    ///
    /// # Errors
    /// A message if the engine cannot load them.
    fn load(&self, bytes: &[u8], format: ModelFormat) -> Result<Self::Loaded, String>;

    /// Runs the model on one `f32` input of the given shape.
    ///
    /// # Errors
    /// A message if the run fails.
    fn run(
        &self,
        loaded: &Self::Loaded,
        input: &[f32],
        shape: &[usize],
    ) -> Result<Vec<f32>, String>;
}

/// Where the host finds approved model modules, by SHA-384.
pub trait ModelStore: Send + Sync + 'static {
    /// The file, its manifest and its approval, if this model module exists.
    fn get(&self, sha384: &[u8; 48]) -> Option<(Vec<u8>, ModelManifest, Approval)>;
}

/// Largest input rank and number of values a module may send.
const MAX_RANK: usize = 8;

/// `model_request` for one module: runs only the model module its policy
/// names. The weights are checked with [`verify_model`] before every load;
/// once loaded they are kept by SHA-384, so a file changed later is never used.
///
/// Request: rank (u32 LE), each dimension (u32 LE), then the `f32` values
/// (LE). Answer: the output `f32` values (LE), within the module's buffer.
pub struct ModelService<E: ModelEngine, S: ModelStore> {
    engine: E,
    store: S,
    allowed: [u8; 48],
    cache: Mutex<HashMap<[u8; 48], Arc<E::Loaded>>>,
}

impl<E: ModelEngine, S: ModelStore> std::fmt::Debug for ModelService<E, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ModelService").finish_non_exhaustive()
    }
}

impl<E: ModelEngine, S: ModelStore> ModelService<E, S> {
    /// A service allowed to run the model module whose SHA-384 is `allowed`
    /// (from the module's approved policy).
    pub fn new(engine: E, store: S, allowed: [u8; 48]) -> Self {
        Self {
            engine,
            store,
            allowed,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn loaded(&self) -> Result<Arc<E::Loaded>, i32> {
        if let Some(model) = self
            .cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&self.allowed)
        {
            return Ok(Arc::clone(model));
        }
        let (bytes, manifest, approval) =
            self.store.get(&self.allowed).ok_or(HOST_MODEL_REFUSED)?;
        if approval.wasm_sha384 != self.allowed {
            return Err(HOST_MODEL_REFUSED);
        }
        verify_model(&bytes, &manifest, &approval).map_err(|_| HOST_MODEL_REFUSED)?;
        let model = Arc::new(
            self.engine
                .load(&bytes, manifest.format)
                .map_err(|_| HOST_MODEL_REFUSED)?,
        );
        self.cache
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(self.allowed, Arc::clone(&model));
        Ok(model)
    }
}

/// Parses `rank | dims | values` (all little-endian).
fn parse_input(request: &[u8]) -> Result<(Vec<usize>, Vec<f32>), i32> {
    let word = |at: usize| -> Result<u32, i32> {
        let bytes = request.get(at..at + 4).ok_or(HOST_MALFORMED)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    };
    let rank = usize::try_from(word(0)?).map_err(|_| HOST_MALFORMED)?;
    if rank == 0 || rank > MAX_RANK {
        return Err(HOST_MALFORMED);
    }
    let mut shape = Vec::with_capacity(rank);
    let mut count = 1_usize;
    for i in 0..rank {
        let dim = usize::try_from(word(4 + 4 * i)?).map_err(|_| HOST_MALFORMED)?;
        count = count.checked_mul(dim).ok_or(HOST_MALFORMED)?;
        shape.push(dim);
    }
    let data = &request[4 + 4 * rank..];
    if count == 0 || data.len() != count.checked_mul(4).ok_or(HOST_MALFORMED)? {
        return Err(HOST_MALFORMED);
    }
    let values = data
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| f32::from_le_bytes(*c))
        .collect();
    Ok((shape, values))
}

impl<E: ModelEngine, S: ModelStore> HostServices for ModelService<E, S> {
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        _token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        if capability != Capability::Model {
            return Err(HOST_DENIED);
        }
        let (shape, input) = parse_input(request)?;
        let model = self.loaded()?;
        let output = self
            .engine
            .run(&model, &input, &shape)
            .map_err(|_| HOST_MODEL_REFUSED)?;
        Ok(output.iter().flat_map(|v| v.to_le_bytes()).collect())
    }
}
