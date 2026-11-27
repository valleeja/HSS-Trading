//! Settings of an interface v2 module (`docs/abi-v2.md`): typed, bounded,
//! part of the signed policy. Settings are not rights; the host checks every
//! chosen value against them before a run.

use serde::{Deserialize, Serialize};

use crate::{push, Language, PolicyError};

/// Most settings a module may declare.
pub const MAX_SETTINGS: usize = 16;
/// Largest number of decimals of a `number` setting.
pub const MAX_SETTING_SCALE: u8 = 6;
/// Most choices of a `choice` setting.
pub const MAX_CHOICES: usize = 32;

/// One setting the user may change, declared in `argos-module.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "RawSetting", into = "RawSetting")]
pub struct SettingSpec {
    /// Stable identifier: 1 to 32 letters, digits, `-` or `_`.
    pub id: String,
    /// Name shown to the user, 1 to 64 bytes.
    pub name: String,
    /// Type, bounds and default.
    pub kind: SettingKind,
}

/// Type of a setting, with its bounds and default.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingKind {
    /// An integer at `scale` decimals (150 at scale 2 means 1.50).
    Number {
        /// Default value, at the scale.
        default: i64,
        /// Smallest value, at the scale.
        min: i64,
        /// Largest value, at the scale.
        max: i64,
        /// Decimals, 0 to [`MAX_SETTING_SCALE`].
        scale: u8,
    },
    /// On or off (given to the module as 1 or 0).
    Bool {
        /// Default value.
        default: bool,
    },
    /// A colour `#rrggbb` (given to the module as `0xRRGGBB`).
    Color {
        /// Default colour, `#rrggbb`.
        default: String,
    },
    /// One of a list (given to the module as the index of the choice).
    Choice {
        /// The choices, 1 to [`MAX_CHOICES`], each 1 to 64 bytes, unique.
        choices: Vec<String>,
        /// Default choice, one of `choices`.
        default: String,
    },
}

/// Type names of the JSON form.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RawType {
    Number,
    Bool,
    Color,
    Choice,
}

/// A default of the JSON form: an integer, a boolean or a text.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum RawDefault {
    Int(i64),
    Bool(bool),
    Text(String),
}

/// The JSON form: every field a type does not use must be absent.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawSetting {
    id: String,
    name: String,
    #[serde(rename = "type")]
    kind: RawType,
    default: RawDefault,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    scale: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    choices: Option<Vec<String>>,
}

impl TryFrom<RawSetting> for SettingSpec {
    type Error = String;

    fn try_from(raw: RawSetting) -> Result<Self, String> {
        let wrong = || format!("setting {:?}: fields do not match its type", raw.id);
        let RawSetting {
            min,
            max,
            scale,
            choices,
            ..
        } = raw.clone();
        let kind = match (raw.kind, raw.default) {
            (RawType::Number, RawDefault::Int(default)) if choices.is_none() => {
                SettingKind::Number {
                    default,
                    min: min.ok_or_else(wrong)?,
                    max: max.ok_or_else(wrong)?,
                    scale: scale.unwrap_or(0),
                }
            }
            (RawType::Bool, RawDefault::Bool(default)) => SettingKind::Bool { default },
            (RawType::Color, RawDefault::Text(default)) => SettingKind::Color { default },
            (RawType::Choice, RawDefault::Text(default)) => SettingKind::Choice {
                choices: choices.clone().ok_or_else(wrong)?,
                default,
            },
            _ => return Err(wrong()),
        };
        let extra = match &kind {
            SettingKind::Number { .. } => choices.is_some(),
            SettingKind::Choice { .. } => min.is_some() || max.is_some() || scale.is_some(),
            _ => min.is_some() || max.is_some() || scale.is_some() || choices.is_some(),
        };
        if extra {
            return Err(wrong());
        }
        Ok(Self {
            id: raw.id,
            name: raw.name,
            kind,
        })
    }
}

impl From<SettingSpec> for RawSetting {
    fn from(spec: SettingSpec) -> Self {
        let mut raw = Self {
            id: spec.id,
            name: spec.name,
            kind: RawType::Bool,
            default: RawDefault::Bool(false),
            min: None,
            max: None,
            scale: None,
            choices: None,
        };
        match spec.kind {
            SettingKind::Number {
                default,
                min,
                max,
                scale,
            } => {
                raw.kind = RawType::Number;
                raw.default = RawDefault::Int(default);
                (raw.min, raw.max, raw.scale) = (Some(min), Some(max), Some(scale));
            }
            SettingKind::Bool { default } => raw.default = RawDefault::Bool(default),
            SettingKind::Color { default } => {
                raw.kind = RawType::Color;
                raw.default = RawDefault::Text(default);
            }
            SettingKind::Choice { choices, default } => {
                raw.kind = RawType::Choice;
                raw.default = RawDefault::Text(default);
                raw.choices = Some(choices);
            }
        }
        raw
    }
}

/// `0xRRGGBB` for a colour written `#rrggbb` (lowercase or uppercase).
pub fn parse_color(text: &str) -> Option<i64> {
    let hex = text.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    i64::from_str_radix(hex, 16).ok()
}

