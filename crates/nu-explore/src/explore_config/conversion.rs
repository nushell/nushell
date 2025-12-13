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
#[allow(clippy::only_used_in_recursion)]
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
            let closure_string =
                val.coerce_into_string(engine_state, value.span())
                    .map_err(|e| ShellError::GenericError {
                        error: "Failed to convert closure to string".to_string(),
                        msg: "".to_string(),
                        span: Some(value.span()),
                        help: None,
                        inner: vec![e],
                    })?;
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

        if trimmed.is_empty() {
            // Empty lines clear the comment buffer - this ensures section headings
            // (which are separated from actual documentation by blank lines)
            // don't get included in the documentation for settings
            current_comments.clear();
        } else if trimmed.starts_with('#') {
            // Collect comment lines (strip the leading # and space)
            let comment = trimmed.trim_start_matches('#').trim();
            if !comment.is_empty() {
                current_comments.push(comment.to_string());
            }
        } else if trimmed.starts_with("$env.config.") {
            // This is a config setting line
            // Extract the path (everything between "$env.config." and " =" or end of relevant part)
            if let Some(path) = extract_config_path(trimmed)
                && !current_comments.is_empty()
            {
                // Join all collected comments as the documentation
                let doc = current_comments.join("\n");
                doc_map.insert(path, doc);
            }
            // Clear comments after processing a setting
            current_comments.clear();
        } else {
            // Non-comment, non-config, non-empty line - might be code examples, clear comments
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
    let path_end = rest.find(['=', ' ']).unwrap_or(rest.len());

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
    let identifier = path_to_identifier(&current_path);

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

/// Build a map of path identifiers to original Nu values for types that can't be roundtripped
/// (like Closures, Dates, Ranges, etc.)
pub fn build_original_value_map(
    value: &nu_protocol::Value,
    current_path: Vec<String>,
    value_map: &mut HashMap<String, nu_protocol::Value>,
) {
    let identifier = path_to_identifier(&current_path);

    // Store values that can't be roundtripped through JSON
    if !identifier.is_empty() {
        match value {
            nu_protocol::Value::Closure { .. }
            | nu_protocol::Value::Date { .. }
            | nu_protocol::Value::Range { .. } => {
                value_map.insert(identifier.clone(), value.clone());
            }
            _ => {}
        }
    }

    match value {
        nu_protocol::Value::Record { val, .. } => {
            for (k, v) in val.iter() {
                let mut path = current_path.clone();
                path.push(k.clone());
                build_original_value_map(v, path, value_map);
            }
        }
        nu_protocol::Value::List { vals, .. } => {
            for (idx, v) in vals.iter().enumerate() {
                let mut path = current_path.clone();
                path.push(idx.to_string());
                build_original_value_map(v, path, value_map);
            }
        }
        _ => {}
    }
}

/// Convert a path vector to an identifier string (e.g., ["history", "file_format"] -> "history.file_format")
fn path_to_identifier(path: &[String]) -> String {
    if path.is_empty() {
        String::new()
    } else {
        path.iter()
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
    }
}

/// Convert a serde_json::Value to a nu_protocol::Value (simple version without type info)
#[allow(dead_code)]
pub fn json_to_nu_value(
    json: &Value,
    span: nu_protocol::Span,
) -> Result<nu_protocol::Value, Box<dyn Error>> {
    json_to_nu_value_with_types(json, span, &None, &None, Vec::new())
}

/// Convert a serde_json::Value to a nu_protocol::Value, using type information to preserve
/// original Nu types like Duration, Filesize, and Closure
pub fn json_to_nu_value_with_types(
    json: &Value,
    span: nu_protocol::Span,
    type_map: &Option<HashMap<String, NuValueType>>,
    original_values: &Option<HashMap<String, nu_protocol::Value>>,
    current_path: Vec<String>,
) -> Result<nu_protocol::Value, Box<dyn Error>> {
    let identifier = path_to_identifier(&current_path);
    let original_type = type_map.as_ref().and_then(|m| m.get(&identifier));

    Ok(match json {
        Value::Null => nu_protocol::Value::nothing(span),
        Value::Bool(b) => nu_protocol::Value::bool(*b, span),
        Value::Number(n) => {
            // Check if we need to convert to a special type based on original
            if let Some(orig_type) = original_type {
                match orig_type {
                    NuValueType::Duration => {
                        if let Some(i) = n.as_i64() {
                            return Ok(nu_protocol::Value::duration(i, span));
                        }
                    }
                    NuValueType::Filesize => {
                        if let Some(i) = n.as_i64() {
                            return Ok(nu_protocol::Value::filesize(i, span));
                        }
                    }
                    _ => {}
                }
            }
            // Default number handling
            if let Some(i) = n.as_i64() {
                nu_protocol::Value::int(i, span)
            } else if let Some(f) = n.as_f64() {
                nu_protocol::Value::float(f, span)
            } else {
                return Err(format!("Unsupported number: {}", n).into());
            }
        }
        Value::String(s) => {
            // Check if we need to restore an original value that can't be roundtripped
            if let Some(orig_type) = original_type {
                match orig_type {
                    NuValueType::Closure | NuValueType::Date | NuValueType::Range => {
                        // Try to get the original value - closures, dates, and ranges
                        // can't be reconstructed from their string representation
                        if let Some(original_values_map) = original_values
                            && let Some(original_value) = original_values_map.get(&identifier)
                        {
                            // Return the original value since we can't reconstruct these types
                            return Ok(original_value.clone());
                        }
                        // If no original value found, keep as string
                        // This will likely cause a config error, but that's the expected behavior
                        // since the user modified something that can't be properly converted
                    }
                    NuValueType::Glob => {
                        return Ok(nu_protocol::Value::glob(s.clone(), false, span));
                    }
                    _ => {}
                }
            }
            nu_protocol::Value::string(s.clone(), span)
        }
        Value::Array(arr) => {
            // Check if this was originally binary data
            if let Some(NuValueType::Binary) = original_type {
                let bytes: Result<Vec<u8>, _> = arr
                    .iter()
                    .map(|v| {
                        v.as_i64()
                            .and_then(|i| u8::try_from(i).ok())
                            .ok_or("Invalid byte value")
                    })
                    .collect();
                if let Ok(bytes) = bytes {
                    return Ok(nu_protocol::Value::binary(bytes, span));
                }
            }

            // Check if this was originally a CellPath
            if let Some(NuValueType::CellPath) = original_type {
                use nu_protocol::ast::PathMember;
                use nu_protocol::casing::Casing;
                let members: Result<Vec<PathMember>, _> = arr
                    .iter()
                    .map(|v| match v {
                        Value::String(s) => Ok(PathMember::String {
                            val: s.clone(),
                            span,
                            optional: false,
                            casing: Casing::Sensitive,
                        }),
                        Value::Number(n) => {
                            if let Some(i) = n.as_u64() {
                                Ok(PathMember::Int {
                                    val: i as usize,
                                    span,
                                    optional: false,
                                })
                            } else {
                                Err("Invalid cell path member")
                            }
                        }
                        _ => Err("Invalid cell path member"),
                    })
                    .collect();
                if let Ok(members) = members {
                    return Ok(nu_protocol::Value::cell_path(
                        nu_protocol::ast::CellPath { members },
                        span,
                    ));
                }
            }

            // Regular array/list
            let values: Result<Vec<_>, _> = arr
                .iter()
                .enumerate()
                .map(|(idx, v)| {
                    let mut path = current_path.clone();
                    path.push(idx.to_string());
                    json_to_nu_value_with_types(v, span, type_map, original_values, path)
                })
                .collect();
            nu_protocol::Value::list(values?, span)
        }
        Value::Object(obj) => {
            let mut record = nu_protocol::Record::new();
            for (k, v) in obj {
                let mut path = current_path.clone();
                path.push(k.clone());
                record.push(
                    k.clone(),
                    json_to_nu_value_with_types(v, span, type_map, original_values, path)?,
                );
            }
            nu_protocol::Value::record(record, span)
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use nu_protocol::Span;

    fn test_span() -> Span {
        Span::test_data()
    }

    #[test]
    fn test_duration_roundtrip() {
        // Create a type map with a duration type
        let mut type_map = HashMap::new();
        type_map.insert("timeout".to_string(), NuValueType::Duration);
        let type_map = Some(type_map);

        // Create JSON with a number that should be converted to duration
        let json = serde_json::json!({
            "timeout": 5000000000_i64  // 5 seconds in nanoseconds
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        // Check that it's a record with a duration value
        if let nu_protocol::Value::Record { val, .. } = result {
            let timeout = val.get("timeout").expect("timeout field should exist");
            assert!(
                matches!(timeout, nu_protocol::Value::Duration { .. }),
                "Expected Duration, got {:?}",
                timeout
            );
            if let nu_protocol::Value::Duration { val, .. } = timeout {
                assert_eq!(*val, 5000000000);
            }
        } else {
            panic!("Expected Record, got {:?}", result);
        }
    }

    #[test]
    fn test_filesize_roundtrip() {
        let mut type_map = HashMap::new();
        type_map.insert("size".to_string(), NuValueType::Filesize);
        let type_map = Some(type_map);

        let json = serde_json::json!({
            "size": 1048576_i64  // 1 MiB in bytes
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let size = val.get("size").expect("size field should exist");
            assert!(
                matches!(size, nu_protocol::Value::Filesize { .. }),
                "Expected Filesize, got {:?}",
                size
            );
        } else {
            panic!("Expected Record, got {:?}", result);
        }
    }

    #[test]
    fn test_nested_duration() {
        let mut type_map = HashMap::new();
        type_map.insert(
            "plugin_gc.default.stop_after".to_string(),
            NuValueType::Duration,
        );
        let type_map = Some(type_map);

        let json = serde_json::json!({
            "plugin_gc": {
                "default": {
                    "stop_after": 0_i64
                }
            }
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        // Navigate to the nested value
        if let nu_protocol::Value::Record { val: outer, .. } = result {
            let plugin_gc = outer.get("plugin_gc").expect("plugin_gc should exist");
            if let nu_protocol::Value::Record { val: inner, .. } = plugin_gc {
                let default = inner.get("default").expect("default should exist");
                if let nu_protocol::Value::Record {
                    val: default_rec, ..
                } = default
                {
                    let stop_after = default_rec
                        .get("stop_after")
                        .expect("stop_after should exist");
                    assert!(
                        matches!(stop_after, nu_protocol::Value::Duration { .. }),
                        "Expected Duration, got {:?}",
                        stop_after
                    );
                } else {
                    panic!("Expected Record for default");
                }
            } else {
                panic!("Expected Record for plugin_gc");
            }
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_closure_restored_from_original() {
        // Create a type map marking this as a closure
        let mut type_map = HashMap::new();
        type_map.insert("hook".to_string(), NuValueType::Closure);
        let type_map = Some(type_map);

        // Create an original value map with the closure
        let mut original_values = HashMap::new();
        // We can't easily create a real closure in tests, so we'll test the path exists
        // In practice, the original closure value would be stored here

        let json = serde_json::json!({
            "hook": "{|| print 'hello'}"
        });

        // Without original value, it stays as string
        let result = json_to_nu_value_with_types(
            &json,
            test_span(),
            &type_map,
            &Some(original_values.clone()),
            Vec::new(),
        )
        .unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let hook = val.get("hook").expect("hook field should exist");
            // Without an original value stored, it remains a string
            assert!(
                matches!(hook, nu_protocol::Value::String { .. }),
                "Expected String when no original closure available, got {:?}",
                hook
            );
        } else {
            panic!("Expected Record");
        }

        // Now test with an original value stored (using a simple value as stand-in)
        // In real usage, this would be the actual Closure value
        original_values.insert(
            "hook".to_string(),
            nu_protocol::Value::string("original_closure_placeholder", test_span()),
        );

        let result = json_to_nu_value_with_types(
            &json,
            test_span(),
            &type_map,
            &Some(original_values),
            Vec::new(),
        )
        .unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let hook = val.get("hook").expect("hook field should exist");
            // With original value stored, it should return that value
            if let nu_protocol::Value::String { val: s, .. } = hook {
                assert_eq!(s, "original_closure_placeholder");
            } else {
                panic!("Expected the original value to be returned");
            }
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_glob_roundtrip() {
        let mut type_map = HashMap::new();
        type_map.insert("pattern".to_string(), NuValueType::Glob);
        let type_map = Some(type_map);

        let json = serde_json::json!({
            "pattern": "*.txt"
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let pattern = val.get("pattern").expect("pattern field should exist");
            assert!(
                matches!(pattern, nu_protocol::Value::Glob { .. }),
                "Expected Glob, got {:?}",
                pattern
            );
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_binary_roundtrip() {
        let mut type_map = HashMap::new();
        type_map.insert("data".to_string(), NuValueType::Binary);
        let type_map = Some(type_map);

        let json = serde_json::json!({
            "data": [0, 1, 2, 255]
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let data = val.get("data").expect("data field should exist");
            assert!(
                matches!(data, nu_protocol::Value::Binary { .. }),
                "Expected Binary, got {:?}",
                data
            );
            if let nu_protocol::Value::Binary { val, .. } = data {
                assert_eq!(val, &vec![0u8, 1, 2, 255]);
            }
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_list_with_typed_elements() {
        let mut type_map = HashMap::new();
        type_map.insert("timeouts[0]".to_string(), NuValueType::Duration);
        type_map.insert("timeouts[1]".to_string(), NuValueType::Duration);
        let type_map = Some(type_map);

        let json = serde_json::json!({
            "timeouts": [1000000000_i64, 2000000000_i64]
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &type_map, &None, Vec::new()).unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let timeouts = val.get("timeouts").expect("timeouts field should exist");
            if let nu_protocol::Value::List { vals, .. } = timeouts {
                assert_eq!(vals.len(), 2);
                for (i, v) in vals.iter().enumerate() {
                    assert!(
                        matches!(v, nu_protocol::Value::Duration { .. }),
                        "Expected Duration at index {}, got {:?}",
                        i,
                        v
                    );
                }
            } else {
                panic!("Expected List");
            }
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_without_type_map_uses_defaults() {
        // Without a type map, numbers stay as numbers, strings as strings
        let json = serde_json::json!({
            "timeout": 5000000000_i64,
            "name": "test"
        });

        let result =
            json_to_nu_value_with_types(&json, test_span(), &None, &None, Vec::new()).unwrap();

        if let nu_protocol::Value::Record { val, .. } = result {
            let timeout = val.get("timeout").expect("timeout field should exist");
            assert!(
                matches!(timeout, nu_protocol::Value::Int { .. }),
                "Expected Int without type map, got {:?}",
                timeout
            );
            let name = val.get("name").expect("name field should exist");
            assert!(
                matches!(name, nu_protocol::Value::String { .. }),
                "Expected String, got {:?}",
                name
            );
        } else {
            panic!("Expected Record");
        }
    }

    #[test]
    fn test_path_to_identifier() {
        assert_eq!(path_to_identifier(&[]), "");
        assert_eq!(path_to_identifier(&["foo".to_string()]), "foo");
        assert_eq!(
            path_to_identifier(&["foo".to_string(), "bar".to_string()]),
            "foo.bar"
        );
        assert_eq!(
            path_to_identifier(&["foo".to_string(), "0".to_string()]),
            "foo[0]"
        );
        assert_eq!(
            path_to_identifier(&["foo".to_string(), "0".to_string(), "bar".to_string()]),
            "foo[0].bar"
        );
    }

    #[test]
    fn test_build_nu_type_map() {
        let span = test_span();

        // Create a nested Nu value structure
        let mut inner_record = nu_protocol::Record::new();
        inner_record.push(
            "stop_after".to_string(),
            nu_protocol::Value::duration(0, span),
        );

        let mut outer_record = nu_protocol::Record::new();
        outer_record.push(
            "default".to_string(),
            nu_protocol::Value::record(inner_record, span),
        );

        let mut root_record = nu_protocol::Record::new();
        root_record.push(
            "plugin_gc".to_string(),
            nu_protocol::Value::record(outer_record, span),
        );

        let root_value = nu_protocol::Value::record(root_record, span);

        let mut type_map = HashMap::new();
        build_nu_type_map(&root_value, Vec::new(), &mut type_map);

        assert_eq!(type_map.get("plugin_gc"), Some(&NuValueType::Record));
        assert_eq!(
            type_map.get("plugin_gc.default"),
            Some(&NuValueType::Record)
        );
        assert_eq!(
            type_map.get("plugin_gc.default.stop_after"),
            Some(&NuValueType::Duration)
        );
    }

    #[test]
    fn test_build_original_value_map() {
        let span = test_span();

        // Create a structure with a duration (which can be roundtripped) and simulate
        // what would happen with non-roundtrippable types
        let mut record = nu_protocol::Record::new();
        record.push(
            "duration".to_string(),
            nu_protocol::Value::duration(0, span),
        );
        record.push(
            "string".to_string(),
            nu_protocol::Value::string("test", span),
        );

        let root_value = nu_protocol::Value::record(record, span);

        let mut value_map = HashMap::new();
        build_original_value_map(&root_value, Vec::new(), &mut value_map);

        // Duration and String are roundtrippable, so they shouldn't be in the map
        assert!(!value_map.contains_key("duration"));
        assert!(!value_map.contains_key("string"));
    }
}
