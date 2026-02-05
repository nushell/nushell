#![doc = include_str!("../README.md")]
mod from;
mod to;

pub use from::from_nuon;
pub use from::from_nuon_into;
pub use to::SerializableClosure;
pub use to::ToNuonConfig;
pub use to::ToStyle;
pub use to::to_nuon;

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use nu_protocol::{
        IntRange, Range, Span, Value,
        ast::{CellPath, PathMember, RangeInclusion},
        casing::Casing,
        engine::{Closure, EngineState},
        record,
    };

    use crate::{ToNuonConfig, ToStyle, from_nuon, to_nuon};

    /// test something of the form
    /// ```nushell
    /// $v | from nuon | to nuon | $in == $v
    /// ```
    ///
    /// an optional "middle" value can be given to test what the value is between `from nuon` and
    /// `to nuon`.
    fn nuon_end_to_end(input: &str, middle: Option<Value>) {
        let engine_state = EngineState::new();
        let val = from_nuon(input, None).unwrap();
        if let Some(m) = middle {
            assert_eq!(val, m);
        }
        assert_eq!(
            to_nuon(&engine_state, &val, ToNuonConfig::default()).unwrap(),
            input
        );
    }

    #[test]
    fn list_of_numbers() {
        nuon_end_to_end(
            "[1, 2, 3]",
            Some(Value::test_list(vec![
                Value::test_int(1),
                Value::test_int(2),
                Value::test_int(3),
            ])),
        );
    }

    #[test]
    fn list_of_strings() {
        nuon_end_to_end(
            "[abc, xyz, def]",
            Some(Value::test_list(vec![
                Value::test_string("abc"),
                Value::test_string("xyz"),
                Value::test_string("def"),
            ])),
        );
    }

    #[test]
    fn table() {
        nuon_end_to_end(
            "[[my, columns]; [abc, xyz], [def, ijk]]",
            Some(Value::test_list(vec![
                Value::test_record(record!(
                    "my" => Value::test_string("abc"),
                    "columns" => Value::test_string("xyz")
                )),
                Value::test_record(record!(
                    "my" => Value::test_string("def"),
                    "columns" => Value::test_string("ijk")
                )),
            ])),
        );
    }

    #[test]
    fn from_nuon_illegal_table() {
        assert!(
            from_nuon("[[repeated repeated]; [abc, xyz], [def, ijk]]", None)
                .unwrap_err()
                .to_string()
                .contains("Record field or table column used twice: repeated")
        );
    }

    #[test]
    fn bool() {
        nuon_end_to_end("false", Some(Value::test_bool(false)));
    }

    #[test]
    fn escaping() {
        nuon_end_to_end(r#""hello\"world""#, None);
    }

    #[test]
    fn escaping2() {
        nuon_end_to_end(r#""hello\\world""#, None);
    }

    #[test]
    fn escaping3() {
        nuon_end_to_end(
            r#"[hello\\world]"#,
            Some(Value::test_list(vec![Value::test_string(
                r#"hello\\world"#,
            )])),
        );
    }

    #[test]
    fn escaping4() {
        nuon_end_to_end(r#"["hello\"world"]"#, None);
    }

    #[test]
    fn escaping5() {
        nuon_end_to_end(r#"{s: "hello\"world"}"#, None);
    }

    #[test]
    fn negative_int() {
        nuon_end_to_end("-1", Some(Value::test_int(-1)));
    }

    #[test]
    fn records() {
        nuon_end_to_end(
            r#"{name: "foo bar", age: 100, height: 10}"#,
            Some(Value::test_record(record!(
                    "name" => Value::test_string("foo bar"),
                    "age" => Value::test_int(100),
                    "height" => Value::test_int(10),
            ))),
        );
    }

    #[test]
    fn range() {
        nuon_end_to_end(
            "1..42",
            Some(Value::test_range(Range::IntRange(
                IntRange::new(
                    Value::test_int(1),
                    Value::test_int(2),
                    Value::test_int(42),
                    RangeInclusion::Inclusive,
                    Span::unknown(),
                )
                .unwrap(),
            ))),
        );
    }

    #[test]
    fn filesize() {
        nuon_end_to_end("1024b", Some(Value::test_filesize(1024)));
        assert_eq!(from_nuon("1kib", None).unwrap(), Value::test_filesize(1024));
    }

    #[test]
    fn duration() {
        nuon_end_to_end("60000000000ns", Some(Value::test_duration(60_000_000_000)));
    }

    #[test]
    fn to_nuon_datetime() {
        nuon_end_to_end(
            "1970-01-01T00:00:00+00:00",
            Some(Value::test_date(DateTime::UNIX_EPOCH.into())),
        );
    }

    #[test]
    fn closure_serialization_roundtrip() {
        use crate::from_nuon_into;
        use nu_protocol::engine::StateWorkingSet;

        // Create an engine state and parse a closure
        let mut engine_state = EngineState::new();
        engine_state.add_env_var("PWD".to_string(), Value::string("", Span::unknown()));

        let mut working_set = StateWorkingSet::new(&engine_state);

        // Parse a simple closure with a parameter
        let closure_source = b"{|x| $x + 1}";
        let block = nu_parser::parse(&mut working_set, None, closure_source, false);

        // Get the closure's block_id from the parsed expression
        let block_id = if let Some(pipeline) = block.pipelines.first() {
            if let Some(element) = pipeline.elements.first() {
                if let nu_protocol::ast::Expr::Closure(id) = element.expr.expr {
                    id
                } else {
                    panic!("Expected closure expression");
                }
            } else {
                panic!("No element in pipeline");
            }
        } else {
            panic!("No pipeline in block");
        };

        // Get the original block before merging
        let original_block = working_set.get_block(block_id).clone();

        // Merge the working set to make the block permanent
        let delta = working_set.render();
        engine_state.merge_delta(delta).unwrap();

        // Create a closure value
        let closure = Closure::new(block_id, vec![]);
        let closure_value = Value::closure(closure, Span::test_data());

        // Serialize the closure
        let nuon_str = to_nuon(&engine_state, &closure_value, ToNuonConfig::default()).unwrap();

        // Deserialize the closure
        let mut working_set2 = StateWorkingSet::new(&engine_state);
        let deserialized = from_nuon_into(&mut working_set2, &nuon_str, None).unwrap();

        // Check that it's a closure
        let Value::Closure {
            val: deserialized_closure,
            ..
        } = &deserialized
        else {
            panic!("Expected closure value");
        };

        // Get the deserialized block
        let deserialized_block = working_set2.get_block(deserialized_closure.block_id);

        // Compare signatures
        assert_eq!(
            original_block.signature.required_positional.len(),
            deserialized_block.signature.required_positional.len(),
            "Required positional args count mismatch"
        );
        for (i, (orig, deser)) in original_block
            .signature
            .required_positional
            .iter()
            .zip(deserialized_block.signature.required_positional.iter())
            .enumerate()
        {
            assert_eq!(orig.name, deser.name, "Positional arg {} name mismatch", i);
            assert_eq!(
                orig.shape, deser.shape,
                "Positional arg {} shape mismatch",
                i
            );
        }

        // Compare IR blocks
        assert!(
            original_block.ir_block.is_some(),
            "Original IR block should exist"
        );
        assert!(
            deserialized_block.ir_block.is_some(),
            "Deserialized IR block should exist"
        );

        let orig_ir = original_block.ir_block.as_ref().unwrap();
        let deser_ir = deserialized_block.ir_block.as_ref().unwrap();

        assert_eq!(
            orig_ir.instructions.len(),
            deser_ir.instructions.len(),
            "IR instruction count mismatch"
        );

        // Compare each instruction
        for (i, (orig_instr, deser_instr)) in orig_ir
            .instructions
            .iter()
            .zip(deser_ir.instructions.iter())
            .enumerate()
        {
            dbg!(i, orig_instr, deser_instr);
            assert_eq!(
                format!("{:?}", orig_instr),
                format!("{:?}", deser_instr),
                "IR instruction {} mismatch",
                i
            );
        }

        // Compare IR data
        assert_eq!(
            orig_ir.data.as_ref(),
            deser_ir.data.as_ref(),
            "IR data mismatch"
        );

        // Compare IR metadata
        assert_eq!(
            orig_ir.register_count, deser_ir.register_count,
            "Register count mismatch"
        );
        assert_eq!(
            orig_ir.file_count, deser_ir.file_count,
            "File count mismatch"
        );
        assert_eq!(orig_ir.spans, deser_ir.spans, "IR spans mismatch");
    }

    #[test]
    fn binary() {
        nuon_end_to_end(
            "0x[ABCDEF]",
            Some(Value::test_binary(vec![0xab, 0xcd, 0xef])),
        );
    }

    #[test]
    fn binary_roundtrip() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &from_nuon("0x[1f ff]", None).unwrap(),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "0x[1FFF]"
        );
    }

    #[test]
    fn read_sample_data() {
        assert_eq!(
            from_nuon(
                include_str!("../../../tests/fixtures/formats/sample.nuon"),
                None,
            )
            .unwrap(),
            Value::test_list(vec![
                Value::test_list(vec![
                    Value::test_record(record!(
                        "a" => Value::test_int(1),
                        "nuon" => Value::test_int(2),
                        "table" => Value::test_int(3)
                    )),
                    Value::test_record(record!(
                        "a" => Value::test_int(4),
                        "nuon" => Value::test_int(5),
                        "table" => Value::test_int(6)
                    )),
                ]),
                Value::test_filesize(100 * 1024),
                Value::test_duration(100 * 1_000_000_000),
                Value::test_bool(true),
                Value::test_record(record!(
                    "name" => Value::test_string("Bobby"),
                    "age" => Value::test_int(99)
                ),),
                Value::test_binary(vec![0x11, 0xff, 0xee, 0x1f]),
            ])
        );
    }

    #[test]
    fn float_doesnt_become_int() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_float(1.0),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "1.0"
        );
    }

    #[test]
    fn float_inf_parsed_properly() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_float(f64::INFINITY),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "inf"
        );
    }

    #[test]
    fn float_neg_inf_parsed_properly() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_float(f64::NEG_INFINITY),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "-inf"
        );
    }

    #[test]
    fn float_nan_parsed_properly() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_float(-f64::NAN),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "NaN"
        );
    }

    #[test]
    fn to_nuon_converts_columns_with_spaces() {
        let engine_state = EngineState::new();

        assert!(
            from_nuon(
                &to_nuon(
                    &engine_state,
                    &Value::test_list(vec![
                        Value::test_record(record!(
                            "a" => Value::test_int(1),
                            "b" => Value::test_int(2),
                            "c d" => Value::test_int(3)
                        )),
                        Value::test_record(record!(
                            "a" => Value::test_int(4),
                            "b" => Value::test_int(5),
                            "c d" => Value::test_int(6)
                        ))
                    ]),
                    ToNuonConfig::default(),
                )
                .unwrap(),
                None,
            )
            .is_ok()
        );
    }

    #[test]
    fn to_nuon_quotes_empty_string() {
        let engine_state = EngineState::new();

        let res = to_nuon(
            &engine_state,
            &Value::test_string(""),
            ToNuonConfig::default(),
        );
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), r#""""#);
    }

    #[test]
    fn to_nuon_quotes_empty_string_in_list() {
        nuon_end_to_end(
            r#"[""]"#,
            Some(Value::test_list(vec![Value::test_string("")])),
        );
    }

    #[test]
    fn to_nuon_quotes_empty_string_in_table() {
        nuon_end_to_end(
            "[[a, b]; [\"\", la], [le, lu]]",
            Some(Value::test_list(vec![
                Value::test_record(record!(
                    "a" => Value::test_string(""),
                    "b" => Value::test_string("la"),
                )),
                Value::test_record(record!(
                    "a" => Value::test_string("le"),
                    "b" => Value::test_string("lu"),
                )),
            ])),
        );
    }

    #[test]
    fn cell_path() {
        nuon_end_to_end(
            r#"$.foo.bar.0"#,
            Some(Value::test_cell_path(CellPath {
                members: vec![
                    PathMember::string(
                        "foo".to_string(),
                        false,
                        Casing::Sensitive,
                        Span::new(2, 5),
                    ),
                    PathMember::string(
                        "bar".to_string(),
                        false,
                        Casing::Sensitive,
                        Span::new(6, 9),
                    ),
                    PathMember::int(0, false, Span::new(10, 11)),
                ],
            })),
        );
    }

    #[test]
    fn does_not_quote_strings_unnecessarily() {
        let engine_state = EngineState::new();

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_list(vec![
                    Value::test_record(record!(
                        "a" => Value::test_int(1),
                        "b" => Value::test_int(2),
                        "c d" => Value::test_int(3)
                    )),
                    Value::test_record(record!(
                        "a" => Value::test_int(4),
                        "b" => Value::test_int(5),
                        "c d" => Value::test_int(6)
                    ))
                ]),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]"
        );

        assert_eq!(
            to_nuon(
                &engine_state,
                &Value::test_record(record!(
                    "ro name" => Value::test_string("sam"),
                    "rank" => Value::test_int(10)
                )),
                ToNuonConfig::default(),
            )
            .unwrap(),
            "{\"ro name\": sam, rank: 10}"
        );
    }

    #[test]
    fn quotes_some_strings_necessarily() {
        nuon_end_to_end(
            r#"["true", "false", "null", "NaN", "NAN", "nan", "+nan", "-nan", "inf", "+inf", "-inf", "INF", "Infinity", "+Infinity", "-Infinity", "INFINITY", "+19.99", "-19.99", "19.99b", "19.99kb", "19.99mb", "19.99gb", "19.99tb", "19.99pb", "19.99eb", "19.99zb", "19.99kib", "19.99mib", "19.99gib", "19.99tib", "19.99pib", "19.99eib", "19.99zib", "19ns", "19us", "19ms", "19sec", "19min", "19hr", "19day", "19wk", "-11.0..-15.0", "11.0..-15.0", "-11.0..15.0", "-11.0..<-15.0", "11.0..<-15.0", "-11.0..<15.0", "-11.0..", "11.0..", "..15.0", "..-15.0", "..<15.0", "..<-15.0", "2000-01-01", "2022-02-02T14:30:00", "2022-02-02T14:30:00+05:00", ", ", "", "&&"]"#,
            None,
        );
    }

    #[test]
    // NOTE: this test could be stronger, but the output of [`from_nuon`] on the content of `../../../tests/fixtures/formats/code.nu` is
    // not the same in the CI and locally...
    //
    // ## locally
    // ```
    // OutsideSpannedLabeledError {
    //     src: "register",
    //     error: "Error when loading",
    //     msg: "calls not supported in nuon",
    //     span: Span { start: 0, end: 8 }
    // }
    // ```
    //
    // ## in the CI
    // ```
    // GenericError {
    //     error: "error when parsing nuon text",
    //     msg: "could not parse nuon text",
    //     span: None,
    //     help: None,
    //     inner: [OutsideSpannedLabeledError {
    //         src: "register",
    //         error: "error when parsing",
    //         msg: "Unknown state.",
    //         span: Span { start: 0, end: 8 }
    //     }]
    // }
    // ```
    fn read_code_should_fail_rather_than_panic() {
        assert!(
            from_nuon(
                include_str!("../../../tests/fixtures/formats/code.nu"),
                None,
            )
            .is_err()
        );
    }

    // Raw string tests

    #[test]
    fn raw_string_parses_correctly() {
        // Verify raw strings are parsed correctly
        let input = r#"r#'hello "world"'#"#;
        let val = from_nuon(input, None).unwrap();
        assert_eq!(val, Value::test_string(r#"hello "world""#));
    }

    #[test]
    fn raw_string_parses_backslash() {
        // Raw string with backslash parses correctly
        let input = r"r#'path\to\file'#";
        let val = from_nuon(input, None).unwrap();
        assert_eq!(val, Value::test_string(r"path\to\file"));
    }

    #[test]
    fn raw_string_parses_with_hashes() {
        // String containing '# parses correctly with more hashes
        let input = r"r##'contains '# in middle'##";
        let val = from_nuon(input, None).unwrap();
        assert_eq!(val, Value::test_string("contains '# in middle"));
    }

    #[test]
    fn raw_strings_option_generates_raw() {
        let engine_state = EngineState::new();
        let val = Value::test_string(r#"hello "world""#);
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        // Nushell requires at least one # in raw strings
        assert_eq!(result, r#"r#'hello "world"'#"#);
    }

    #[test]
    fn raw_strings_option_with_backslash() {
        let engine_state = EngineState::new();
        let val = Value::test_string(r"path\to\file");
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        // Nushell requires at least one # in raw strings
        assert_eq!(result, r"r#'path\to\file'#");
    }

    #[test]
    fn raw_strings_option_no_raw_when_not_needed() {
        // Should use regular quoting when no escaping needed
        let engine_state = EngineState::new();
        let val = Value::test_string("hello world");
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        assert_eq!(result, r#""hello world""#);
    }

    #[test]
    fn raw_strings_option_in_list() {
        let engine_state = EngineState::new();
        let val = Value::test_list(vec![Value::test_string(r#"a "b" c"#)]);
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        assert_eq!(result, r#"[r#'a "b" c'#]"#);
    }

    #[test]
    fn raw_strings_option_in_record() {
        let engine_state = EngineState::new();
        let val = Value::test_record(record!(
            "key" => Value::test_string(r#"value "quoted""#)
        ));
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        assert_eq!(result, r#"{key: r#'value "quoted"'#}"#);
    }

    #[test]
    fn raw_strings_combined_with_raw_style() {
        // Test that raw_strings works with ToStyle::Raw (no whitespace)
        let engine_state = EngineState::new();
        let val = Value::test_record(record!(
            "a" => Value::test_string(r#"hello "world""#),
            "b" => Value::test_int(42)
        ));
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default()
                .style(ToStyle::Raw)
                .raw_strings(true),
        )
        .unwrap();
        assert_eq!(result, r#"{a:r#'hello "world"'#,b:42}"#);
    }

    #[test]
    fn raw_strings_roundtrip_with_raw_strings_option() {
        // Verify roundtrip: Value -> raw NUON -> Value
        let engine_state = EngineState::new();
        let original = Value::test_string(r#"path\to\"file""#);
        let nuon = to_nuon(
            &engine_state,
            &original,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        let parsed = from_nuon(&nuon, None).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn raw_strings_needs_more_hashes_when_content_has_quote_hash() {
        // Content with '# AND a quote/backslash needs at least two hashes
        let engine_state = EngineState::new();
        let val = Value::test_string(r#"contains '# and "quote""#);
        let result = to_nuon(
            &engine_state,
            &val,
            ToNuonConfig::default().raw_strings(true),
        )
        .unwrap();
        // Has '# so needs r##'...'##
        assert_eq!(result, r#"r##'contains '# and "quote"'##"#);
    }
}
