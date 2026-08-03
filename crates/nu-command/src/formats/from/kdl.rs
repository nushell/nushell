use crate::formats::kdl::{
    KdlFormat, KdlMetadata, KdlSpec, jik_document_to_value, nodes_document_to_value,
    parse_kdl_document,
};
use chrono::TimeZone;
use nu_engine::command_prelude::*;
use std::str::FromStr;

#[derive(Clone)]
pub struct FromKdl;

impl Command for FromKdl {
    fn name(&self) -> &str {
        "from kdl"
    }

    fn description(&self) -> &str {
        "Convert KDL text into structured data."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "import", "config"]
    }

    fn signature(&self) -> Signature {
        Signature::build("from kdl")
            .input_output_types(vec![(Type::String, Type::Any)])
            .param(
                Flag::new("spec")
                    .arg(SyntaxShape::Int)
                    .desc("KDL language version (1 or 2 (default)).")
                    .completion(Completion::new_list(&["1", "2"])),
            )
            .param(
                Flag::new("format")
                    .arg(SyntaxShape::String)
                    .desc(
                        "Data model: 'nodes' (default) for KDL AST rows, or 'jik' for JSON-in-KDL values.",
                    )
                    .completion(Completion::new_list(&["nodes", "jik"])),
            )
            .switch(
                "ignore-types",
                "Ignore type annotations (return base KDL types only).",
                None,
            )
            .category(Category::Formats)
    }

    fn examples(&self) -> Vec<Example<'_>> {
        let span = Span::unknown();

        vec![
            Example {
                example: r#""node attr=1 attr2=#true {bloc}" | from kdl"#,
                description: "Convert KDL to node rows (default format, KDL v2).",
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
                description: "Parse a package-style KDL document into node rows.",
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
                description: "Parse JSON-in-KDL (v2 keywords use #true / #null).",
                example: "'- a=1 b=#true' | from kdl --format jik",
                result: Some(Value::test_record(record! {
                    "a" => Value::int(1, span),
                    "b" => Value::bool(true, span),
                })),
            },
            Example {
                description: "Parse KDL v2 keyword arguments with --spec 2 (default).",
                example: r#""node #true #false #null" | from kdl --spec 2"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("node", span),
                    "args" => Value::test_list(vec![
                        Value::bool(true, span),
                        Value::bool(false, span),
                        Value::nothing(span),
                    ]),
                    "props" => Value::test_record(record! {}),
                    "children" => Value::test_list(vec![]),
                })])),
            },
            Example {
                description: "Parse KDL v1 keyword arguments with --spec 1 (bare true/false/null).",
                example: r#""node true false null" | from kdl --spec 1"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("node", span),
                    "args" => Value::test_list(vec![
                        Value::bool(true, span),
                        Value::bool(false, span),
                        Value::nothing(span),
                    ]),
                    "props" => Value::test_record(record! {}),
                    "children" => Value::test_list(vec![]),
                })])),
            },
            Example {
                description: "Parse a KDL v1 property boolean with --spec 1.",
                example: r#""item 1 enabled=true" | from kdl --spec 1"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("item", span),
                    "args" => Value::test_list(vec![Value::int(1, span)]),
                    "props" => Value::test_record(record! {
                        "enabled" => Value::bool(true, span),
                    }),
                    "children" => Value::test_list(vec![]),
                })])),
            },
            Example {
                description: "Parse a KDL v2 property boolean with --spec 2.",
                example: r#""item 1 enabled=#true" | from kdl --spec 2"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("item", span),
                    "args" => Value::test_list(vec![Value::int(1, span)]),
                    "props" => Value::test_record(record! {
                        "enabled" => Value::bool(true, span),
                    }),
                    "children" => Value::test_list(vec![]),
                })])),
            },
            Example {
                description: "Parse JSON-in-KDL written in KDL v1 keyword style.",
                example: "'- a=1 b=true c=null' | from kdl --format jik --spec 1",
                result: Some(Value::test_record(record! {
                    "a" => Value::int(1, span),
                    "b" => Value::bool(true, span),
                    "c" => Value::nothing(span),
                })),
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
            Example {
                description: "Promote Nushell type annotations on node arguments (filesize, duration).",
                example: r#""node (filesize)1024 (duration)5000000000" | from kdl"#,
                result: Some(Value::test_list(vec![Value::test_record(record! {
                    "name" => Value::string("node", span),
                    "args" => Value::test_list(vec![
                        Value::test_filesize(1024),
                        Value::test_duration(5_000_000_000),
                    ]),
                    "props" => Value::test_record(record! {}),
                    "children" => Value::test_list(vec![]),
                })])),
            },
            Example {
                description: "Parse JSON-in-KDL with filesize, duration, and datetime (timestamp) annotations.",
                example: r#"'- size=(filesize)1024 wait=(duration)5000000000 when=(timestamp)"2020-01-02T03:04:05+00:00"' | from kdl --format jik"#,
                result: Some(Value::test_record(record! {
                    "size" => Value::test_filesize(1024),
                    "wait" => Value::test_duration(5_000_000_000),
                    "when" => Value::test_date(
                        chrono::FixedOffset::east_opt(0)
                            .expect("offset")
                            .with_ymd_and_hms(2020, 1, 2, 3, 4, 5)
                            .unwrap()
                    ),
                })),
            },
            Example {
                description: "Accept (datetime) as an alias for (timestamp) when parsing.",
                example: r#"'- when=(datetime)"2020-01-02T03:04:05+00:00"' | from kdl --format jik"#,
                result: Some(Value::test_record(record! {
                    "when" => Value::test_date(
                        chrono::FixedOffset::east_opt(0)
                            .expect("offset")
                            .with_ymd_and_hms(2020, 1, 2, 3, 4, 5)
                            .unwrap()
                    ),
                })),
            },
            Example {
                description: "Parse cell-path, range, glob, and binary type annotations.",
                example: r#"'- path=(cell-path)$.1.abc span=(range)"1..3" pat=(glob)*.rs blob=(binary)AQID' | from kdl --format jik"#,
                result: Some(Value::test_record(record! {
                    "path" => Value::test_cell_path(nu_protocol::ast::CellPath {
                        members: vec![
                            nu_protocol::ast::PathMember::test_int(1, false),
                            nu_protocol::ast::PathMember::test_string(
                                "abc",
                                false,
                                nu_protocol::casing::Casing::Sensitive,
                            ),
                        ],
                    }),
                    "span" => Value::test_range(
                        nu_protocol::Range::from_str("1..3").expect("range")
                    ),
                    "pat" => Value::test_glob("*.rs"),
                    "blob" => Value::test_binary(vec![1, 2, 3]),
                })),
            },
            Example {
                description: "Ignore type annotations and keep base KDL types with --ignore-types.",
                example: "'- size=(filesize)1024' | from kdl --format jik --ignore-types",
                result: Some(Value::test_record(record! {
                    "size" => Value::test_int(1024),
                })),
            },
        ]
    }

    fn run(
        &self,
        engine_state: &EngineState,
        stack: &mut Stack,
        call: &Call,
        mut input: PipelineData,
    ) -> Result<PipelineData, ShellError> {
        let span = input.span().unwrap_or(call.head);
        let mut metadata = input
            .take_metadata()
            .unwrap_or_default()
            .with_content_type(None);

        let kdl_string = input.collect_string_strict(span)?;

        let spec = match call.get_flag::<i64>(engine_state, stack, "spec")? {
            Some(n) => {
                let flag_span = call.get_flag_span(stack, "spec").unwrap_or(call.head);
                KdlSpec::from_i64(n, flag_span)?
            }
            None => KdlSpec::default(),
        };

        let format = match call.get_flag::<String>(engine_state, stack, "format")? {
            Some(s) => {
                let flag_span = call.get_flag_span(stack, "format").unwrap_or(call.head);
                KdlFormat::parse(&s, flag_span)?
            }
            None => KdlFormat::Nodes,
        };

        let ignore_types = call.has_flag(engine_state, stack, "ignore-types")?;

        let document = parse_kdl_document(&kdl_string.0, spec, span)?;

        let value = match format {
            KdlFormat::Nodes => nodes_document_to_value(&document, span, ignore_types)?,
            KdlFormat::Jik => jik_document_to_value(&document, span, ignore_types)?,
        };

        KdlMetadata { format, spec }.write_to(&mut metadata, span);

        Ok(value.into_pipeline_data_with_metadata(Some(metadata)))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::formats::kdl::{kdl_diagnostics_message, parse_kdl_document};
    use kdl::KdlDocument;
    use nu_protocol::shell_error::generic::GenericError;

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

        let output =
            nodes_document_to_value(&kdl_document, span, false).expect("conversion failed");
        let output_rows = output.as_list().expect("list");

        assert_eq!(output_rows.len(), 3);
        assert_eq!(node_name(&output_rows[0]), "node");
        assert_eq!(node_name(&output_rows[1]), "node");
        assert_eq!(node_name(&output_rows[2]), "node");
    }

    #[test]
    fn duplicate_properties_use_at_suffix() {
        let span = Span::test_data();
        let kdl_document = KdlDocument::parse("node attr=1 attr=2")
            .expect("failed to parse duplicate property document");

        let output =
            nodes_document_to_value(&kdl_document, span, false).expect("conversion failed");
        let props = output
            .as_list()
            .ok()
            .and_then(|rows| rows.first())
            .and_then(|row| row.as_record().ok())
            .and_then(|record| record.get("props"))
            .and_then(|value| value.as_record().ok())
            .expect("missing props record")
            .clone();

        assert_eq!(props.len(), 2);
        assert_eq!(props.get("attr"), Some(&Value::int(1, span)));
        assert_eq!(props.get("attr@2"), Some(&Value::int(2, span)));
    }

    #[test]
    fn parse_errors_use_structured_kdl_diagnostics() {
        let error =
            parse_kdl_document("node 1.", KdlSpec::V2, Span::test_data()).expect_err("invalid KDL");

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
    fn jik_object_round_trip_shape() {
        let span = Span::test_data();
        let doc = parse_kdl_document("- a=1 b=#true", KdlSpec::V2, span).expect("parse");
        let value = jik_document_to_value(&doc, span, false).expect("jik");
        let record = value.as_record().expect("record");
        assert_eq!(record.get("a"), Some(&Value::int(1, span)));
        assert_eq!(record.get("b"), Some(&Value::bool(true, span)));
    }

    #[test]
    fn filesize_type_annotation_is_promoted() {
        let span = Span::test_data();
        let doc = parse_kdl_document("node (filesize)1024", KdlSpec::V2, span).expect("parse");
        let output = nodes_document_to_value(&doc, span, false).expect("convert");
        let args = output
            .as_list()
            .ok()
            .and_then(|rows| rows.first())
            .and_then(|row| row.as_record().ok())
            .and_then(|record| record.get("args"))
            .and_then(|value| value.as_list().ok())
            .expect("args");
        assert_eq!(args.first(), Some(&Value::filesize(1024, span)));
    }

    #[test]
    fn v1_spec_parses_v1_booleans() {
        let span = Span::test_data();
        // KDL v1 uses bare `true`/`false`/`null` keywords (not #true).
        let doc = parse_kdl_document("node flag=true", KdlSpec::V1, span).expect("v1 parse");
        let output = nodes_document_to_value(&doc, span, false).expect("convert");
        let props = output
            .as_list()
            .ok()
            .and_then(|rows| rows.first())
            .and_then(|row| row.as_record().ok())
            .and_then(|record| record.get("props"))
            .and_then(|value| value.as_record().ok())
            .expect("props");
        assert_eq!(props.get("flag"), Some(&Value::bool(true, span)));
    }

    #[test]
    fn v2_rejects_v1_style_bare_true_property() {
        assert!(
            parse_kdl_document("node flag=true", KdlSpec::V2, Span::test_data()).is_err(),
            "v2 should reject bare true property"
        );
    }

    #[test]
    fn v1_rejects_v2_style_hash_true_property() {
        assert!(
            parse_kdl_document("node flag=#true", KdlSpec::V1, Span::test_data()).is_err(),
            "v1 should reject #true property"
        );
    }

    #[test]
    fn v1_and_v2_keyword_args_decode_identically() {
        let span = Span::test_data();
        let v1 = parse_kdl_document("node true false null", KdlSpec::V1, span).unwrap();
        let v2 = parse_kdl_document("node #true #false #null", KdlSpec::V2, span).unwrap();
        let rows_v1 = nodes_document_to_value(&v1, span, false).unwrap();
        let rows_v2 = nodes_document_to_value(&v2, span, false).unwrap();
        assert_eq!(rows_v1, rows_v2);
    }

    #[test]
    fn v1_jik_parse_bare_keywords() {
        let span = Span::test_data();
        let doc = parse_kdl_document("- a=1 b=true c=null", KdlSpec::V1, span).unwrap();
        let value = jik_document_to_value(&doc, span, false).unwrap();
        assert_eq!(
            value,
            Value::test_record(record! {
                "a" => Value::int(1, span),
                "b" => Value::bool(true, span),
                "c" => Value::nothing(span),
            })
        );
    }

    #[test]
    fn v2_jik_parse_hash_keywords() {
        let span = Span::test_data();
        let doc = parse_kdl_document("- a=1 b=#true c=#null", KdlSpec::V2, span).unwrap();
        let value = jik_document_to_value(&doc, span, false).unwrap();
        assert_eq!(
            value,
            Value::test_record(record! {
                "a" => Value::int(1, span),
                "b" => Value::bool(true, span),
                "c" => Value::nothing(span),
            })
        );
    }

    #[test]
    fn v1_parse_error_is_structured() {
        // #true is invalid in v1
        let error =
            parse_kdl_document("node #true", KdlSpec::V1, Span::test_data()).expect_err("v1 fail");
        assert!(matches!(
            error,
            ShellError::Generic(_) | ShellError::CantConvert { .. }
        ));
    }

    #[test]
    fn kdl_error_source_is_bounded() {
        let mut input = String::with_capacity(50_000);
        for _ in 0..2000 {
            input.push_str("node1 key=1; ");
        }
        input.push_str("node2 \"unclosed");

        let result = parse_kdl_document(&input, KdlSpec::V2, Span::test_data());
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
}