fn valid_id(id: &str) -> bool {
    (1..=32).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn valid_label(text: &str) -> bool {
    (1..=64).contains(&text.len()) && !text.chars().any(char::is_control)
}

impl SettingSpec {
    /// Checks the identifier, the name, the bounds and the default.
    ///
    /// # Errors
    /// [`PolicyError::InvalidSetting`] with the setting's identifier.
    pub fn check(&self) -> Result<(), PolicyError> {
        let ok = valid_id(&self.id)
            && valid_label(&self.name)
            && match &self.kind {
                SettingKind::Number {
                    default,
                    min,
                    max,
                    scale,
                } => *scale <= MAX_SETTING_SCALE && min <= default && default <= max,
                SettingKind::Bool { .. } => true,
                SettingKind::Color { default } => parse_color(default).is_some(),
                SettingKind::Choice { choices, default } => {
                    (1..=MAX_CHOICES).contains(&choices.len())
                        && choices.iter().all(|c| valid_label(c))
                        && choices
                            .iter()
                            .enumerate()
                            .all(|(i, c)| !choices[..i].contains(c))
                        && choices.contains(default)
                }
            };
        if ok {
            Ok(())
        } else {
            Err(PolicyError::InvalidSetting(self.id.clone()))
        }
    }

    /// The default, as given to the module.
    pub fn default_value(&self) -> i64 {
        match &self.kind {
            SettingKind::Number { default, .. } => *default,
            SettingKind::Bool { default } => i64::from(*default),
            SettingKind::Color { default } => parse_color(default).unwrap_or(0),
            SettingKind::Choice { choices, default } => choices
                .iter()
                .position(|c| c == default)
                .and_then(|i| i64::try_from(i).ok())
                .unwrap_or(0),
        }
    }

    /// True if `value` (as given to the module) is allowed.
    pub fn allows(&self, value: i64) -> bool {
        match &self.kind {
            SettingKind::Number { min, max, .. } => (*min..=*max).contains(&value),
            SettingKind::Bool { .. } => value == 0 || value == 1,
            SettingKind::Color { .. } => (0..=0x00ff_ffff).contains(&value),
            SettingKind::Choice { choices, .. } => {
                usize::try_from(value).is_ok_and(|i| i < choices.len())
            }
        }
    }

    /// The setting in plain words, for the rights sheet.
    pub fn describe(&self, language: Language) -> String {
        let french = language == Language::French;
        let name = &self.name;
        match &self.kind {
            SettingKind::Number {
                default,
                min,
                max,
                scale,
            } => {
                let f = |v: i64| fixed(v, *scale);
                if french {
                    format!(
                        "{name} : nombre de {} à {}, {} par défaut",
                        f(*min),
                        f(*max),
                        f(*default)
                    )
                } else {
                    format!(
                        "{name}: number from {} to {}, default {}",
                        f(*min),
                        f(*max),
                        f(*default)
                    )
                }
            }
            SettingKind::Bool { default } => match (french, default) {
                (true, true) => format!("{name} : oui ou non, oui par défaut"),
                (true, false) => format!("{name} : oui ou non, non par défaut"),
                (false, true) => format!("{name}: yes or no, default yes"),
                (false, false) => format!("{name}: yes or no, default no"),
            },
            SettingKind::Color { default } => {
                if french {
                    format!("{name} : couleur, {default} par défaut")
                } else {
                    format!("{name}: colour, default {default}")
                }
            }
            SettingKind::Choice { choices, default } => {
                let list = choices.join(", ");
                if french {
                    format!("{name} : un choix parmi {list}, {default} par défaut")
                } else {
                    format!("{name}: one of {list}, default {default}")
                }
            }
        }
    }

    pub(crate) fn encode(&self, out: &mut Vec<u8>) {
        push(out, self.id.as_bytes());
        push(out, self.name.as_bytes());
        match &self.kind {
            SettingKind::Number {
                default,
                min,
                max,
                scale,
            } => {
                out.push(0);
                for v in [default, min, max] {
                    out.extend_from_slice(&v.to_be_bytes());
                }
                out.push(*scale);
            }
            SettingKind::Bool { default } => out.extend_from_slice(&[1, u8::from(*default)]),
            SettingKind::Color { default } => {
                out.push(2);
                push(out, default.as_bytes());
            }
            SettingKind::Choice { choices, default } => {
                out.push(3);
                let count = u32::try_from(choices.len()).unwrap_or(u32::MAX);
                out.extend_from_slice(&count.to_be_bytes());
                for choice in choices {
                    push(out, choice.as_bytes());
                }
                push(out, default.as_bytes());
            }
        }
    }
}

/// `value` at `scale` decimals, for display: `fixed(-150, 2)` is `-1.50`.
pub fn fixed(value: i64, scale: u8) -> String {
    let digits = value.unsigned_abs().to_string();
    let sign = if value < 0 { "-" } else { "" };
    let scale = usize::from(scale);
    if scale == 0 {
        return format!("{sign}{digits}");
    }
    let padded = format!("{digits:0>width$}", width = scale + 1);
    let (int, frac) = padded.split_at(padded.len() - scale);
    format!("{sign}{int}.{frac}")
}
