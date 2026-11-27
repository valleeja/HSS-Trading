//! Argos module kit (step 6, piece D1).
//!
//! A module written with this kit keeps `#![forbid(unsafe_code)]`: the only
//! `unsafe` it needs (calling the host and exporting its entry) lives here,
//! reviewed once, and only when compiled to `wasm32`.
//!
//! - [`entry!`] exports the module's entry as [`ENTRY`] (`() -> i32`). The
//!   name `main` is avoided: on `wasm32` the toolchain rewrites it to the C
//!   `main(argc, argv)` signature.
//! - [`read_file`], [`write_file`], [`network_request`] and [`ai_request`] pass
//!   request bytes to the `argos` host functions and receive the answer in a
//!   buffer. A function the module never calls is never imported, and the
//!   sandbox refuses any import the approved manifest does not grant.
//!
//! Outside `wasm32` (for unit tests of a module's logic), no host exists and
//! every call returns [`HostError::Unavailable`].

#![no_std]
#![cfg_attr(not(target_arch = "wasm32"), forbid(unsafe_code))]
#![cfg_attr(target_arch = "wasm32", deny(unsafe_code))]

extern crate alloc;

pub mod data;

pub use data::{Bar, Card, DecodeError, Input, Label, Line, Output, EMPTY};

/// Name of the WASM import module that holds the host functions.
pub const HOST_MODULE: &str = "argos";

/// Name of the exported entry function the sandbox calls.
pub const ENTRY: &str = "run";

/// Why a host call failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostError {
    /// The request or buffer does not fit the 32-bit host interface.
    TooLarge,
    /// No Argos host (not running inside the sandbox).
    Unavailable,
    /// The host refused or failed: a negative status (see the sandbox's
    /// `HOST_*` constants, for example -5 for a refused path).
    Status(i32),
}

/// Exports `$run` (a `fn() -> i32`) as the module entry [`ENTRY`].
///
/// ```ignore
/// #![forbid(unsafe_code)]
/// fn run() -> i32 { 0 }
/// argos_module_sdk::entry!(run);
/// ```
#[macro_export]
macro_rules! entry {
    ($run:path) => {
        /// Module entry called by the Argos sandbox (exported as `run`).
        #[cfg_attr(target_arch = "wasm32", unsafe(export_name = "run"))]
        pub extern "C" fn argos_entry() -> i32 {
            $run()
        }
    };
}

/// Exports `$run` (a `fn(Input) -> Output`) as an interface v2 module
/// (`docs/abi-v2.md`): `argos_abi`, `alloc`, `free` and `run_data`. A v2
/// module does not use [`entry!`].
///
/// ```ignore
/// #![forbid(unsafe_code)]
/// use argos_module_sdk::{Input, Output};
/// fn run(input: Input) -> Output { Output::default() }
/// argos_module_sdk::entry_data!(run);
/// ```
#[macro_export]
macro_rules! entry_data {
    ($run:path) => {
        /// Interface version of this module (exported as `argos_abi`).
        #[cfg_attr(target_arch = "wasm32", unsafe(export_name = "argos_abi"))]
        pub extern "C" fn argos_entry_abi() -> i32 {
            $crate::__abi::abi()
        }
        /// Reserves `len` bytes for the input (exported as `alloc`).
        #[cfg_attr(target_arch = "wasm32", unsafe(export_name = "alloc"))]
        pub extern "C" fn argos_entry_alloc(len: i32) -> i32 {
            $crate::__abi::alloc(len)
        }
        /// Releases a reservation; a no-op (exported as `free`).
        #[cfg_attr(target_arch = "wasm32", unsafe(export_name = "free"))]
        pub extern "C" fn argos_entry_free(ptr: i32, len: i32) {
            $crate::__abi::free(ptr, len)
        }
        /// Runs the module on encoded input (exported as `run_data`).
        #[cfg_attr(target_arch = "wasm32", unsafe(export_name = "run_data"))]
        pub extern "C" fn argos_entry_run_data(ptr: i32, len: i32) -> i64 {
            $crate::__abi::run_data(ptr, len, $run)
        }
    };
}

