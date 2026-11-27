//! Argos module sandbox (step 6, piece C), built on wasmtime.
//!
//! - **Approved code only**: the WASM bytes must hash (SHA-384) to the digest
//!   of the approved release, or nothing runs.
//! - **Deny by default**: no WASI, no host function at all except those of the
//!   capabilities granted in the approved manifest. A module importing
//!   anything else fails to instantiate.
//! - **Limits**: fuel (instructions), memory and wall-clock time.
//! - The host never gives a module access to keys or ledgers: there is no
//!   import for them.

#![forbid(unsafe_code)]

pub mod ai;
pub mod data;
pub mod files;
pub mod models;
pub mod net;
pub mod services;

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use argos_module_policy::sha384;
pub use argos_module_policy::{Approval, Capability, ServicePolicy};
use wasmtime::{
    Caller, Config, Engine, Extern, Instance, Linker, Module, ResourceLimiter, Store, Trap,
};

/// Name of the import module that groups the host functions.
pub const HOST_MODULE: &str = "argos";

/// Name of the entry function a module built with the kit exports.
pub const ENTRY: &str = "run";

/// Largest request a module may pass to one host function.
pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
/// Most table elements (function references) a module may have.
pub const MAX_TABLE_ELEMENTS: usize = 10_000;

/// Host function status: done.
pub const HOST_OK: i32 = 0;
/// Host function status: negative pointer or length, or bytes outside the
/// module's memory (or no exported `memory`).
pub const HOST_BAD_REQUEST: i32 = -1;
/// Host function status: request larger than [`MAX_REQUEST_BYTES`].
pub const HOST_TOO_LARGE: i32 = -2;
/// Host function status: the capability is not granted.
pub const HOST_DENIED: i32 = -3;
/// Host function status: no such file.
pub const HOST_NOT_FOUND: i32 = -4;
/// Host function status: path refused (absolute, `..`, link, special name,
/// outside the module's folder or its allowed sub-folders).
pub const HOST_FORBIDDEN_PATH: i32 = -5;
/// Host function status: the module's storage quota would be exceeded.
pub const HOST_QUOTA: i32 = -6;
/// Host function status: the service failed.
pub const HOST_IO: i32 = -7;
/// Host function status: network target refused (not HTTPS, domain not
/// allowed, IP literal, local or private address, redirect elsewhere).
pub const HOST_FORBIDDEN_NET: i32 = -8;
/// Host function status: the request could not be understood.
pub const HOST_MALFORMED: i32 = -9;
/// Host function status: the model module was refused (missing, not the
/// approved one, bad format or bytes) or failed to run.
pub const HOST_MODEL_REFUSED: i32 = -10;

/// Resource limits for one run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Fuel: roughly one unit per WASM instruction.
    pub fuel: u64,
    /// Largest linear memory, in bytes.
    pub memory_bytes: usize,
    /// Wall-clock time for the whole run, WASM and host calls together.
    pub timeout: Duration,
    /// Longest single host call; each call gets the smaller of this and the
    /// time left in [`Limits::timeout`].
    pub host_call: Duration,
}

impl Limits {
    /// Conservative defaults: 10 million fuel, 16 MiB, 2 seconds in total,
    /// 1 second per host call.
    pub const DEFAULT: Self = Self {
        fuel: 10_000_000,
        memory_bytes: 16 * 1024 * 1024,
        timeout: Duration::from_secs(2),
        host_call: Duration::from_secs(1),
    };
}

