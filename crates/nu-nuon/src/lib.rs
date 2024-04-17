//! Support for the NUON format.
//!
//! The NUON format is a superset of JSON designed to fit the feel of Nushell.
//! Some of its extra features are
//! - trailing commas are allowed
//! - quotes are not required around keys
mod from;
mod to;

pub use from::from_nuon;
pub use to::to_nuon;

#[cfg(test)]
mod tests {
    use nu_protocol::{ast::RangeInclusion, record, IntRange, Range, Span, Value};

    use crate::{from_nuon, to_nuon};

    /// test something of the form
    /// ```nushell
    /// $v | to nuon | from nuon | $in == $v
    /// ```
    fn nuon_back_and_forth(v: &str) {
        assert_eq!(
            to_nuon(&from_nuon(v, None, None).unwrap(), true, None, None, None).unwrap(),
            v
        );
    }

    #[test]
    fn to_nuon_list_of_numbers() {
        nuon_back_and_forth("[1, 2, 3]");
    }

    #[test]
    fn to_nuon_list_of_strings() {
        nuon_back_and_forth("[abc, xyz, def]");
    }

    #[test]
    fn to_nuon_table() {
        nuon_back_and_forth("[[my, columns]; [abc, xyz], [def, ijk]]");
    }

    #[test]
    fn from_nuon_illegal_table() {
        assert!(
            from_nuon("[[repeated repeated]; [abc, xyz], [def, ijk]]", None, None)
                .unwrap_err()
                .to_string()
                .contains("Record field or table column used twice: repeated")
        );
    }

    #[test]
    fn to_nuon_bool() {
        nuon_back_and_forth("false");
    }

