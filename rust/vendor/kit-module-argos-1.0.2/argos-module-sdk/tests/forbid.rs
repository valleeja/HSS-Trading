//! A module crate with `forbid(unsafe_code)` uses the kit on the host.
#![forbid(unsafe_code)]

use argos_module_sdk::{
    ai_request, model_request, network_request, read_file, write_file, HostError,
};

fn run() -> i32 {
    7
}

argos_module_sdk::entry!(run);

#[test]
fn the_entry_calls_the_module_function() {
    assert_eq!(argos_entry(), 7);
}

#[test]
fn outside_the_sandbox_no_host_is_reachable() {
    let mut out = [0_u8; 8];
    for call in [
        read_file,
        write_file,
        network_request,
        ai_request,
        model_request,
    ] {
        assert_eq!(call(b"anything", &mut out), Err(HostError::Unavailable));
    }
}
