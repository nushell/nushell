use crate::formats::{KDL_CANONICAL_METADATA_KEY, KDL_CANONICAL_METADATA_VALUE};
use kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue};
use nu_engine::command_prelude::*;
use nu_protocol::PipelineMetadata;

#[derive(Clone)]
pub struct ToKdl;

impl Command for ToKdl {
    fn name(&self) -> &str {
        "to kdl"
    }

    fn signature(&self) -> Signature {
        Signature::build("to kdl")
            .input_output_types(vec![(Type::Any, Type::String)])
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized.",
                Some('s'),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts table data into KDL text."
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let call_span = input.span().unwrap_or(call.head);
        let mut metadata = input.take_metadata().unwrap_or_default();
        // Consume the explicit round-trip marker written by `from kdl`.
        // We require this marker to opt into canonical node-row interpretation.
        let canonical_node_rows = metadata_marks_canonical_kdl_rows(&metadata);
        // Remove the internal marker from outgoing metadata so it does not leak
        // beyond this conversion boundary.
        metadata.custom.remove(KDL_CANONICAL_METADATA_KEY);
        let metadata = metadata.with_content_type(Some("application/x-kdl".to_owned()));

        // get args
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        let value = input.into_value(call_span)?;
        let output_string = value_to_kdl_document(
            engine_state,
            &value,
            canonical_node_rows,
            serialize_types,
            call_span,
        )?
        .to_string();

        Ok(output_string
            .into_value(call_span)
            .into_pipeline_data_with_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert yaml file to kdl file",
                example: "{this: that list: [1 2 3 {bool: true} {this: should be: a-block}]} | to yaml | from yaml | to kdl",
                result: Some(Value::test_string(
                    "this that\nlist 1 2 3 bool=#true {\n    this should\n    be a-block\n}\n",
                )),
            },
            Example {
                description: "Convert nu record to kdl",
                example: "{one: [{one: two, 1: 2} {three: 3} [1 2 3] 4 5 6 {bool: true}] } | to kdl",
                result: Some(Value::test_string(
                    "one three=3 1 2 3 4 5 6 bool=#true {\n    one two\n    \"1\" 2\n}\n",
                )),
            },
            Example {
                description: "Convert nu list to kdl string",
                example: "[1 2 3] | to kdl",
                result: Some(Value::test_string("root 1 2 3\n")),
            },
            Example {
                description: "Convert nu closure to kdl string",
                example: "{2: {|| 1 + 1} } | to kdl --serialize",
                result: Some(Value::test_string("\"2\" \"{|| 1 + 1}\"\n")),
            },
            Example {
                description: "Round-trip KDL through canonical node rows.",
                example: "'node one; node two' | from kdl | to kdl",
                result: Some(Value::test_string("node one\nnode two\n")),
            },
        ]
    }
}

fn metadata_marks_canonical_kdl_rows(metadata: &PipelineMetadata) -> bool {
    // Canonical KDL mode is opt-in and carried via pipeline metadata from
    // `from kdl`, so ordinary records with `name/args/props/children` keys
    // are not reinterpreted accidentally.
    matches!(
        metadata.custom.get(KDL_CANONICAL_METADATA_KEY),
        Some(Value::String { val, .. }) if val == KDL_CANONICAL_METADATA_VALUE
    )
}

fn value_is_canonical_node_row(value: &Value) -> bool {
    let Value::Record { val, .. } = value else {
        return false;
    };

    record_is_canonical_node_row(val)
}

fn record_is_canonical_node_row(record: &Record) -> bool {
    record.len() == 4
        && matches!(record.get("name"), Some(Value::String { .. }))
        && record
            .get("args")
            .is_some_and(|value| value.as_list().is_ok())
        && record
            .get("props")
            .is_some_and(|value| value.as_record().is_ok())
        && record
            .get("children")
            .is_some_and(|value| value.as_list().is_ok())
}

fn list_is_canonical_node_rows(rows: &[Value]) -> bool {
    rows.iter().all(value_is_canonical_node_row)
}

