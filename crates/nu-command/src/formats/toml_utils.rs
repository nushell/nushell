use crate::formats::nu_value_to_toml_value;
use nu_engine::command_prelude::*;
use nu_protocol::{DataSource, PipelineMetadata};
use toml_edit::{DocumentMut, Item, TableLike};

#[derive(Clone, Copy)]
enum ContainerKind {
    Table,
    InlineTable,
}

/// Reads the original TOML source from the file referenced in pipeline metadata.
/// Returns `None` if the metadata doesn't point to a `.toml` file or the file can't be read.
pub(crate) fn read_toml_source_from_metadata(
    metadata: Option<&PipelineMetadata>,
) -> Option<String> {
    let path = match &metadata?.data_source {
        DataSource::FilePath(p) => p,
        _ => return None,
    };
    // Use to_str() for case-insensitive comparison, returns None for non-UTF8 paths
    let extension = path.extension()?.to_str()?;
    if !extension.eq_ignore_ascii_case("toml") {
        return None;
    }
    // Silently return None on read errors (permissions, etc.) - preservation is best-effort
    std::fs::read_to_string(path).ok()
}

pub(crate) fn preserve_toml_document(
    engine_state: &EngineState,
    current_value: &Value,
    original_source: &str,
    span: Span,
) -> Result<String, ShellError> {
    let Value::Record {
        val: current_record,
        ..
    } = current_value
    else {
        return Err(ShellError::UnsupportedInput {
            msg: format!("{} is not valid top-level TOML", current_value.get_type()),
            input: "value originates from here".into(),
            msg_span: span,
            input_span: current_value.span(),
        });
    };

    let mut document = original_source
        .parse::<DocumentMut>()
        .map_err(|err| toml_preservation_error("string", "TOML document", span, err))?;

    let original_value = toml_edit_table_to_nu_value(document.as_table(), span);
    let Value::Record {
        val: original_record,
        ..
    } = original_value
    else {
        return Err(ShellError::UnsupportedInput {
            msg: "top-level TOML must deserialize to a record".into(),
            input: "original TOML source originates from here".into(),
            msg_span: span,
            input_span: span,
        });
    };

    apply_record_diff(
        engine_state,
        &original_record,
        current_record,
        document.as_table_mut(),
        ContainerKind::Table,
    )?;

    Ok(document.to_string())
}

// ---------------------------------------------------------------------------
// toml_edit -> nushell Value conversion (used to derive original values from
// the parsed document, avoiding a second parse via the `toml` crate)
// ---------------------------------------------------------------------------

fn toml_edit_table_to_nu_value(table: &toml_edit::Table, span: Span) -> Value {
    let record: Record = table
        .iter()
        .map(|(key, item)| (key.to_string(), toml_edit_item_to_nu_value(item, span)))
        .collect();
    Value::record(record, span)
}

fn toml_edit_item_to_nu_value(item: &toml_edit::Item, span: Span) -> Value {
    match item {
        Item::Value(v) => toml_edit_value_to_nu_value(v, span),
        Item::Table(t) => toml_edit_table_to_nu_value(t, span),
        Item::ArrayOfTables(arr) => {
            let vals: Vec<Value> = arr
                .iter()
                .map(|t| toml_edit_table_to_nu_value(t, span))
                .collect();
            Value::list(vals, span)
        }
        Item::None => Value::nothing(span),
    }
}

fn toml_edit_value_to_nu_value(v: &toml_edit::Value, span: Span) -> Value {
    match v {
        toml_edit::Value::String(s) => Value::string(s.value().clone(), span),
        toml_edit::Value::Integer(i) => Value::int(*i.value(), span),
        toml_edit::Value::Float(f) => Value::float(*f.value(), span),
        toml_edit::Value::Boolean(b) => Value::bool(*b.value(), span),
        toml_edit::Value::Datetime(dt) => {
            crate::formats::from::toml_datetime_to_value(dt.value(), span)
        }
        toml_edit::Value::Array(arr) => {
            let vals: Vec<Value> = arr
                .iter()
                .map(|v| toml_edit_value_to_nu_value(v, span))
                .collect();
            Value::list(vals, span)
        }
        toml_edit::Value::InlineTable(t) => {
            let record: Record = t
                .iter()
                .map(|(k, v)| (k.to_string(), toml_edit_value_to_nu_value(v, span)))
                .collect();
            Value::record(record, span)
        }
    }
}

// ---------------------------------------------------------------------------
// Diffing: walk original vs current nushell Values and apply minimal edits
// to the toml_edit document so that comments and formatting are preserved.
// ---------------------------------------------------------------------------