    #[test]
    fn to_nuon_escaping() {
        nuon_back_and_forth(r#""hello\"world""#);
    }

    #[test]
    fn to_nuon_escaping2() {
        nuon_back_and_forth(r#""hello\\world""#);
    }

    #[test]
    fn to_nuon_escaping3() {
        nuon_back_and_forth(r#"[hello\\world]"#);
    }

    #[test]
    fn to_nuon_escaping4() {
        nuon_back_and_forth(r#"["hello\"world"]"#);
    }

    #[test]
    fn to_nuon_escaping5() {
        nuon_back_and_forth(r#"{s: "hello\"world"}"#);
    }

    #[test]
    fn to_nuon_negative_int() {
        nuon_back_and_forth("-1");
    }

    #[test]
    fn to_nuon_records() {
        nuon_back_and_forth(r#"{name: "foo bar", age: 100, height: 10}"#);
    }

    #[test]
    fn nuon_range() {
        let range = IntRange::new(
            Value::test_int(1),
            Value::test_int(2),
            Value::test_int(42),
            RangeInclusion::Inclusive,
            Span::unknown(),
        )
        .unwrap();

        assert_eq!(
            to_nuon(
                &Value::test_range(Range::IntRange(range)),
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "1..42"
        );

        assert_eq!(
            from_nuon("1..42", None, None).unwrap(),
            Value::test_range(Range::IntRange(range)),
        );
    }

    #[test]
    fn to_nuon_filesize() {
        assert_eq!(
            to_nuon(&Value::test_filesize(1024), true, None, None, None).unwrap(),
            "1024b"
        );
    }

    #[test]
    fn from_nuon_filesize() {
        assert_eq!(
            from_nuon("1kib", None, None).unwrap(),
            Value::test_filesize(1024),
        );
    }

    #[test]
    fn to_nuon_duration() {
        assert_eq!(
            to_nuon(
                &Value::test_duration(60_000_000_000),
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "60000000000ns"
        )
    }

    #[test]
    fn from_nuon_duration() {
        assert_eq!(
            from_nuon("60000000000ns", None, None).unwrap(),
            Value::test_duration(60_000_000_000),
        );
    }

    // #[test]
    // fn to_nuon_datetime() {
    //     let actual = nu!(pipeline(
    //         r#"
    //             2019-05-10
    //             | to nuon
    //         "#
    //     ));
    //
    //     assert_eq!(actual.out, "2019-05-10T00:00:00+00:00");
    // }

    // #[test]
    // fn from_nuon_datetime() {
    //     let actual = nu!(pipeline(
    //         r#"
    //             "2019-05-10T00:00:00+00:00"
    //             | from nuon
    //             | describe
    //         "#
    //     ));
    //
    //     assert_eq!(actual.out, "date");
    // }

    // #[test]
    // fn to_nuon_errs_on_closure() {
    //     let actual = nu!(pipeline(
    //         r#"
    //             {|| to nuon}
    //             | to nuon
    //         "#
    //     ));
    //
    //     assert!(actual.err.contains("can't convert closure to NUON"));
    // }

    #[test]
    fn binary_to() {
        assert_eq!(
            to_nuon(
                &Value::test_binary(vec![0xab, 0xcd, 0xef]),
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "0x[ABCDEF]"
        );
    }

    #[test]
    fn binary_roundtrip() {
        assert_eq!(
            to_nuon(
                &from_nuon("0x[1f ff]", None, None).unwrap(),
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "0x[1FFF]"
        );
    }

    #[test]
    fn read_binary_data() {
        assert_eq!(
            from_nuon(
                include_str!("../../../tests/fixtures/formats/sample.nuon"),
                None,
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
            to_nuon(&Value::test_float(1.0), true, None, None, None).unwrap(),
            "1.0"
        );
    }

    #[test]
    fn float_inf_parsed_properly() {
        assert_eq!(
            to_nuon(&Value::test_float(f64::INFINITY), true, None, None, None).unwrap(),
            "inf"
        );
    }

    #[test]
    fn float_neg_inf_parsed_properly() {
        assert_eq!(
            to_nuon(
                &Value::test_float(f64::NEG_INFINITY),
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "-inf"
        );
    }

    #[test]
    fn float_nan_parsed_properly() {
        assert_eq!(
            to_nuon(&Value::test_float(-f64::NAN), true, None, None, None).unwrap(),
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
                true,
                None,
                None,
                None
            )
            .unwrap(),
            None,
            None,
        )
        .is_ok());
    }

    #[test]
    fn to_nuon_quotes_empty_string() {
        let res = to_nuon(&Value::test_string(""), true, None, None, None);
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), r#""""#);
    }

    #[test]
    fn to_nuon_quotes_empty_string_in_list() {
        nuon_back_and_forth(r#"[""]"#);
    }

    #[test]
    fn to_nuon_quotes_empty_string_in_table() {
        nuon_back_and_forth("[[a, b]; [\"\", la], [le, lu]]");
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
                true,
                None,
                None,
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
                true,
                None,
                None,
                None
            )
            .unwrap(),
            "{\"ro name\": sam, rank: 10}"
        );
    }

    #[test]
    fn quotes_some_strings_necessarily() {
        nuon_back_and_forth(
            r#"["true", "false", "null", "NaN", "NAN", "nan", "+nan", "-nan", "inf", "+inf", "-inf", "INF", "Infinity", "+Infinity", "-Infinity", "INFINITY", "+19.99", "-19.99", "19.99b", "19.99kb", "19.99mb", "19.99gb", "19.99tb", "19.99pb", "19.99eb", "19.99zb", "19.99kib", "19.99mib", "19.99gib", "19.99tib", "19.99pib", "19.99eib", "19.99zib", "19ns", "19us", "19ms", "19sec", "19min", "19hr", "19day", "19wk", "-11.0..-15.0", "11.0..-15.0", "-11.0..15.0", "-11.0..<-15.0", "11.0..<-15.0", "-11.0..<15.0", "-11.0..", "11.0..", "..15.0", "..-15.0", "..<15.0", "..<-15.0", "2000-01-01", "2022-02-02T14:30:00", "2022-02-02T14:30:00+05:00", ", ", "", "&&"]"#,
        );
    }

    #[test]
    #[ignore = "can't find PWD in the CI..."]
    fn read_code_should_fail_rather_than_panic() {
        assert!(from_nuon(
            include_str!("../../../tests/fixtures/formats/code.nu"),
            None,
            None,
        )
        .unwrap_err()
        .to_string()
        .contains("Error when loading"));
    }
}
