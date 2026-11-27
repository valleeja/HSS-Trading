//! A v2 module crate with `forbid(unsafe_code)` uses `entry_data!` on the host.
#![forbid(unsafe_code)]

use argos_module_sdk::__abi::{pack, OUTPUT_TOO_LARGE, UNAVAILABLE};
use argos_module_sdk::{Input, Output};

fn run(_input: Input) -> Output {
    Output::default()
}

argos_module_sdk::entry_data!(run);

#[test]
fn the_v2_exports_answer_on_the_host() {
    assert_eq!(argos_entry_abi(), 2);
    // Host addresses do not fit 32 bits, and no input memory exists.
    assert_eq!(argos_entry_alloc(-1), -1);
    assert_eq!(argos_entry_alloc(i32::MAX), -1);
    argos_entry_free(0, 0);
    assert_eq!(argos_entry_run_data(0, 0), UNAVAILABLE);
}

#[test]
fn results_pack_address_and_length() {
    assert_eq!(pack(0x10, 3), (0x10 << 32) | 3);
    assert_eq!(pack(0, 0), 0);
    // An address at or over 2^31 would read as a negative status.
    assert_eq!(pack(0x8000_0000, 1), OUTPUT_TOO_LARGE);
    assert_eq!(pack(usize::MAX, 1), OUTPUT_TOO_LARGE);
}
