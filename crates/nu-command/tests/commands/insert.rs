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
    let actual = nu!("{a:1} | insert b 2 | to nuon");

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
    let actual = nu!("[1, 2, 3] | insert 1 abc | to json -r");

    assert_eq!(actual.out, r#"[1,"abc",2,3]"#);
}

#[test]
fn insert_into_list_begin() {
    let actual = nu!("[1, 2, 3] | insert 0 abc | to json -r");

    assert_eq!(actual.out, r#"["abc",1,2,3]"#);
}

#[test]
fn insert_into_list_end() {
    let actual = nu!("[1, 2, 3] | insert 3 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn insert_past_end_list() {
    let actual = nu!("[1, 2, 3] | insert 5 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,null,null,"abc"]"#);
}

#[test]
fn insert_uses_enumerate_index() {
    let actual = nu!(
        "[[a]; [7] [6]] | enumerate | insert b {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"
    );

    assert_eq!(actual.out, "[[index, a, b]; [0, 7, 8], [1, 6, 8]]");
}

#[test]
fn insert_support_lazy_record() {
    let actual =
        nu!(r#"let x = (lazy make -c ["h"] -g {|a| $a | str upcase}); $x | insert a 10 | get a"#);
    assert_eq!(actual.out, "10");
}

#[test]
fn lazy_record_test_values() {
    let actual = nu!(
        r#"lazy make --columns ["haskell", "futures", "nushell"] --get-value { |lazything| $lazything + "!" } | values | length"#
    );
    assert_eq!(actual.out, "3");
}

#[test]
fn deep_cell_path_creates_all_nested_records() {
    let actual = nu!(r#"{a: {}} | insert a.b.c 0 | get a.b.c"#);
    assert_eq!(actual.out, "0");
}

#[test]
fn inserts_all_rows_in_table_in_record() {
    let actual = nu!(
        r#"{table: [[col]; [{a: 1}], [{a: 1}]]} | insert table.col.b 2 | get table.col.b | to nuon"#
    );
    assert_eq!(actual.out, "[2, 2]");
}
