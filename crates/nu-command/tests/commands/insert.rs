use nu_test_support::{nu, pipeline};

#[test]
fn insert_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.new_assertions "0.7.0"
            | get dev-dependencies.new_assertions
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn insert_the_column_conflict() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | insert dev-dependencies.pretty_assertions "0.7.0"
        "#
    ));

    assert!(actual.err.contains("column already exists"));
}

#[test]
fn insert_into_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | insert 1 abc | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[1,"abc",2,3]"#);
}

#[test]
fn insert_into_list_begin() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | insert 0 abc | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"["abc",1,2,3]"#);
}

#[test]
fn insert_into_list_end() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | insert 3 abc | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn insert_past_end_list() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
            [1, 2, 3] | insert 5 abc | to json -r
        "#
    ));

    assert_eq!(actual.out, r#"[1,2,3,null,null,"abc"]"#);
}