/// Why a module did not run to the end.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SandboxError {
    /// The WASM bytes are not the approved ones.
    #[error("WASM does not match the approved digest")]
    DigestMismatch,
    /// Not valid WASM, or it imports something it was not granted.
    #[error("module refused: {0}")]
    Refused(String),
    /// The entry function does not exist or has the wrong type.
    #[error("missing entry function {0}")]
    MissingEntry(String),
    /// Ran out of fuel (too many instructions).
    #[error("out of fuel")]
    OutOfFuel,
    /// Ran past the time limit.
    #[error("time limit reached")]
    Timeout,
    /// Asked for more memory than allowed.
    #[error("memory limit reached")]
    MemoryLimit,
    /// Asked for more table elements than allowed.
    #[error("table limit reached")]
    TableLimit,
    /// Any other trap (division by zero, unreachable, ...).
    #[error("module trapped: {0}")]
    Trapped(String),
    /// The interface version of the approval and the module's exports do
    /// not match, or the wrong call was used for it (`docs/abi-v2.md`).
    #[error("wrong interface: {0}")]
    WrongAbi(String),
    /// The input data or settings were refused before the run.
    #[error("input refused: {0}")]
    BadInput(String),
    /// The module returned an error status or an invalid output.
    #[error("output refused: {0}")]
    BadOutput(String),
}

/// Cancellation token of one host call. When the call's time budget is
/// spent, the sandbox cancels it; a service must make every effect (write,
/// send, AI request) through [`CallToken::commit`], last and in one step, so
/// that no effect can start after the deadline.
///
/// Remaining limit: a system call already running inside `commit` (for
/// example a rename) cannot be stopped halfway; the sandbox then waits for it
/// to finish before cancelling, which is why effects are short and atomic.
#[derive(Debug, Clone, Default)]
pub struct CallToken {
    cancelled: Arc<Mutex<bool>>,
}

impl CallToken {
    /// True once the call was cancelled.
    pub fn is_cancelled(&self) -> bool {
        *self
            .cancelled
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Runs `effect` only if the call is not cancelled, holding the token so
    /// cancellation waits for it. Returns `None` if cancelled.
    pub fn commit<R>(&self, effect: impl FnOnce() -> R) -> Option<R> {
        let cancelled = self
            .cancelled
            .lock()
            .unwrap_or_else(PoisonError::into_inner);
        if *cancelled {
            return None;
        }
        Some(effect())
    }

    /// Writes `bytes` to a temporary file next to `path`, then renames it
    /// onto `path` only if the call is still valid. Returns `Ok(false)` (and
    /// leaves no file) if it was cancelled.
    ///
    /// # Errors
    /// A file system error.
    pub fn write_atomically(&self, path: &Path, bytes: &[u8]) -> std::io::Result<bool> {
        let mut tmp = path.as_os_str().to_owned();
        tmp.push(".argos-tmp");
        std::fs::write(&tmp, bytes)?;
        match self.commit(|| std::fs::rename(&tmp, path)) {
            Some(renamed) => renamed.map(|()| true),
            None => std::fs::remove_file(&tmp).map(|()| false),
        }
    }

    /// A token already cancelled, as after a timeout (for service tests).
    pub fn cancelled() -> Self {
        let token = Self::default();
        token.cancel();
        token
    }

    fn cancel(&self) {
        *self
            .cancelled
            .lock()
            .unwrap_or_else(PoisonError::into_inner) = true;
    }
}

/// The services behind the host functions (files, network, AI). A call gets
/// the capability (already checked), the request bytes (already bounded) and
/// its cancellation token, and returns a status. The sandbox runs each call
/// on its own thread and stops waiting when its time budget is spent, so a
/// service that blocks can never hold the module past its time limit, and
/// cancels the token so no effect happens afterwards.
pub trait HostServices: Send + Sync + 'static {
    /// Handles one host call: the answer bytes (copied into the module's
    /// output buffer) or a negative `HOST_*` status.
    ///
    /// # Errors
    /// A negative `HOST_*` status.
    fn call(
        &self,
        capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32>;
}

/// No service: every valid granted call answers nothing ([`HOST_OK`], 0 bytes).
#[derive(Debug, Clone, Copy)]
pub struct NoServices;

impl HostServices for NoServices {
    fn call(
        &self,
        _capability: Capability,
        _request: &[u8],
        _token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        Ok(Vec::new())
    }
}

/// Per-run host state: limits, deadline and what the module was granted.
pub(crate) struct HostState {
    memory_limit: usize,
    memory_denied: bool,
    table_denied: bool,
    host_timed_out: bool,
    granted: Vec<Capability>,
    deadline: Instant,
    host_call: Duration,
    services: Arc<dyn HostServices>,
}

impl ResourceLimiter for HostState {
    fn memory_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > self.memory_limit {
            self.memory_denied = true;
            return Err(wasmtime::format_err!("memory limit reached"));
        }
        Ok(true)
    }