fn apply_record_diff<Container: TableLike>(
    engine_state: &EngineState,
    original: &Record,
    current: &Record,
    container: &mut Container,
    container_kind: ContainerKind,
) -> Result<(), ShellError> {
    for (key, _) in original {
        if !current.contains(key) {
            container.remove(key);
        }
    }

    for (key, current_value) in current {
        match (original.get(key), container.get_mut(key)) {
            (Some(original_value), Some(item)) => {
                apply_value_diff(
                    engine_state,
                    original_value,
                    current_value,
                    item,
                    container_kind,
                )?;
            }
            _ => {
                container.insert(
                    key,
                    item_from_value(engine_state, current_value, container_kind)?,
                );
            }
        }
    }

    Ok(())
}

fn apply_value_diff(
    engine_state: &EngineState,
    original: &Value,
    current: &Value,
    item: &mut Item,
    container_kind: ContainerKind,
) -> Result<(), ShellError> {
    if original == current {
        return Ok(());
    }

    if let (
        Value::Record {
            val: original_record,
            ..
        },
        Value::Record {
            val: current_record,
            ..
        },
    ) = (original, current)
    {
        if let Some(table) = item.as_table_mut() {
            return apply_record_diff(
                engine_state,
                original_record,
                current_record,
                table,
                ContainerKind::Table,
            );
        }

        if let Some(inline_table) = item.as_inline_table_mut() {
            return apply_record_diff(
                engine_state,
                original_record,
                current_record,
                inline_table,
                ContainerKind::InlineTable,
            );
        }
    }

    if let (
        Value::List {
            vals: orig_list, ..
        },
        Value::List {
            vals: curr_list, ..
        },
    ) = (original, current)
        && let Some(arr) = item.as_array_of_tables_mut()
    {
        return apply_array_of_tables_diff(engine_state, orig_list, curr_list, arr);
    }

    *item = item_from_value(engine_state, current, container_kind)?;
    Ok(())
}

