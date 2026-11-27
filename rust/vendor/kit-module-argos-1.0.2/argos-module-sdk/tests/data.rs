//! Interface v2: encoding round trips, refused inputs and outputs, fuzzing.

use argos_module_sdk::__abi::{run_bytes, BAD_INPUT};
use argos_module_sdk::data::{DecodeError, MAX_BARS, MAX_BYTES};
use argos_module_sdk::{Bar, Card, Input, Label, Line, Output, EMPTY};
use proptest::prelude::*;

fn bars(n: usize) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let i = i64::try_from(i).expect("small");
            Bar {
                t: 1_700_000_000 + 60 * i,
                o: 10_000 + i,
                h: 10_050 + i,
                l: 9_950 + i,
                c: 10_010 + i,
                v: 5,
            }
        })
        .collect()
}

fn input(n: usize) -> Input {
    Input {
        timeframe: 60,
        price_scale: 2,
        volume_scale: 0,
        symbol: "XAUUSD".into(),
        bars: bars(n),
        settings: vec![20, 1],
    }
}

#[test]
fn inputs_and_outputs_round_trip() {
    let i = input(5);
    assert_eq!(Input::decode(&i.encode()), Ok(i));
    let o = Output {
        lines: vec![Line {
            name: "Moyenne 20".into(),
            scale: 2,
            values: vec![EMPTY, EMPTY, 100, 101, 102],
        }],
        labels: vec![Label {
            bar: 4,
            text: "haut".into(),
        }],
        cards: vec![Card {
            name: "Dernière".into(),
            value: 102,
            scale: 2,
        }],
    };
    assert_eq!(Output::decode(&o.encode(), 5), Ok(o));
    assert_eq!(Input::decode(&input(0).encode()), Ok(input(0)));
}

#[test]
fn malformed_inputs_are_refused() {
    let good = input(3).encode();
    // Version, truncation, trailing bytes.
    let mut v1 = good.clone();
    v1[0] = 1;
    assert_eq!(Input::decode(&v1), Err(DecodeError::Version));
    assert_eq!(
        Input::decode(&good[..good.len() - 1]),
        Err(DecodeError::Truncated)
    );
    let mut more = good.clone();
    more.push(0);
    assert_eq!(Input::decode(&more), Err(DecodeError::Trailing));
    // Scale, empty symbol, zero timeframe, non-increasing times, too many settings.
    let mut bad = input(3);
    bad.price_scale = 13;
    assert_eq!(Input::decode(&bad.encode()), Err(DecodeError::Invalid));
    let mut bad = input(3);
    bad.symbol.clear();
    assert_eq!(Input::decode(&bad.encode()), Err(DecodeError::Invalid));
    let mut bad = input(3);
    bad.timeframe = 0;
    assert_eq!(Input::decode(&bad.encode()), Err(DecodeError::Invalid));
    let mut bad = input(3);
    bad.bars[2].t = bad.bars[1].t;
    assert_eq!(Input::decode(&bad.encode()), Err(DecodeError::Invalid));
    let mut bad = input(3);
    bad.settings = vec![0; 17];
    assert_eq!(Input::decode(&bad.encode()), Err(DecodeError::Invalid));
    // Invalid UTF-8 in the symbol.
    let mut utf = good.clone();
    utf[12] = 0xff;
    assert_eq!(Input::decode(&utf), Err(DecodeError::Invalid));
    // A bar count larger than the bytes (no huge allocation), and over the limit.
    let mut lie = good.clone();
    lie[18..22].copy_from_slice(&40_000_u32.to_le_bytes());
    assert_eq!(Input::decode(&lie), Err(DecodeError::Truncated));
    let mut over = good;
    over[18..22].copy_from_slice(&u32::try_from(MAX_BARS + 1).expect("small").to_le_bytes());
    assert_eq!(Input::decode(&over), Err(DecodeError::Invalid));
    assert_eq!(
        Input::decode(&vec![0; MAX_BYTES + 1]),
        Err(DecodeError::TooLarge)
    );
}

#[test]
fn malformed_outputs_are_refused() {
    let line = |values: Vec<i64>| Output {
        lines: vec![Line {
            name: "x".into(),
            scale: 0,
            values,
        }],
        ..Output::default()
    };
    // One value per bar, no more, no less.
    assert_eq!(
        Output::decode(&line(vec![1, 2]).encode(), 3),
        Err(DecodeError::Invalid)
    );
    // A label on a bar that does not exist.
    let label = Output {
        labels: vec![Label {
            bar: 3,
            text: "x".into(),
        }],
        ..Output::default()
    };
    assert_eq!(
        Output::decode(&label.encode(), 3),
        Err(DecodeError::Invalid)
    );
    // Empty name, too many lines.
    let unnamed = Output {
        lines: vec![Line {
            name: String::new(),
            scale: 0,
            values: vec![],
        }],
        ..Output::default()
    };
    assert_eq!(
        Output::decode(&unnamed.encode(), 0),
        Err(DecodeError::Invalid)
    );
    let many = Output {
        lines: (0..17)
            .map(|_| Line {
                name: "x".into(),
                scale: 0,
                values: vec![],
            })
            .collect(),
        ..Output::default()
    };
    assert_eq!(Output::decode(&many.encode(), 0), Err(DecodeError::Invalid));
}

fn close_line(i: Input) -> Output {
    Output {
        lines: vec![Line {
            name: "close".into(),
            scale: i.price_scale,
            values: i.bars.into_iter().map(|b| b.c).collect(),
        }],
        ..Output::default()
    }
}

#[test]
fn the_entry_runs_on_bytes_and_refuses_bad_input() {
    let out = run_bytes(&input(4).encode(), close_line).expect("run");
    assert_eq!(
        Output::decode(&out, 4).expect("valid").lines[0].values,
        vec![10_010, 10_011, 10_012, 10_013]
    );
    assert_eq!(run_bytes(b"not an input", close_line), Err(BAD_INPUT));
}

proptest! {
    #[test]
    fn random_bytes_never_panic(bytes in prop::collection::vec(any::<u8>(), 0..2000), bars in 0usize..50) {
        let _ = Input::decode(&bytes);
        let _ = Output::decode(&bytes, bars);
    }

    #[test]
    fn mutated_inputs_never_panic(index in any::<prop::sample::Index>(), byte in any::<u8>()) {
        let mut bytes = input(10).encode();
        let i = index.index(bytes.len());
        bytes[i] = byte;
        let _ = Input::decode(&bytes);
    }
}
