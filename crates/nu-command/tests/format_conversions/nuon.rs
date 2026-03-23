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
    test()
        .run_with_data(code, test_data.clone())
        .expect_value_eq(test_data)
}

#[test]
fn to_nuon_correct_compaction() -> Result {
    let code = "
        open appveyor.yml
        | to nuon
        | str length
        | $in > 500
    ";

    let outcome: bool = test().cwd("tests/fixtures/formats").run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_list_of_numbers() -> Result {
    let code = "
        [1, 2, 3, 4]
        | to nuon
        | from nuon
        | $in == [1, 2, 3, 4]
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_list_of_strings() -> Result {
    let code = "
        [abc, xyz, def]
        | to nuon
        | from nuon
        | $in == [abc, xyz, def]
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_table() -> Result {
    let code = "
        [[my, columns]; [abc, xyz], [def, ijk]]
        | to nuon
        | from nuon
        | $in == [[my, columns]; [abc, xyz], [def, ijk]]
    ";

    let outcome: bool = test().run(code)?;
    assert!(outcome);
    Ok(())
}

#[test]
fn to_nuon_table_as_list_of_records() -> Result {
    let code = "
        [[a, b]; [1, 2], [3, 4]]
        | to nuon --list-of-records
    ";

    test()
        .run(code)
        .expect_value_eq("[{a: 1, b: 2}, {a: 3, b: 4}]")
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
    let code = "
        {outer: [[a, b]; [1, 2], [3, 4]]}
        | to nuon --list-of-records
    ";

    test()
        .run(code)
        .expect_value_eq("{outer: [{a: 1, b: 2}, {a: 3, b: 4}]}")
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
    let code = "
        false
        | to nuon
        | from nuon
    ";

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

    test().run(code).expect_value_eq("hello\"world")
}

#[test]
fn to_nuon_escaping2() -> Result {
    let code = r#"
        "hello\\world"
        | to nuon
        | from nuon
    "#;

    test().run(code).expect_value_eq("hello\\world")
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
    let code = "
        -1
        | to nuon
        | from nuon
    ";

    test().run(code).expect_value_eq(-1)
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
    test().run(code).expect_value_eq("1..42")?;

    let code = "1..<42 | to nuon";
    test().run(code).expect_value_eq("1..<42")?;

    let code = "1..4..42 | to nuon";
    test().run(code).expect_value_eq("1..4..42")?;

    let code = "1..4..<42 | to nuon";
    test().run(code).expect_value_eq("1..4..<42")?;

    let code = "1.0..42.0 | to nuon";
    test().run(code).expect_value_eq("1.0..42.0")?;

    let code = "1.0..<42.0 | to nuon";
    test().run(code).expect_value_eq("1.0..<42.0")?;

    let code = "1.0..4.0..42.0 | to nuon";
    test().run(code).expect_value_eq("1.0..4.0..42.0")?;

    let code = "1.0..4.0..<42.0 | to nuon";
    test().run(code).expect_value_eq("1.0..4.0..<42.0")
}

#[test]
fn from_nuon_range() -> Result {
    let code = "'1..42' | from nuon | to nuon";
    test().run(code).expect_value_eq("1..42")?;

    let code = "'1..<42' | from nuon | to nuon";
    test().run(code).expect_value_eq("1..<42")?;

    let code = "'1..4..42' | from nuon | to nuon";
    test().run(code).expect_value_eq("1..4..42")?;

    let code = "'1..4..<42' | from nuon | to nuon";
    test().run(code).expect_value_eq("1..4..<42")?;

    let code = "'1.0..42.0' | from nuon | to nuon";
    test().run(code).expect_value_eq("1.0..42.0")?;

    let code = "'1.0..<42.0' | from nuon | to nuon";
    test().run(code).expect_value_eq("1.0..<42.0")?;

    let code = "'1.0..4.0..42.0' | from nuon | to nuon";
    test().run(code).expect_value_eq("1.0..4.0..42.0")?;

    let code = "'1.0..4.0..<42.0' | from nuon | to nuon";
    test().run(code).expect_value_eq("1.0..4.0..<42.0")
}

#[test]
fn to_nuon_filesize() -> Result {
    let code = "
        1kib
        | to nuon
    ";

    test().run(code).expect_value_eq("1024b")
}

#[test]
fn from_nuon_filesize() -> Result {
    let code = r#"
        "1024b"
        | from nuon
        | describe
    "#;

    test().run(code).expect_value_eq("filesize")
}

#[test]
fn to_nuon_duration() -> Result {
    let code = "
        1min
        | to nuon
    ";

    test().run(code).expect_value_eq("60000000000ns")
}

#[test]
fn from_nuon_duration() -> Result {
    let code = r#"
        "60000000000ns"
        | from nuon
        | describe
    "#;

    test().run(code).expect_value_eq("duration")
}

#[test]
fn to_nuon_datetime() -> Result {
    let code = "
        2019-05-10
        | to nuon
    ";

    test()
        .run(code)
        .expect_value_eq("2019-05-10T00:00:00+00:00")
}

#[test]
fn from_nuon_datetime() -> Result {
    let code = r#"
        "2019-05-10T00:00:00+00:00"
        | from nuon
        | describe
    "#;

    test().run(code).expect_value_eq("datetime")
}

#[test]
fn to_nuon_errs_on_closure() -> Result {
    let code = "
        {|| to nuon}
        | to nuon
    ";

    // the error is wrapped inside a value
    let err: ShellError = test().run(code)?;
    assert!(matches!(err, ShellError::UnsupportedInput { .. }));
    Ok(())
}

#[test]
fn to_nuon_closure_coerced_to_quoted_string() -> Result {
    let code = "
        {|| to nuon}
        | to nuon --serialize
    ";

    test().run(code).expect_value_eq("\"{|| to nuon}\"")
}

#[test]
fn binary_to() -> Result {
    let code = "0x[ab cd ef] | to nuon";

    test().run(code).expect_value_eq("0x[ABCDEF]")
}

#[test]
fn binary_roundtrip() -> Result {
    let code = r#""0x[1f ff]" | from nuon | to nuon"#;

    test().run(code).expect_value_eq("0x[1FFF]")
}

#[test]
fn read_binary_data() -> Result {
    let code = "open sample.nuon | get 5.3";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(31)
}

#[test]
fn read_record() -> Result {
    let code = "open sample.nuon | get 4.name";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("Bobby")
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

    test().run(code).expect_value_eq("1.0")
}

#[test]
fn float_inf_parsed_properly() -> Result {
    let code = "inf | to nuon";

    test().run(code).expect_value_eq("inf")
}

#[test]
fn float_neg_inf_parsed_properly() -> Result {
    let code = "-inf | to nuon";

    test().run(code).expect_value_eq("-inf")
}

#[test]
fn float_nan_parsed_properly() -> Result {
    let code = "NaN | to nuon";

    test().run(code).expect_value_eq("NaN")
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

    test().run(code).expect_value_eq(r#""""#)
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

    test()
        .run(code)
        .expect_value_eq("[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]")?;

    let code = r#"let a = {"ro name": "sam" rank: 10}; $a | to nuon"#;

    test()
        .run(code)
        .expect_value_eq("{\"ro name\": sam, rank: 10}")
}

#[test]
fn quotes_some_strings_necessarily() -> Result {
    let code = "
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
    ";

    test().run(code).expect_value_eq("list<string>")
}

#[test]
fn quotes_some_strings_necessarily_in_record_keys() -> Result {
    let code = "
        ['=', 'a=', '=a'] | each {
           {$in : 42}
        } | reduce {|elt, acc| $acc | merge $elt} | to nuon | from nuon | columns | describe
    ";

    test().run(code).expect_value_eq("list<string>")
}

#[test]
fn read_code_should_fail_rather_than_panic() -> Result {
    let code = "open code.nu | from nuon";

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    let ShellError::Generic(err) = err else {
        return Err(err.into());
    };
    assert_eq!(err.error, "error when loading nuon text");
    Ok(())
}
