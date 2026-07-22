use pretty_assertions::assert_matches;
use rstest::rstest;

use nu_experimental::REORDER_CELL_PATHS;
use nu_protocol::{test_table, test_value};
use nu_test_support::prelude::*;

#[test]
fn insert_the_column() -> Result {
    let code = r#"
        open cargo_sample.toml
        | insert dev-dependencies.new_assertions "0.7.0"
        | get dev-dependencies.new_assertions
    "#;
    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("0.7.0")
}

#[test]
fn doesnt_convert_record_to_table() -> Result {
    test()
        .run("{a:1} | insert b 2")
        .expect_value_eq(test_value!({a: 1, b: 2}))
}

#[test]
fn insert_the_column_conflict() -> Result {
    let code = r#"
        open cargo_sample.toml
        | insert dev-dependencies.pretty_assertions "0.7.0"
    "#;
    let err = test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    assert_matches!(err, ShellError::ColumnAlreadyExists { col_name, .. } if col_name == "pretty_assertions");
    Ok(())
}

#[test]
fn insert_into_list() -> Result {
    test()
        .run("[1, 2, 3] | insert 1 abc")
        .expect_value_eq(test_value!([1, "abc", 2, 3]))
}

#[test]
fn insert_at_start_of_list() -> Result {
    test()
        .run("[1, 2, 3] | insert 0 abc")
        .expect_value_eq(test_value!(["abc", 1, 2, 3]))
}

#[test]
fn insert_at_end_of_list() -> Result {
    test()
        .run("[1, 2, 3] | insert 3 abc")
        .expect_value_eq(test_value!([1, 2, 3, "abc"]))
}

#[test]
fn insert_past_end_of_list() -> Result {
    test()
        .run("[1, 2, 3] | insert 5 abc")
        .expect_error_code_eq("nu::shell::access_beyond_end")
}

#[test]
fn insert_into_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | insert 1 abc")
        .expect_value_eq(test_value!([1, "abc", 2, 3]))
}

#[test]
fn insert_at_end_of_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | insert 3 abc")
        .expect_value_eq(test_value!([1, 2, 3, "abc"]))
}

#[test]
fn insert_past_end_of_list_stream() -> Result {
    test()
        .run("[1, 2, 3] | every 1 | insert 5 abc")
        .expect_error_code_eq("nu::shell::access_beyond_end")
}

#[test]
fn insert_uses_enumerate_index() -> Result {
    let code = "
        [[a]; [7] [6]]
        | enumerate
        | insert b {|el| $el.index + 1 + $el.item.a }
        | flatten
    ";
    test().run(code).expect_value_eq(test_table![
        ["index", "a", "b"];
        [0, 7, 8],
        [1, 6, 8],
    ])
}

#[test]
fn deep_cell_path_creates_all_nested_records() -> Result {
    test()
        .run("{a: {}} | insert a.b.c 0 | get a.b.c")
        .expect_value_eq(0)
}

#[test]
fn inserts_all_rows_in_table_in_record() -> Result {
    test()
        .run("{table: [[col]; [{a: 1}], [{a: 1}]]} | insert table.col.b 2 | get table.col.b")
        .expect_value_eq(test_value!([2, 2]))
}

#[rstest]
#[case("[1, 2] | insert 1 {|i| $i + 1 }", [1, 3, 2])]
#[case("[1, 2] | insert 1 { $in + 1 }", [1, 3, 2])]
#[case("[1, 2] | insert 2 {|i| if $i == null { 0 } else { $in + 1 } }", [1, 2, 0])]
#[case("[1, 2] | insert 2 { if $in == null { 0 } else { $in + 1 } }", [1, 2, 0])]
fn list_replacement_closure(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case("{ a: text } | insert b {|r| $r.a | str upcase }", test_value!({a: "text", b: "TEXT"}))]
#[case("{ a: text } | insert b { $in.a | str upcase }", test_value!({a: "text", b: "TEXT"}))]
#[case("{ a: { b: 1 } } | insert a.c {|r| $r.a.b }", test_value!({a: {b: 1, c: 1}}))]
#[case("{ a: { b: 1 } } | insert a.c { $in.a.b }", test_value!({a: {b: 1, c: 1}}))]
fn record_replacement_closure(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case("[[a]; [text]] | insert b {|r| $r.a | str upcase }", test_value!([{"a":"text","b":"TEXT"}]))]
#[case("[[a]; [text]] | insert b { $in.a | str upcase }", test_value!([{"a":"text","b":"TEXT"}]))]
#[case("[[b]; [1]] | wrap a | insert a.c {|r| $r.a.b }", test_value!([{"a":{"b":1,"c":1}}]))]
#[case("[[b]; [1]] | wrap a | insert a.c { $in.a.b }", test_value!([{"a":{"b":1,"c":1}}]))]
fn table_replacement_closure(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[rstest]
#[case("[1, 2] | every 1 | insert 1 {|i| $i + 1 }", test_value!([1, 3, 2]))]
#[case("[1, 2] | every 1 | insert 1 { $in + 1 }", test_value!([1, 3, 2]))]
#[case("[1, 2] | every 1 | insert 2 {|i| if $i == null { 0 } else { $in + 1 } }", test_value!([1, 2, 0]))]
#[case("[1, 2] | every 1 | insert 2 { if $in == null { 0 } else { $in + 1 } }", test_value!([1, 2, 0]))]
#[case("[[a]; [text]] | every 1 | insert b {|r| $r.a | str upcase }", test_table![["a", "b"]; ["text", "TEXT"]])]
#[case("[[a]; [text]] | every 1 | insert b { $in.a | str upcase }", test_table![["a", "b"]; ["text", "TEXT"]])]
fn list_stream_replacement_closure(#[case] code: &str, #[case] expect: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expect)
}

#[test]
#[exp(REORDER_CELL_PATHS)]
fn insert_new_to_table_cell_mixed_rows() -> Result {
    let code = "
        let table = [ [foo]; ['a'] ['b'] ];
        let t = ($table | insert bar.0 'z');
        $t.0.bar
    ";
    test().run(code).expect_value_eq("z")
}

#[test]
fn insert_nested_path_into_empty_list_errors_without_underflow() -> Result {
    // Regression test for #18426: inserting into a nested path of an empty list
    // used to underflow `pre_elems.len() - 1` to usize::MAX and print a garbage
    // error containing 18446744073709551615.
    let err = test().run("[] | insert 0.0 1").expect_shell_error()?;
    assert_matches!(err, ShellError::AccessEmptyContent { .. });
    Ok(())
}
