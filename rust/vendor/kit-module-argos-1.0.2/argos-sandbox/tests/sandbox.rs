//! Sandbox tests with tiny WASM modules written in WAT. No key involved.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use argos_sandbox::{
    Approval, CallToken, Capability, HostServices, Limits, Sandbox, SandboxError, ServicePolicy,
    HOST_BAD_REQUEST, HOST_OK, HOST_TOO_LARGE, MAX_REQUEST_BYTES,
};

/// Compiles WAT and approves exactly those bytes.
fn approved(wat: &str, capabilities: Vec<Capability>) -> (Approval, Vec<u8>) {
    let wasm = wat::parse_str(wat).expect("valid WAT");
    (
        Approval::of(&wasm, capabilities, ServicePolicy::default()),
        wasm,
    )
}

fn sandbox() -> Sandbox {
    Sandbox::new().expect("engine")
}

const ANSWER: &str = r#"(module (func (export "main") (result i32) i32.const 42))"#;

const READ_FILE: &str = r#"(module
  (import "argos" "read_file" (func $read (param i32 i32 i32 i32) (result i32)))
  (memory (export "memory") 1)
  (func (export "main") (result i32)
    (call $read (i32.const 0) (i32.const 4) (i32.const 1024) (i32.const 64))))"#;

#[test]
fn an_approved_module_runs() {
    let (release, wasm) = approved(ANSWER, vec![]);
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Ok(42)
    );
}

#[test]
fn only_the_approved_wasm_is_loaded() {
    let (release, _) = approved(ANSWER, vec![]);
    let other =
        wat::parse_str(r#"(module (func (export "main") (result i32) i32.const 7))"#).expect("WAT");
    assert_eq!(
        sandbox().run(&release, &other, "main", Limits::DEFAULT),
        Err(SandboxError::DigestMismatch)
    );
}

#[test]
fn a_file_read_without_the_capability_is_refused() {
    let (release, wasm) = approved(READ_FILE, vec![]);
    assert!(matches!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::Refused(_))
    ));
    // With the capability in the approved manifest, the import resolves.
    let (release, wasm) = approved(READ_FILE, vec![Capability::ReadFiles]);
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Ok(0)
    );
}

#[test]
fn wasi_and_unknown_host_imports_are_refused() {
    let wasi_import = r#"(module
      (import "wasi_snapshot_preview1" "fd_write"
        (func (param i32 i32 i32 i32) (result i32)))
      (func (export "main") (result i32) i32.const 0))"#;
    let key_import = r#"(module
      (import "argos" "read_root_key" (func (result i32)))
      (func (export "main") (result i32) i32.const 0))"#;
    for wat in [wasi_import, key_import] {
        let (release, wasm) = approved(
            wat,
            vec![
                Capability::ReadFiles,
                Capability::WriteFiles,
                Capability::Network,
                Capability::Ai,
            ],
        );
        assert!(
            matches!(
                sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
                Err(SandboxError::Refused(_))
            ),
            "{wat}"
        );
    }
}

#[test]
fn an_endless_loop_runs_out_of_fuel() {
    let (release, wasm) = approved(
        r#"(module (func (export "main") (result i32) (loop $l (br $l)) i32.const 0))"#,
        vec![],
    );
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::OutOfFuel)
    );
}

#[test]
fn an_endless_loop_is_stopped_by_the_time_limit() {
    let (release, wasm) = approved(
        r#"(module (func (export "main") (result i32) (loop $l (br $l)) i32.const 0))"#,
        vec![],
    );
    let limits = Limits {
        fuel: u64::MAX,
        timeout: Duration::from_millis(200),
        ..Limits::DEFAULT
    };
    let start = Instant::now();
    assert_eq!(
        sandbox().run(&release, &wasm, "main", limits),
        Err(SandboxError::Timeout)
    );
    assert!(
        start.elapsed() < Duration::from_secs(10),
        "stopped promptly"
    );
}