fn convert_record_into_formatted_kdl_document_recursively(
    engine_state: &EngineState,
    record: &Record,
    serialize_types: bool,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    let mut kdl_document = KdlDocument::new();

    for (key, value) in record.iter() {
        let mut node = KdlNode::new(identifier_for_key(key));
        append_value_to_kdl_node(engine_state, &mut node, value, serialize_types, call_span)?;

        kdl_document.nodes_mut().push(node);
    }

    // format the document before returning it
    kdl_document.autoformat();
    Ok(kdl_document)
}

fn convert_list_into_entries_of_kdl_node_recursively(
    engine_state: &EngineState,
    node: &mut KdlNode,
    list: &[Value],
    serialize_types: bool,
    call_span: Span,
) -> Result<(), ShellError> {
    for value in list {
        match value {
            Value::Record { val, .. } => {
                append_record_to_kdl_node(engine_state, node, val, serialize_types, call_span)?;
            }
            Value::List { vals, .. } => {
                convert_list_into_entries_of_kdl_node_recursively(
                    engine_state,
                    node,
                    vals,
                    serialize_types,
                    call_span,
                )?;
            }
            val => {
                ensure_closure_is_serializable(val, serialize_types, call_span)?;
                node.push(convert_nu_value_to_kdl_value(engine_state, call_span, val)?);
            }
        };
    }

    Ok(())
}

fn value_to_kdl_document(
    engine_state: &EngineState,
    value: &Value,
    canonical_node_rows: bool,
    serialize_types: bool,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    match value {
        // Only enter canonical mode when both metadata and value shape agree.
        // This guards against stale metadata after downstream transformations.
        Value::Record { val, .. } if canonical_node_rows && record_is_canonical_node_row(val) => {
            canonical_node_rows_to_kdl_document(
                std::slice::from_ref(value),
                engine_state,
                serialize_types,
                call_span,
            )
        }
        Value::Record { val, .. } => convert_record_into_formatted_kdl_document_recursively(
            engine_state,
            val,
            serialize_types,
            call_span,
        ),
        Value::List { vals, .. } if canonical_node_rows && list_is_canonical_node_rows(vals) => {
            canonical_node_rows_to_kdl_document(vals, engine_state, serialize_types, call_span)
        }
        Value::List { vals, .. } => {
            let mut kdl_document = KdlDocument::new();
            let mut node = KdlNode::new("root");
            convert_list_into_entries_of_kdl_node_recursively(
                engine_state,
                &mut node,
                vals,
                serialize_types,
                call_span,
            )?;
            kdl_document.nodes_mut().push(node);
            kdl_document.autoformat();
            Ok(kdl_document)
        }
        val => {
            ensure_closure_is_serializable(val, serialize_types, call_span)?;
            let mut kdl_document = KdlDocument::new();
            let mut node = KdlNode::new("root");
            node.push(convert_nu_value_to_kdl_value(engine_state, call_span, val)?);
            kdl_document.nodes_mut().push(node);
            kdl_document.autoformat();
            Ok(kdl_document)
        }
    }
}

fn canonical_node_rows_to_kdl_document(
    rows: &[Value],
    engine_state: &EngineState,
    serialize_types: bool,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    let mut kdl_document = KdlDocument::new();

    for row in rows {
        let Value::Record { val, .. } = row else {
            return Err(ShellError::UnsupportedInput {
                msg: "canonical node rows must be records".into(),
                input: "value originates from here".into(),
                msg_span: call_span,
                input_span: row.span(),
            });
        };

        kdl_document
            .nodes_mut()
            .push(convert_canonical_node_row_to_kdl_node(
                engine_state,
                val,
                serialize_types,
                call_span,
            )?);
    }

    kdl_document.autoformat();
    Ok(kdl_document)
}

