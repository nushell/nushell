use nu_test_support::{nu, pipeline};

#[test]
fn sets_the_column() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.pretty_assertions "0.7.0"
            | get dev-dependencies.pretty_assertions
        "#
    ));

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn doesnt_convert_record_to_table() {
    let actual = nu!("{a:1} | update a 2 | to nuon");

    assert_eq!(actual.out, "{a: 2}");
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            {content: null}
            | update content {|| open --raw cargo_sample.toml | lines | first 5 }
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn sets_the_column_from_a_subexpression() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            {content: null}
            | update content (open --raw cargo_sample.toml | lines | first 5)
            | get content.1
            | str contains "nu"
        "#
    ));

    assert_eq!(actual.out, "true");
}

#[test]
fn upsert_column_missing() {
    let actual = nu!(
        cwd: "tests/fixtures/formats", pipeline(
        r#"
            open cargo_sample.toml
            | update dev-dependencies.new_assertions "0.7.0"
        "#
    ));

    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn update_list() {
    let actual = nu!("[1, 2, 3] | update 1 abc | to json -r");
    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn update_past_end_list() {
    let actual = nu!("[1, 2, 3] | update 5 abc | to json -r");
    assert!(actual.err.contains("too large"));
}

#[test]
fn update_nonexistent_column() {
    let actual = nu!("{a:1} | update b 2");
    assert!(actual.err.contains("cannot find column 'b'"));
}

#[test]
fn update_uses_enumerate_index() {
    let actual = nu!(
        r#"[[a]; [7] [6]] | enumerate | update item.a {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"#
    );

    assert_eq!(actual.out, "[[index, a]; [0, 8], [1, 8]]");
}

#[test]
fn update_support_lazy_record() {
    let actual =
        nu!(r#"let x = (lazy make -c ["h"] -g {|a| $a | str upcase}); $x | update h 10 | get h"#);
    assert_eq!(actual.out, "10");
}
