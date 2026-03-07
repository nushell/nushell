use nu_test_support::prelude::*;

#[test]
fn nuon_roundtrip() -> Result {
    #[derive(IntoValue, FromValue, Debug, Clone, PartialEq)]
    struct TestData {
        a: String,
        b: u32,
        c: Vec<u8>,
    }

    let test_data = TestData {
        a: "something".into(),
        b: 42,
        c: vec![1, 2, 3, 4],
    };

    let code = "to nuon | from nuon";
    let outcome: TestData = test().run_with_data(code, test_data.clone())?;
    assert_eq!(outcome, test_data);

    Ok(())
}

#[test]
fn to_nuon_correct_compaction() -> Result {
    let code = r#"
        open appveyor.yml
        | to nuon
        | str length
        | $in > 500
    "#;

    let outcome: bool = test().cwd("tests/fixtures/formats").run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_list_of_numbers() -> Result {
    let code = r#"
        [1, 2, 3, 4]
        | to nuon
        | from nuon
        | $in == [1, 2, 3, 4]
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_list_of_strings() -> Result {
    let code = r#"
        [abc, xyz, def]
        | to nuon
        | from nuon
        | $in == [abc, xyz, def]
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_table() -> Result {
    let code = r#"
        [[my, columns]; [abc, xyz], [def, ijk]]
        | to nuon
        | from nuon
        | $in == [[my, columns]; [abc, xyz], [def, ijk]]
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_table_as_list_of_records() -> Result {
    let code = r#"
        [[a, b]; [1, 2], [3, 4]]
        | to nuon --list-of-records
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "[{a: 1, b: 2}, {a: 3, b: 4}]");
    Ok(())
}

#[test]
fn to_nuon_table_as_list_of_records_indented() -> Result {
    let code = r#"
        [[a, b]; [1, 2], [3, 4]]
        | to nuon --list-of-records --indent 2
        | str contains "\n"
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_table_as_list_of_records_is_recursive() -> Result {
    let code = r#"
        {outer: [[a, b]; [1, 2], [3, 4]]}
        | to nuon --list-of-records
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "{outer: [{a: 1, b: 2}, {a: 3, b: 4}]}");
    Ok(())
}

#[test]
fn from_nuon_illegal_table() -> Result {
    let code = r#"
        "[[repeated repeated]; [abc, xyz], [def, ijk]]"
        | from nuon
    "#;

    let err = test().run(code).expect_shell_error()?;
    let inner = err.into_inner()?;
    assert!(matches!(inner, ShellError::ColumnDefinedTwice { .. }));
    Ok(())
}

#[test]
fn to_nuon_bool() -> Result {
    let code = r#"
        false
        | to nuon
        | from nuon
    "#;

    let outcome: bool = test().run(code)?;
    assert!(!outcome);
    Ok(())
}

#[test]
fn to_nuon_escaping() -> Result {
    let code = r#"
        "hello\"world"
        | to nuon
        | from nuon
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "hello\"world");
    Ok(())
}

#[test]
fn to_nuon_escaping2() -> Result {
    let code = r#"
        "hello\\world"
        | to nuon
        | from nuon
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "hello\\world");
    Ok(())
}

#[test]
fn to_nuon_escaping3() -> Result {
    let code = r#"
        ["hello\\world"]
        | to nuon
        | from nuon
        | $in == [hello\world]
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_escaping4() -> Result {
    let code = r#"
        ["hello\"world"]
        | to nuon
        | from nuon
        | $in == ["hello\"world"]
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_escaping5() -> Result {
    let code = r#"
        {s: "hello\"world"}
        | to nuon
        | from nuon
        | $in == {s: "hello\"world"}
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_negative_int() -> Result {
    let code = r#"
        -1
        | to nuon
        | from nuon
    "#;

    let outcome: i64 = test().run(code)?;
    assert_eq!(outcome, -1);
    Ok(())
}

#[test]
fn to_nuon_records() -> Result {
    let code = r#"
        {name: "foo bar", age: 100, height: 10}
        | to nuon
        | from nuon
        | $in == {name: "foo bar", age: 100, height: 10}
    "#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_range() -> Result {
    let code = "1..42 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..42");

    let code = "1..<42 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..<42");

    let code = "1..4..42 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..4..42");

    let code = "1..4..<42 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..4..<42");

    let code = "1.0..42.0 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..42.0");

    let code = "1.0..<42.0 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..<42.0");

    let code = "1.0..4.0..42.0 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..4.0..42.0");

    let code = "1.0..4.0..<42.0 | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..4.0..<42.0");
    Ok(())
}

#[test]
fn from_nuon_range() -> Result {
    let code = "'1..42' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..42");

    let code = "'1..<42' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..<42");

    let code = "'1..4..42' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..4..42");

    let code = "'1..4..<42' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1..4..<42");

    let code = "'1.0..42.0' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..42.0");

    let code = "'1.0..<42.0' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..<42.0");

    let code = "'1.0..4.0..42.0' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..4.0..42.0");

    let code = "'1.0..4.0..<42.0' | from nuon | to nuon";
    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0..4.0..<42.0");
    Ok(())
}

#[test]
fn to_nuon_filesize() -> Result {
    let code = r#"
        1kib
        | to nuon
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1024b");
    Ok(())
}

#[test]
fn from_nuon_filesize() -> Result {
    let code = r#"
        "1024b"
        | from nuon
        | describe
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "filesize");
    Ok(())
}

#[test]
fn to_nuon_duration() -> Result {
    let code = r#"
        1min
        | to nuon
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "60000000000ns");
    Ok(())
}

