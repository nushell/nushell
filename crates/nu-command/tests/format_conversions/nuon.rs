use nu_test_support::{nu, pipeline};
use proptest::prelude::*;

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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
fn to_nuon_bool() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
fn binary_to() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            0x[ab cd ef] | to nuon
        "#
    ));

    assert_eq!(actual.out, "0x[ABCDEF]");
}

#[test]
fn binary_roundtrip() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
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
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            1.0 | to nuon
        "#
    ));

    assert_eq!(actual.out, "1.0")
}

#[test]
fn float_inf_parsed_properly() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            inf | to nuon
        "#
    ));

    assert_eq!(actual.out, "inf")
}

#[test]
fn float_neg_inf_parsed_properly() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            -inf | to nuon
        "#
    ));

    assert_eq!(actual.out, "-inf")
}

#[test]
fn float_nan_parsed_properly() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            NaN | to nuon
        "#
    ));

    assert_eq!(actual.out, "NaN")
}

#[test]
fn to_nuon_converts_columns_with_spaces() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
            r#"
    let test = [[a, b, "c d"]; [1 2 3] [4 5 6]]; $test | to nuon | from nuon
    "#
    ));
    assert!(actual.err.is_empty());
}

#[test]
fn to_nuon_does_not_quote_unnecessarily() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
            r#"
        let test = [["a", "b", "c d"]; [1 2 3] [4 5 6]]; $test | to nuon
    "#
    ));
    assert_eq!(actual.out, "[[a, b, \"c d\"]; [1, 2, 3], [4, 5, 6]]");
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
            r#"
         let a = {"ro name": "sam" rank: 10}; $a | to nuon
    "#
    ));
    assert_eq!(actual.out, "{\"ro name\": sam, rank: 10}");
}

proptest! {
    #[test]
    fn to_nuon_from_nuon(c: char) {
        if c != '\0' && c!='\r' {
        let actual = nu!(
            cwd: "tests/fixtures/formats", pipeline(
                format!(r#"
             {{"prop{0}test": "sam"}} | to nuon | from nuon;
             [ [ "prop{0}test" ]; [ 'test' ] ] | to nuon | from nuon;
             [ [ "{0}" ]; [ 'test' ] ] | to nuon | from nuon;
             {{"{0}": "sam"}} | to nuon | from nuon;
        "#, c).as_ref()
        ));
        assert!(actual.err.is_empty() || actual.err.contains("Unexpected end of code") || actual.err.contains("only strings can be keys"));
        // The second is for weird escapes due to backslashes
        // The third is for chars like '0'
        }
    }
    #[test]
    fn to_nuon_from_nuon_string(s: String) {
        if s != "\\0" && !s.is_empty() && !s.contains('\\') && !s.contains('"'){
        let actual = nu!(
            cwd: "tests/fixtures/formats", pipeline(
                format!(r#"
             {{"prop{0}test": "sam"}} | to nuon | from nuon;
             [ [ "prop{0}test" ]; [ 'test' ] ] | to nuon | from nuon;
             [ [ "{0}" ]; [ 'test' ] ] | to nuon | from nuon;
             {{"{0}": "sam"}} | to nuon | from nuon;
        "#, s).as_ref()
        ));
        assert!(actual.err.is_empty() || actual.err.contains("only strings can be keys") || actual.err.contains("unknown command"));
        // TODO: fix parser error for "unknown command" when '=$' is the name
    }
    }
}
