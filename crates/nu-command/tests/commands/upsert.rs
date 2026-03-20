use nu_experimental::REORDER_CELL_PATHS;
use nu_test_support::prelude::*;

#[test]
fn sets_the_column() -> Result {
    let code = r#"
        open cargo_sample.toml
        | upsert dev-dependencies.pretty_assertions "0.7.0"
        | get dev-dependencies.pretty_assertions
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("0.7.0")
}

#[test]
fn doesnt_convert_record_to_table() -> Result {
    test()
        .run("{a:1} | upsert a 2 | to nuon")
        .expect_value_eq("{a: 2}")
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() -> Result {
    let code = r#"
        {content: null}
        | upsert content {|| open --raw cargo_sample.toml | lines | first 5 }
        | get content.1
        | str contains "nu"
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(true)
}

#[test]
fn sets_the_column_from_a_subexpression() -> Result {
    let code = r#"
        {content: null}
        | upsert content (open --raw cargo_sample.toml | lines | first 5)
        | get content.1
        | str contains "nu"
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(true)
}

#[test]
fn upsert_uses_enumerate_index_inserting() -> Result {
    let code = "
        [
            [a];
            [7]
            [6]
        ]
        | enumerate
        | upsert b {|el| $el.index + 1 + $el.item.a }
        | flatten
        | to nuon
    ";

    test()
        .run(code)
        .expect_value_eq("[[index, a, b]; [0, 7, 8], [1, 6, 8]]")
}

#[test]
fn upsert_uses_enumerate_index_updating() -> Result {
    let code = "
        [
            [a];
            [7]
            [6]
        ]
        | enumerate
        | upsert a {|el| $el.index + 1 + $el.item.a }
        | flatten
        | to nuon
    ";

    test()
        .run(code)
        .expect_value_eq("[[index, a]; [0, 8], [1, 8]]")
}

#[test]
fn upsert_into_list() -> Result {
    test()
        .run("[1, 2, 3] | upsert 1 abc | to json -r")
        .expect_value_eq(r#"[1,"abc",3]"#)
}

#[test]
fn upsert_at_end_of_list() -> Result {
    test()
        .run("[1, 2, 3] | upsert 3 abc | to json -r")
        .expect_value_eq(r#"[1,2,3,"abc"]"#)
}

#[test]
fn upsert_past_end_of_list() -> Result {
    let err = test()
        .run("[1, 2, 3] | upsert 5 abc")
        .expect_shell_error()?;

    match err {
        ShellError::InsertAfterNextFreeIndex { available_idx, .. } => {
            assert_eq!(available_idx, 3);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn upsert_into_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | upsert 1 abc | to json -r")
        .expect_value_eq(r#"[1,"abc",3]"#)
}

#[test]
fn upsert_at_end_of_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | upsert 3 abc | to json -r")
        .expect_value_eq(r#"[1,2,3,"abc"]"#)
}

#[test]
fn upsert_past_end_of_list_stream() -> Result {
    let err = test()
        .run("[1, 2, 3] | every 1 | upsert 5 abc")
        .expect_shell_error()?;

    match err {
        ShellError::InsertAfterNextFreeIndex { available_idx, .. } => {
            assert_eq!(available_idx, 3);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn deep_cell_path_creates_all_nested_records() -> Result {
    test()
        .run("{a: {}} | upsert a.b.c 0 | get a.b.c")
        .expect_value_eq(0)
}

#[test]
fn upserts_all_rows_in_table_in_record() -> Result {
    let code = "
        {table: [[col]; [{a: 1}], [{a: 1}]]}
        | upsert table.col.b 2
        | get table.col.b
    ";

    test().run(code).expect_value_eq([2, 2])
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn upsert_table_cell_respects_reorder_option() -> Result {
    let code = "
        let a = [[foo]; [bar]];
        let b = ($a | upsert foo.0 'baz');
        $b.0.foo
    ";

    test().run(code).expect_value_eq("baz")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn upsert_table_cell_multiple_ints_reorder() -> Result {
    let code = "
        let a = [ [[foo]; [bar]] ];
        let b = ($a | upsert 0.0.foo 'hi');
        $b.0.0.foo
    ";

    test().run(code).expect_value_eq("hi")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn upsert_table_cell_mixed_rows() -> Result {
    let code = "
        let table = [ [foo]; ['a'] ['b'] ];
        let t = ($table | upsert foo.0 'z');
        $t.foo.0
    ";

    test().run(code).expect_value_eq("z")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn upsert_new_to_table_cell_mixed_rows() -> Result {
    let code = "
        let table = [ [foo]; ['a'] ['b'] ];
        let t = ($table | upsert bar.0 'z');
        $t.0.bar
    ";

    test().run(code).expect_value_eq("z")
}

#[test]
fn list_replacement_closure() -> Result {
    test()
        .run("[1, 2] | upsert 1 {|i| $i + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[1, 2] | upsert 1 { $in + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[1, 2] | upsert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon")
        .expect_value_eq("[1, 2, 0]")?;

    test()
        .run("[1, 2] | upsert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon")
        .expect_value_eq("[1, 2, 0]")
}

#[test]
fn record_replacement_closure() -> Result {
    test()
        .run("{ a: text } | upsert a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("{a: TEXT}")?;

    test()
        .run("{ a: text } | upsert a { str upcase } | to nuon")
        .expect_value_eq("{a: TEXT}")?;

    test()
        .run("{ a: text } | upsert b {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("{a: text, b: TEXT}")?;

    test()
        .run("{ a: text } | upsert b { default TEXT } | to nuon")
        .expect_value_eq("{a: text, b: TEXT}")?;

    test()
        .run("{ a: { b: 1 } } | upsert a.c {|r| $r.a.b } | to nuon")
        .expect_value_eq("{a: {b: 1, c: 1}}")?;

    test()
        .run("{ a: { b: 1 } } | upsert a.c { default 0 } | to nuon")
        .expect_value_eq("{a: {b: 1, c: 0}}")
}

#[test]
fn table_replacement_closure() -> Result {
    test()
        .run("[[a]; [text]] | upsert a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | upsert a { str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | upsert b {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a, b]; [text, TEXT]]")?;

    test()
        .run("[[a]; [text]] | upsert b { default TEXT } | to nuon")
        .expect_value_eq("[[a, b]; [text, TEXT]]")?;

    test()
        .run("[[b]; [1]] | wrap a | upsert a.c {|r| $r.a.b } | to nuon")
        .expect_value_eq("[[a]; [{b: 1, c: 1}]]")?;

    test()
        .run("[[b]; [1]] | wrap a | upsert a.c { default 0 } | to nuon")
        .expect_value_eq("[[a]; [{b: 1, c: 0}]]")
}

#[test]
fn list_stream_replacement_closure() -> Result {
    test()
        .run("[1, 2] | every 1 | upsert 1 {|i| $i + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[1, 2] | every 1 | upsert 1 { $in + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[1, 2] | every 1 | upsert 2 {|i| if $i == null { 0 } else { $in + 1 } } | to nuon")
        .expect_value_eq("[1, 2, 0]")?;

    test()
        .run("[1, 2] | every 1 | upsert 2 { if $in == null { 0 } else { $in + 1 } } | to nuon")
        .expect_value_eq("[1, 2, 0]")?;

    test()
        .run("[[a]; [text]] | every 1 | upsert a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | every 1 | upsert a { str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | every 1 | upsert b {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a, b]; [text, TEXT]]")?;

    test()
        .run("[[a]; [text]] | every 1 | upsert b { default TEXT } | to nuon")
        .expect_value_eq("[[a, b]; [text, TEXT]]")
}
