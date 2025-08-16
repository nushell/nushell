use nu_test_support::{nu, pipeline};

#[test]
fn to_nuon_correct_compaction() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open appveyor.yml
            | to nuon
            | str length
            | $in > 500
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_list_of_numbers() {
    let actual = nu!(pipeline(
        r#"
            [1, 2, 3, 4]
            | to nuon
            | from nuon
            | $in == [1, 2, 3, 4]
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_list_of_strings() {
    let actual = nu!(pipeline(
        r#"
            [abc, xyz, def]
            | to nuon
            | from nuon
            | $in == [abc, xyz, def]
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_table() {
    let actual = nu!(pipeline(
        r#"
            [[my, columns]; [abc, xyz], [def, ijk]]
            | to nuon
            | from nuon
            | $in == [[my, columns]; [abc, xyz], [def, ijk]]
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn from_nuon_illegal_table() {
    let actual = nu!(pipeline(
        r#"
            "[[repeated repeated]; [abc, xyz], [def, ijk]]"
            | from nuon
        "#
    ));

    assert!(actual.err.contains("column_defined_twice"));
}

#[test]
fn to_nuon_bool() {
    let actual = nu!(pipeline(
        r#"
            false
            | to nuon
            | from nuon
        "#
    ));

    assert_eq!(actual.out, "false");
}

#[test]
fn to_nuon_escaping() {
    let actual = nu!(pipeline(
        r#"
            "hello\"world"
            | to nuon
            | from nuon
        "#
    ));

    assert_eq!(actual.out, "hello\"world");
}

#[test]
fn to_nuon_escaping2() {
    let actual = nu!(pipeline(
        r#"
            "hello\\world"
            | to nuon
            | from nuon
        "#
    ));

    assert_eq!(actual.out, "hello\\world");
}

#[test]
fn to_nuon_escaping3() {
    let actual = nu!(pipeline(
        r#"
            ["hello\\world"]
            | to nuon
            | from nuon
            | $in == [hello\world]
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_escaping4() {
    let actual = nu!(pipeline(
        r#"
            ["hello\"world"]
            | to nuon
            | from nuon
            | $in == ["hello\"world"]
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_escaping5() {
    let actual = nu!(pipeline(
        r#"
            {s: "hello\"world"}
            | to nuon
            | from nuon
            | $in == {s: "hello\"world"}
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_negative_int() {
    let actual = nu!(pipeline(
        r#"
            -1
            | to nuon
            | from nuon
        "#
    ));

    assert_eq!(actual.out, "-1");
}

#[test]
fn to_nuon_records() {
    let actual = nu!(pipeline(
        r#"
            {name: "foo bar", age: 100, height: 10}
            | to nuon
            | from nuon
            | $in == {name: "foo bar", age: 100, height: 10}
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn to_nuon_range() {
    let actual = nu!(r#"1..42 | to nuon"#);
    assert_eq!(actual.out, "1..42");

    let actual = nu!(r#"1..<42 | to nuon"#);
    assert_eq!(actual.out, "1..<42");

    let actual = nu!(r#"1..4..42 | to nuon"#);
    assert_eq!(actual.out, "1..4..42");

    let actual = nu!(r#"1..4..<42 | to nuon"#);
    assert_eq!(actual.out, "1..4..<42");

    let actual = nu!(r#"1.0..42.0 | to nuon"#);
    assert_eq!(actual.out, "1.0..42.0");

    let actual = nu!(r#"1.0..<42.0 | to nuon"#);
    assert_eq!(actual.out, "1.0..<42.0");

    let actual = nu!(r#"1.0..4.0..42.0 | to nuon"#);
    assert_eq!(actual.out, "1.0..4.0..42.0");

    let actual = nu!(r#"1.0..4.0..<42.0 | to nuon"#);
    assert_eq!(actual.out, "1.0..4.0..<42.0");
}

#[test]
fn from_nuon_range() {
    let actual = nu!(r#"'1..42' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1..42");

    let actual = nu!(r#"'1..<42' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1..<42");

    let actual = nu!(r#"'1..4..42' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1..4..42");

    let actual = nu!(r#"'1..4..<42' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1..4..<42");

    let actual = nu!(r#"'1.0..42.0' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1.0..42.0");

    let actual = nu!(r#"'1.0..<42.0' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1.0..<42.0");

    let actual = nu!(r#"'1.0..4.0..42.0' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1.0..4.0..42.0");

    let actual = nu!(r#"'1.0..4.0..<42.0' | from nuon | to nuon"#);
    assert_eq!(actual.out, "1.0..4.0..<42.0");
}

#[test]
fn to_nuon_filesize() {
    let actual = nu!(pipeline(
        r#"
            1kib
            | to nuon
        "#
    ));

    assert_eq!(actual.out, "1024b");
}

#[test]
fn from_nuon_filesize() {
    let actual = nu!(pipeline(
        r#"
            "1024b"
            | from nuon
            | describe
        "#
    ));

    assert_eq!(actual.out, "filesize");
}

#[test]
fn to_nuon_duration() {
    let actual = nu!(pipeline(
        r#"
            1min
            | to nuon
        "#
    ));

    assert_eq!(actual.out, "60000000000ns");
}

#[test]
fn from_nuon_duration() {
    let actual = nu!(pipeline(
        r#"
            "60000000000ns"
            | from nuon
            | describe
        "#
    ));

    assert_eq!(actual.out, "duration");
}

#[test]
fn to_nuon_datetime() {
    let actual = nu!(pipeline(
        r#"
            2019-05-10
            | to nuon
        "#
    ));

    assert_eq!(actual.out, "2019-05-10T00:00:00+00:00");
}

#[test]
fn from_nuon_datetime() {
    let actual = nu!(pipeline(
        r#"
            "2019-05-10T00:00:00+00:00"
            | from nuon
            | describe
        "#
    ));

    assert_eq!(actual.out, "datetime");
}

#[test]
fn to_nuon_errs_on_closure() {
    let actual = nu!(pipeline(
        r#"
            {|| to nuon}
            | to nuon
        "#
    ));

    assert!(actual.err.contains("not deserializable"));
}

#[test]
fn to_nuon_closure_coerced_to_quoted_string() {
    let actual = nu!(pipeline(
        r#"
            {|| to nuon}
            | to nuon --serialize
        "#
    ));

    assert_eq!(actual.out, "\"{|| to nuon}\"");
}

#[test]
fn binary_to() {
    let actual = nu!(pipeline(
        r#"
            0x[ab cd ef] | to nuon
        "#
    ));

    assert_eq!(actual.out, "0x[ABCDEF]");
}

#[test]
fn binary_roundtrip() {
    let actual = nu!(pipeline(
        r#"
            "0x[1f ff]" | from nuon | to nuon
        "#
    ));

    assert_eq!(actual.out, "0x[1FFF]");
}

#[test]
fn read_binary_data() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.nuon | get 5.3
        "#
    ));

    assert_eq!(actual.out, "31")
}

#[test]
fn read_record() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.nuon | get 4.name
        "#
    ));

    assert_eq!(actual.out, "Bobby")
}

#[test]
fn read_bool() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open sample.nuon | get 3 | $in == true
        "#
    ));

    assert_eq!(actual.out, "true")
}

#[test]
fn float_doesnt_become_int() {
    let actual = nu!(pipeline(
        r#"
            1.0 | to nuon
        "#
    ));

    assert_eq!(actual.out, "1.0")
}

#[test]
fn float_inf_parsed_properly() {
    let actual = nu!(pipeline(
        r#"
            inf | to nuon
        "#
    ));

    assert_eq!(actual.out, "inf")
}

#[test]
fn float_neg_inf_parsed_properly() {
    let actual = nu!(pipeline(
        r#"
            -inf | to nuon
        "#
    ));

    assert_eq!(actual.out, "-inf")
}

#[test]
fn float_nan_parsed_properly() {
    let actual = nu!(pipeline(
        r#"
            NaN | to nuon
        "#
    ));

    assert_eq!(actual.out, "NaN")
}

#[test]
fn to_nuon_converts_columns_with_spaces() {
    let actual = nu!(pipeline(
        r#"
    let test = [[a, b, "c d"]; [1 2 3] [4 5 6]]; $test | to nuon | from nuon
    "#
    ));
    assert!(actual.err.is_empty());
}

#[test]
fn to_nuon_quotes_empty_string() {
    let actual = nu!(pipeline(
        r#"
    let test = ""; $test | to nuon
    "#
    ));
    assert!(actual.err.is_empty());
    assert_eq!(actual.out, r#""""#)
}

#[test]
fn to_nuon_quotes_empty_string_in_list() {
    let actual = nu!(pipeline(
        r#"
    let test = [""]; $test | to nuon | from nuon | $in == [""]
    "#
    ));
    assert!(actual.err.is_empty());
    assert_eq!(actual.out, "true")
}

#[test]
fn to_nuon_quotes_empty_string_in_table() {
    let actual = nu!(pipeline(
        r#"
    let test = [[a, b]; ['', la] [le lu]]; $test | to nuon | from nuon
    "#
    ));
    assert!(actual.err.is_empty());
}

#[test]
fn does_not_quote_strings_unnecessarily() {
    let actual = nu!(pipeline(
        r#"
        let test = [["a", "b", "c d"]; [1 2 3] [4 5 6]]; $test | to nuon
    "#
    ));
    assert_eq!(actual.out, "[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]");
    let actual = nu!(pipeline(
        r#"
         let a = {"ro name": "sam" rank: 10}; $a | to nuon
    "#
    ));
    assert_eq!(actual.out, "{\"ro name\": sam, rank: 10}");
}

#[test]
fn quotes_some_strings_necessarily() {
    let actual = nu!(pipeline(
        r#"
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
        "#
    ));

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn quotes_some_strings_necessarily_in_record_keys() {
    let actual = nu!(pipeline(
        r#"
        ['=', 'a=', '=a'] | each {
           {$in : 42}
        } | reduce {|elt, acc| $acc | merge $elt} | to nuon | from nuon | columns | describe
        "#
    ));

    assert_eq!(actual.out, "list<string>");
}

#[test]
fn read_code_should_fail_rather_than_panic() {
    let actual = nu!(cwd: "tests/fixtures/formats", pipeline(
        r#"open code.nu | from nuon"#
    ));
    assert!(actual.err.contains("Error when loading"))
}