/// Support code of [`entry_data!`]; not a stable interface.
#[doc(hidden)]
pub mod __abi {
    use crate::data::{Input, Output, ABI, MAX_BYTES};
    use alloc::vec::Vec;

    /// The input bytes are not a valid v2 input.
    pub const BAD_INPUT: i64 = -1;
    /// The encoded output is over the size limit.
    pub const OUTPUT_TOO_LARGE: i64 = -2;
    /// No input memory (not running inside the sandbox).
    pub const UNAVAILABLE: i64 = -3;

    /// The interface version.
    #[must_use]
    pub fn abi() -> i32 {
        i32::try_from(ABI).unwrap_or(-1)
    }

    /// Leaks a zeroed buffer of `len` bytes and returns its address, or -1.
    /// A fresh instance serves every call, so nothing needs freeing.
    #[must_use]
    pub fn alloc(len: i32) -> i32 {
        let Some(n) = usize::try_from(len).ok().filter(|n| *n <= MAX_BYTES) else {
            return -1;
        };
        let buf: &'static mut [u8] = alloc::vec![0; n].leak();
        i32::try_from(buf.as_mut_ptr().expose_provenance()).unwrap_or(-1)
    }

    /// Does nothing (see [`alloc`]).
    pub fn free(_ptr: i32, _len: i32) {}

    /// Decodes `input`, runs `run` and encodes its output: the pure part
    /// of [`run_data`].
    ///
    /// # Errors
    /// [`BAD_INPUT`] or [`OUTPUT_TOO_LARGE`].
    pub fn run_bytes(input: &[u8], run: impl FnOnce(Input) -> Output) -> Result<Vec<u8>, i64> {
        let input = Input::decode(input).map_err(|_| BAD_INPUT)?;
        let out = run(input).encode();
        if out.len() > MAX_BYTES {
            return Err(OUTPUT_TOO_LARGE);
        }
        Ok(out)
    }

    /// Runs the module on the `len` input bytes at `ptr`; returns
    /// `(output address << 32) | output length`, or a negative status.
    pub fn run_data(ptr: i32, len: i32, run: impl FnOnce(Input) -> Output) -> i64 {
        let Some(input) = input_bytes(ptr, len) else {
            return UNAVAILABLE;
        };
        match run_bytes(input, run) {
            Ok(out) => {
                let out: &'static [u8] = out.leak();
                pack(out.as_ptr().expose_provenance(), out.len())
            }
            Err(status) => status,
        }
    }

    /// Packs an output address and length, or [`OUTPUT_TOO_LARGE`].
    #[must_use]
    pub fn pack(addr: usize, len: usize) -> i64 {
        match (u32::try_from(addr), u32::try_from(len)) {
            (Ok(addr), Ok(len)) => {
                i64::try_from((u64::from(addr) << 32) | u64::from(len)).unwrap_or(OUTPUT_TOO_LARGE)
            }
            _ => OUTPUT_TOO_LARGE,
        }
    }

    /// The input bytes the host copied into a buffer from [`alloc`].
    #[cfg(target_arch = "wasm32")]
    #[expect(
        unsafe_code,
        reason = "read the input the host wrote into module memory"
    )]
    fn input_bytes(ptr: i32, len: i32) -> Option<&'static [u8]> {
        let addr = usize::try_from(ptr).ok()?;
        let n = usize::try_from(len).ok().filter(|n| *n <= MAX_BYTES)?;
        if n == 0 {
            return Some(&[]);
        }
        // SAFETY: by the v2 contract (docs/abi-v2.md), the host calls
        // `run_data` with the address returned by `alloc(len)` (exposed
        // above, never freed) and the same `len`, after writing the input
        // there; nothing in the module writes to that leaked buffer again.
        Some(unsafe {
            core::slice::from_raw_parts(core::ptr::with_exposed_provenance::<u8>(addr), n)
        })
    }

    /// No host memory outside `wasm32`.
    #[cfg(not(target_arch = "wasm32"))]
    fn input_bytes(_ptr: i32, _len: i32) -> Option<&'static [u8]> {
        None
    }
}

