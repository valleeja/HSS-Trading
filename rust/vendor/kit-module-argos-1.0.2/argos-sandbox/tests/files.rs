//! The file service: own folder only, traversal, links, quota, protected
//! folders, and an end-to-end write then read through the sandbox.

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use argos_sandbox::files::{FileService, FileSetupError};
use argos_sandbox::{Approval, Capability, ServicePolicy};
use argos_sandbox::{
    CallToken, HostServices, Limits, Sandbox, HOST_FORBIDDEN_PATH, HOST_NOT_FOUND, HOST_QUOTA,
    HOST_TOO_LARGE,
};

fn path(p: &str) -> &str {
    p
}

fn service(root: &Path, quota: u64) -> FileService {
    FileService::new(root, path("hss/notes"), None, quota, &[]).expect("service")
}

fn read(s: &FileService, name: &str) -> Result<Vec<u8>, i32> {
    s.call(
        Capability::ReadFiles,
        name.as_bytes(),
        &CallToken::default(),
    )
}

fn write(s: &FileService, name: &str, content: &[u8]) -> Result<Vec<u8>, i32> {
    let mut request = u16::try_from(name.len())
        .expect("short")
        .to_be_bytes()
        .to_vec();
    request.extend_from_slice(name.as_bytes());
    request.extend_from_slice(content);
    s.call(Capability::WriteFiles, &request, &CallToken::default())
}

#[test]
fn a_module_writes_and_reads_its_own_files() {
    let root = tempfile::tempdir().expect("root");
    let s = service(root.path(), 1_000);
    assert_eq!(write(&s, "notes/today.txt", b"hello"), Ok(Vec::new()));
    assert_eq!(read(&s, "notes/today.txt"), Ok(b"hello".to_vec()));
    assert!(root.path().join("hss/notes/notes/today.txt").is_file());
    assert_eq!(read(&s, "missing.txt"), Err(HOST_NOT_FOUND));
}

#[test]
fn traversal_absolute_and_special_names_are_refused() {
    let root = tempfile::tempdir().expect("root");
    fs::write(root.path().join("secret.txt"), b"keys").expect("w");
    let s = service(root.path(), 1_000);
    write(&s, "a/b.txt", b"x").expect("write");
    for bad in [
        "../secret.txt",
        "../../secret.txt",
        "a/../../../secret.txt",
        "/etc/passwd",
        "",
        "a//b.txt",
        "./a/b.txt",
        "a/./b.txt",
        "a\\b.txt",
        "c:secret",
        "CON",
        "nul.txt",
        "com1",
        "a/b.txt.argos-tmp",
        "bad\u{0}name",
        "bad\nname",
        &"d/".repeat(20),
        &"n".repeat(300),
    ] {
        assert_eq!(read(&s, bad), Err(HOST_FORBIDDEN_PATH), "read {bad:?}");
        assert_eq!(
            write(&s, bad, b"x"),
            Err(HOST_FORBIDDEN_PATH),
            "write {bad:?}"
        );
    }
    assert_eq!(
        s.call(Capability::ReadFiles, b"\xff\xfe", &CallToken::default()),
        Err(HOST_FORBIDDEN_PATH)
    );
}

