//! Interface v2 on the host side, against hand-written hostile modules: bad
//! statuses, outputs too large or outside memory, garbage, wrong versions,
//! mixed exports, endless loops.

use std::sync::Arc;

use argos_sandbox::data::{Bar, Input};
use argos_sandbox::{Approval, Limits, NoServices, Sandbox, SandboxError, ServicePolicy, ENTRY};

/// A valid empty output: version 2, no line, no label, no card.
const EMPTY_OUTPUT: &str = r"\02\00\00\00\00\00\00\00\00\00";

/// A v2 module whose pieces can be swapped.
fn module(abi: &str, alloc: &str, run_data: &str, data: &str, extra: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (data (i32.const 2048) "{data}")
            (func (export "argos_abi") (result i32) {abi})
            (func (export "alloc") (param i32) (result i32) {alloc})
            (func (export "free") (param i32 i32))
            (func (export "run_data") (param i32 i32) (result i64) {run_data})
            {extra})"#
    ))
    .expect("wat")
}

/// `(2048 << 32) | len`: the output at address 2048.
fn at_2048(len: u64) -> String {
    format!("i64.const {}", (2048_u64 << 32) | len)
}

fn wrong<T>(result: &Result<T, SandboxError>) -> bool {
    matches!(result, Err(SandboxError::WrongAbi(_)))
}

fn good() -> Vec<u8> {
    module(
        "i32.const 2",
        "i32.const 1024",
        &at_2048(10),
        EMPTY_OUTPUT,
        "",
    )
}

fn v2_policy() -> ServicePolicy {
    ServicePolicy {
        abi: 2,
        ..ServicePolicy::default()
    }
}

fn input(n: i64) -> Input {
    Input {
        timeframe: 60,
        price_scale: 2,
        volume_scale: 0,
        symbol: "TEST".into(),
        bars: (0..n)
            .map(|t| Bar {
                t,
                o: 1,
                h: 1,
                l: 1,
                c: 1,
                v: 1,
            })
            .collect(),
        settings: Vec::new(),
    }
}

fn run(wasm: &[u8], input: &Input) -> Result<argos_sandbox::data::Output, SandboxError> {
    let approval = Approval::of(wasm, Vec::new(), v2_policy());
    let limits = Limits {
        fuel: 1_000_000,
        ..Limits::DEFAULT
    };
    Sandbox::new()
        .expect("engine")
        .run_data(&approval, wasm, input, limits, Arc::new(NoServices))
}

#[test]
fn a_valid_module_returns_its_output() {
    let out = run(&good(), &input(0)).expect("run");
    assert!(out.lines.is_empty() && out.labels.is_empty() && out.cards.is_empty());
}

#[test]
fn bad_outputs_are_refused() {
    let bad_output = |run_data: &str, data: &str| {
        let wasm = module("i32.const 2", "i32.const 1024", run_data, data, "");
        run(&wasm, &input(0))
    };
    let refused = |r: Result<_, SandboxError>| matches!(r, Err(SandboxError::BadOutput(_)));
    // An error status of the module.
    assert!(refused(bad_output("i64.const -1", EMPTY_OUTPUT)));
    // More than 4 MiB, outside the memory, garbage, trailing bytes.
    assert!(refused(bad_output(&at_2048(5 * 1024 * 1024), EMPTY_OUTPUT)));
    assert!(refused(bad_output(
        &format!("i64.const {}", (100_000_u64 << 32) | 0xa),
        EMPTY_OUTPUT
    )));
    assert!(refused(bad_output(
        &format!("i64.const {}", (65_530_u64 << 32) | 0xa),
        EMPTY_OUTPUT
    )));
    assert!(refused(bad_output(
        &at_2048(10),
        r"\01\00\00\00\00\00\00\00\00\00"
    )));
    assert!(refused(bad_output(&at_2048(11), EMPTY_OUTPUT)));
    // One line "x" with no value: valid for 0 bars, refused for 3.
    let one_line = r"\02\00\00\00\01\00\01\00x\00\00\00\00\00\00\00\00\00";
    let wasm = module("i32.const 2", "i32.const 1024", &at_2048(18), one_line, "");
    assert!(run(&wasm, &input(0)).is_ok());
    assert!(refused(run(&wasm, &input(3))));
}

