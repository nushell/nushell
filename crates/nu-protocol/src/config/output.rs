use super::{config_update_string_enum, prelude::*};

use crate::{self as nu_protocol};

#[derive(Clone, Copy, Debug, IntoValue, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorStyle {
    Plain,
    Fancy,
}

impl FromStr for ErrorStyle {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "fancy" => Ok(Self::Fancy),
            "plain" => Ok(Self::Plain),
            _ => Err("'fancy' or 'plain'"),
        }
    }
}

impl UpdateFromValue for ErrorStyle {
    fn update(&mut self, value: &Value, path: &mut ConfigPath, errors: &mut ConfigErrors) {
        config_update_string_enum(self, value, path, errors)
    }
}

/// Option: show_banner
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BannerKind {
    /// No banner on startup
    None,
    /// Abbreviated banner just containing the startup-time
    Short,
    /// The full banner including Ellie
    #[default]
    Full,
}

impl IntoValue for BannerKind {
    fn into_value(self, span: Span) -> Value {
        match self {
            // This uses a custom implementation to reflect common config
            // bool: true, false was used for a long time
            // string: short was added later
            BannerKind::None => Value::bool(false, span),
            BannerKind::Short => Value::string("short", span),
            BannerKind::Full => Value::bool(true, span),
        }
    }
}

impl UpdateFromValue for BannerKind {
    fn update<'a>(
        &mut self,
        value: &'a Value,
        path: &mut ConfigPath<'a>,
        errors: &mut ConfigErrors,
    ) {
        match value {
            Value::Bool { val, .. } => match val {
                true => {
                    *self = BannerKind::Full;
                }
                false => {
                    *self = BannerKind::None;
                }
            },
            Value::String { val, .. } => match val.as_str() {
                "true" => {
                    *self = BannerKind::Full;
                }
                "full" => {
                    *self = BannerKind::Full;
                }
                "short" => {
                    *self = BannerKind::Short;
                }
                "false" => {
                    *self = BannerKind::None;
                }
                "none" => {
                    *self = BannerKind::None;
                }
                _ => {
                    errors.invalid_value(path, "true/'full', 'short', false/'none'", value);
                }
            },
            _ => {
                errors.invalid_value(path, "true/'full', 'short', false/'none'", value);
            }
        }
    }
}