#[test]
fn symbolic_links_are_never_followed() {
    let root = tempfile::tempdir().expect("root");
    let outside = tempfile::tempdir().expect("outside");
    fs::write(outside.path().join("secret.txt"), b"keys").expect("w");
    let s = service(root.path(), 1_000);
    write(&s, "keep.txt", b"x").expect("create the folder");
    let dir = root.path().join("hss/notes");
    // A link to a folder outside, and a link to a file outside.
    symlink(outside.path(), dir.join("out")).expect("link");
    symlink(outside.path().join("secret.txt"), dir.join("file")).expect("link");
    assert_eq!(read(&s, "out/secret.txt"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(read(&s, "file"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(write(&s, "out/new.txt", b"x"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(write(&s, "file", b"x"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(
        fs::read(outside.path().join("secret.txt")).expect("r"),
        b"keys"
    );
    assert!(!outside.path().join("new.txt").exists());

    // The module folder itself replaced by a link.
    let other = tempfile::tempdir().expect("root");
    fs::create_dir_all(other.path().join("hss")).expect("dir");
    symlink(outside.path(), other.path().join("hss/notes")).expect("link");
    assert!(matches!(
        FileService::new(other.path(), path("hss/notes"), None, 1_000, &[]),
        Err(FileSetupError::SymbolicLink(_))
    ));
}

#[test]
fn protected_folders_cannot_overlap_the_module_folder() {
    let root = tempfile::tempdir().expect("root");
    for protected in [
        root.path().join("hss"),            // contains the module folder
        root.path().join("hss/notes/keys"), // inside the module folder
        root.path().to_path_buf(),          // the whole data root
    ] {
        fs::create_dir_all(&protected).expect("dir");
        assert!(
            matches!(
                FileService::new(
                    root.path(),
                    path("hss/notes"),
                    None,
                    1_000,
                    std::slice::from_ref(&protected)
                ),
                Err(FileSetupError::Protected(_))
            ),
            "{}",
            protected.display()
        );
    }
    // A protected sibling is fine: it is simply out of reach.
    let keys: PathBuf = root.path().join("keys");
    fs::create_dir_all(&keys).expect("dir");
    let s = FileService::new(root.path(), path("trading/x"), None, 1_000, &[keys]).expect("ok");
    assert_eq!(read(&s, "../../keys/root.key"), Err(HOST_FORBIDDEN_PATH));
}

#[test]
fn the_quota_and_sub_folders_are_enforced() {
    let root = tempfile::tempdir().expect("root");
    let s = service(root.path(), 10);
    assert_eq!(write(&s, "a.txt", b"12345678"), Ok(Vec::new()));
    assert_eq!(write(&s, "b.txt", b"123"), Err(HOST_QUOTA));
    // Replacing a file counts only the difference.
    assert_eq!(write(&s, "a.txt", b"1234567890"), Ok(Vec::new()));

    let limited = FileService::new(
        root.path(),
        path("hss/limited"),
        Some(vec!["exports".into()]),
        1_000,
        &[],
    )
    .expect("service");
    assert_eq!(write(&limited, "exports/plan.dxf", b"x"), Ok(Vec::new()));
    assert_eq!(
        write(&limited, "other/plan.dxf", b"x"),
        Err(HOST_FORBIDDEN_PATH)
    );
    assert_eq!(write(&limited, "plan.dxf", b"x"), Err(HOST_FORBIDDEN_PATH));
}

#[test]
fn a_large_file_is_refused_on_read() {
    let root = tempfile::tempdir().expect("root");
    let s = service(root.path(), 1_000_000);
    write(&s, "x", b"").expect("folder");
    fs::write(root.path().join("hss/notes/big"), vec![0_u8; 70_000]).expect("w");
    assert_eq!(read(&s, "big"), Err(HOST_TOO_LARGE));
}

#[test]
fn a_module_writes_then_reads_through_the_sandbox() {
    let root = tempfile::tempdir().expect("root");
    let wat = r#"(module
      (import "argos" "write_file" (func $write (param i32 i32 i32 i32) (result i32)))
      (import "argos" "read_file" (func $read (param i32 i32 i32 i32) (result i32)))
      (memory (export "memory") 1)
      ;; write request: path length 7, "log.txt", content "argos!"
      (data (i32.const 0) "\00\07log.txtargos!")
      (data (i32.const 100) "log.txt")
      (func (export "main") (result i32)
        (if (i32.ne (call $write (i32.const 0) (i32.const 15) (i32.const 200) (i32.const 0)) (i32.const 0))
          (then (return (i32.const -100))))
        ;; read back into offset 300, then check it fits a too-small buffer is refused
        (if (i32.ne (call $read (i32.const 100) (i32.const 7) (i32.const 300) (i32.const 2)) (i32.const -2))
          (then (return (i32.const -200))))
        (call $read (i32.const 100) (i32.const 7) (i32.const 300) (i32.const 64))))"#;
    let wasm = wat::parse_str(wat).expect("WAT");
    let release = Approval::of(
        &wasm,
        vec![Capability::ReadFiles, Capability::WriteFiles],
        ServicePolicy::default(),
    );
    let files = Arc::new(service(root.path(), 1_000));
    let result =
        Sandbox::new()
            .expect("engine")
            .run_with(&release, &wasm, "main", Limits::DEFAULT, files);
    assert_eq!(result, Ok(6));
    assert_eq!(
        fs::read(root.path().join("hss/notes/log.txt")).expect("r"),
        b"argos!"
    );
}

#[test]
fn a_folder_swapped_for_a_link_after_setup_is_refused_at_use() {
    let root = tempfile::tempdir().expect("root");
    let outside = tempfile::tempdir().expect("outside");
    fs::write(outside.path().join("secret.txt"), b"keys").expect("w");
    let s = service(root.path(), 1_000);
    write(&s, "docs/a.txt", b"x").expect("write");
    // After the service was set up and used, its folders are replaced by links.
    let docs = root.path().join("hss/notes/docs");
    fs::remove_dir_all(&docs).expect("rm");
    symlink(outside.path(), &docs).expect("link");
    assert_eq!(read(&s, "docs/secret.txt"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(write(&s, "docs/new.txt", b"x"), Err(HOST_FORBIDDEN_PATH));
    let module = root.path().join("hss/notes");
    fs::remove_dir_all(&module).expect("rm");
    symlink(outside.path(), &module).expect("link");
    assert_eq!(read(&s, "secret.txt"), Err(HOST_FORBIDDEN_PATH));
    assert_eq!(write(&s, "new.txt", b"x"), Err(HOST_FORBIDDEN_PATH));
    assert!(!outside.path().join("new.txt").exists());
    assert_eq!(
        fs::read(outside.path().join("secret.txt")).expect("r"),
        b"keys"
    );
}
