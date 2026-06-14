use crate::formats::{KDL_CANONICAL_METADATA_KEY, KDL_CANONICAL_METADATA_VALUE};
use kdl::{KdlDocument, KdlError, KdlNode, KdlValue};
use nu_engine::command_prelude::*;
use nu_protocol::{
    DEFAULT_ERROR_CONTEXT, shell_error::generic::GenericError, truncated_source_window,
};
use num_traits::ToPrimitive;

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
                description: "Converts KDL formatted string to canonical node rows.",
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("node", span),
                    "args" => Value::test_list(vec![]),
                    "props" => Value::test_record(record! {
                        "attr" => 1.into_value(span),
                        "attr2" => true.into_value(span),
                    }),
                    "children" => Value::test_list(vec![Value::test_record(record! {
                        "name" => Value::string("bloc", span),
                        "args" => Value::test_list(vec![]),
                        "props" => Value::test_record(record! {}),
                        "children" => Value::test_list(vec![]),
                    })]),
                })])),
            },
            Example {
                description: "Converts KDL formatted string to canonical node rows.",
                example: r#"'package { name nu; version 0.1; description "new type of shell" }' | from kdl"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("package", span),
                    "args" => Value::test_list(vec![]),
                    "props" => Value::test_record(record! {}),
                    "children" => Value::test_list(vec![
                        Value::test_record(record! {
                            "name" => Value::string("name", span),
                            "args" => Value::test_list(vec![Value::string("nu", span)]),
                            "props" => Value::test_record(record! {}),
                            "children" => Value::test_list(vec![]),
                        }),
                        Value::test_record(record! {
                            "name" => Value::string("version", span),
                            "args" => Value::test_list(vec![Value::float(0.1, span)]),
                            "props" => Value::test_record(record! {}),
                            "children" => Value::test_list(vec![]),
                        }),
                        Value::test_record(record! {
                            "name" => Value::string("description", span),
                            "args" => Value::test_list(vec![Value::string("new type of shell", span)]),
                            "props" => Value::test_record(record! {}),
                            "children" => Value::test_list(vec![]),
                        }),
                    ]),
                })])),
            },
            Example {
                description: "Duplicate sibling node names are preserved in-order.",
                example: r#""node one; node two" | from kdl"#,
                result: Some(Value::test_list(vec![
                    Value::test_record(record! {
                        "name" => Value::string("node", span),
                        "args" => Value::test_list(vec![Value::string("one", span)]),
                        "props" => Value::test_record(record! {}),
                        "children" => Value::test_list(vec![]),
                    }),
                    Value::test_record(record! {
                        "name" => Value::string("node", span),
                        "args" => Value::test_list(vec![Value::string("two", span)]),
                        "props" => Value::test_record(record! {}),
                        "children" => Value::test_list(vec![]),
                    }),
                ])),
            },
        ]
    }

    fn run(
        &self,
        _engine: &EngineState,
        _stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = input.span().unwrap_or(call.head);
        let mut metadata = input
            .take_metadata()
            .unwrap_or_default()
            .with_content_type(None);

        let kdl_string_object = input.collect_string_strict(span)?;

        let kdl_data = parse_kdl_document_with_diagnostics(&kdl_string_object.0, span)?;
        let rows = convert_kdl_document_to_node_rows(kdl_data.nodes(), span)?;

        // Mark this output as canonical KDL node rows so `to kdl` can round-trip
        // without guessing by shape and accidentally reinterpreting normal records.
        metadata.custom.insert(
            KDL_CANONICAL_METADATA_KEY,
            Value::string(KDL_CANONICAL_METADATA_VALUE, span),
        );

        Ok(Value::list(rows, span).into_pipeline_data_with_metadata(Some(metadata)))
    }
}

fn parse_kdl_document_with_diagnostics(input: &str, span: Span) -> Result<KdlDocument, ShellError> {
    KdlDocument::parse(input).map_err(|err| kdl_error_to_shell_error(input, span, &err))
}

fn kdl_error_to_shell_error(input: &str, span: Span, err: &KdlError) -> ShellError {
    if let Some(diagnostic) = err.diagnostics.first() {
        let diagnostic_message = kdl_diagnostics_message(&err.diagnostics);
        let byte_offset = diagnostic.span.offset();
        let (src, label_span) = truncated_source_window(
            input,
            Span::new(byte_offset, byte_offset),
            DEFAULT_ERROR_CONTEXT,
        );

        return ShellError::Generic(
            GenericError::new(
                "Error while parsing KDL text",
                "error parsing KDL text",
                span,
            )
            .with_inner([ShellError::OutsideSpannedLabeledError {
                src,
                error: "Error while parsing KDL text".into(),
                msg: diagnostic_message,
                span: label_span,
            }]),
        );
    }

    ShellError::CantConvert {
        to_type: format!("structured kdl data ({err})"),
        from_type: "string".into(),
        span,
        help: None,
    }
}