#[test]
fn from_nuon_duration() -> Result {
    let code = r#"
        "60000000000ns"
        | from nuon
        | describe
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "duration");
    Ok(())
}

#[test]
fn to_nuon_datetime() -> Result {
    let code = r#"
        2019-05-10
        | to nuon
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "2019-05-10T00:00:00+00:00");
    Ok(())
}

#[test]
fn from_nuon_datetime() -> Result {
    let code = r#"
        "2019-05-10T00:00:00+00:00"
        | from nuon
        | describe
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "datetime");
    Ok(())
}

#[test]
fn to_nuon_errs_on_closure() -> Result {
    let code = r#"
        {|| to nuon}
        | to nuon
    "#;

    let err = test().run(code).expect_shell_error()?;
    assert!(matches!(err, ShellError::CantConvert { .. }));
    Ok(())
}

#[test]
fn to_nuon_closure_coerced_to_quoted_string() -> Result {
    let code = r#"
        {|| to nuon}
        | to nuon --serialize
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "\"{|| to nuon}\"");
    Ok(())
}

#[test]
fn binary_to() -> Result {
    let code = "0x[ab cd ef] | to nuon";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "0x[ABCDEF]");
    Ok(())
}

#[test]
fn binary_roundtrip() -> Result {
    let code = r#""0x[1f ff]" | from nuon | to nuon"#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "0x[1FFF]");
    Ok(())
}

#[test]
fn read_binary_data() -> Result {
    let code = "open sample.nuon | get 5.3";

    let outcome: i64 = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, 31);
    Ok(())
}

#[test]
fn read_record() -> Result {
    let code = "open sample.nuon | get 4.name";

    let outcome: String = test().cwd("tests/fixtures/formats").run(code)?;
    assert_eq!(outcome, "Bobby");
    Ok(())
}

#[test]
fn read_bool() -> Result {
    let code = "open sample.nuon | get 3 | $in == true";

    let outcome: bool = test().cwd("tests/fixtures/formats").run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn float_doesnt_become_int() -> Result {
    let code = "1.0 | to nuon";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "1.0");
    Ok(())
}

#[test]
fn float_inf_parsed_properly() -> Result {
    let code = "inf | to nuon";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "inf");
    Ok(())
}

#[test]
fn float_neg_inf_parsed_properly() -> Result {
    let code = "-inf | to nuon";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "-inf");
    Ok(())
}

#[test]
fn float_nan_parsed_properly() -> Result {
    let code = "NaN | to nuon";

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "NaN");
    Ok(())
}

#[test]
fn to_nuon_converts_columns_with_spaces() -> Result {
    let code = r#"let test = [[a, b, "c d"]; [1 2 3] [4 5 6]]; $test | to nuon | from nuon"#;

    let _: Value = test().run(code)?;
    Ok(())
}

#[test]
fn to_nuon_quotes_empty_string() -> Result {
    let code = r#"let test = ""; $test | to nuon"#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, r#""""#);
    Ok(())
}

#[test]
fn to_nuon_quotes_empty_string_in_list() -> Result {
    let code = r#"let test = [""]; $test | to nuon | from nuon | $in == [""]"#;

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_quotes_empty_string_in_table() -> Result {
    let code = "let test = [[a, b]; ['', la] [le lu]]; $test | to nuon | from nuon";

    let _: Value = test().run(code)?;
    Ok(())
}

#[test]
fn does_not_quote_strings_unnecessarily() -> Result {
    let code = r#"let test = [["a", "b", "c d"]; [1 2 3] [4 5 6]]; $test | to nuon"#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]");

    let code = r#"let a = {"ro name": "sam" rank: 10}; $a | to nuon"#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "{\"ro name\": sam, rank: 10}");
    Ok(())
}

#[test]
fn quotes_some_strings_necessarily() -> Result {
    let code = r#"
        ['true','false','null',
        'NaN','NAN','nan','+nan','-nan',
        'inf','+inf','-inf','INF',
        'Infinity','+Infinity','-Infinity','INFINITY',
        '+19.99','-19.99', '19.99b',
        '19.99kb','19.99mb','19.99gb','19.99tb','19.99pb','19.99eb','19.99zb',
        '19.99kib','19.99mib','19.99gib','19.99tib','19.99pib','19.99eib','19.99zib',
        '19ns', '19us', '19ms', '19sec', '19min', '19hr', '19day', '19wk',
        '-11.0..-15.0', '11.0..-15.0', '-11.0..15.0',
        '-11.0..<-15.0', '11.0..<-15.0', '-11.0..<15.0',
        '-11.0..', '11.0..', '..15.0', '..-15.0', '..<15.0', '..<-15.0',
        '2000-01-01', '2022-02-02T14:30:00', '2022-02-02T14:30:00+05:00',
        ',',''
        '&&'
        ] | to nuon | from nuon | describe
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "list<string>");
    Ok(())
}

#[test]
fn quotes_some_strings_necessarily_in_record_keys() -> Result {
    let code = r#"
        ['=', 'a=', '=a'] | each {
           {$in : 42}
        } | reduce {|elt, acc| $acc | merge $elt} | to nuon | from nuon | columns | describe
    "#;

    let outcome: String = test().run(code)?;
    assert_eq!(outcome, "list<string>");
    Ok(())
}

#[test]
fn read_code_should_fail_rather_than_panic() -> Result {
    let code = "open code.nu | from nuon";

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    let ShellError::GenericError { error, .. } = err else {
        return Err(err.into());
    };
    assert_eq!(error, "error when loading nuon text");
    Ok(())
}