#[test]
fn asking_for_too_much_memory_is_refused() {
    // Grows memory by 1024 pages (64 MiB) at run time.
    let (release, wasm) = approved(
        r#"(module (memory 1)
          (func (export "main") (result i32) (memory.grow (i32.const 1024))))"#,
        vec![],
    );
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::MemoryLimit)
    );
    // Declares 64 MiB up front.
    let (release, wasm) = approved(
        r#"(module (memory 1024) (func (export "main") (result i32) i32.const 0))"#,
        vec![],
    );
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::MemoryLimit)
    );
}

#[test]
fn a_missing_entry_or_a_trap_is_reported() {
    let (release, wasm) = approved(ANSWER, vec![]);
    assert_eq!(
        sandbox().run(&release, &wasm, "other", Limits::DEFAULT),
        Err(SandboxError::MissingEntry("other".into()))
    );
    let (release, wasm) = approved(
        r#"(module (func (export "main") (result i32) unreachable))"#,
        vec![],
    );
    assert!(matches!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::Trapped(_))
    ));
}

/// A module that calls `read_file(ptr, len)` once with constants.
fn read_call(ptr: i64, len: i64, memory: &str) -> String {
    format!(
        r#"(module
          (import "argos" "read_file" (func $read (param i32 i32 i32 i32) (result i32)))
          {memory}
          (func (export "main") (result i32)
            (call $read (i32.const {ptr}) (i32.const {len}) (i32.const 0) (i32.const 64))))"#
    )
}

fn run_read(ptr: i64, len: i64, memory: &str) -> Result<i32, SandboxError> {
    let (release, wasm) = approved(&read_call(ptr, len, memory), vec![Capability::ReadFiles]);
    sandbox().run(&release, &wasm, "main", Limits::DEFAULT)
}

#[test]
fn host_calls_check_pointer_length_and_size() {
    let memory = r#"(memory (export "memory") 1)"#; // 64 KiB
    assert_eq!(run_read(0, 4, memory), Ok(HOST_OK));
    assert_eq!(run_read(65_532, 4, memory), Ok(HOST_OK), "last bytes");
    // Outside the module's memory.
    assert_eq!(run_read(65_533, 4, memory), Ok(HOST_BAD_REQUEST));
    assert_eq!(run_read(1 << 30, 4, memory), Ok(HOST_BAD_REQUEST));
    // Negative pointer or length (i32 values seen as negative).
    assert_eq!(run_read(-1, 4, memory), Ok(HOST_BAD_REQUEST));
    assert_eq!(run_read(0, -5, memory), Ok(HOST_BAD_REQUEST));
    // Pointer plus length past the end of the address space.
    assert_eq!(
        run_read(i64::from(i32::MAX), 16, memory),
        Ok(HOST_BAD_REQUEST)
    );
    // Too large, checked before any read.
    let too_big = i64::try_from(MAX_REQUEST_BYTES + 1).expect("fits");
    assert_eq!(run_read(0, too_big, memory), Ok(HOST_TOO_LARGE));
    assert_eq!(run_read(0, i64::from(i32::MAX), memory), Ok(HOST_TOO_LARGE));
    // No exported memory: nothing can be read.
    assert_eq!(run_read(0, 4, "(memory 1)"), Ok(HOST_BAD_REQUEST));
}

#[test]
fn a_host_call_without_the_permission_cannot_even_load() {
    // The function of an ungranted capability is never linked, so the module
    // is refused before running (and the function itself checks again).
    let (release, wasm) = approved(&read_call(0, 4, r#"(memory (export "memory") 1)"#), vec![]);
    assert!(matches!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::Refused(_))
    ));
}

