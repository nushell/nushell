use nu_experimental::REORDER_CELL_PATHS;
use nu_test_support::prelude::*;

#[test]
fn sets_the_column() -> Result {
    let code = r#"
        open cargo_sample.toml
        | update dev-dependencies.pretty_assertions "0.7.0"
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
        .run("{a:1} | update a 2 | to nuon")
        .expect_value_eq("{a: 2}")
}

#[test]
fn sets_the_column_from_a_block_full_stream_output() -> Result {
    let code = r#"
        {content: null}
        | update content {|| open --raw cargo_sample.toml | lines | first 5 }
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
        | update content (open --raw cargo_sample.toml | lines | first 5)
        | get content.1
        | str contains "nu"
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(true)
}

#[test]
fn upsert_column_missing() -> Result {
    let code = r#"
        open cargo_sample.toml
        | update dev-dependencies.new_assertions "0.7.0"
    "#;

    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "new_assertions");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn update_list() -> Result {
    test()
        .run("[1, 2, 3] | update 1 abc | to json -r")
        .expect_value_eq(r#"[1,"abc",3]"#)
}

#[test]
fn update_past_end_of_list() -> Result {
    let err = test()
        .run("[1, 2, 3] | update 5 abc | to json -r")
        .expect_shell_error()?;

    match err {
        ShellError::AccessBeyondEnd { max_idx, .. } => {
            assert_eq!(max_idx, 2);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn update_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | update 1 abc")
        .expect_value_eq((1, "abc", 3))
}

#[test]
fn update_past_end_of_list_stream() -> Result {
    let err = test()
        .run("[1, 2, 3] | every 1 | update 5 abc")
        .expect_shell_error()?;

    match err {
        ShellError::AccessBeyondEnd { max_idx, .. } => {
            assert_eq!(max_idx, 2);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn update_nonexistent_column() -> Result {
    let err = test().run("{a:1} | update b 2").expect_shell_error()?;

    match err {
        ShellError::CantFindColumn { col_name, .. } => {
            assert_eq!(col_name, "b");
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn update_uses_enumerate_index() -> Result {
    let code = "
        [
            [a]; 
            [7] 
            [6]
        ]
        | enumerate
        | update item.a {|el| $el.index + 1 + $el.item.a }
        | flatten
        | to nuon
    ";

    test()
        .run(code)
        .expect_value_eq("[[index, a]; [0, 8], [1, 8]]")
}

#[test]
fn list_replacement_closure() -> Result {
    test()
        .run("[1, 2] | update 1 {|i| $i + 1 }")
        .expect_value_eq([1, 3])?;

    test()
        .run("[1, 2] | update 1 { $in + 1 }")
        .expect_value_eq([1, 3])
}

#[test]
fn record_replacement_closure() -> Result {
    test()
        .run("{ a: text } | update a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("{a: TEXT}")?;

    test()
        .run("{ a: text } | update a { str upcase } | to nuon")
        .expect_value_eq("{a: TEXT}")
}

#[test]
fn table_replacement_closure() -> Result {
    test()
        .run("[[a]; [text]] | update a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | update a { str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")
}

#[test]
fn list_stream_replacement_closure() -> Result {
    test()
        .run("[1, 2] | every 1 | update 1 {|i| $i + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[1, 2] | every 1 | update 1 { $in + 1 } | to nuon")
        .expect_value_eq("[1, 3]")?;

    test()
        .run("[[a]; [text]] | every 1 | update a {|r| $r.a | str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")?;

    test()
        .run("[[a]; [text]] | every 1 | update a { str upcase } | to nuon")
        .expect_value_eq("[[a]; [TEXT]]")
}

#[test]
fn update_optional_column_present() -> Result {
    test()
        .run("{a: 1} | update a? 2 | to nuon")
        .expect_value_eq("{a: 2}")
}

#[test]
fn update_optional_column_absent() -> Result {
    test()
        .run("{a: 1} | update b? 2 | to nuon")
        .expect_value_eq("{a: 1}")
}

#[test]
fn update_optional_column_in_table_present() -> Result {
    test()
        .run("[[a, b]; [1, 2], [3, 4]] | update a? 10 | to nuon")
        .expect_value_eq("[[a, b]; [10, 2], [10, 4]]")
}

#[test]
fn update_optional_column_in_table_absent() -> Result {
    test()
        .run("[[a, b]; [1, 2], [3, 4]] | update c? 10 | to nuon")
        .expect_value_eq("[[a, b]; [1, 2], [3, 4]]")
}

#[test]
fn update_optional_column_in_table_mixed() -> Result {
    test()
        .run("[{a: 1, b: 2}, {b: 3}, {a: 4, b: 5}] | update a? 10 | to nuon")
        .expect_value_eq("[{a: 10, b: 2}, {b: 3}, {a: 10, b: 5}]")
}

#[test]
fn update_optional_index_present() -> Result {
    test()
        .run("[1, 2, 3] | update 1? 10 | to nuon")
        .expect_value_eq("[1, 10, 3]")
}

#[test]
fn update_optional_index_absent() -> Result {
    test()
        .run("[1, 2, 3] | update 5? 10 | to nuon")
        .expect_value_eq("[1, 2, 3]")
}

#[test]
fn update_optional_column_with_closure_present() -> Result {
    test()
        .run("{a: 5} | update a? {|x| $x.a * 2 } | to nuon")
        .expect_value_eq("{a: 10}")
}

#[test]
fn update_optional_column_with_closure_absent() -> Result {
    test()
        .run("{a: 5} | update b? {|x| 10 } | to nuon")
        .expect_value_eq("{a: 5}")
}

#[test]
fn update_optional_column_in_table_with_closure() -> Result {
    test()
        .run("[[a]; [1], [2]] | update a? { $in * 2 } | to nuon")
        .expect_value_eq("[[a]; [2], [4]]")
}

#[test]
fn update_optional_column_in_table_with_closure_mixed() -> Result {
    test()
        .run("[{a: 1, b: 2}, {b: 3}, {a: 4, b: 5}] | update a? { $in * 10 } | to nuon")
        .expect_value_eq("[{a: 10, b: 2}, {b: 3}, {a: 40, b: 5}]")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn update_table_cell_respects_reorder_option() -> Result {
    let code = "
        let a = [[foo]; [bar]];
        let b = ($a | update foo.0 'baz');
        $b.0.foo
    ";

    test().run(code).expect_value_eq("baz")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn update_table_cell_multiple_ints_reorder() -> Result {
    let code = "
        let a = [ [[foo]; [bar]] ];
        let b = ($a | update 0.0.foo 'hi');
        $b.0.0.foo
    ";

    test().run(code).expect_value_eq("hi")
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn update_table_cell_mixed_rows() -> Result {
    let code = "
        let table = [ [foo]; ['a'] ['b'] ];
        let t = ($table | update foo.0 'z');
        $t.foo.0
    ";

    test().run(code).expect_value_eq("z")
}

#[test]
fn update_optional_index_with_closure_present() -> Result {
    test()
        .run("[1, 2, 3] | update 1? { $in * 10 } | to nuon")
        .expect_value_eq("[1, 20, 3]")
}

#[test]
fn update_optional_index_with_closure_absent() -> Result {
    test()
        .run("[1, 2, 3] | update 5? { $in * 10 } | to nuon")
        .expect_value_eq("[1, 2, 3]")
}

#[test]
fn update_optional_in_list_stream() -> Result {
    test()
        .run("[[a, b]; [1, 2], [3, 4]] | every 1 | update c? 10 | to nuon")
        .expect_value_eq("[[a, b]; [1, 2], [3, 4]]")
}

#[test]
fn update_optional_in_list_stream_with_closure() -> Result {
    test()
        .run("[{a: 1}, {b: 2}, {a: 3}] | every 1 | update a? { $in * 10 } | to nuon")
        .expect_value_eq("[{a: 10}, {b: 2}, {a: 30}]")
}
