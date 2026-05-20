/*
 * NOTE: read before modifying, the `convert_nu_value_to_kdl_value` function has a unreachable branch, make sure to not reachable it or the shell may panic insha'Allah
 *
 * */

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
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts table data into KDL text."
    }

    fn run(
        &self,
        _: &EngineState,
        _: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = input.span().unwrap_or(call.head);
        let metadata = input.take_metadata().map(|md| md.with_content_type(None));

        let output_string = match &input.into_value(span)? {
            Value::Record { val, .. } => {
                convert_record_into_formatted_kdl_document_recursively(val, span)?.to_string()
            }
            Value::List { vals, .. } => {
                let mut kdl_document = KdlDocument::new();
                let mut node = KdlNode::new("root");
                convert_list_into_entries_of_kdl_node_recursively(&mut node, vals, span)?;

                kdl_document.nodes_mut().push(node);
                kdl_document.autoformat();
                kdl_document.to_string()
            }
            val => convert_nu_value_to_kdl_value(val).to_string(),
        };

        Ok(output_string
            .into_value(span)
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
        ]
    }
}

fn convert_record_into_formatted_kdl_document_recursively(
    record: &Record,
    span: Span,
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
                    let value = convert_nu_value_to_kdl_value(v);

                    let entry = KdlEntry::new_prop(identifier, value);
                    node.push(entry);
                } else {
                    let _ = node.children_mut().insert(
                        convert_record_into_formatted_kdl_document_recursively(val, span)?,
                    );
                }
            }
            Value::List { vals, .. } => {
                convert_list_into_entries_of_kdl_node_recursively(&mut node, vals, span)?;
            }
            val => {
                node.push(convert_nu_value_to_kdl_value(val));
            }
        };

        kdl_document.nodes_mut().push(node);
    }

    // format the document before returning it
    kdl_document.autoformat();
    Ok(kdl_document)
}

fn convert_list_into_entries_of_kdl_node_recursively(
    node: &mut KdlNode,
    list: &[Value],
    span: Span,
) -> Result<(), ShellError> {
    let children = list
        .iter()
        .filter(|val| val.as_record().is_ok_and(|it| it.len() > 1));

    if children.clone().count() > 1 {
        return Err(ShellError::UnsupportedInput { msg: "can't have more than one child for each node in kdl, make sure input don't contain a node has a value of multiple records and more then one of these records has more then one item".to_owned(), input: format!("issue in node: '{}'", node.name().value()), msg_span: span, input_span: node.into_spanned(span).span });
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
                    let identifier =
                        KdlIdentifier::parse(k).map_err(|_| ShellError::NushellFailed {
                            msg: "failed to make an identifier for a kdl node".to_owned(),
                        })?;
                    let value = convert_nu_value_to_kdl_value(v);
                    let entry = KdlEntry::new_prop(identifier, value);
                    node.push(entry);
                } else {
                    let _ = node.children_mut().insert(
                        convert_record_into_formatted_kdl_document_recursively(val, span)?,
                    );
                }
            }
            Value::List { vals, .. } => {
                convert_list_into_entries_of_kdl_node_recursively(node, vals, span)?;
            }
            val => {
                node.push(convert_nu_value_to_kdl_value(val));
            }
        };
    }

    Ok(())
}

fn convert_nu_value_to_kdl_value(value: &Value) -> KdlValue {
    match value {
        Value::Bool { val, .. } => KdlValue::Bool(*val),
        Value::Int { val, .. } => KdlValue::Integer(*val as i128),
        Value::Float { val, .. } => KdlValue::Float(*val),
        Value::Filesize { val, .. } => KdlValue::String(val.to_string()),
        Value::Duration { val, .. } => KdlValue::String(val.to_string()),
        Value::Date { val, .. } => KdlValue::String(val.to_string()),
        Value::Range { val, .. } => KdlValue::String(val.to_string()),
        Value::String { val, .. } => KdlValue::String(val.clone()),
        Value::Glob { val, .. } => KdlValue::String(val.clone()),
        Value::Closure { val, .. } => KdlValue::String(format!("closure_{}", val.block_id.get())),
        Value::Nothing { .. } => KdlValue::Null,
        Value::Error { error, .. } => KdlValue::String(format!("{error:?}")),
        Value::Binary { val, .. } => KdlValue::String(format!("{val:?}")),
        Value::CellPath { val, .. } => KdlValue::String(val.to_string()),
        // If we fail to collapse the custom value, just print <{type_name}> - failure is not
        // that critical here
        Value::Custom { val, .. } => KdlValue::String(format!("<{}>", val.type_name())),
        // UNSAFE: i struct the code above to ensure insha'Allah that this is never reached but it's still danger because this project has many people work on it so this can be change.
        _ => unreachable!("can't convert record and list values to kdl"),
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