fn kdl_diagnostics_message(diagnostics: &[kdl::KdlDiagnostic]) -> String {
    if diagnostics.len() == 1 {
        return kdl_diagnostic_message(&diagnostics[0]);
    }

    diagnostics
        .iter()
        .enumerate()
        .map(|(index, diagnostic)| {
            format!(
                "diagnostic {}:\n{}",
                index + 1,
                kdl_diagnostic_message(diagnostic)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn kdl_diagnostic_message(diagnostic: &kdl::KdlDiagnostic) -> String {
    let mut parts = Vec::new();

    if let Some(message) = &diagnostic.message {
        parts.push(message.clone());
    }

    if let Some(label) = &diagnostic.label
        && parts.last() != Some(label)
    {
        parts.push(label.clone());
    }

    if let Some(help) = &diagnostic.help {
        parts.push(format!("help: {help}"));
    }

    if parts.is_empty() {
        "error parsing KDL text".to_owned()
    } else {
        parts.join("\n")
    }
}

fn convert_kdl_document_to_node_rows(
    nodes: &[KdlNode],
    span: Span,
) -> Result<Vec<Value>, ShellError> {
    let mut rows = Vec::with_capacity(nodes.len());

    for node in nodes {
        rows.push(convert_kdl_node_to_node_row(node, span)?);
    }

    Ok(rows)
}

fn convert_kdl_node_to_node_row(kdl_node: &KdlNode, span: Span) -> Result<Value, ShellError> {
    let mut args = Vec::new();
    let mut props = Record::new();

    for entry in kdl_node.entries() {
        if let Some(name) = entry.name() {
            props.insert(
                name.value().to_string(),
                convert_kdl_value_to_nu_value(entry.value(), span)?,
            );
            continue;
        }

        args.push(convert_kdl_value_to_nu_value(entry.value(), span)?);
    }

    let children = if let Some(children_doc) = kdl_node.children() {
        convert_kdl_document_to_node_rows(children_doc.nodes(), span)?
    } else {
        Vec::new()
    };

    let row = record! {
        "name" => Value::string(kdl_node.name().value(), span),
        "args" => Value::list(args, span),
        "props" => props.into_value(span),
        "children" => Value::list(children, span),
    };

    Ok(row.into_value(span))
}

fn convert_kdl_value_to_nu_value(value: &KdlValue, span: Span) -> Result<Value, ShellError> {
    match value {
        KdlValue::String(val) => Ok(Value::string(val, span)),
        KdlValue::Integer(val) => Ok(Value::int(
            val.to_i64().ok_or(ShellError::UnsupportedInput {
                msg: "integer value is too large to fit in i64".to_owned(),
                input: "value originates from here".to_owned(),
                msg_span: span,
                input_span: span,
            })?,
            span,
        )),
        KdlValue::Float(val) => Ok(Value::float(*val, span)),
        KdlValue::Bool(val) => Ok(Value::bool(*val, span)),
        KdlValue::Null => Ok(Value::nothing(span)),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn node_name(row: &Value) -> &str {
        row.as_record()
            .ok()
            .and_then(|record| record.get("name"))
            .and_then(|value| value.as_str().ok())
            .expect("row should contain string name")
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(FromKdl)
    }

    #[test]
    fn duplicate_sibling_names_are_preserved_in_order() {
        let span = Span::test_data();
        let kdl_document = KdlDocument::parse("node one\nnode two\nnode three")
            .expect("failed to parse duplicate sibling document");

        let output_rows = convert_kdl_document_to_node_rows(kdl_document.nodes(), span)
            .expect("conversion failed");

        assert_eq!(output_rows.len(), 3);
        assert_eq!(node_name(&output_rows[0]), "node");
        assert_eq!(node_name(&output_rows[1]), "node");
        assert_eq!(node_name(&output_rows[2]), "node");

        let second_args = output_rows[1]
            .as_record()
            .ok()
            .and_then(|record| record.get("args"))
            .and_then(|value| value.as_list().ok())
            .expect("missing args list");

        assert_eq!(
            second_args.first().cloned(),
            Some(Value::string("two", span))
        );
    }

    #[test]
    fn duplicate_properties_use_rightmost_value() {
        let span = Span::test_data();
        let kdl_document = KdlDocument::parse("node attr=1 attr=2")
            .expect("failed to parse duplicate property document");

        let output_rows = convert_kdl_document_to_node_rows(kdl_document.nodes(), span)
            .expect("conversion failed");

        let props = output_rows[0]
            .as_record()
            .ok()
            .and_then(|record| record.get("props"))
            .and_then(|value| value.as_record().ok())
            .expect("missing props record");

        assert_eq!(props.len(), 1);
        assert_eq!(props.get("attr"), Some(&Value::int(2, span)));
    }

    #[test]
    fn parse_errors_use_structured_kdl_diagnostics() {
        let error = parse_kdl_document_with_diagnostics("node 1.", Span::test_data())
            .expect_err("invalid KDL should fail");

        let ShellError::Generic(generic) = error else {
            panic!("expected generic shell error");
        };

        let Some(ShellError::OutsideSpannedLabeledError { msg, .. }) = generic.inner.first() else {
            panic!("expected structured inner parse diagnostic");
        };

        assert!(!msg.trim().is_empty());
        assert_ne!(msg.trim(), "error parsing KDL text");
    }

    #[test]
    fn multiple_kdl_diagnostics_are_aggregated() {
        let err = KdlDocument::parse("node 1.").expect_err("input should fail to parse");
        let mut diagnostics = err.diagnostics.clone();

        diagnostics.push(
            diagnostics
                .first()
                .expect("expected at least one diagnostic")
                .clone(),
        );

        let message = kdl_diagnostics_message(&diagnostics);

        assert!(message.contains("diagnostic 1:"));
        assert!(message.contains("diagnostic 2:"));
    }

    #[test]
    fn canonical_row_shape_splits_args_props_and_children() {
        let span = Span::test_data();
        let kdl_document = KdlDocument::parse("item 1 2 enabled=#true { child 9 }")
            .expect("failed to parse mixed kdl node");

        let output_rows = convert_kdl_document_to_node_rows(kdl_document.nodes(), span)
            .expect("conversion failed");

        let row = output_rows
            .first()
            .and_then(|value| value.as_record().ok())
            .expect("missing top-level row");

        assert_eq!(row.get("name").cloned(), Some(Value::string("item", span)));
        assert_eq!(
            row.get("args")
                .and_then(|value| value.as_list().ok())
                .map(|args| args.len()),
            Some(2)
        );
        assert_eq!(
            row.get("props")
                .and_then(|value| value.as_record().ok())
                .and_then(|props| props.get("enabled"))
                .cloned(),
            Some(Value::bool(true, span))
        );
        assert_eq!(
            row.get("children")
                .and_then(|value| value.as_list().ok())
                .map(|children| children.len()),
            Some(1)
        );
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

        let output_rows = convert_kdl_document_to_node_rows(kdl_document.nodes(), span)
            .expect("kdl conversion failed");

        let package = output_rows
            .first()
            .and_then(|row| row.as_record().ok())
            .expect("missing package node");

        let package_children = package
            .get("children")
            .and_then(|value| value.as_list().ok())
            .expect("missing package children");

        let matrix_row = package_children
            .iter()
            .find(|node| {
                node.as_record()
                    .ok()
                    .and_then(|record| record.get("name"))
                    .and_then(|name| name.as_str().ok())
                    == Some("the-matrix")
            })
            .expect("missing matrix row");

        assert_eq!(
            matrix_row
                .as_record()
                .ok()
                .and_then(|record| record.get("args"))
                .and_then(|args| args.as_list().ok())
                .and_then(|args| args.first())
                .cloned()
                .expect("missing matrix first arg"),
            Value::int(1, span)
        );

        let scripts_row = package_children
            .iter()
            .find(|node| {
                node.as_record()
                    .ok()
                    .and_then(|record| record.get("name"))
                    .and_then(|name| name.as_str().ok())
                    == Some("scripts")
            })
            .expect("missing scripts row");

        let message_row = scripts_row
            .as_record()
            .ok()
            .and_then(|record| record.get("children"))
            .and_then(|children| children.as_list().ok())
            .and_then(|children| {
                children.iter().find(|node| {
                    node.as_record()
                        .ok()
                        .and_then(|record| record.get("name"))
                        .and_then(|name| name.as_str().ok())
                        == Some("message")
                })
            })
            .expect("missing message row");

        assert_eq!(
            message_row
                .as_record()
                .ok()
                .and_then(|record| record.get("args"))
                .and_then(|args| args.as_list().ok())
                .and_then(|args| args.first())
                .cloned()
                .expect("missing message text"),
            Value::string("hello\nworld", span)
        );
    }

    #[test]
    fn kdl_error_source_is_bounded() {
        let mut input = String::with_capacity(50_000);
        for _ in 0..2000 {
            input.push_str("node1 key=1; ");
        }
        input.push_str("node2 \"unclosed"); // Syntax error: unclosed string

        let result = parse_kdl_document_with_diagnostics(&input, Span::test_data());
        assert!(result.is_err(), "should fail to parse");

        let err = result.unwrap_err();
        match &err {
            ShellError::Generic(GenericError { inner, .. }) => {
                let inner_err = inner.first().expect("should have inner error");
                match inner_err {
                    ShellError::OutsideSpannedLabeledError { src, .. } => {
                        assert!(
                            src.len() < 20_000,
                            "error source should be bounded, got {} bytes",
                            src.len()
                        );
                    }
                    other => panic!("expected OutsideSpannedLabeledError, got {other:?}"),
                }
            }
            other => panic!("expected Generic error, got {other:?}"),
        }
    }

    #[test]
    fn kdl_parse_success_not_affected() {
        let result = parse_kdl_document_with_diagnostics(
            r#"node1 key=1; node2 key="val""#,
            Span::test_data(),
        );
        assert!(result.is_ok(), "valid KDL should still parse");
    }
}
