use nu_test_support::nu;

#[test]
fn sets_the_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | update dev-dependencies.pretty_assertions "0.7.0"
        | get dev-dependencies.pretty_assertions
    "#);

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn doesnt_convert_record_to_table() {
    let actual = nu!("{a:1} | update a 2 | to nuon");

    assert_eq!(actual.out, "{a: 2}");
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {content: null}
        | update content {|| open --raw cargo_sample.toml | lines | first 5 }
        | get content.1
        | str contains "nu"
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn sets_the_column_from_a_subexpression() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {content: null}
        | update content (open --raw cargo_sample.toml | lines | first 5)
        | get content.1
        | str contains "nu"
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn upsert_column_missing() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | update dev-dependencies.new_assertions "0.7.0"
    "#);

    assert!(actual.err.contains("cannot find column"));
}

#[test]
fn update_list() {
    let actual = nu!("[1, 2, 3] | update 1 abc | to json -r");
    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn update_past_end_of_list() {
    let actual = nu!("[1, 2, 3] | update 5 abc | to json -r");
    assert!(actual.err.contains("too large"));
}

#[test]
fn update_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | update 1 abc | to json -r");
    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn update_past_end_of_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | update 5 abc | to json -r");
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
        "[[a]; [7] [6]] | enumerate | update item.a {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"
    );

    assert_eq!(actual.out, "[[index, a]; [0, 8], [1, 8]]");
}

#[test]
fn list_replacement_closure() {
    let actual = nu!("[1, 2] | update 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[1, 2] | update 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");
}

#[test]
fn record_replacement_closure() {
    let actual = nu!("{ a: text } | update a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "{a: TEXT}");

    let actual = nu!("{ a: text } | update a { str upcase } | to nuon");
    assert_eq!(actual.out, "{a: TEXT}");
}

#[test]
fn table_replacement_closure() {
    let actual = nu!("[[a]; [text]] | update a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | update a { str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");
}

#[test]
fn list_stream_replacement_closure() {
    let actual = nu!("[1, 2] | every 1 | update 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[1, 2] | every 1 | update 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[[a]; [text]] | every 1 | update a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | every 1 | update a { str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");
}

#[test]
fn update_optional_column_present() {
    let actual = nu!("{a: 1} | update a? 2 | to nuon");
    assert_eq!(actual.out, "{a: 2}");
}

#[test]
fn update_optional_column_absent() {
    let actual = nu!("{a: 1} | update b? 2 | to nuon");
    assert_eq!(actual.out, "{a: 1}");
}

#[test]
fn update_optional_column_in_table_present() {
    let actual = nu!("[[a, b]; [1, 2], [3, 4]] | update a? 10 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [10, 2], [10, 4]]");
}

#[test]
fn update_optional_column_in_table_absent() {
    let actual = nu!("[[a, b]; [1, 2], [3, 4]] | update c? 10 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2], [3, 4]]");
}

#[test]
fn update_optional_column_in_table_mixed() {
    let actual = nu!("[{a: 1, b: 2}, {b: 3}, {a: 4, b: 5}] | update a? 10 | to nuon");
    assert_eq!(actual.out, "[{a: 10, b: 2}, {b: 3}, {a: 10, b: 5}]");
}

#[test]
fn update_optional_index_present() {
    let actual = nu!("[1, 2, 3] | update 1? 10 | to nuon");
    assert_eq!(actual.out, "[1, 10, 3]");
}

#[test]
fn update_optional_index_absent() {
    let actual = nu!("[1, 2, 3] | update 5? 10 | to nuon");
    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn update_optional_column_with_closure_present() {
    let actual = nu!("{a: 5} | update a? {|x| $x.a * 2 } | to nuon");
    assert_eq!(actual.out, "{a: 10}");
}

#[test]
fn update_optional_column_with_closure_absent() {
    let actual = nu!("{a: 5} | update b? {|x| 10 } | to nuon");
    assert_eq!(actual.out, "{a: 5}");
}

#[test]
fn update_optional_column_in_table_with_closure() {
    let actual = nu!("[[a]; [1], [2]] | update a? { $in * 2 } | to nuon");
    assert_eq!(actual.out, "[[a]; [2], [4]]");
}

#[test]
fn update_optional_column_in_table_with_closure_mixed() {
    let actual = nu!("[{a: 1, b: 2}, {b: 3}, {a: 4, b: 5}] | update a? { $in * 10 } | to nuon");
    assert_eq!(actual.out, "[{a: 10, b: 2}, {b: 3}, {a: 40, b: 5}]");
}

#[test]
fn update_optional_index_with_closure_present() {
    let actual = nu!("[1, 2, 3] | update 1? { $in * 10 } | to nuon");
    assert_eq!(actual.out, "[1, 20, 3]");
}

#[test]
fn update_optional_index_with_closure_absent() {
    let actual = nu!("[1, 2, 3] | update 5? { $in * 10 } | to nuon");
    assert_eq!(actual.out, "[1, 2, 3]");
}

#[test]
fn update_optional_in_list_stream() {
    let actual = nu!("[[a, b]; [1, 2], [3, 4]] | every 1 | update c? 10 | to nuon");
    assert_eq!(actual.out, "[[a, b]; [1, 2], [3, 4]]");
}

#[test]
fn update_optional_in_list_stream_with_closure() {
    let actual = nu!("[{a: 1}, {b: 2}, {a: 3}] | every 1 | update a? { $in * 10 } | to nuon");
    assert_eq!(actual.out, "[{a: 10}, {b: 2}, {a: 30}]");
}