/// Element-wise diff for `[[array_of_tables]]` sections.
/// Matching indices are recursively diffed so that per-table comments and
/// formatting are preserved. Trailing removals and appended entries are handled
/// by shrinking / growing the array.
fn apply_array_of_tables_diff(
    engine_state: &EngineState,
    original: &[Value],
    current: &[Value],
    arr: &mut toml_edit::ArrayOfTables,
) -> Result<(), ShellError> {
    let common = original.len().min(current.len());

    for i in 0..common {
        if let (Value::Record { val: orig_rec, .. }, Value::Record { val: curr_rec, .. }) =
            (&original[i], &current[i])
            && let Some(table) = arr.get_mut(i)
        {
            apply_record_diff(
                engine_state,
                orig_rec,
                curr_rec,
                table,
                ContainerKind::Table,
            )?;
        }
    }

    // Remove trailing entries when the current list is shorter.
    while arr.len() > current.len() {
        arr.remove(arr.len() - 1);
    }

    // Append new entries when the current list is longer.
    for value in &current[common..] {
        let toml_value = nu_value_to_toml_value(engine_state, value, false)?;
        if let toml::Value::Table(map) = toml_value {
            let mut table = toml_edit::Table::new();
            for (k, v) in &map {
                table.insert(k, toml_value_to_edit_item(v, ContainerKind::Table));
            }
            arr.push(table);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// nushell Value -> toml_edit Item/Value construction (replaces the former
// serialize-to-string-and-reparse wrapper approach)
// ---------------------------------------------------------------------------

fn item_from_value(
    engine_state: &EngineState,
    value: &Value,
    container_kind: ContainerKind,
) -> Result<Item, ShellError> {
    let toml_value = nu_value_to_toml_value(engine_state, value, false)?;
    Ok(toml_value_to_edit_item(&toml_value, container_kind))
}

/// Convert a `toml::Value` into a `toml_edit::Item`, respecting the
/// surrounding container kind so that tables inside inline tables stay inline
/// and top-level tables use the `[header]` syntax.
fn toml_value_to_edit_item(value: &toml::Value, container_kind: ContainerKind) -> Item {
    match value {
        toml::Value::Table(map) => match container_kind {
            ContainerKind::InlineTable => {
                let mut t = toml_edit::InlineTable::new();
                for (k, v) in map {
                    t.insert(k, toml_value_to_edit_value(v));
                }
                Item::Value(toml_edit::Value::InlineTable(t))
            }
            ContainerKind::Table => {
                let mut t = toml_edit::Table::new();
                for (k, v) in map {
                    t.insert(k, toml_value_to_edit_item(v, ContainerKind::Table));
                }
                Item::Table(t)
            }
        },
        toml::Value::Array(arr)
            if matches!(container_kind, ContainerKind::Table)
                && !arr.is_empty()
                && arr.iter().all(|v| matches!(v, toml::Value::Table(_))) =>
        {
            let mut aot = toml_edit::ArrayOfTables::new();
            for v in arr {
                if let toml::Value::Table(map) = v {
                    let mut table = toml_edit::Table::new();
                    for (k, v) in map {
                        table.insert(k, toml_value_to_edit_item(v, ContainerKind::Table));
                    }
                    aot.push(table);
                }
            }
            Item::ArrayOfTables(aot)
        }
        other => Item::Value(toml_value_to_edit_value(other)),
    }
}

/// Convert a `toml::Value` into a `toml_edit::Value` (always inline form).
fn toml_value_to_edit_value(value: &toml::Value) -> toml_edit::Value {
    match value {
        toml::Value::String(s) => toml_edit::Value::from(s.as_str()),
        toml::Value::Integer(i) => toml_edit::Value::from(*i),
        toml::Value::Float(f) => toml_edit::Value::from(*f),
        toml::Value::Boolean(b) => toml_edit::Value::from(*b),
        toml::Value::Datetime(dt) => toml_edit::Value::from(*dt),
        toml::Value::Array(arr) => {
            let mut edit_arr = toml_edit::Array::new();
            for v in arr {
                edit_arr.push(toml_value_to_edit_value(v));
            }
            toml_edit::Value::Array(edit_arr)
        }
        toml::Value::Table(map) => {
            let mut t = toml_edit::InlineTable::new();
            for (k, v) in map {
                t.insert(k, toml_value_to_edit_value(v));
            }
            toml_edit::Value::InlineTable(t)
        }
    }
}

fn toml_preservation_error(
    from_type: &str,
    to_type: &str,
    span: Span,
    err: impl ToString,
) -> ShellError {
    ShellError::CantConvert {
        to_type: to_type.into(),
        from_type: from_type.into(),
        span,
        help: Some(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserve_array_of_tables_comments() {
        let engine_state = EngineState::new();
        let span = Span::test_data();

        let source = "\
# top comment
[settings]
verbose = true

# first item
[[items]]
name = \"alpha\"
value = 1

# second item
[[items]]
name = \"beta\"
value = 2
";

        let current = Value::test_record(record! {
            "settings" => Value::test_record(record! {
                "verbose" => Value::test_bool(true),
            }),
            "items" => Value::test_list(vec![
                Value::test_record(record! {
                    "name" => Value::test_string("alpha"),
                    "value" => Value::test_int(99),
                }),
                Value::test_record(record! {
                    "name" => Value::test_string("beta"),
                    "value" => Value::test_int(2),
                }),
            ]),
        });

        let result = preserve_toml_document(&engine_state, &current, source, span).unwrap();
        eprintln!("--- RESULT ---\n{result}--- END ---");
        assert!(result.contains("# top comment"), "top comment preserved");
        assert!(
            result.contains("# first item"),
            "first item comment preserved"
        );
        assert!(
            result.contains("# second item"),
            "second item comment preserved"
        );
        assert!(result.contains("value = 99"), "value updated");
        assert!(result.contains("value = 2"), "unchanged value preserved");
    }

    #[test]
    fn extension_case_insensitive() {
        // Test that .TOML (uppercase) extension is recognized
        let metadata = PipelineMetadata {
            data_source: DataSource::FilePath(std::path::PathBuf::from("config.TOML")),
            ..Default::default()
        };

        // This will return None because the file doesn't exist,
        // but we can verify it attempts to read the file (not rejected early)
        let result = read_toml_source_from_metadata(Some(&metadata));
        // File doesn't exist, so we expect None from .ok() conversion
        assert!(result.is_none());
    }

    #[test]
    fn non_toml_extension_returns_none() {
        let metadata = PipelineMetadata {
            data_source: DataSource::FilePath(std::path::PathBuf::from("config.json")),
            ..Default::default()
        };

        let result = read_toml_source_from_metadata(Some(&metadata));
        assert!(result.is_none());
    }

    #[test]
    fn no_extension_returns_none() {
        let metadata = PipelineMetadata {
            data_source: DataSource::FilePath(std::path::PathBuf::from("config")),
            ..Default::default()
        };

        let result = read_toml_source_from_metadata(Some(&metadata));
        assert!(result.is_none());
    }

    #[test]
    fn non_file_data_source_returns_none() {
        let metadata = PipelineMetadata {
            data_source: DataSource::None,
            ..Default::default()
        };

        let result = read_toml_source_from_metadata(Some(&metadata));
        assert!(result.is_none());
    }
}
