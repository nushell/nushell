//! Conversion utilities between Nu values and JSON, and config documentation parsing.

use crate::explore_config::types::NuValueType;
use nu_protocol::ShellError;
use nu_protocol::engine::EngineState;
use nu_utils::ConfigFileKind;
use serde_json::Value;
use std::collections::HashMap;
use std::error::Error;

/// Convert a nu_protocol::Value to a serde_json::Value
/// This properly handles closures by converting them to their string representation
pub fn nu_value_to_json(
    engine_state: &EngineState,
    value: &nu_protocol::Value,
    span: nu_protocol::Span,
) -> Result<Value, ShellError> {
    Ok(match value {
        nu_protocol::Value::Bool { val, .. } => Value::Bool(*val),
        nu_protocol::Value::Int { val, .. } => Value::Number((*val).into()),
        nu_protocol::Value::Float { val, .. } => serde_json::Number::from_f64(*val)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        nu_protocol::Value::String { val, .. } => Value::String(val.clone()),
        nu_protocol::Value::Nothing { .. } => Value::Null,
        nu_protocol::Value::List { vals, .. } => {
            let json_vals: Result<Vec<_>, _> = vals
                .iter()
                .map(|v| nu_value_to_json(engine_state, v, span))
                .collect();
            Value::Array(json_vals?)
        }
        nu_protocol::Value::Record { val, .. } => {
            let mut map = serde_json::Map::new();
            for (k, v) in val.iter() {
                map.insert(k.clone(), nu_value_to_json(engine_state, v, span)?);
            }
            Value::Object(map)
        }
        nu_protocol::Value::Closure { val, .. } => {
            // Convert closure to its string representation instead of serializing internal structure
            let closure_string = val.coerce_into_string(engine_state, value.span())?;
            Value::String(closure_string.to_string())
        }
        nu_protocol::Value::Filesize { val, .. } => Value::Number(val.get().into()),
        nu_protocol::Value::Duration { val, .. } => Value::Number((*val).into()),
        nu_protocol::Value::Date { val, .. } => Value::String(val.to_string()),
        nu_protocol::Value::Glob { val, .. } => Value::String(val.to_string()),
        nu_protocol::Value::CellPath { val, .. } => {
            let parts: Vec<Value> = val
                .members
                .iter()
                .map(|m| match m {
                    nu_protocol::ast::PathMember::String { val, .. } => Value::String(val.clone()),
                    nu_protocol::ast::PathMember::Int { val, .. } => {
                        Value::Number((*val as i64).into())
                    }
                })
                .collect();
            Value::Array(parts)
        }
        nu_protocol::Value::Binary { val, .. } => Value::Array(
            val.iter()
                .map(|b| Value::Number((*b as i64).into()))
                .collect(),
        ),
        nu_protocol::Value::Range { .. } => Value::Null,
        nu_protocol::Value::Error { error, .. } => {
            return Err(*error.clone());
        }
        nu_protocol::Value::Custom { val, .. } => {
            let collected = val.to_base_value(value.span())?;
            nu_value_to_json(engine_state, &collected, span)?
        }
    })
}

/// Parse the doc_config.nu file to extract documentation for each config path
/// Returns a HashMap mapping config paths (e.g., "history.file_format") to their documentation
pub fn parse_config_documentation() -> HashMap<String, String> {
    let doc_content = ConfigFileKind::Config.doc();
    let mut doc_map = HashMap::new();
    let mut current_comments: Vec<String> = Vec::new();

    for line in doc_content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('#') {
            // Collect comment lines (strip the leading # and space)
            let comment = trimmed.trim_start_matches('#').trim();
            if !comment.is_empty() {
                current_comments.push(comment.to_string());
            }
        } else if trimmed.starts_with("$env.config.") {
            // This is a config setting line
            // Extract the path (everything between "$env.config." and " =" or end of relevant part)
            if let Some(path) = extract_config_path(trimmed) {
                if !current_comments.is_empty() {
                    // Join all collected comments as the documentation
                    let doc = current_comments.join("\n");
                    doc_map.insert(path, doc);
                }
            }
            // Clear comments after processing a setting
            current_comments.clear();
        } else if !trimmed.is_empty() {
            // Non-comment, non-config line - might be code examples, clear comments
            // But keep comments if the line is empty (paragraph break in docs)
            current_comments.clear();
        }
    }

    doc_map
}

/// Extract the config path from a line like "$env.config.history.file_format = ..."
/// Returns the path without "$env.config." prefix (e.g., "history.file_format")
pub fn extract_config_path(line: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with("$env.config.") {
        return None;
    }

    // Remove "$env.config." prefix
    let rest = &line["$env.config.".len()..];

    // Find where the path ends (at '=' or end of line for bare references)
    let path_end = rest
        .find(|c: char| c == '=' || c == ' ')
        .unwrap_or(rest.len());

    let path = rest[..path_end].trim();
    if path.is_empty() {
        None
    } else {
        Some(path.to_string())
    }
}

/// Build a map of path identifiers to NuValueType for tracking original nushell types
pub fn build_nu_type_map(
    value: &nu_protocol::Value,
    current_path: Vec<String>,
    type_map: &mut HashMap<String, NuValueType>,
) {
    let identifier = if current_path.is_empty() {
        String::new()
    } else {
        current_path
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if p.parse::<usize>().is_ok() {
                    format!("[{}]", p)
                } else if i == 0 {
                    p.clone()
                } else {
                    format!(".{}", p)
                }
            })
            .collect::<String>()
    };

    if !identifier.is_empty() {
        type_map.insert(identifier.clone(), NuValueType::from_nu_value(value));
    }

    match value {
        nu_protocol::Value::Record { val, .. } => {
            for (k, v) in val.iter() {
                let mut path = current_path.clone();
                path.push(k.clone());
                build_nu_type_map(v, path, type_map);
            }
        }
        nu_protocol::Value::List { vals, .. } => {
            for (idx, v) in vals.iter().enumerate() {
                let mut path = current_path.clone();
                path.push(idx.to_string());
                build_nu_type_map(v, path, type_map);
            }
        }
        _ => {}
    }
}

/// Convert a serde_json::Value to a nu_protocol::Value
pub fn json_to_nu_value(
    json: &Value,
    span: nu_protocol::Span,
) -> Result<nu_protocol::Value, Box<dyn Error>> {
    Ok(match json {
        Value::Null => nu_protocol::Value::nothing(span),
        Value::Bool(b) => nu_protocol::Value::bool(*b, span),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                nu_protocol::Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                nu_protocol::Value::float(f, span)
            } else {
                return Err(format!("Unsupported number: {}", n).into());
            }
        }
        Value::String(s) => nu_protocol::Value::string(s.clone(), span),
        Value::Array(arr) => {
            let values: Result<Vec<_>, _> = arr.iter().map(|v| json_to_nu_value(v, span)).collect();
            nu_protocol::Value::list(values?, span)
        }
        Value::Object(obj) => {
            let mut record = nu_protocol::Record::new();
            for (k, v) in obj {
                record.push(k.clone(), json_to_nu_value(v, span)?);
            }
            nu_protocol::Value::record(record, span)
        }
    })
}
