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
fn doesnt_convert_record_to_table() {
    let actual = nu!(
        cwd: ".", r#"{a:1} | insert b 2 | to nuon"#
    );

    assert_eq!(actual.out, "{a: 1, b: 2}");
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

    assert!(actual
        .err
        .contains("column 'pretty_assertions' already exists"));
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

#[test]
fn insert_uses_enumerate_index() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"[[a]; [7] [6]] | enumerate | insert b {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"#
    ));

    assert_eq!(actual.out, "[[index, a, b]; [0, 7, 8], [1, 6, 8]]");
}
