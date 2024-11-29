#![doc = include_str!("../README.md")]
mod from;
mod to;

pub use from::from_nuon;
pub use to::to_nuon;
pub use to::ToStyle;

#[cfg(test)]
mod tests {
    use chrono::DateTime;
    use nu_protocol::{
        ast::{CellPath, PathMember, RangeInclusion},
        engine::Closure,
        record, BlockId, IntRange, Range, Span, Value,
    };

    use crate::{from_nuon, to_nuon, ToStyle};

    /// test something of the form
    /// ```nushell
    /// $v | from nuon | to nuon | $in == $v
    /// ```
    ///
    /// an optional "middle" value can be given to test what the value is between `from nuon` and
    /// `to nuon`.
    fn nuon_end_to_end(input: &str, middle: Option<Value>) {
        let val = from_nuon(input, None).unwrap();
        if let Some(m) = middle {
            assert_eq!(val, m);
        }
        assert_eq!(to_nuon(&val, ToStyle::Raw, None).unwrap(), input);
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
    fn to_nuon_errs_on_closure() {
        assert!(to_nuon(
            &Value::test_closure(Closure {
                block_id: BlockId::new(0),
                captures: vec![]
            }),
            ToStyle::Raw,
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("Unsupported input"));
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
        assert_eq!(
            to_nuon(&from_nuon("0x[1f ff]", None).unwrap(), ToStyle::Raw, None).unwrap(),
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
        assert_eq!(
            to_nuon(&Value::test_float(1.0), ToStyle::Raw, None).unwrap(),
            "1.0"
        );
    }

    #[test]
    fn float_inf_parsed_properly() {
        assert_eq!(
            to_nuon(&Value::test_float(f64::INFINITY), ToStyle::Raw, None).unwrap(),
            "inf"
        );
    }

    #[test]
    fn float_neg_inf_parsed_properly() {
        assert_eq!(
            to_nuon(&Value::test_float(f64::NEG_INFINITY), ToStyle::Raw, None).unwrap(),
            "-inf"
        );
    }

    #[test]
    fn float_nan_parsed_properly() {
        assert_eq!(
            to_nuon(&Value::test_float(-f64::NAN), ToStyle::Raw, None).unwrap(),
            "NaN"
        );
    }

    #[test]
    fn to_nuon_converts_columns_with_spaces() {
        assert!(from_nuon(
            &to_nuon(
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
                ToStyle::Raw,
                None
            )
            .unwrap(),
            None,
        )
        .is_ok());
    }

    #[test]
    fn to_nuon_quotes_empty_string() {
        let res = to_nuon(&Value::test_string(""), ToStyle::Raw, None);
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
                    PathMember::string("foo".to_string(), false, Span::new(2, 5)),
                    PathMember::string("bar".to_string(), false, Span::new(6, 9)),
                    PathMember::int(0, false, Span::new(10, 11)),
                ],
            })),
        );
    }

    #[test]
    fn does_not_quote_strings_unnecessarily() {
        assert_eq!(
            to_nuon(
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
                ToStyle::Raw,
                None
            )
            .unwrap(),
            "[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]"
        );

        assert_eq!(
            to_nuon(
                &Value::test_record(record!(
                    "ro name" => Value::test_string("sam"),
                    "rank" => Value::test_int(10)
                )),
                ToStyle::Raw,
                None
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
        assert!(from_nuon(
            include_str!("../../../tests/fixtures/formats/code.nu"),
            None,
        )
        .is_err());
    }
}
