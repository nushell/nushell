use nu_engine::command_prelude::*;

use kdl::{KdlDocument, KdlError, KdlNode, KdlValue};

#[derive(Debug)]
pub struct FromKdlError;

// TODO: make better error handling
impl FromKdlError {
    fn cant_convert(err: KdlError) -> ShellError {
        ShellError::CantConvert {
            to_type: "structured kdl data".into(),
            from_type: "string".into(),
            span: Span::unknown(),
            help: Some(err.to_string()),
        }
    }
}

#[derive(Clone)]
pub struct FromKdl;

impl Command for FromKdl {
    fn name(&self) -> &str {
        "from kdl"
    }

    fn description(&self) -> &str {
        "Convert KDL text into structured data."
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("from kdl")
            .input_output_types(vec![(Type::String, Type::Any)])
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            // TODO: add examples
            Example {
                example: r#"'{ "a": 1 }' | from json"#,
                description: "Converts json formatted string to table.",
                result: Some(Value::test_record(record! {
                    "a" => Value::test_int(1),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        _: &mut Stack,
        call: &Call,
        input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = call.head;

        let kdl_string_object = input.collect_string_strict(span)?;

        // parse the string into a KDL document
        let kdl_data =
            KdlDocument::parse(&kdl_string_object.0).map_err(FromKdlError::cant_convert)?;

        // make the output table to iject the data in
        // the table format is [name, attr, children]
        let mut output_table: Vec<Value> = Vec::new();

        inject_kdl_document_into_table_recursively(&mut output_table, &kdl_data, span)?;

        Ok(output_table.into_pipeline_data(span, engine_state.signals().clone()))
    }
}

// helpers
fn inject_kdl_document_into_table_recursively(
    output_table: &mut Vec<Value>,
    kdl_document: &KdlDocument,
    span: Span,
) -> Result<(), ShellError> {
    let nodes = kdl_document.nodes();

    for node in nodes {
        let mut row = Record::new();

        row.push("name", Value::string(node.name().value(), span));
        row.push(
            "entries",
            get_kdl_node_entries(node, span)?.into_value(span),
        );
        if let Some(children) = node.children() {
            let mut children_list: Vec<Value> = Vec::new();
            inject_kdl_document_into_table_recursively(&mut children_list, children, span)?;
            row.push("children", children_list.into_value(span));
        }

        output_table.push(row.into_value(span));
    }

    Ok(())
}

fn get_kdl_node_entries(kdl_node: &KdlNode, span: Span) -> Result<Vec<Value>, ShellError> {
    let mut output_table: Vec<Value> = Vec::new();

    for entry in kdl_node.entries() {
        let mut row = Record::new();
        if let Some(name) = entry.name() {
            row.push("name", Value::string(name.value(), span));
        } else {
            row.push("name", Value::nothing(span));
        }

        row.push("value", convert_kdl_value_to_nu_value(entry.value(), span));

        output_table.push(row.into_value(span));
    }

    Ok(output_table)
}

fn convert_kdl_value_to_nu_value(value: &KdlValue, span: Span) -> Value {
    match value {
        KdlValue::String(val) => Value::string(val, span),
        KdlValue::Integer(val) => Value::int(*val as i64, span),
        KdlValue::Float(val) => Value::float(*val, span),
        KdlValue::Bool(val) => Value::bool(*val, span),
        KdlValue::Null => Value::nothing(span),
    }
}

#[cfg(test)]
mod test {
    // use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        // TODO: add tests
        todo!("add tests")
    }
}
