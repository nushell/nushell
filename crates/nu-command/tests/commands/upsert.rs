use nu_test_support::nu;

#[test]
fn sets_the_column() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        open cargo_sample.toml
        | upsert dev-dependencies.pretty_assertions "0.7.0"
        | get dev-dependencies.pretty_assertions
    "#);

    assert_eq!(actual.out, "0.7.0");
}

#[test]
fn doesnt_convert_record_to_table() {
    let actual = nu!("{a:1} | upsert a 2 | to nuon");
    assert_eq!(actual.out, "{a: 2}");
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {content: null}
        | upsert content {|| open --raw cargo_sample.toml | lines | first 5 }
        | get content.1
        | str contains "nu"
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn sets_the_column_from_a_subexpression() {
    let actual = nu!(cwd: "tests/fixtures/formats", r#"
        {content: null}
        | upsert content (open --raw cargo_sample.toml | lines | first 5)
        | get content.1
        | str contains "nu"
    "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn upsert_uses_enumerate_index_inserting() {
    let actual = nu!(
        "[[a]; [7] [6]] | enumerate | upsert b {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"
    );

    assert_eq!(actual.out, "[[index, a, b]; [0, 7, 8], [1, 6, 8]]");
}

#[test]
fn upsert_uses_enumerate_index_updating() {
    let actual = nu!(
        "[[a]; [7] [6]] | enumerate | upsert a {|el| $el.index + 1 + $el.item.a } | flatten | to nuon"
    );

    assert_eq!(actual.out, "[[index, a]; [0, 8], [1, 8]]");
}

#[test]
fn upsert_into_list() {
    let actual = nu!("[1, 2, 3] | upsert 1 abc | to json -r");

    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn upsert_at_end_of_list() {
    let actual = nu!("[1, 2, 3] | upsert 3 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn upsert_past_end_of_list() {
    let actual = nu!("[1, 2, 3] | upsert 5 abc");

    assert!(
        actual
            .err
            .contains("can't insert at index (the next available index is 3)")
    );
}

#[test]
fn upsert_into_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | upsert 1 abc | to json -r");

    assert_eq!(actual.out, r#"[1,"abc",3]"#);
}

#[test]
fn upsert_at_end_of_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | upsert 3 abc | to json -r");

    assert_eq!(actual.out, r#"[1,2,3,"abc"]"#);
}

#[test]
fn upsert_past_end_of_list_stream() {
    let actual = nu!("[1, 2, 3] | every 1 | upsert 5 abc");

    assert!(
        actual
            .err
            .contains("can't insert at index (the next available index is 3)")
    );
}

#[test]
fn deep_cell_path_creates_all_nested_records() {
    let actual = nu!("{a: {}} | upsert a.b.c 0 | get a.b.c");
    assert_eq!(actual.out, "0");
}

#[test]
fn upserts_all_rows_in_table_in_record() {
    let actual = nu!(
        "{table: [[col]; [{a: 1}], [{a: 1}]]} | upsert table.col.b 2 | get table.col.b | to nuon"
    );
    assert_eq!(actual.out, "[2, 2]");
}

#[test]
fn list_replacement_closure() {
    let actual = nu!("[1, 2] | upsert 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[1, 2] | upsert 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[1, 2] | upsert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual = nu!("[1, 2] | upsert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");
}

#[test]
fn record_replacement_closure() {
    let actual = nu!("{ a: text } | upsert a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "{a: TEXT}");

    let actual = nu!("{ a: text } | upsert a { str upcase } | to nuon");
    assert_eq!(actual.out, "{a: TEXT}");

    let actual = nu!("{ a: text } | upsert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "{a: text, b: TEXT}");

    let actual = nu!("{ a: text } | upsert b { default TEXT } | to nuon");
    assert_eq!(actual.out, "{a: text, b: TEXT}");

    let actual = nu!("{ a: { b: 1 } } | upsert a.c {|r| $r.a.b } | to nuon");
    assert_eq!(actual.out, "{a: {b: 1, c: 1}}");

    let actual = nu!("{ a: { b: 1 } } | upsert a.c { default 0 } | to nuon");
    assert_eq!(actual.out, "{a: {b: 1, c: 0}}");
}

#[test]
fn table_replacement_closure() {
    let actual = nu!("[[a]; [text]] | upsert a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | upsert a { str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | upsert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[a]; [text]] | upsert b { default TEXT } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[b]; [1]] | wrap a | upsert a.c {|r| $r.a.b } | to nuon");
    assert_eq!(actual.out, "[[a]; [{b: 1, c: 1}]]");

    let actual = nu!("[[b]; [1]] | wrap a | upsert a.c { default 0 } | to nuon");
    assert_eq!(actual.out, "[[a]; [{b: 1, c: 0}]]");
}

#[test]
fn list_stream_replacement_closure() {
    let actual = nu!("[1, 2] | every 1 | upsert 1 {|i| $i + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual = nu!("[1, 2] | every 1 | upsert 1 { $in + 1 } | to nuon");
    assert_eq!(actual.out, "[1, 3]");

    let actual =
        nu!("[1, 2] | every 1 | upsert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual =
        nu!("[1, 2] | every 1 | upsert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon");
    assert_eq!(actual.out, "[1, 2, 0]");

    let actual = nu!("[[a]; [text]] | every 1 | upsert a {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | every 1 | upsert a { str upcase } | to nuon");
    assert_eq!(actual.out, "[[a]; [TEXT]]");

    let actual = nu!("[[a]; [text]] | every 1 | upsert b {|r| $r.a | str upcase } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");

    let actual = nu!("[[a]; [text]] | every 1 | upsert b { default TEXT } | to nuon");
    assert_eq!(actual.out, "[[a, b]; [text, TEXT]]");
}