#[test]
fn tables_instances_and_memories_are_limited() {
    // A huge table.grow traps with a clear error.
    let (release, wasm) = approved(
        r#"(module (table 1 funcref)
          (func (export "main") (result i32)
            (table.grow (ref.null func) (i32.const 1000000))))"#,
        vec![],
    );
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::TableLimit)
    );
    // A huge table declared up front.
    let (release, wasm) = approved(
        r#"(module (table 1000000 funcref) (func (export "main") (result i32) i32.const 0))"#,
        vec![],
    );
    assert_eq!(
        sandbox().run(&release, &wasm, "main", Limits::DEFAULT),
        Err(SandboxError::TableLimit)
    );
    // Two memories or two tables: refused.
    for wat in [
        r#"(module (memory 1) (memory 1) (func (export "main") (result i32) i32.const 0))"#,
        r#"(module (table 1 funcref) (table 1 funcref) (func (export "main") (result i32) i32.const 0))"#,
    ] {
        let (release, wasm) = approved(wat, vec![]);
        assert!(
            sandbox()
                .run(&release, &wasm, "main", Limits::DEFAULT)
                .is_err(),
            "{wat}"
        );
    }
}

/// A host service that sleeps, then answers 7.
struct Slow(Duration);

impl HostServices for Slow {
    fn call(
        &self,
        _capability: Capability,
        _request: &[u8],
        _token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        std::thread::sleep(self.0);
        Ok(vec![0; 7])
    }
}

/// Calls `read_file(0, 2)` `n` times and returns the last status.
fn calls(n: usize) -> String {
    let body = "(drop (call $read (i32.const 0) (i32.const 2) (i32.const 1024) (i32.const 64)))\n"
        .repeat(n.saturating_sub(1));
    format!(
        r#"(module
          (import "argos" "read_file" (func $read (param i32 i32 i32 i32) (result i32)))
          (memory (export "memory") 1)
          (data (i32.const 0) "hi")
          (func (export "main") (result i32)
            {body}
            (call $read (i32.const 0) (i32.const 2) (i32.const 1024) (i32.const 64))))"#
    )
}

#[test]
fn a_blocking_host_call_is_cut_by_its_own_time_limit() {
    let (release, wasm) = approved(&calls(1), vec![Capability::ReadFiles]);
    let start = Instant::now();
    let result = sandbox().run_with(
        &release,
        &wasm,
        "main",
        Limits::DEFAULT,
        Arc::new(Slow(Duration::from_secs(30))),
    );
    assert_eq!(result, Err(SandboxError::Timeout));
    // One host call may take at most 1 s of the 2 s budget.
    assert!(
        start.elapsed() < Duration::from_millis(1500),
        "{:?}",
        start.elapsed()
    );
}

#[test]
fn host_time_counts_in_the_total_budget() {
    // Each call takes 600 ms and is allowed (1 s limit), but four calls
    // exceed the 2 s total: the fourth is cut when the budget is spent.
    let (release, wasm) = approved(&calls(4), vec![Capability::ReadFiles]);
    let start = Instant::now();
    let result = sandbox().run_with(
        &release,
        &wasm,
        "main",
        Limits::DEFAULT,
        Arc::new(Slow(Duration::from_millis(600))),
    );
    assert_eq!(result, Err(SandboxError::Timeout));
    let elapsed = start.elapsed();
    assert!(elapsed >= Duration::from_millis(1900), "{elapsed:?}");
    assert!(elapsed < Duration::from_millis(2600), "{elapsed:?}");
    // Two such calls fit in the budget.
    let (release, wasm) = approved(&calls(2), vec![Capability::ReadFiles]);
    assert_eq!(
        sandbox().run_with(
            &release,
            &wasm,
            "main",
            Limits::DEFAULT,
            Arc::new(Slow(Duration::from_millis(600)))
        ),
        Ok(7)
    );
}

/// Records each request and counts calls.
#[derive(Default)]
struct Recorder {
    calls: AtomicUsize,
    last: Mutex<Vec<u8>>,
}

