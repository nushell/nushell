use nu_engine::command_prelude::*;

use kdl::{KdlDocument, KdlNode, KdlValue};

#[derive(Debug)]
pub struct FromKdlError;

impl FromKdlError {
    fn cant_convert(span: Span, help: Option<String>) -> ShellError {
        ShellError::CantConvert {
            to_type: "structured kdl data".into(),
            from_type: "string".into(),
            span,
            help,
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
            Example {
                example: r#""node attr=1 attr2=#true {bloc}" | from kdl"#,
                description: "Converts kdl formatted string to record.",
                result: Some(Value::test_record(record! {
                    "node" => vec![
                        record! {
                            "attr" => 1.into_value(span),
                        },
                        record! {
                            "attr2" => true.into_value(span),
                        },
                        record! {
                            "bloc" => Value::nothing(span),
                        }
                    ].into_value(span),
                })),
            },
            Example {
                description: "Converts kdl formatted string to record.",
                example: r#"'package { name nu; version 0.1; description "new type of shell" }' | from kdl"#,
                result: Some(Value::test_record(record! {
                    "package" => record! {
                        "name" => Value::string("nu", span),
                        "version" => Value::float(0.1, span),
                        "description" => Value::string("new type of shell", span),
                    }
                    .into_value(span),
                })),
            },
        ]
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

        let kdl_string_object = input.collect_string_strict(span)?;

        // parse the string into a KDL document
        let kdl_data = KdlDocument::parse(&kdl_string_object.0).map_err(|_| {
            FromKdlError::cant_convert(
                span,
                Some("failed to parse kdl string - check the input string syntax".to_owned()),
            )
        })?;

        // make the output record to inject the data in
        let mut output_record = Record::new();

        inject_kdl_document_into_record_recursively(&mut output_record, &kdl_data, span)?;

        Ok(output_record
            .into_value(span)
            .into_pipeline_data_with_metadata(metadata))
    }
}

fn inject_kdl_document_into_record_recursively(
    output_record: &mut Record,
    kdl_document: &KdlDocument,
    span: Span,
) -> Result<(), ShellError> {
    let nodes = kdl_document.nodes();

    for node in nodes {
        let entries = get_kdl_node_entries(node, span)?;

        let mut value: Value;

        if !entries.is_empty() {
            if let Some(children) = node.children() {
                let mut children_record = Record::new();
                inject_kdl_document_into_record_recursively(&mut children_record, children, span)?;

                value = entries.into_value(span);
                let mut list = value.as_list()?.to_vec();
                list.push(children_record.into_value(span));
                value = Value::list(list, span);
            } else if entries.len() == 1 {
                value = entries
                    .first()
                    .ok_or(ShellError::NushellFailed {
                        msg: "entries is empty".to_owned(),
                    })?
                    .clone();
            } else {
                value = entries.into_value(span);
            }
        } else if let Some(children) = node.children() {
            let mut children_record = Record::new();
            inject_kdl_document_into_record_recursively(&mut children_record, children, span)?;
            value = children_record.into_value(span);
        } else {
            value = Value::nothing(span);
        }

        output_record.insert(node.name().value().to_string(), value);
    }

    Ok(())
}

fn get_kdl_node_entries(kdl_node: &KdlNode, span: Span) -> Result<Vec<Value>, ShellError> {
    let mut output_list: Vec<Value> = Vec::new();

    for entry in kdl_node.entries() {
        if let Some(name) = entry.name() {
            let mut row = Record::new();

            row.insert(
                name.value().to_string(),
                convert_kdl_value_to_nu_value(entry.value(), span),
            );

            output_list.push(row.into_value(span));
            continue;
        }

        output_list.push(convert_kdl_value_to_nu_value(entry.value(), span).into_value(span));
    }

    Ok(output_list)
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
        const KDL_WEBSITE_EXAMPLE: &str = r#"
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
        }"#;

        let kdl_document =
            KdlDocument::parse(KDL_WEBSITE_EXAMPLE).expect("failed to parse kdl string");

        let span = Span::test_data();

        let mut output_record = Record::new();
        inject_kdl_document_into_record_recursively(&mut output_record, &kdl_document, span)
            .expect("injecing kdl document data into record recursively failed");

        assert_eq!(
            output_record
                .get("package")
                .unwrap()
                .as_record()
                .unwrap()
                .get("the-matrix")
                .unwrap()
                .as_list()
                .unwrap()
                .first()
                .unwrap()
                .clone()
                .into_value(span),
            Value::int(1, span)
        );

        assert_eq!(
            output_record
                .get("package")
                .unwrap()
                .as_record()
                .unwrap()
                .get("scripts")
                .unwrap()
                .as_record()
                .unwrap()
                .get("message")
                .unwrap()
                .clone()
                .into_value(span),
            Value::string("hello\nworld", span)
        );
    }
}
