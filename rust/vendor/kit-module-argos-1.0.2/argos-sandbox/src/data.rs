//! Interface v2 (`docs/abi-v2.md`): the host gives a module typed input data
//! (bars and checked settings) and receives bounded outputs (lines, labels,
//! cards). Nothing else changes: same approval, rights, limits and services.

use std::sync::Arc;
use std::time::Duration;

pub use argos_module_sdk::data::{MAX_BARS, MAX_BYTES};
pub use argos_module_sdk::{Bar, Card, Input, Label, Line, Output, EMPTY};
use wasmtime::{Instance, Memory, Module, Store};

use crate::{
    classify, Approval, HostServices, HostState, Limits, Phase, Sandbox, SandboxError, ENTRY,
};

/// The export that returns the interface version.
pub const ABI_EXPORT: &str = "argos_abi";
/// The functions only an interface v2 module exports.
pub const V2_EXPORTS: [&str; 4] = [ABI_EXPORT, "alloc", "free", "run_data"];

/// Refuses a module whose exports mix the two interfaces: a v1 module must
/// not export [`V2_EXPORTS`], a v2 module must not export [`ENTRY`].
pub(crate) fn check_exports(module: &Module, abi: u32) -> Result<(), SandboxError> {
    let exported = |name: &str| module.exports().any(|e| e.name() == name);
    let mixed = match abi {
        1 => V2_EXPORTS.iter().copied().find(|n| exported(n)),
        _ => exported(ENTRY).then_some(ENTRY),
    };
    match mixed {
        Some(name) => Err(SandboxError::WrongAbi(format!(
            "interface v{abi} module exports {name}"
        ))),
        None => Ok(()),
    }
}

impl Sandbox {
    /// Runs an interface v2 module on `input` and returns its checked output.
    /// The chosen settings (`input.settings`) are checked against the
    /// approved policy; the time budget is the policy's, as in
    /// [`Sandbox::run_budgeted`]; fuel and memory come from `limits`.
    ///
    /// # Errors
    /// [`SandboxError::WrongAbi`] if the approval or the module is not v2;
    /// [`SandboxError::BadInput`] for an input or a setting refused before
    /// the run; [`SandboxError::BadOutput`] for an error status or an output
    /// that is too large, outside the module's memory or invalid; any other
    /// [`SandboxError`] as for a v1 run.
    pub fn run_data(
        &self,
        approval: &Approval,
        wasm: &[u8],
        input: &Input,
        limits: Limits,
        services: Arc<dyn HostServices>,
    ) -> Result<Output, SandboxError> {
        if approval.policy.abi != 2 {
            return Err(SandboxError::WrongAbi(
                "the approval is not for an interface v2 module".to_owned(),
            ));
        }
        approval
            .policy
            .check_values(&input.settings)
            .map_err(|e| SandboxError::BadInput(e.to_string()))?;
        let bytes = input.encode();
        if bytes.len() > MAX_BYTES {
            return Err(SandboxError::BadInput("input over 4 MiB".to_owned()));
        }
        // The same checks a module applies: what the host sends is valid.
        Input::decode(&bytes).map_err(|e| SandboxError::BadInput(format!("{e:?}")))?;
        let budget = Duration::from_secs(u64::from(approval.policy.budget_secs));
        let limits = Limits {
            timeout: budget,
            host_call: budget,
            ..limits
        };
        let raw = self.run_instance(approval, wasm, limits, services, 2, |store, instance| {
            call_v2(store, instance, &bytes)
        })?;
        Output::decode(&raw, input.bars.len())
            .map_err(|e| SandboxError::BadOutput(format!("{e:?}")))
    }
}

/// Calls the v2 exports: version, `alloc`, copy, `run_data`, read back.
fn call_v2(
    store: &mut Store<HostState>,
    instance: &Instance,
    input: &[u8],
) -> Result<Vec<u8>, SandboxError> {
    let missing = |name: &str| SandboxError::WrongAbi(format!("missing or mistyped export {name}"));
    let trap =
        |e: &wasmtime::Error, store: &Store<HostState>| classify(e, store.data(), Phase::Call);
    let abi = instance
        .get_typed_func::<(), i32>(&mut *store, ABI_EXPORT)
        .map_err(|_| missing(ABI_EXPORT))?;
    let alloc = instance
        .get_typed_func::<i32, i32>(&mut *store, "alloc")
        .map_err(|_| missing("alloc"))?;
    instance
        .get_typed_func::<(i32, i32), ()>(&mut *store, "free")
        .map_err(|_| missing("free"))?;
    let run = instance
        .get_typed_func::<(i32, i32), i64>(&mut *store, "run_data")
        .map_err(|_| missing("run_data"))?;
    let memory = instance
        .get_memory(&mut *store, "memory")
        .ok_or_else(|| missing("memory"))?;

    let version = abi.call(&mut *store, ()).map_err(|e| trap(&e, store))?;
    if version != 2 {
        return Err(SandboxError::WrongAbi(format!(
            "module says version {version}"
        )));
    }
    let len = i32::try_from(input.len()).map_err(|_| SandboxError::BadInput("too large".into()))?;
    let ptr = alloc.call(&mut *store, len).map_err(|e| trap(&e, store))?;
    let at = range(store, memory, i64::from(ptr), input.len())
        .ok_or_else(|| SandboxError::BadOutput(format!("alloc returned {ptr}")))?;
    memory
        .write(&mut *store, at, input)
        .map_err(|e| SandboxError::BadOutput(e.to_string()))?;
    let packed = run
        .call(&mut *store, (ptr, len))
        .map_err(|e| trap(&e, store))?;
    if packed < 0 {
        return Err(SandboxError::BadOutput(format!("module status {packed}")));
    }
    let out_ptr = packed >> 32;
    let out_len = usize::try_from(packed & 0xffff_ffff).unwrap_or(usize::MAX);
    if out_len > MAX_BYTES {
        return Err(SandboxError::BadOutput("output over 4 MiB".to_owned()));
    }
    let at = range(store, memory, out_ptr, out_len)
        .ok_or_else(|| SandboxError::BadOutput("output outside the module's memory".into()))?;
    let mut out = vec![0; out_len];
    memory
        .read(&*store, at, &mut out)
        .map_err(|e| SandboxError::BadOutput(e.to_string()))?;
    Ok(out)
}

/// The start of `len` bytes at `ptr` if they lie inside `memory`.
fn range(store: &Store<HostState>, memory: Memory, ptr: i64, len: usize) -> Option<usize> {
    let start = usize::try_from(ptr).ok()?;
    let end = start.checked_add(len)?;
    (end <= memory.data_size(store)).then_some(start)
}
