//! Model files checked before loading: format by bytes, digest, size, no
//! right. No key involved (signatures are checked by whoever approved).

use argos_sandbox::models::{verify_model, ModelError, ModelFormat, ModelManifest};
use argos_sandbox::{Approval, Capability, ServicePolicy};

/// A toy safetensors file: `w` (2x2 f32) and `b` (2 f32).
fn safetensors(header_padding: usize) -> Vec<u8> {
    let mut header = String::from(
        r#"{"w":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]},"b":{"dtype":"F32","shape":[2],"data_offsets":[16,24]}}"#,
    );
    header.push_str(&" ".repeat(header_padding));
    let mut out = u64::try_from(header.len())
        .expect("small")
        .to_le_bytes()
        .to_vec();
    out.extend_from_slice(header.as_bytes());
    for v in [1.0_f32, 0.0, 0.0, 1.0, 0.5, -0.5] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

/// A minimal ONNX-shaped protobuf: `ir_version` 8, a graph with a name.
fn onnx() -> Vec<u8> {
    let graph = [0x0a, 0x04, b'd', b'e', b'm', b'o']; // GraphProto.name = "demo"
    let mut out = vec![0x08, 0x08]; // field 1 (ir_version) = 8
    out.extend_from_slice(&[0x12, 0x05]); // field 2 (producer_name) = "argos"
    out.extend_from_slice(b"argos");
    out.push(0x3a); // field 7 (graph), length-delimited
    out.push(u8::try_from(graph.len()).expect("small"));
    out.extend_from_slice(&graph);
    out
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// Manifest and approval of exactly these bytes, no right.
fn approved(bytes: &[u8], format: ModelFormat) -> (ModelManifest, Approval) {
    let approval = Approval::of(bytes, vec![], ServicePolicy::default());
    let manifest = ModelManifest {
        name: "argos-modele-demo".into(),
        format,
        sha384: hex(&approval.wasm_sha384),
        max_bytes: 1024 * 1024,
    };
    (manifest, approval)
}

#[test]
fn valid_safetensors_and_onnx_files_pass() {
    for (bytes, format) in [
        (safetensors(0), ModelFormat::Safetensors),
        // A 640-byte header starts with the bytes of a pickle (80 02): still valid.
        (safetensors(640 - 112), ModelFormat::Safetensors),
        (onnx(), ModelFormat::Onnx),
    ] {
        let (manifest, approval) = approved(&bytes, format);
        assert_eq!(
            verify_model(&bytes, &manifest, &approval),
            Ok(()),
            "{format:?}"
        );
    }
    assert_eq!(&safetensors(640 - 112)[..2], &[0x80, 0x02]);
}

#[test]
fn one_changed_byte_is_refused() {
    let bytes = safetensors(0);
    let (manifest, approval) = approved(&bytes, ModelFormat::Safetensors);
    for i in [0, 9, bytes.len() - 1] {
        let mut changed = bytes.clone();
        changed[i] ^= 1;
        assert_eq!(
            verify_model(&changed, &manifest, &approval),
            Err(ModelError::DigestMismatch),
            "byte {i}"
        );
    }
}

#[test]
fn pickles_and_pytorch_files_are_refused_even_renamed() {
    let pickle_v4 =
        b"\x80\x04\x95\x10\x00\x00\x00\x00\x00\x00\x00\x8c\x02os\x94\x8c\x06system\x94\x93\x94."
            .to_vec();
    let pickle_v0 = b"cos\nsystem\n(S'echo pwned'\ntR.".to_vec();
    let pytorch_zip = b"PK\x03\x04\x14\x00\x00\x00\x08\x00archive/data.pkl\x80\x02}q\x00.".to_vec();
    for bytes in [pickle_v4, pickle_v0, pytorch_zip] {
        for format in [ModelFormat::Safetensors, ModelFormat::Onnx] {
            // Approved by mistake and declared as a safe format: still refused.
            let (manifest, approval) = approved(&bytes, format);
            assert_eq!(
                verify_model(&bytes, &manifest, &approval),
                Err(ModelError::Pickle),
                "{format:?}"
            );
        }
    }
}

#[test]
fn too_large_files_are_refused() {
    let bytes = safetensors(0);
    let (mut manifest, approval) = approved(&bytes, ModelFormat::Safetensors);
    manifest.max_bytes = 100;
    assert!(matches!(
        verify_model(&bytes, &manifest, &approval),
        Err(ModelError::TooLarge(_))
    ));
}

#[test]
fn a_model_asking_for_any_right_is_refused() {
    let bytes = onnx();
    let (manifest, mut approval) = approved(&bytes, ModelFormat::Onnx);
    approval.capabilities = vec![Capability::Network];
    assert_eq!(
        verify_model(&bytes, &manifest, &approval),
        Err(ModelError::RightRequested)
    );
    let (manifest, mut approval) = approved(&bytes, ModelFormat::Onnx);
    approval.policy.domains = vec!["models.example.org".into()];
    assert_eq!(
        verify_model(&bytes, &manifest, &approval),
        Err(ModelError::RightRequested)
    );
}

#[test]
fn malformed_files_and_names_are_refused() {
    // Tensor data offsets past the end, or a header that is not JSON.
    let mut bad = safetensors(0);
    bad.truncate(bad.len() - 4);
    let (manifest, approval) = approved(&bad, ModelFormat::Safetensors);
    assert_eq!(
        verify_model(&bad, &manifest, &approval),
        Err(ModelError::BadFormat(ModelFormat::Safetensors))
    );
    // ONNX bytes declared as safetensors, and the reverse.
    let (manifest, approval) = approved(&onnx(), ModelFormat::Safetensors);
    assert!(verify_model(&onnx(), &manifest, &approval).is_err());
    let st = safetensors(0);
    let (manifest, approval) = approved(&st, ModelFormat::Onnx);
    assert!(verify_model(&st, &manifest, &approval).is_err());
    // A truncated ONNX file.
    let mut cut = onnx();
    cut.pop();
    let (manifest, approval) = approved(&cut, ModelFormat::Onnx);
    assert_eq!(
        verify_model(&cut, &manifest, &approval),
        Err(ModelError::BadFormat(ModelFormat::Onnx))
    );
    // A name that is not argos-modele-<name>.
    let (mut manifest, approval) = approved(&onnx(), ModelFormat::Onnx);
    manifest.name = "modele-demo".into();
    assert_eq!(
        verify_model(&onnx(), &manifest, &approval),
        Err(ModelError::BadName)
    );
}