    fn table_growing(
        &mut self,
        _current: usize,
        desired: usize,
        _maximum: Option<usize>,
    ) -> wasmtime::Result<bool> {
        if desired > MAX_TABLE_ELEMENTS {
            self.table_denied = true;
            return Err(wasmtime::format_err!("table limit reached"));
        }
        Ok(true)
    }

    // One module per store: one instance, one memory, one table.
    fn instances(&self) -> usize {
        1
    }

    fn memories(&self) -> usize {
        1
    }

    fn tables(&self) -> usize {
        1
    }
}

/// The sandbox: one engine, reused for every run.
#[derive(Debug)]
pub struct Sandbox {
    engine: Engine,
}

impl Sandbox {
    /// Builds the engine with fuel metering and epoch interruption on.
    ///
    /// # Errors
    /// [`SandboxError::Refused`] if wasmtime cannot build the engine.
    pub fn new() -> Result<Self, SandboxError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.epoch_interruption(true);
        let engine = Engine::new(&config).map_err(|e| SandboxError::Refused(e.to_string()))?;
        Ok(Self { engine })
    }

    /// Runs `entry` (a function `() -> i32`) of an approved module, with no
    /// host service ([`NoServices`]) and the caller's limits.
    ///
    /// # Errors
    /// [`SandboxError`] describing why the module was refused or stopped.
    pub fn run(
        &self,
        approval: &Approval,
        wasm: &[u8],
        entry: &str,
        limits: Limits,
    ) -> Result<i32, SandboxError> {
        self.run_with(approval, wasm, entry, limits, Arc::new(NoServices))
    }

    /// Runs `entry` with the given host services and the caller's limits.
    ///
    /// # Errors
    /// [`SandboxError`] describing why the module was refused or stopped;
    /// [`SandboxError::Timeout`] also when a host call outlives its budget.
    pub fn run_with(
        &self,
        approval: &Approval,
        wasm: &[u8],
        entry: &str,
        limits: Limits,
        services: Arc<dyn HostServices>,
    ) -> Result<i32, SandboxError> {
        self.run_approved(approval, wasm, entry, limits, services)
    }

    /// Runs `entry` with the time budget of the approved policy, for both the
    /// whole run and the longest host call (the normal way to run a module).
    ///
    /// # Errors
    /// As [`Sandbox::run_with`].
    pub fn run_budgeted(
        &self,
        approval: &Approval,
        wasm: &[u8],
        entry: &str,
        limits: Limits,
        services: Arc<dyn HostServices>,
    ) -> Result<i32, SandboxError> {
        let budget = Duration::from_secs(u64::from(approval.policy.budget_secs));
        let limits = Limits {
            timeout: budget,
            host_call: budget,
            ..limits
        };
        self.run_approved(approval, wasm, entry, limits, services)
    }

    /// The interface v1 path: `entry` of type `() -> i32`.
    fn run_approved(
        &self,
        approval: &Approval,
        wasm: &[u8],
        entry: &str,
        limits: Limits,
        services: Arc<dyn HostServices>,
    ) -> Result<i32, SandboxError> {
        if approval.policy.abi != 1 {
            return Err(SandboxError::WrongAbi(
                "an interface v2 module runs with run_data".to_owned(),
            ));
        }
        self.run_instance(approval, wasm, limits, services, 1, |store, instance| {
            let func = instance
                .get_typed_func::<(), i32>(&mut *store, entry)
                .map_err(|_| SandboxError::MissingEntry(entry.to_owned()))?;
            func.call(&mut *store, ())
                .map_err(|e| classify(&e, store.data(), Phase::Call))
        })
    }

    /// The common path: approved digest and rights, whoever approved them;
    /// exports checked against interface `abi`; then `call` on the instance,
    /// under the fuel, memory and time limits.
    fn run_instance<R>(
        &self,
        approval: &Approval,
        wasm: &[u8],
        limits: Limits,
        services: Arc<dyn HostServices>,
        abi: u32,
        call: impl FnOnce(&mut Store<HostState>, &Instance) -> Result<R, SandboxError>,
    ) -> Result<R, SandboxError> {
        let start = Instant::now();
        let capabilities = &approval.capabilities;
        if sha384(wasm) != approval.wasm_sha384 {
            return Err(SandboxError::DigestMismatch);
        }
        let module =
            Module::new(&self.engine, wasm).map_err(|e| SandboxError::Refused(e.to_string()))?;
        data::check_exports(&module, abi)?;

        let mut linker: Linker<HostState> = Linker::new(&self.engine);
        for capability in capabilities {
            link_capability(&mut linker, *capability)?;
        }

        let mut store = Store::new(
            &self.engine,
            HostState {
                memory_limit: limits.memory_bytes,
                memory_denied: false,
                table_denied: false,
                host_timed_out: false,
                granted: capabilities.clone(),
                deadline: start + limits.timeout,
                host_call: limits.host_call,
                services,
            },
        );
        store.limiter(|state| state);
        store
            .set_fuel(limits.fuel)
            .map_err(|e| SandboxError::Refused(e.to_string()))?;
        store.set_epoch_deadline(1);

        // A watchdog bumps the epoch once the time is up, which interrupts the module.
        let done = Arc::new(AtomicBool::new(false));
        let watchdog = {
            let engine = self.engine.clone();
            let done = Arc::clone(&done);
            std::thread::spawn(move || {
                let step = Duration::from_millis(10);
                let mut waited = Duration::ZERO;
                while waited < limits.timeout && !done.load(Ordering::Relaxed) {
                    std::thread::sleep(step);
                    waited += step;
                }
                engine.increment_epoch();
            })
        };

        let result = linker
            .instantiate(&mut store, &module)
            .map_err(|e| classify(&e, store.data(), Phase::Instantiate))
            .and_then(|instance| call(&mut store, &instance));
        done.store(true, Ordering::Relaxed);
        let _ = watchdog.join();
        result
    }
}