fn convert_canonical_node_row_to_kdl_node(
    engine_state: &EngineState,
    row: &Record,
    serialize_types: bool,
    call_span: Span,
) -> Result<KdlNode, ShellError> {
    let name = node_row_string_field(row, "name", call_span)?;
    let args = node_row_list_field(row, "args", call_span)?;
    let props = node_row_record_field(row, "props", call_span)?;
    let children = node_row_list_field(row, "children", call_span)?;

    let mut node = KdlNode::new(identifier_for_key(name));

    for arg in args {
        ensure_closure_is_serializable(arg, serialize_types, call_span)?;
        node.push(convert_nu_value_to_kdl_value(engine_state, call_span, arg)?);
    }

    for (key, value) in props.iter() {
        ensure_closure_is_serializable(value, serialize_types, call_span)?;
        node.push(KdlEntry::new_prop(
            identifier_for_key(key),
            convert_nu_value_to_kdl_value(engine_state, call_span, value)?,
        ));
    }

    if !children.is_empty() {
        let child_document = canonical_node_rows_to_kdl_document(
            children,
            engine_state,
            serialize_types,
            call_span,
        )?;
        merge_children(&mut node, child_document, call_span, children[0].span())?;
    }

    Ok(node)
}

fn node_row_string_field<'a>(
    row: &'a Record,
    field: &str,
    call_span: Span,
) -> Result<&'a str, ShellError> {
    let value = node_row_field(row, field, call_span)?;
    value.as_str().map_err(|_| ShellError::UnsupportedInput {
        msg: format!("canonical node row field '{field}' must be a string"),
        input: "value originates from here".into(),
        msg_span: call_span,
        input_span: value.span(),
    })
}

fn node_row_list_field<'a>(
    row: &'a Record,
    field: &str,
    call_span: Span,
) -> Result<&'a [Value], ShellError> {
    let value = node_row_field(row, field, call_span)?;
    value.as_list().map_err(|_| ShellError::UnsupportedInput {
        msg: format!("canonical node row field '{field}' must be a list"),
        input: "value originates from here".into(),
        msg_span: call_span,
        input_span: value.span(),
    })
}

fn node_row_record_field<'a>(
    row: &'a Record,
    field: &str,
    call_span: Span,
) -> Result<&'a Record, ShellError> {
    let value = node_row_field(row, field, call_span)?;
    value.as_record().map_err(|_| ShellError::UnsupportedInput {
        msg: format!("canonical node row field '{field}' must be a record"),
        input: "value originates from here".into(),
        msg_span: call_span,
        input_span: value.span(),
    })
}

fn node_row_field<'a>(
    row: &'a Record,
    field: &str,
    call_span: Span,
) -> Result<&'a Value, ShellError> {
    row.get(field).ok_or_else(|| ShellError::UnsupportedInput {
        msg: format!("canonical node row is missing '{field}' field"),
        input: "value originates from here".into(),
        msg_span: call_span,
        input_span: call_span,
    })
}

fn append_value_to_kdl_node(
    engine_state: &EngineState,
    node: &mut KdlNode,
    value: &Value,
    serialize_types: bool,
    call_span: Span,
) -> Result<(), ShellError> {
    match value {
        Value::Record { val, .. } => {
            append_record_to_kdl_node(engine_state, node, val, serialize_types, call_span)
        }
        Value::List { vals, .. } => convert_list_into_entries_of_kdl_node_recursively(
            engine_state,
            node,
            vals,
            serialize_types,
            call_span,
        ),
        val => {
            ensure_closure_is_serializable(val, serialize_types, call_span)?;
            node.push(convert_nu_value_to_kdl_value(engine_state, call_span, val)?);
            Ok(())
        }
    }
}