#[test]
fn bad_allocations_are_refused() {
    for alloc in ["i32.const -5", "i32.const 65530", "i32.const 70000"] {
        let wasm = module("i32.const 2", alloc, &at_2048(10), EMPTY_OUTPUT, "");
        assert!(
            matches!(run(&wasm, &input(1)), Err(SandboxError::BadOutput(_))),
            "{alloc}"
        );
    }
}

#[test]
fn wrong_versions_and_mixed_exports_are_refused() {
    let v3 = module(
        "i32.const 3",
        "i32.const 1024",
        &at_2048(10),
        EMPTY_OUTPUT,
        "",
    );
    assert!(wrong(&run(&v3, &input(0))));
    let both = module(
        "i32.const 2",
        "i32.const 1024",
        &at_2048(10),
        EMPTY_OUTPUT,
        r#"(func (export "run") (result i32) i32.const 0)"#,
    );
    assert!(wrong(&run(&both, &input(0))));
    // A v2 approval cannot run a v1 module, and a v1 run cannot use a v2 module.
    let v1 =
        wat::parse_str(r#"(module (func (export "run") (result i32) i32.const 7))"#).expect("wat");
    assert!(wrong(&run(&v1, &input(0))));
    let v1_approval = Approval::of(&good(), Vec::new(), ServicePolicy::default());
    let sandbox = Sandbox::new().expect("engine");
    assert!(wrong(&sandbox.run(
        &v1_approval,
        &good(),
        ENTRY,
        Limits::DEFAULT
    )));
    let v2_approval = Approval::of(&v1, Vec::new(), v2_policy());
    assert!(wrong(&sandbox.run(
        &v2_approval,
        &v1,
        ENTRY,
        Limits::DEFAULT
    )));
    // A v1 module that also exports run_data is refused too.
    let mixed_v1 = wat::parse_str(
        r#"(module
            (func (export "run") (result i32) i32.const 7)
            (func (export "run_data") (param i32 i32) (result i64) i64.const 0))"#,
    )
    .expect("wat");
    let approval = Approval::of(&mixed_v1, Vec::new(), ServicePolicy::default());
    assert!(wrong(&sandbox.run(
        &approval,
        &mixed_v1,
        ENTRY,
        Limits::DEFAULT
    )));
    // A missing or mistyped v2 export.
    let no_free = wat::parse_str(
        r#"(module
            (memory (export "memory") 1)
            (func (export "argos_abi") (result i32) i32.const 2)
            (func (export "alloc") (param i32) (result i32) i32.const 1024)
            (func (export "run_data") (param i32 i32) (result i64) i64.const -1))"#,
    )
    .expect("wat");
    assert!(wrong(&run(&no_free, &input(0))));
    let i32_result = wat::parse_str(
        r#"(module
            (memory (export "memory") 1)
            (func (export "argos_abi") (result i32) i32.const 2)
            (func (export "alloc") (param i32) (result i32) i32.const 1024)
            (func (export "free") (param i32 i32))
            (func (export "run_data") (param i32 i32) (result i32) i32.const 0))"#,
    )
    .expect("wat");
    assert!(wrong(&run(&i32_result, &input(0))));
}

#[test]
fn endless_modules_are_stopped() {
    let endless = module(
        "i32.const 2",
        "i32.const 1024",
        "(loop (br 0)) i64.const 0",
        EMPTY_OUTPUT,
        "",
    );
    assert_eq!(run(&endless, &input(0)), Err(SandboxError::OutOfFuel));
    let endless_alloc = module(
        "i32.const 2",
        "(loop (br 0)) i32.const 0",
        &at_2048(10),
        EMPTY_OUTPUT,
        "",
    );
    assert_eq!(run(&endless_alloc, &input(0)), Err(SandboxError::OutOfFuel));
}

#[test]
fn the_digest_is_still_checked() {
    let wasm = good();
    let approval = Approval::of(&wasm, Vec::new(), v2_policy());
    let mut tampered = wasm;
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    let result = Sandbox::new().expect("engine").run_data(
        &approval,
        &tampered,
        &input(0),
        Limits::DEFAULT,
        Arc::new(NoServices),
    );
    assert_eq!(result, Err(SandboxError::DigestMismatch));
}
