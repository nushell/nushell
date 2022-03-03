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