/// Name of the host function (in [`HOST_MODULE`]) behind a capability.
pub fn host_function(capability: Capability) -> &'static str {
    match capability {
        Capability::ReadFiles => "read_file",
        Capability::WriteFiles => "write_file",
        Capability::Network => "network_request",
        Capability::Ai => "ai_request",
        Capability::Model => "model_request",
    }
}

/// Adds the host functions of one granted capability. Nothing is linked for
/// capabilities that are not granted, so their imports cannot resolve.
///
/// Every host function has the shape `(req_ptr, req_len, out_ptr, out_cap) -> i32`:
/// the module passes request bytes and an output buffer; the result is the
/// number of bytes written to the buffer, or a negative `HOST_*` status.
fn link_capability(
    linker: &mut Linker<HostState>,
    capability: Capability,
) -> Result<(), SandboxError> {
    let name = host_function(capability);
    linker
        .func_wrap(
            HOST_MODULE,
            name,
            move |mut caller: Caller<'_, HostState>,
                  ptr: i32,
                  len: i32,
                  out_ptr: i32,
                  out_cap: i32|
                  -> wasmtime::Result<i32> {
                // Permission first, before reading anything.
                if !caller.data().granted.contains(&capability) {
                    return Ok(HOST_DENIED);
                }
                let request = match request(&mut caller, ptr, len) {
                    Ok(request) => request,
                    Err(status) => return Ok(status),
                };
                let out = match output_range(&mut caller, out_ptr, out_cap) {
                    Ok(out) => out,
                    Err(status) => return Ok(status),
                };
                // Budget: this call's limit, within the time left for the run.
                let state = caller.data();
                let left = state.deadline.saturating_duration_since(Instant::now());
                let budget = state.host_call.min(left);
                let services = Arc::clone(&state.services);
                let token = CallToken::default();
                let service_token = token.clone();
                let (send, receive) = mpsc::channel();
                std::thread::spawn(move || {
                    let _ = send.send(services.call(capability, &request, &service_token));
                });
                let Ok(answer) = receive.recv_timeout(budget) else {
                    // No effect may start from now on; the module stops here.
                    token.cancel();
                    caller.data_mut().host_timed_out = true;
                    return Err(wasmtime::format_err!("host call timed out"));
                };
                Ok(match answer {
                    Ok(bytes) => write_output(&mut caller, out, &bytes),
                    Err(status) if status < 0 => status,
                    Err(_) => HOST_IO,
                })
            },
        )
        .map(|_| ())
        .map_err(|e| SandboxError::Refused(e.to_string()))
}

