use kdl::{KdlDocument, KdlEntry, KdlIdentifier, KdlNode, KdlValue};
use nu_engine::command_prelude::*;

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
        let metadata = input
            .take_metadata()
            .unwrap_or_default()
            .with_content_type(Some("application/x-kdl".to_owned()));

        // get args
        let serialize_types = call.has_flag(engine_state, stack, "serialize")?;

        let output_string = match &input.into_value(call_span)? {
            Value::Record { val, .. } => convert_record_into_formatted_kdl_document_recursively(
                engine_state,
                val,
                serialize_types,
                call_span,
            )?
            .to_string(),
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
                kdl_document.to_string()
            }
            val => {
                if val.as_closure().is_ok() && !serialize_types {
                    return Err(errors::should_use_serialize(call_span, val.span()));
                }
                convert_nu_value_to_kdl_value(engine_state, call_span, val)?.to_string()
            }
        };

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
        ]
    }
}

fn convert_record_into_formatted_kdl_document_recursively(
    engine_state: &EngineState,
    record: &Record,
    serialize_types: bool,
    call_span: Span,
) -> Result<KdlDocument, ShellError> {
    let mut kdl_document = KdlDocument::new();

    for (key, value) in record.iter() {
        let mut node = KdlNode::new(key.clone());

        match value {
            Value::Record { val, .. } => {
                let val_vec = val.iter().collect::<Vec<_>>();
                if val_vec.len() == 1
                    && val_vec[0].1.as_record().is_err()
                    && val_vec[0].1.as_list().is_err()
                {
                    let (k, v) = val_vec[0];
                    let identifier =
                        KdlIdentifier::parse(k).map_err(|_| ShellError::NushellFailed {
                            msg: "failed to make an identifier for a kdl node".to_owned(),
                        })?;

                    if v.as_closure().is_ok() && !serialize_types {
                        return Err(errors::should_use_serialize(call_span, v.span()));
                    }

                    let value = convert_nu_value_to_kdl_value(engine_state, call_span, v)?;

                    let entry = KdlEntry::new_prop(identifier, value);
                    node.push(entry);
                } else {
                    let _ = node.children_mut().insert(
                        convert_record_into_formatted_kdl_document_recursively(
                            engine_state,
                            val,
                            serialize_types,
                            call_span,
                        )?,
                    );
                }
            }
            Value::List { vals, .. } => {
                convert_list_into_entries_of_kdl_node_recursively(
                    engine_state,
                    &mut node,
                    vals,
                    serialize_types,
                    call_span,
                )?;
            }
            val => {
                if val.as_closure().is_ok() && !serialize_types {
                    return Err(errors::should_use_serialize(call_span, val.span()));
                }
                node.push(convert_nu_value_to_kdl_value(engine_state, call_span, val)?);
            }
        };

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
    let children = list
        .iter()
        .filter(|val| val.as_record().is_ok_and(|it| it.len() > 1));

    if children.clone().count() > 1 {
        return Err(errors::cant_have_more_than_one_child_for_each_node_in_kdl(
            node,
            call_span,
            list.first().expect("").span(),
        ));
    };

    for value in list {
        match value {
            Value::Record { val, .. } => {
                let val_vec = val.iter().collect::<Vec<_>>();
                if val_vec.len() == 1
                    && val_vec[0].1.as_record().is_err()
                    && val_vec[0].1.as_list().is_err()
                {
                    let (k, v) = val_vec[0];
                    let identifier = KdlIdentifier::parse(k).map_err(|_| {
                        errors::nushell_failed("failed to make an identifier for a kdl node")
                    })?;

                    if v.as_closure().is_ok() && !serialize_types {
                        return Err(errors::should_use_serialize(call_span, v.span()));
                    }

                    let value = convert_nu_value_to_kdl_value(engine_state, call_span, v)?;
                    let entry = KdlEntry::new_prop(identifier, value);
                    node.push(entry);
                } else {
                    let _ = node.children_mut().insert(
                        convert_record_into_formatted_kdl_document_recursively(
                            engine_state,
                            val,
                            serialize_types,
                            call_span,
                        )?,
                    );
                }
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
                if val.as_closure().is_ok() && !serialize_types {
                    return Err(errors::should_use_serialize(call_span, val.span()));
                }
                node.push(convert_nu_value_to_kdl_value(engine_state, call_span, val)?);
            }
        };
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
        Value::Custom { val, .. } => Ok(KdlValue::String(format!("<{}>", val.type_name()))),
        Value::Error { error, .. } => Err(*(error.clone())),
        _ => Err(ShellError::NushellFailed {
            msg: "Can't stringify record and list values".to_owned(),
        }),
    }
}

mod errors {
    use super::*;

    pub fn should_use_serialize(call_span: Span, value_span: Span) -> ShellError {
        ShellError::UnsupportedInput {
            msg: "closures are currently not deserializable (use --serialize to serialize as a string)".into(),
            input: "value originates from here".into(),
            msg_span: call_span,
            input_span: value_span,
        }
    }

    pub fn cant_have_more_than_one_child_for_each_node_in_kdl(
        node: &KdlNode,
        call_span: Span,
        first_span: Span,
    ) -> ShellError {
        ShellError::UnsupportedInput {
            msg: "a node can't have multiple child records with more than one field each"
                .to_owned(),
            input: format!("issue in node: '{}'", node.name().value()),
            msg_span: call_span,
            input_span: first_span,
        }
    }

    pub fn nushell_failed(msg: &str) -> ShellError {
        ShellError::NushellFailed {
            msg: msg.to_owned(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToKdl)
    }
}