impl HostServices for Recorder {
    fn call(
        &self,
        _capability: Capability,
        request: &[u8],
        _token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        request.clone_into(&mut self.last.lock().expect("lock"));
        Ok(vec![0; 5])
    }
}

#[test]
fn the_service_receives_exactly_the_request_bytes() {
    let recorder = Arc::new(Recorder::default());
    let (release, wasm) = approved(&calls(1), vec![Capability::ReadFiles]);
    assert_eq!(
        sandbox().run_with(
            &release,
            &wasm,
            "main",
            Limits::DEFAULT,
            Arc::<Recorder>::clone(&recorder)
        ),
        Ok(5)
    );
    assert_eq!(*recorder.last.lock().expect("lock"), b"hi".to_vec());
}

#[test]
fn unapproved_imports_are_refused_at_load_before_anything_runs() {
    // Each module calls the granted read_file from its start function, so a
    // single call reaching the service would show it ran.
    for (module, name) in [
        ("argos", "write_file"),        // a real right, not granted
        ("argos", "delete_everything"), // an unknown function
        ("env", "read_file"),           // another import module
    ] {
        let wat = format!(
            r#"(module
              (import "argos" "read_file" (func $read (param i32 i32 i32 i32) (result i32)))
              (import "{module}" "{name}" (func (param i32 i32 i32 i32) (result i32)))
              (memory (export "memory") 1)
              (func $start (drop (call $read (i32.const 0) (i32.const 1) (i32.const 8) (i32.const 8))))
              (start $start)
              (func (export "main") (result i32) i32.const 0))"#
        );
        let recorder = Arc::new(Recorder::default());
        let (release, wasm) = approved(&wat, vec![Capability::ReadFiles]);
        let result = sandbox().run_with(
            &release,
            &wasm,
            "main",
            Limits::DEFAULT,
            Arc::<Recorder>::clone(&recorder),
        );
        assert!(
            matches!(result, Err(SandboxError::Refused(_))),
            "{module}::{name}: {result:?}"
        );
        assert_eq!(
            recorder.calls.load(Ordering::SeqCst),
            0,
            "{module}::{name} ran"
        );
    }
}

/// Sleeps, then tries to write a file through the token.
struct SlowWriter {
    delay: Duration,
    path: std::path::PathBuf,
}

impl HostServices for SlowWriter {
    fn call(
        &self,
        _capability: Capability,
        request: &[u8],
        token: &CallToken,
    ) -> Result<Vec<u8>, i32> {
        std::thread::sleep(self.delay);
        match token.write_atomically(&self.path, request) {
            Ok(true) => Ok(vec![1]),
            Ok(false) => Ok(Vec::new()),
            Err(_) => Err(-9),
        }
    }
}

#[test]
fn no_effect_happens_after_a_host_call_times_out() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("out.txt");
    let (release, wasm) = approved(&calls(1), vec![Capability::ReadFiles]);
    let writer = Arc::new(SlowWriter {
        delay: Duration::from_millis(1500),
        path: path.clone(),
    });
    assert_eq!(
        sandbox().run_with(&release, &wasm, "main", Limits::DEFAULT, writer),
        Err(SandboxError::Timeout)
    );
    // Let the abandoned service thread wake up and try its write.
    std::thread::sleep(Duration::from_millis(1000));
    assert!(!path.exists(), "write happened after the timeout");
    let leftovers: Vec<_> = std::fs::read_dir(dir.path()).expect("dir").collect();
    assert!(leftovers.is_empty(), "temporary file left: {leftovers:?}");

    // In time, the same write happens.
    let writer = Arc::new(SlowWriter {
        delay: Duration::from_millis(10),
        path: path.clone(),
    });
    assert_eq!(
        sandbox().run_with(&release, &wasm, "main", Limits::DEFAULT, writer),
        Ok(1)
    );
    assert_eq!(std::fs::read(&path).expect("written"), b"hi".to_vec());
}
