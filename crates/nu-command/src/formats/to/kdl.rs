use crate::formats::kdl::{
    KdlFormat, KdlMetadata, KdlSpec, document_to_string, node_rows_to_kdl_document,
    resolve_non_roundtrip, value_to_jik_document,
};
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
                        "Data model: 'jik' (default) for JSON-in-KDL, or 'nodes' for KDL AST rows.",
                    )
                    .completion(Completion::new_list(&["nodes", "jik"])),
            )
            .switch(
                "serialize",
                "Serialize nushell types that cannot be deserialized (shorthand for --non-roundtrip lossy).",
                Some('s'),
            )
            .param(
                Flag::new("non-roundtrip")
                    .arg(SyntaxShape::String)
                    .desc("How to handle values that are non-roundtrippable ('error' (default), 'null', or 'lossy').")
                    .completion(Completion::new_list(&["error", "null", "lossy"])),
            )
            .category(Category::Formats)
    }

    fn description(&self) -> &str {
        "Converts structured data into KDL text."
    }

    fn search_terms(&self) -> Vec<&str> {
        vec!["convert", "export", "config"]
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

        let from_meta = KdlMetadata::read_from(&metadata);
        KdlMetadata::clear(&mut metadata);
        let metadata = metadata.with_content_type(Some("application/x-kdl".to_owned()));

        let format = match call.get_flag::<String>(engine_state, stack, "format")? {
            Some(s) => {
                let flag_span = call.get_flag_span(stack, "format").unwrap_or(call.head);
                KdlFormat::parse(&s, flag_span)?
            }
            None => from_meta
                .as_ref()
                .map(|m| m.format)
                .unwrap_or(KdlFormat::Jik),
        };

        let spec = match call.get_flag::<i64>(engine_state, stack, "spec")? {
            Some(n) => {
                let flag_span = call.get_flag_span(stack, "spec").unwrap_or(call.head);
                KdlSpec::from_i64(n, flag_span)?
            }
            None => from_meta.as_ref().map(|m| m.spec).unwrap_or_default(),
        };

        let non_roundtrip_flag =
            call.get_flag::<Spanned<String>>(engine_state, stack, "non-roundtrip")?;
        let serialize = call.has_flag(engine_state, stack, "serialize")?;

        // Match YAML mutual exclusion for --serialize vs --non-roundtrip when both set with non-lossy.
        if serialize
            && let Some(nr) = non_roundtrip_flag.as_ref()
            && nr.item != "lossy"
        {
            return Err(ShellError::IncompatibleParameters {
                left_message: "this is a shorthand to".into(),
                left_span: call.get_flag_span(stack, "serialize").unwrap_or(call.head),
                right_message: "this with `lossy`".into(),
                right_span: nr.span,
            });
        }

        let non_roundtrip = resolve_non_roundtrip(serialize, non_roundtrip_flag, engine_state)?;

        let value = input.into_value(call_span)?;

        let document = match format {
            KdlFormat::Jik => value_to_jik_document(&value, &non_roundtrip, call_span)?,
            KdlFormat::Nodes => node_rows_to_kdl_document(&value, &non_roundtrip, call_span)?,
        };

        let output_string = document_to_string(document, spec);

        Ok(output_string
            .into_value(call_span)
            .into_pipeline_data_with_metadata(metadata))
    }

    fn examples(&self) -> Vec<Example<'_>> {
        vec![
            Example {
                description: "Convert a record to JSON-in-KDL (default KDL v2 keywords).",
                example: "{a: 1, b: true} | to kdl",
                result: Some(Value::test_string("- a=1 b=#true\n")),
            },
            Example {
                description: "Emit KDL v2 explicitly with --spec 2.",
                example: "{a: 1, b: true} | to kdl --spec 2",
                result: Some(Value::test_string("- a=1 b=#true\n")),
            },
            Example {
                description: "Emit KDL v1 keywords with --spec 1 (bare true/false/null).",
                example: "{a: 1, b: true} | to kdl --spec 1",
                result: Some(Value::test_string("- a=1 b=true\n")),
            },
            Example {
                description: "Convert a list to JSON-in-KDL.",
                example: "[1 2 3] | to kdl",
                result: Some(Value::test_string("- 1 2 3\n")),
            },
            Example {
                description: "Emit null/bool keywords as KDL v2.",
                example: "[null false true] | to kdl --spec 2",
                result: Some(Value::test_string("- #null #false #true\n")),
            },
            Example {
                description: "Emit null/bool keywords as KDL v1.",
                example: "[null false true] | to kdl --spec 1",
                result: Some(Value::test_string("- null false true\n")),
            },
            Example {
                description: "Round-trip KDL through node rows (default v2).",
                example: "'node one; node two' | from kdl | to kdl",
                result: Some(Value::test_string("node one\nnode two\n")),
            },
            Example {
                description: "Round-trip a KDL v1 document; metadata keeps --spec 1 on to kdl.",
                example: r#""item 1 enabled=true" | from kdl --spec 1 | to kdl"#,
                result: Some(Value::test_string("item 1 enabled=true\n")),
            },
            Example {
                description: "Round-trip a KDL v2 document with --spec 2 on both sides.",
                example: r#""item 1 enabled=#true" | from kdl --spec 2 | to kdl --spec 2"#,
                result: Some(Value::test_string("item 1 enabled=#true\n")),
            },
            Example {
                description: "Override metadata: parse as v1 but emit as v2.",
                example: r#""item 1 enabled=true" | from kdl --spec 1 | to kdl --spec 2"#,
                result: Some(Value::test_string("item 1 enabled=#true\n")),
            },
            Example {
                description: "Serialize a closure as a string.",
                example: "{|| 1 + 1} | to kdl --serialize",
                result: Some(Value::test_string("- \"{|| 1 + 1}\"\n")),
            },
            Example {
                description: "Emit Nushell filesize and duration with type annotations.",
                example: "{size: 1kib, wait: 5sec} | to kdl",
                result: Some(Value::test_string(
                    "- size=(filesize)1024 wait=(duration)5000000000\n",
                )),
            },
            Example {
                description: "Emit a datetime as a (timestamp) annotation (RFC 3339 string).",
                example: "{when: 2020-01-02T03:04:05+00:00} | to kdl",
                result: Some(Value::test_string(
                    "- when=(timestamp)\"2020-01-02T03:04:05+00:00\"\n",
                )),
            },
            Example {
                description: "Emit a cell-path with a (cell-path) annotation.",
                example: "$.1.abc | to kdl",
                result: Some(Value::test_string("- (cell-path)$.1.abc\n")),
            },
            Example {
                description: "Emit a range with a (range) annotation.",
                example: "1..3 | to kdl",
                result: Some(Value::test_string("- (range)\"1..3\"\n")),
            },
            Example {
                description: "Emit a glob with a (glob) annotation.",
                example: r#""*.rs" | into glob | to kdl"#,
                result: Some(Value::test_string("- (glob)*.rs\n")),
            },
            Example {
                description: "Emit binary data as base64 with a (binary) annotation.",
                example: "0x[01 02 03] | to kdl",
                result: Some(Value::test_string("- (binary)AQID\n")),
            },
            Example {
                description: "Round-trip annotated Nushell types through KDL.",
                example: "{size: 1kib, wait: 5sec} | to kdl | from kdl --format jik",
                result: Some(Value::test_record(record! {
                    "size" => Value::test_filesize(1024),
                    "wait" => Value::test_duration(5_000_000_000),
                })),
            },
            Example {
                description: "Round-trip a list of records (table-shaped data) as JSON-in-KDL.",
                example: "[{a: 1}, {a: 2}] | to kdl | from kdl --format jik",
                result: Some(Value::test_list(vec![
                    Value::test_record(record! { "a" => Value::test_int(1) }),
                    Value::test_record(record! { "a" => Value::test_int(2) }),
                ])),
            },
        ]
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::formats::kdl::{
        KdlMetadata, NonRoundtrip, document_to_string, node_rows_to_kdl_document,
        value_to_jik_document,
    };
    use crate::{Get, Metadata};
    use nu_cmd_lang::eval_pipeline_without_terminal_expression;
    use nu_protocol::PipelineMetadata;

    fn eval_kdl(cmd: &str) -> Value {
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
        eval_pipeline_without_terminal_expression(
            cmd,
            std::env::temp_dir().as_ref(),
            &mut engine_state,
        )
        .expect("pipeline should succeed")
    }

    #[test]
    fn test_examples() -> nu_test_support::Result {
        nu_test_support::test().examples(ToKdl)
    }

    #[test]
    fn jik_wraps_scalars_in_anon_node() {
        let document =
            value_to_jik_document(&Value::test_int(5), &NonRoundtrip::Error, Span::test_data())
                .expect("scalar should serialize");

        assert_eq!(document.to_string(), "- 5\n");
    }

    #[test]
    fn jik_empty_list_and_record() {
        let span = Span::test_data();
        let list_doc = value_to_jik_document(&Value::test_list(vec![]), &NonRoundtrip::Error, span)
            .expect("empty list");
        assert_eq!(list_doc.to_string(), "(array)-\n");

        let rec_doc =
            value_to_jik_document(&Value::test_record(record! {}), &NonRoundtrip::Error, span)
                .expect("empty record");
        assert_eq!(rec_doc.to_string(), "(object)-\n");
    }

    #[test]
    fn node_rows_round_trip_shape() {
        let span = Span::test_data();
        let rows = Value::test_list(vec![Value::test_record(record! {
            "name" => Value::string("item", span),
            "args" => Value::test_list(vec![Value::int(1, span)]),
            "props" => Value::test_record(record! { "enabled" => Value::bool(true, span) }),
            "children" => Value::test_list(vec![]),
        })]);

        let document =
            node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).expect("serialize");
        assert_eq!(document.to_string(), "item 1 enabled=#true\n");
    }

    #[test]
    fn nodes_format_rejects_plain_records() {
        let span = Span::test_data();
        let value = Value::test_record(record! {
            "plain" => Value::int(7, span),
        });

        let err = node_rows_to_kdl_document(&value, &NonRoundtrip::Error, span)
            .expect_err("should reject");
        match err {
            ShellError::UnsupportedInput { msg, .. } => {
                assert!(
                    msg.contains("nodes format") || msg.contains("node row"),
                    "unexpected error message: {msg}"
                );
            }
            other => panic!("expected UnsupportedInput, got {other:?}"),
        }
    }

    #[test]
    fn metadata_round_trip_defaults_format_and_spec() {
        let span = Span::test_data();
        let mut metadata = PipelineMetadata::default();
        KdlMetadata {
            format: KdlFormat::Nodes,
            spec: KdlSpec::V1,
        }
        .write_to(&mut metadata, span);

        let read = KdlMetadata::read_from(&metadata).expect("metadata");
        assert_eq!(read.format, KdlFormat::Nodes);
        assert_eq!(read.spec, KdlSpec::V1);
    }

    #[test]
    fn from_kdl_marker_flows_to_to_kdl_command() {
        // eval_pipeline_without_terminal_expression drops the last pipeline element,
        // so end with `$in` (or any dummy) to keep the expression under test.
        let result = eval_kdl(
            "'node one; node two' | from kdl | to kdl | metadata | get content_type | $in",
        );
        assert_eq!(result, Value::test_string("application/x-kdl"));
    }

    #[test]
    fn pipeline_to_kdl_spec_1_emits_bare_keywords() {
        let result = eval_kdl("{a: 1, b: true} | to kdl --spec 1 | $in");
        assert_eq!(result, Value::test_string("- a=1 b=true\n"));
    }

    #[test]
    fn pipeline_to_kdl_spec_2_emits_hash_keywords() {
        let result = eval_kdl("{a: 1, b: true} | to kdl --spec 2 | $in");
        assert_eq!(result, Value::test_string("- a=1 b=#true\n"));
    }

    #[test]
    fn pipeline_default_spec_is_v2() {
        let result = eval_kdl("{a: 1, b: true} | to kdl | $in");
        assert_eq!(result, Value::test_string("- a=1 b=#true\n"));
    }

    #[test]
    fn pipeline_from_spec_1_metadata_keeps_v1_on_to_kdl() {
        let result = eval_kdl(r#""item 1 enabled=true" | from kdl --spec 1 | to kdl | $in"#);
        assert_eq!(result, Value::test_string("item 1 enabled=true\n"));
    }

    #[test]
    fn pipeline_from_spec_2_metadata_keeps_v2_on_to_kdl() {
        let result = eval_kdl(r#""item 1 enabled=#true" | from kdl --spec 2 | to kdl | $in"#);
        assert_eq!(result, Value::test_string("item 1 enabled=#true\n"));
    }

    #[test]
    fn pipeline_explicit_spec_overrides_metadata() {
        let to_v2 =
            eval_kdl(r#""item 1 enabled=true" | from kdl --spec 1 | to kdl --spec 2 | $in"#);
        assert_eq!(to_v2, Value::test_string("item 1 enabled=#true\n"));
        let to_v1 =
            eval_kdl(r#""item 1 enabled=#true" | from kdl --spec 2 | to kdl --spec 1 | $in"#);
        assert_eq!(to_v1, Value::test_string("item 1 enabled=true\n"));
    }

    #[test]
    fn pipeline_list_keywords_follow_spec() {
        assert_eq!(
            eval_kdl("[null false true] | to kdl --spec 1 | $in"),
            Value::test_string("- null false true\n")
        );
        assert_eq!(
            eval_kdl("[null false true] | to kdl --spec 2 | $in"),
            Value::test_string("- #null #false #true\n")
        );
    }

    #[test]
    fn pipeline_from_spec_1_and_2_decode_same_data() {
        let v1 = eval_kdl(r#""node true false null" | from kdl --spec 1 | get 0.args | $in"#);
        let v2 = eval_kdl(r#""node #true #false #null" | from kdl --spec 2 | get 0.args | $in"#);
        assert_eq!(v1, v2);
    }

    #[test]
    fn pipeline_from_v1_jik_and_to_v1_jik_round_trip() {
        let result = eval_kdl(
            "'- a=1 b=true' | from kdl --format jik --spec 1 | to kdl --format jik --spec 1 | $in",
        );
        assert_eq!(result, Value::test_string("- a=1 b=true\n"));
    }

    #[test]
    fn pipeline_from_v2_jik_and_to_v2_jik_round_trip() {
        let result = eval_kdl(
            "'- a=1 b=#true' | from kdl --format jik --spec 2 | to kdl --format jik --spec 2 | $in",
        );
        assert_eq!(result, Value::test_string("- a=1 b=#true\n"));
    }

    #[test]
    fn pipeline_list_of_records_jik_round_trip() {
        // Regression: multi-row tables used to be misread as objects (duplicate key '-').
        let result = eval_kdl("[{a: 1}, {a: 2}] | to kdl | from kdl --format jik | $in");
        let span = Span::test_data();
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_record(record! { "a" => Value::int(1, span) }),
                Value::test_record(record! { "a" => Value::int(2, span) }),
            ])
        );
    }

    #[test]
    fn pipeline_jik_multi_anon_children_is_list() {
        let result = eval_kdl("'- { - a=1; - a=2 }' | from kdl --format jik | $in");
        let span = Span::test_data();
        assert_eq!(
            result,
            Value::test_list(vec![
                Value::test_record(record! { "a" => Value::int(1, span) }),
                Value::test_record(record! { "a" => Value::int(2, span) }),
            ])
        );
    }

    #[test]
    fn pipeline_jik_sole_anon_child_is_list_not_object() {
        let result = eval_kdl("'- { - 1 }' | from kdl --format jik | $in");
        assert_eq!(result, Value::test_list(vec![Value::test_int(1)]));
    }

    #[test]
    fn pipeline_jik_object_key_dash_needs_annotation() {
        let result = eval_kdl("'(object)- { - 1 }' | from kdl --format jik | $in");
        let span = Span::test_data();
        assert_eq!(
            result,
            Value::test_record(record! { "-" => Value::int(1, span) })
        );
    }

    #[test]
    fn document_to_string_respects_spec_for_node_rows() {
        let span = Span::test_data();
        let rows = Value::test_list(vec![Value::test_record(record! {
            "name" => Value::string("item", span),
            "args" => Value::test_list(vec![Value::int(1, span)]),
            "props" => Value::test_record(record! { "enabled" => Value::bool(true, span) }),
            "children" => Value::test_list(vec![]),
        })]);
        let doc = node_rows_to_kdl_document(&rows, &NonRoundtrip::Error, span).unwrap();
        assert_eq!(
            document_to_string(doc.clone(), KdlSpec::V1),
            "item 1 enabled=true\n"
        );
        assert_eq!(
            document_to_string(doc, KdlSpec::V2),
            "item 1 enabled=#true\n"
        );
    }
}