/// The module's exported memory, if any.
fn memory(caller: &mut Caller<'_, HostState>) -> Option<wasmtime::Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => Some(memory),
        _ => None,
    }
}

/// Checks a pointer and length against the module's memory; returns the range.
fn range(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    len: i32,
) -> Result<std::ops::Range<usize>, i32> {
    let (Ok(ptr), Ok(len)) = (usize::try_from(ptr), usize::try_from(len)) else {
        return Err(HOST_BAD_REQUEST);
    };
    if len > MAX_REQUEST_BYTES {
        return Err(HOST_TOO_LARGE);
    }
    let end = ptr.checked_add(len).ok_or(HOST_BAD_REQUEST)?;
    let size = memory(caller).ok_or(HOST_BAD_REQUEST)?.data_size(&*caller);
    if end > size {
        return Err(HOST_BAD_REQUEST);
    }
    Ok(ptr..end)
}

/// Copies the request bytes out of the module's exported memory, refusing a
/// negative pointer or length, an oversized request, an overflow and bytes
/// outside the memory.
fn request(caller: &mut Caller<'_, HostState>, ptr: i32, len: i32) -> Result<Vec<u8>, i32> {
    let bytes = range(caller, ptr, len)?;
    let memory = memory(caller).ok_or(HOST_BAD_REQUEST)?;
    memory
        .data(&*caller)
        .get(bytes)
        .map(<[u8]>::to_vec)
        .ok_or(HOST_BAD_REQUEST)
}

/// Checks the output buffer before the call (same rules as the request).
fn output_range(
    caller: &mut Caller<'_, HostState>,
    ptr: i32,
    cap: i32,
) -> Result<std::ops::Range<usize>, i32> {
    range(caller, ptr, cap)
}

/// Writes the service's answer into the output buffer; returns its length or
/// [`HOST_TOO_LARGE`] if it does not fit.
fn write_output(
    caller: &mut Caller<'_, HostState>,
    out: std::ops::Range<usize>,
    bytes: &[u8],
) -> i32 {
    if bytes.len() > out.len() {
        return HOST_TOO_LARGE;
    }
    let Some(memory) = memory(caller) else {
        return HOST_BAD_REQUEST;
    };
    let target = out.start..out.start + bytes.len();
    match memory.data_mut(&mut *caller).get_mut(target) {
        Some(slot) => {
            slot.copy_from_slice(bytes);
            i32::try_from(bytes.len()).unwrap_or(HOST_TOO_LARGE)
        }
        None => HOST_BAD_REQUEST,
    }
}

/// Which step failed.
#[derive(Clone, Copy)]
pub(crate) enum Phase {
    Instantiate,
    Call,
}

pub(crate) fn classify(error: &wasmtime::Error, state: &HostState, phase: Phase) -> SandboxError {
    if state.memory_denied {
        return SandboxError::MemoryLimit;
    }
    if state.table_denied {
        return SandboxError::TableLimit;
    }
    if state.host_timed_out {
        return SandboxError::Timeout;
    }
    match error.downcast_ref::<Trap>() {
        Some(Trap::OutOfFuel) => SandboxError::OutOfFuel,
        Some(Trap::Interrupt) => SandboxError::Timeout,
        Some(trap) => SandboxError::Trapped(trap.to_string()),
        None => match phase {
            Phase::Instantiate => SandboxError::Refused(error.to_string()),
            Phase::Call => SandboxError::Trapped(error.to_string()),
        },
    }
}
