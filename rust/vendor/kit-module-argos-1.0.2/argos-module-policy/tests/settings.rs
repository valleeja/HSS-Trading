//! Interface version and settings in the signed policy.

use argos_module_policy::{
    fixed, parse_color, Language, PolicyError, ServicePolicy, SettingKind, SettingSpec,
};

fn spec(json: &str) -> SettingSpec {
    serde_json::from_str(json).expect("setting")
}

fn length() -> SettingSpec {
    spec(
        r#"{ "id": "length", "name": "Longueur", "type": "number", "default": 20, "min": 1, "max": 500 }"#,
    )
}

fn v2(settings: Vec<SettingSpec>) -> ServicePolicy {
    ServicePolicy {
        abi: 2,
        settings,
        ..ServicePolicy::default()
    }
}

#[test]
fn the_four_kinds_parse_from_the_manifest() {
    let all = [
        length(),
        spec(r#"{ "id": "show", "name": "Afficher", "type": "bool", "default": true }"#),
        spec(r##"{ "id": "color", "name": "Couleur", "type": "color", "default": "#2962ff" }"##),
        spec(
            r#"{ "id": "mode", "name": "Mode", "type": "choice", "choices": ["simple", "exp"], "default": "exp" }"#,
        ),
    ];
    assert_eq!(
        all[0].kind,
        SettingKind::Number {
            default: 20,
            min: 1,
            max: 500,
            scale: 0
        }
    );
    let policy = v2(all.to_vec());
    assert_eq!(policy.check(&[]), Ok(()));
    assert_eq!(policy.default_settings(), vec![20, 1, 0x0029_62ff, 1]);
    assert_eq!(policy.check_values(&[500, 0, 0x00ff_ffff, 0]), Ok(()));
    // Unknown fields and unknown types are refused.
    assert!(serde_json::from_str::<SettingSpec>(
        r#"{ "id": "x", "name": "x", "type": "bool", "default": true, "extra": 1 }"#
    )
    .is_err());
    assert!(serde_json::from_str::<SettingSpec>(
        r#"{ "id": "x", "name": "x", "type": "float", "default": 1.5 }"#
    )
    .is_err());
    assert!(serde_json::from_str::<SettingSpec>(
        r#"{ "id": "x", "name": "x", "type": "number", "default": 1.5, "min": 0, "max": 2 }"#
    )
    .is_err());
}

#[test]
fn chosen_values_are_checked_before_a_run() {
    let policy = v2(vec![
        length(),
        spec(r#"{ "id": "show", "name": "Afficher", "type": "bool", "default": true }"#),
        spec(
            r#"{ "id": "mode", "name": "Mode", "type": "choice", "choices": ["a", "b"], "default": "a" }"#,
        ),
    ]);
    let refused = |values: &[i64]| policy.check_values(values).unwrap_err();
    assert_eq!(
        refused(&[0, 1, 0]),
        PolicyError::SettingValue("length".into())
    );
    assert_eq!(
        refused(&[501, 1, 0]),
        PolicyError::SettingValue("length".into())
    );
    assert_eq!(
        refused(&[20, 2, 0]),
        PolicyError::SettingValue("show".into())
    );
    assert_eq!(
        refused(&[20, 1, 2]),
        PolicyError::SettingValue("mode".into())
    );
    assert_eq!(
        refused(&[20, 1, -1]),
        PolicyError::SettingValue("mode".into())
    );
    assert_eq!(refused(&[20, 1]), PolicyError::SettingValue(String::new()));
    assert_eq!(
        refused(&[20, 1, 0, 0]),
        PolicyError::SettingValue(String::new())
    );
    let color = v2(vec![spec(
        r##"{ "id": "c", "name": "C", "type": "color", "default": "#000000" }"##,
    )]);
    assert!(color.check_values(&[0x0100_0000]).is_err());
    assert!(color.check_values(&[-1]).is_err());
}

#[test]
fn bad_declarations_are_refused() {
    let bad = |json: &str| v2(vec![spec(json)]).check(&[]);
    let invalid = |id: &str| Err(PolicyError::InvalidSetting(id.into()));
    assert_eq!(
        bad(r#"{ "id": "n", "name": "N", "type": "number", "default": 0, "min": 1, "max": 5 }"#),
        invalid("n")
    );
    assert_eq!(
        bad(r#"{ "id": "n", "name": "N", "type": "number", "default": 3, "min": 5, "max": 1 }"#),
        invalid("n")
    );
    assert_eq!(
        bad(
            r#"{ "id": "n", "name": "N", "type": "number", "default": 1, "min": 1, "max": 5, "scale": 7 }"#
        ),
        invalid("n")
    );
    assert_eq!(
        bad(r##"{ "id": "c", "name": "C", "type": "color", "default": "#12345" }"##),
        invalid("c")
    );
    assert_eq!(
        bad(r#"{ "id": "c", "name": "C", "type": "color", "default": "red" }"#),
        invalid("c")
    );
    assert_eq!(
        bad(r#"{ "id": "m", "name": "M", "type": "choice", "choices": [], "default": "a" }"#),
        invalid("m")
    );
    assert_eq!(
        bad(
            r#"{ "id": "m", "name": "M", "type": "choice", "choices": ["a", "a"], "default": "a" }"#
        ),
        invalid("m")
    );
    assert_eq!(
        bad(r#"{ "id": "m", "name": "M", "type": "choice", "choices": ["a"], "default": "b" }"#),
        invalid("m")
    );
    assert_eq!(
        bad(r#"{ "id": "bad id", "name": "N", "type": "bool", "default": true }"#),
        invalid("bad id")
    );
    assert_eq!(
        bad(r#"{ "id": "", "name": "N", "type": "bool", "default": true }"#),
        invalid("")
    );
    assert_eq!(
        bad(r#"{ "id": "n", "name": "", "type": "bool", "default": true }"#),
        invalid("n")
    );
    // Repeated identifier, too many, settings on v1, unknown version.
    assert_eq!(v2(vec![length(), length()]).check(&[]), invalid("length"));
    let many = (0..17)
        .map(|i| {
            spec(&format!(
                r#"{{ "id": "s{i}", "name": "S", "type": "bool", "default": true }}"#
            ))
        })
        .collect();
    assert_eq!(v2(many).check(&[]), Err(PolicyError::TooManySettings));
    let v1 = ServicePolicy {
        settings: vec![length()],
        ..ServicePolicy::default()
    };
    assert_eq!(v1.check(&[]), Err(PolicyError::SettingsNeedAbi2));
    assert_eq!(
        ServicePolicy {
            abi: 3,
            ..ServicePolicy::default()
        }
        .check(&[]),
        Err(PolicyError::Abi(3))
    );
    assert_eq!(
        ServicePolicy {
            abi: 0,
            ..ServicePolicy::default()
        }
        .check(&[]),
        Err(PolicyError::Abi(0))
    );
}

#[test]
fn the_signed_encoding_covers_version_and_settings() {
    let encode = |p: &ServicePolicy| {
        let mut out = Vec::new();
        p.encode(&mut out);
        out
    };
    let base = v2(vec![length()]);
    let mut other_max = base.clone();
    other_max.settings[0].kind = SettingKind::Number {
        default: 20,
        min: 1,
        max: 501,
        scale: 0,
    };
    let mut renamed = base.clone();
    renamed.settings[0].name = "Length".into();
    for changed in [ServicePolicy::default(), v2(vec![]), other_max, renamed] {
        assert_ne!(encode(&base), encode(&changed));
    }
    assert_ne!(encode(&ServicePolicy::default()), encode(&v2(vec![])));
}

#[test]
fn values_and_colours_display_plainly() {
    assert_eq!(fixed(150, 2), "1.50");
    assert_eq!(fixed(-5, 2), "-0.05");
    assert_eq!(fixed(20, 0), "20");
    assert_eq!(fixed(i64::MIN, 0), "-9223372036854775808");
    assert_eq!(parse_color("#FFfF00"), Some(0x00ff_ff00));
    assert_eq!(parse_color("ffff00"), None);
    assert_eq!(parse_color("#+fff00"), None);
    assert_eq!(
        length().describe(Language::French),
        "Longueur : nombre de 1 à 500, 20 par défaut"
    );
    assert_eq!(
        length().describe(Language::English),
        "Longueur: number from 1 to 500, default 20"
    );
}

#[test]
fn fields_must_match_the_type_and_survive_a_round_trip() {
    for json in [
        r#"{ "id": "n", "name": "N", "type": "number", "default": 1, "min": 0 }"#,
        r#"{ "id": "n", "name": "N", "type": "number", "default": true, "min": 0, "max": 2 }"#,
        r#"{ "id": "n", "name": "N", "type": "number", "default": 1, "min": 0, "max": 2, "choices": ["a"] }"#,
        r#"{ "id": "b", "name": "B", "type": "bool", "default": true, "min": 0 }"#,
        r#"{ "id": "b", "name": "B", "type": "bool", "default": 1 }"#,
        r#"{ "id": "m", "name": "M", "type": "choice", "default": "a" }"#,
        r#"{ "id": "m", "name": "M", "type": "choice", "choices": ["a"], "default": "a", "scale": 1 }"#,
    ] {
        assert!(serde_json::from_str::<SettingSpec>(json).is_err(), "{json}");
    }
    for spec in [
        length(),
        spec(
            r#"{ "id": "m", "name": "M", "type": "choice", "choices": ["a", "b"], "default": "b" }"#,
        ),
        spec(r##"{ "id": "c", "name": "C", "type": "color", "default": "#2962ff" }"##),
    ] {
        let json = serde_json::to_string(&spec).expect("json");
        assert_eq!(
            serde_json::from_str::<SettingSpec>(&json).expect("back"),
            spec
        );
    }
}