/// A length or address as the host's `i32` argument.
fn to_host(value: usize) -> Result<i32, HostError> {
    i32::try_from(value).map_err(|_| HostError::TooLarge)
}

/// Host imports, `wasm32` only. This is the kit's whole `unsafe` surface.
#[cfg(target_arch = "wasm32")]
#[expect(unsafe_code, reason = "WASM host imports: the module's only boundary")]
mod host {
    #[link(wasm_import_module = "argos")]
    unsafe extern "C" {
        pub(super) fn read_file(ptr: i32, len: i32, out_ptr: i32, out_cap: i32) -> i32;
        pub(super) fn write_file(ptr: i32, len: i32, out_ptr: i32, out_cap: i32) -> i32;
        pub(super) fn network_request(ptr: i32, len: i32, out_ptr: i32, out_cap: i32) -> i32;
        pub(super) fn ai_request(ptr: i32, len: i32, out_ptr: i32, out_cap: i32) -> i32;
        pub(super) fn model_request(ptr: i32, len: i32, out_ptr: i32, out_cap: i32) -> i32;
    }
}

macro_rules! host_call {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        ///
        /// Returns the number of answer bytes written to `out`.
        ///
        /// # Errors
        /// [`HostError::TooLarge`] if the request or buffer does not fit in
        /// 32 bits; [`HostError::Status`] if the host refused or failed;
        /// [`HostError::Unavailable`] outside the sandbox.
        pub fn $name(request: &[u8], out: &mut [u8]) -> Result<usize, HostError> {
            let len = to_host(request.len())?;
            let cap = to_host(out.len())?;
            #[cfg(target_arch = "wasm32")]
            {
                let ptr = to_host(request.as_ptr().addr())?;
                let out_ptr = to_host(out.as_mut_ptr().addr())?;
                #[expect(unsafe_code, reason = "call into the Argos host")]
                // SAFETY: `ptr`/`len` describe `request` and `out_ptr`/`cap`
                // describe `out`, both borrowed for the whole call (`out`
                // mutably, so nothing else aliases it); the host checks both
                // ranges against this module's memory, only reads the request
                // and writes at most `cap` bytes into `out`, and keeps no
                // reference after it returns.
                let status = unsafe { host::$name(ptr, len, out_ptr, cap) };
                usize::try_from(status).map_err(|_| HostError::Status(status))
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                let _ = (len, cap);
                Err(HostError::Unavailable)
            }
        }
    };
}

host_call!(
    /// Asks the host to read a file (needs the `ReadFiles` right).
    read_file
);
host_call!(
    /// Asks the host to write a file (needs the `WriteFiles` right).
    write_file
);
host_call!(
    /// Asks the host for a network request (needs the `Network` right).
    network_request
);
host_call!(
    /// Asks the host for an AI request (needs the `Ai` right).
    ai_request
);
host_call!(
    /// Runs the approved model module (needs the `Model` right). Request:
    /// rank (u32 LE), dimensions (u32 LE), `f32` values (LE); answer: `f32`
    /// values (LE).
    model_request
);

#[cfg(test)]
mod tests {
    use super::{to_host, HostError};

    #[test]
    fn small_values_fit_the_host_interface() {
        assert_eq!(to_host(0), Ok(0));
        assert_eq!(to_host(3), Ok(3));
    }

    #[test]
    fn values_beyond_32_bits_are_refused() {
        assert_eq!(to_host(usize::MAX), Err(HostError::TooLarge));
    }
}
