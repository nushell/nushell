use crate::{Record, ShellError, Span, Value};
use std::{collections::HashMap, fmt::Display, str::FromStr};

pub(super) trait ReconstructVal {
    fn reconstruct_value(&self, span: Span) -> Value;
}

pub(super) fn process_string_enum<T, S, E>(
    config_point: &mut T,
    config_path: S,
    value: &mut Value,
    errors: &mut Vec<ShellError>,
) where
    T: FromStr<Err = E> + ReconstructVal,
    E: Display,
    S: AsRef<str> + Display,
{
    let span = value.span();
    if let Ok(v) = value.as_string() {
        match v.parse() {
            Ok(format) => {
                *config_point = format;
            }
            Err(err) => {
                errors.push(ShellError::GenericError(
                    "Error while applying config changes".into(),
                    format!("unrecognized {config_path} option '{v}'"),
                    Some(span),
                    Some(err.to_string()),
                    vec![],
                ));
                // Reconstruct
                *value = config_point.reconstruct_value(span);
            }
        }
    } else {
        errors.push(ShellError::GenericError(
            "Error while applying config changes".into(),
            format!("{config_path} should be a string"),
            Some(span),
            Some("This value will be ignored.".into()),
            vec![],
        ));
        // Reconstruct
        *value = config_point.reconstruct_value(span);
    }
}

pub(super) fn create_map(value: &Value) -> Result<HashMap<String, Value>, ShellError> {
    Ok(value
        .as_record()?
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect())
}

pub fn extract_value<'record>(
    name: &str,
    record: &'record Record,
    span: Span,
) -> Result<&'record Value, ShellError> {
    record
        .get(name)
        .ok_or_else(|| ShellError::MissingConfigValue(name.to_string(), span))
}
