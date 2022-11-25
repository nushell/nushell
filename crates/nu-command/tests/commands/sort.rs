use nu_test_support::{nu, pipeline};


#[test]
fn by_invalid_types() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | echo ["foo" 1]
            | sort
            | to json -r
        "#
    ));

    let json_output = r#"[1,"foo"]"#;
    assert_eq!(actual.out, json_output);
}

#[test]
fn sort_primitive_values() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml --raw
            | lines
            | skip 1
            | first 6
            | sort
            | first
        "#
    ));

    assert_eq!(actual.out, "authors = [\"The Nushell Project Developers\"]");
}

#[test]
fn sort_different_types() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            [a, 1, b, 2, c, 3, [4, 5, 6], d, 4, [1, 2, 3]] | sort | to json --raw
        "#
    ));

    let json_output = r#"[1,2,3,4,"a","b","c","d",[1,2,3],[4,5,6]]"#;
    assert_eq!(actual.out, json_output);
}