fn append_record_to_kdl_node(
    engine_state: &EngineState,
    node: &mut KdlNode,
    record: &Record,
    serialize_types: bool,
    call_span: Span,
) -> Result<(), ShellError> {
    let entries = record.iter().collect::<Vec<_>>();

    if let [(key, value)] = entries.as_slice()
        && value.as_record().is_err()
        && value.as_list().is_err()
    {
        ensure_closure_is_serializable(value, serialize_types, call_span)?;
        node.push(KdlEntry::new_prop(
            identifier_for_key(key),
            convert_nu_value_to_kdl_value(engine_state, call_span, value)?,
        ));
        return Ok(());
    }

    let children = convert_record_into_formatted_kdl_document_recursively(
        engine_state,
        record,
        serialize_types,
        call_span,
    )?;
    merge_children(
        node,
        children,
        call_span,
        entries.first().map_or(call_span, |(_, value)| value.span()),
    )
}

fn merge_children(
    node: &mut KdlNode,
    mut children: KdlDocument,
    _call_span: Span,
    _input_span: Span,
) -> Result<(), ShellError> {
    node.ensure_children()
        .nodes_mut()
        .append(children.nodes_mut());
    Ok(())
}

fn identifier_for_key(key: &str) -> KdlIdentifier {
    let mut identifier = KdlIdentifier::from(key.to_owned());
    identifier.clear_format();
    identifier
}

fn ensure_closure_is_serializable(
    value: &Value,
    serialize_types: bool,
    call_span: Span,
) -> Result<(), ShellError> {
    if value.as_closure().is_ok() && !serialize_types {
        return Err(ShellError::UnsupportedInput {
            msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: value.span(),
        });
    }

    Ok(())
}

