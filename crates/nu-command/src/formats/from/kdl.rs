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
        let span = Span::unknown();

        vec![
            // TODO: add examples
            Example {
                example: r#""node attr=1 attr2=#true {bloc}" | from kdl"#,
                description: "Converts kdl formatted string to table.",
                result: Some(Value::test_list(vec![
                    record! {
                        "name" => "node".into_value(span),
                        "entries" => vec![
                            record! {
                                "name" => "attr".into_value(span),
                                "value" => Value::int(1, span),
                            },
                            record! {
                                "name" => "attr2".into_value(span),
                                "value" => Value::bool(true, span),
                            },
                        ].into_value(span),
                        "children" => vec![
                            record! {
                                "name" => "bloc".into_value(span),
                                "entries" => Vec::<Value>::new().into_value(span),
                            }
                        ].into_value(span),
                    }
                    .into_value(span),
                ])),
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
        // the table format is [name, entries, children]
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
    use super::*;

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromKdl)
    }

    #[test]
    fn test_official_kdl_website_example() {
        let kdl_website_example = r#"
            package {
                name my-pkg
                    version "1.2.3"

                    dependencies {
                        // Nodes can have standalone values as well as
                        // key/value pairs.
                        lodash "^3.2.1" optional=#true alias=underscore
                    }

                scripts {
                    // "Raw" and dedented multi-line strings are supported.
                    message """
                        hello
                        world
                        """
                }

                // `\` breaks up a single node across multiple lines.
                the-matrix 1 2 3 \
                    4 5 6 \
                    7 8 9

                    // "Slashdash" comments operate at the node level,
                    // with just `/-`.
                    /-this-is-commented {
                        this entire node {
                            is gone
                        }
                    }
            }
            "#;

        let kdl_document =
            KdlDocument::parse(kdl_website_example).expect("fiald to parse kdl string");

        let span = Span::test_data();

        let mut output_table: Vec<Value> = Vec::new();
        inject_kdl_document_into_table_recursively(&mut output_table, &kdl_document, span)
            .expect("injecing kdl document data into table recursively fiald");

        assert_eq!(
            output_table[0]
                .clone()
                .into_value(span)
                .get_data_by_key("name")
                .unwrap(),
            Value::string("package", span)
        );

        assert_eq!(
            output_table[0]
                .clone()
                .into_value(span)
                .get_data_by_key("children")
                .unwrap()
                .as_list()
                .unwrap()[2]
                .clone()
                .into_value(span),
            Value::record(
                record! {
                    "name" => "dependencies".into_value(span),
                    "entries" => Vec::<Value>::new().into_value(span),
                    "children" => vec![
                        record! {
                            "name" => "lodash".into_value(span),
                            "entries" => vec![
                                record! {
                                    "name" => Value::nothing(span),
                                    "value" => Value::string("^3.2.1", span),
                                },
                                record! {
                                    "name" => "optional".into_value(span),
                                    "value" => Value::bool(true, span),
                                },
                                record! {
                                    "name" => "alias".into_value(span),
                                    "value" => Value::string("underscore", span),
                                },
                            ].into_value(span),
                        }
                    ].into_value(span),
                },
                span
            )
        );
        // TODO: add even more tests
    }
}
