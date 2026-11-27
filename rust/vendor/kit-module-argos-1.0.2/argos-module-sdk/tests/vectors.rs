//! Conformance vectors of interface v2 (`tests/vectors/abi-v2.json`), written
//! from `docs/abi-v2.md` by an independent encoder. Any second host (the
//! browser Web Worker of Argos HSS Trading) must give the same answers.

use argos_module_sdk::{DecodeError, Input, Output};
use serde_json::Value;

fn vectors() -> Value {
    serde_json::from_str(include_str!("vectors/abi-v2.json")).expect("vectors")
}

fn bytes(case: &Value) -> Vec<u8> {
    hex::decode(case["hex"].as_str().expect("hex")).expect("hex")
}

fn error(name: &str) -> DecodeError {
    match name {
        "Truncated" => DecodeError::Truncated,
        "Trailing" => DecodeError::Trailing,
        "Version" => DecodeError::Version,
        "TooLarge" => DecodeError::TooLarge,
        _ => DecodeError::Invalid,
    }
}

fn cases<'a>(v: &'a Value, key: &str) -> &'a Vec<Value> {
    v[key].as_array().expect("cases")
}

fn bars(case: &Value) -> usize {
    usize::try_from(case["bars"].as_u64().expect("bars")).expect("small")
}

#[test]
fn valid_vectors_decode_and_encode_back_to_the_same_bytes() {
    let v = vectors();
    for case in cases(&v, "valid_inputs") {
        let input =
            Input::decode(&bytes(case)).unwrap_or_else(|e| panic!("{}: {e:?}", case["name"]));
        assert_eq!(input.encode(), bytes(case), "{}", case["name"]);
    }
    for case in cases(&v, "valid_outputs") {
        let output = Output::decode(&bytes(case), bars(case)).expect("valid output");
        assert_eq!(output.encode(), bytes(case), "{}", case["name"]);
    }
}

#[test]
fn invalid_vectors_are_refused_with_the_expected_error() {
    let v = vectors();
    for case in cases(&v, "invalid_inputs") {
        let expected = error(case["error"].as_str().expect("error"));
        assert_eq!(
            Input::decode(&bytes(case)),
            Err(expected),
            "{}",
            case["name"]
        );
    }
    for case in cases(&v, "invalid_outputs") {
        let expected = error(case["error"].as_str().expect("error"));
        assert_eq!(
            Output::decode(&bytes(case), bars(case)),
            Err(expected),
            "{}",
            case["name"]
        );
    }
}
