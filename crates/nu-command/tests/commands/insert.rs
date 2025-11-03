use nu_test_support::nu;

#[test]
fn insert_the_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | insert dev-dependencies.new_assertions "0.7.0"
        | get dev-dependencies.new_assertions
    "#);

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn doesnt_convert_record_to_table() {
    let actual = nu!("{a:1} | insert b 2 | to nuon");

    assert_eq!(actual.out, "{a: 1, b: 2}");
}

#[test]
fn insert_the_column_conflict() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | insert dev-dependencies.pretty_assertions "0.7.0"
    "#);

    assert!(
        actual
            .err
            .contains("column 'pretty_assertions' already exists")
    );
}

#[test]
fn insert_into_list() {
    let actual = nu!("[1, 2, 3] | insert 1 abc | to json -r");

    assert_eq!(actual.out, r#"[1,"abc",2,3]"#);
}

#[test]
fn insert_at_start_of_list() {
    let actual = nu!("[1, 2, 3] | insert 0 abc | to json -r");

    assert_eq!(actual.out, r#"["abc",1,2,3]"#);
}

#[test]
fn insert_at_end_of_list() {
    let actual = nu!("[1, 2, 3] | insert 3 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn insert_past_end_of_list() {
    let actual = nu!("[1, 2, 3] | insert 5 abc");

    assert!(
        actual
            .err
            .contains("can't insert at index (the next available index is 3)")
    );
}

#[test]
fn insert_into_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | insert 1 abc | to json -r");

    assert_eq!(actual.out, r#"[1,"abc",2,3]"#);
}

#[test]
fn insert_at_end_of_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | insert 3 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn insert_past_end_of_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | insert 5 abc");

    assert!(
        actual
            .err
            .contains("can't insert at index (the next available index is 3)")
    );
}

#[test]
fn insert_uses_enumerate_index() {
    let actual = nu!(
        "[[a]; [7] [6]] | enumerate | insert b {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"
    );

    assert_eq!(actual.out, "[[index, a, b]; [0, 7, 8], [1, 6, 8]]");
}

#[test]
fn deep_cell_path_creates_all_nested_records() {
    let actual = nu!("{a: {}} | insert a.b.c 0 | get a.b.c");
    assert_eq!(actual.out, "0");
}

#[test]
fn inserts_all_rows_in_table_in_record() {
    let actual = nu!(
        "{table: [[col]; [{a: 1}], [{a: 1}]]} | insert table.col.b 2 | get table.col.b | to nuon"
    );
    assert_eq!(actual.out, "[2, 2]");
}

#[test]
fn list_replacement_closure() {
    let actual = nu!("[1, 2] | insert 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3, 2]");

    let actual = nu!("[1, 2] | insert 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3, 2]");

    let actual = nu!("[1, 2] | insert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual = nu!("[1, 2] | insert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");
}

#[test]
fn record_replacement_closure() {
    let actual = nu!("{ a: text } | insert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "{a: text, b: TEXT}");

    let actual = nu!("{ a: text } | insert b { $in.a | str upcase } | to nuon");
    assert_eq!(actual.out, "{a: text, b: TEXT}");

    let actual = nu!("{ a: { b: 1 } } | insert a.c {|r| $r.a.b } | to nuon");
    assert_eq!(actual.out, "{a: {b: 1, c: 1}}");

    let actual = nu!("{ a: { b: 1 } } | insert a.c { $in.a.b } | to nuon");
    assert_eq!(actual.out, "{a: {b: 1, c: 1}}");
}

#[test]
fn table_replacement_closure() {
    let actual = nu!("[[a]; [text]] | insert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[a]; [text]] | insert b { $in.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[b]; [1]] | wrap a | insert a.c {|r| $r.a.b } | to nuon");
    assert_eq!(actual.out, "[[a]; [{b: 1, c: 1}]]");

    let actual = nu!("[[b]; [1]] | wrap a | insert a.c { $in.a.b } | to nuon");
    assert_eq!(actual.out, "[[a]; [{b: 1, c: 1}]]");
}

#[test]
fn list_stream_replacement_closure() {
    let actual = nu!("[1, 2] | every 1 | insert 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3, 2]");

    let actual = nu!("[1, 2] | every 1 | insert 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3, 2]");

    let actual =
        nu!("[1, 2] | every 1 | insert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual =
        nu!("[1, 2] | every 1 | insert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual = nu!("[[a]; [text]] | every 1 | insert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[a]; [text]] | every 1 | insert b { $in.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");
}