fn convert_nu_value_to_kdl_value(
    engine_state: &EngineState,
    span: Span,
    value: &Value,
) -> Result<KdlValue, ShellError> {
    match value {
        Value::Bool { val, .. } => Ok(KdlValue::Bool(*val)),
        Value::Int { val, .. } => Ok(KdlValue::Integer(*val as i128)),
        Value::Float { val, .. } => Ok(KdlValue::Float(*val)),
        Value::Filesize { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::Duration { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::Date { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::Range { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::String { val, .. } => Ok(KdlValue::String(val.clone())),
        Value::Glob { val, .. } => Ok(KdlValue::String(val.clone())),
        Value::Closure { val, .. } => Ok(KdlValue::String(
            val.coerce_into_string(engine_state, span)?.to_string(),
        )),
        Value::Nothing { .. } => Ok(KdlValue::Null),
        Value::Binary { val, .. } => Ok(KdlValue::String(format!("{val:?}"))),
        Value::CellPath { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::SemVer { val, .. } => Ok(KdlValue::String(val.to_string())),
        Value::Custom { val, .. } => Ok(KdlValue::String(format!("<{}>", val.type_name()))),
        Value::Error { error, .. } => Err(*(error.clone())),
        _ => Err(ShellError::UnsupportedInput {
            msg: "value cannot be stringified".to_owned(),
            input: value.get_type().to_string(),
            msg_span: span,
            input_span: value.span(),
        }),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{Get, Metadata};
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToKdl)
    }

    #[test]
    fn top_level_scalars_are_wrapped_in_root_node() {
        let engine_state = EngineState::default();
        let result = value_to_kdl_document(
            &engine_state,
            &Value::test_int(5),
            false,
            false,
            Span::test_data(),
        )
        .expect("scalar document should serialize");

        assert_eq!(result.to_string(), "root 5\n");
    }

    #[test]
    fn canonical_node_rows_round_trip_to_document_shape() {
        let engine_state = EngineState::default();
        let span = Span::test_data();
        let rows = Value::test_list(vec![Value::test_record(record! {
            "name" => Value::string("item", span),
            "args" => Value::test_list(vec![Value::int(1, span)]),
            "props" => Value::test_record(record! { "enabled" => Value::bool(true, span) }),
            "children" => Value::test_list(vec![]),
        })]);

        let result = value_to_kdl_document(&engine_state, &rows, true, false, span)
            .expect("canonical rows should serialize");

        assert_eq!(result.to_string(), "item 1 enabled=#true\n");
    }

    #[test]
    fn list_conversion_merges_multiple_child_blocks() {
        let engine_state = EngineState::default();
        let span = Span::test_data();
        let value = Value::test_list(vec![
            Value::test_record(
                record! { "a" => Value::test_record(record! { "b" => Value::int(1, span) }) },
            ),
            Value::test_record(
                record! { "c" => Value::test_record(record! { "d" => Value::int(2, span) }) },
            ),
        ]);

        let document = value_to_kdl_document(&engine_state, &value, false, false, span)
            .expect("multiple child blocks should merge");

        assert_eq!(
            document.to_string(),
            "root {
    a b=1
    c d=2
}
"
        );
    }

    #[test]
    fn shape_matching_record_is_not_treated_as_canonical_without_metadata() {
        let engine_state = EngineState::default();
        let span = Span::test_data();
        let value = Value::test_record(record! {
            "name" => Value::string("item", span),
            "args" => Value::test_list(vec![]),
            "props" => Value::test_record(record! {}),
            "children" => Value::test_list(vec![]),
        });

        let document = value_to_kdl_document(&engine_state, &value, false, false, span)
            .expect("plain records should use normal record serialization");

        assert_eq!(
            document.to_string(),
            "name item\nargs\nprops {\n}\nchildren\n"
        );
    }

    #[test]
    fn metadata_marker_enables_canonical_row_serialization() {
        let span = Span::test_data();
        let mut metadata = PipelineMetadata::default();
        // `from kdl` writes this key/value pair to mark that values are
        // canonical KDL node rows and should be interpreted in round-trip mode.
        metadata.custom.insert(
            KDL_CANONICAL_METADATA_KEY,
            Value::string(KDL_CANONICAL_METADATA_VALUE, span),
        );

        assert!(metadata_marks_canonical_kdl_rows(&metadata));
    }

    #[test]
    fn from_kdl_marker_flows_to_to_kdl_command() {
        let mut engine_state = Box::new(EngineState::new());
        let delta = {
            let mut working_set = StateWorkingSet::new(&engine_state);

            working_set.add_decl(Box::new(crate::formats::FromKdl));
            working_set.add_decl(Box::new(ToKdl));
            working_set.add_decl(Box::new(Metadata {}));
            working_set.add_decl(Box::new(Get {}));

            working_set.render()
        };

        engine_state
            .merge_delta(delta)
            .expect("error merging delta");

        let cmd = "'node one; node two' | from kdl | to kdl | metadata | get content_type | $in";
        let result = eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        )
        .expect("pipeline should succeed");

        assert_eq!(result, Value::test_string("application/x-kdl"));
    }

    #[test]
    fn canonical_metadata_constants_define_round_trip_contract() {
        let span = Span::test_data();
        let mut metadata = PipelineMetadata::default();

        metadata.custom.insert(
            KDL_CANONICAL_METADATA_KEY,
            Value::string(KDL_CANONICAL_METADATA_VALUE, span),
        );

        assert!(metadata_marks_canonical_kdl_rows(&metadata));

        metadata.custom.insert(
            KDL_CANONICAL_METADATA_KEY,
            Value::string("wrong_version", span),
        );

        assert!(!metadata_marks_canonical_kdl_rows(&metadata));
    }

    #[test]
    fn stale_canonical_metadata_falls_back_to_regular_record_serialization() {
        let engine_state = EngineState::default();
        let span = Span::test_data();
        let value = Value::test_record(record! {
            "plain" => Value::int(7, span),
        });

        let document = value_to_kdl_document(&engine_state, &value, true, false, span)
            .expect("stale canonical marker should not force canonical conversion");

        assert_eq!(document.to_string(), "plain 7\n");
    }

    #[test]
    fn stale_canonical_metadata_falls_back_to_regular_list_serialization() {
        let engine_state = EngineState::default();
        let span = Span::test_data();
        let value = Value::test_list(vec![Value::test_record(record! {
            "plain" => Value::int(7, span),
        })]);

        let document = value_to_kdl_document(&engine_state, &value, true, false, span)
            .expect("stale canonical marker should not force canonical conversion");

        assert_eq!(document.to_string(), "root plain=7\n");
    }
}
