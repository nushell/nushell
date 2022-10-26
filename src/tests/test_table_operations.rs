use crate::tests::{run_test, TestResult};

#[test]
fn cell_path_subexpr1() -> TestResult {
    run_test("([[lang, gems]; [nu, 100]]).lang | get 0", "nu")
}

#[test]
fn cell_path_subexpr2() -> TestResult {
    run_test("([[lang, gems]; [nu, 100]]).lang.0", "nu")
}

#[test]
fn cell_path_var1() -> TestResult {
    run_test("let x = [[lang, gems]; [nu, 100]]; $x.lang | get 0", "nu")
}

#[test]
fn cell_path_var2() -> TestResult {
    run_test("let x = [[lang, gems]; [nu, 100]]; $x.lang.0", "nu")
}

#[test]
fn flatten_simple_list() -> TestResult {
    run_test(
        "[[N, u, s, h, e, l, l]] | flatten | str join (char nl)",
        "N\nu\ns\nh\ne\nl\nl",
    )
}

#[test]
fn flatten_get_simple_list() -> TestResult {
    run_test("[[N, u, s, h, e, l, l]] | flatten | get 0", "N")
}

#[test]
fn flatten_table_get() -> TestResult {
    run_test(
        "[[origin, people]; [Ecuador, ([[name, meal]; ['Andres', 'arepa']])]] | flatten --all | get meal.0",
        "arepa",
    )
}

#[test]
fn flatten_table_column_get_last() -> TestResult {
    run_test(
        "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten versions --all | last | get versions",
        "0.22",
    )
}

#[test]
fn flatten_should_just_flatten_one_level() -> TestResult {
    run_test(
        "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten crate | get crate.name.0",
        "nu-cli"
    )
}

#[test]
fn flatten_nest_table_when_all_provided() -> TestResult {
    run_test(
        "[[origin, crate, versions]; [World, ([[name]; ['nu-cli']]), ['0.21', '0.22']]] | flatten crate --all | get name.0",
        "nu-cli"
    )
}

#[test]
fn get_table_columns_1() -> TestResult {
    run_test(
        "[[name, age, grade]; [paul,21,a]] | columns | first",
        "name",
    )
}

#[test]
fn get_table_columns_2() -> TestResult {
    run_test("[[name, age, grade]; [paul,21,a]] | columns | get 1", "age")
}

#[test]
fn flatten_should_flatten_inner_table() -> TestResult {
    run_test(
        "[[[name, value]; [abc, 123]]] | flatten --all | get value.0",
        "123",
    )
}

#[test]
fn command_filter_reject_1() -> TestResult {
    run_test(
        "[[lang, gems]; [nu, 100]] | reject gems | to json",
        r#"[
  {
    "lang": "nu"
  }
]"#,
    )
}

#[test]
fn command_filter_reject_2() -> TestResult {
    run_test(
        "[[lang, gems, grade]; [nu, 100, a]] | reject gems grade | to json",
        r#"[
  {
    "lang": "nu"
  }
]"#,
    )
}

#[test]
fn command_filter_reject_3() -> TestResult {
    run_test(
        "[[lang, gems, grade]; [nu, 100, a]] | reject grade gems | to json",
        r#"[
  {
    "lang": "nu"
  }
]"#,
    )
}

#[test]
#[rustfmt::skip]
fn command_filter_reject_4() -> TestResult {
    run_test(
        "[[lang, gems, grade]; [nu, 100, a]] | reject gems | to json -r",
        r#"[{"lang": "nu","grade": "a"}]"#,
    )
}

#[test]
fn command_drop_column_1() -> TestResult {
    run_test(
        "[[lang, gems, grade]; [nu, 100, a]] | drop column 2 | to json",
        r#"[
  {
    "lang": "nu"
  }
]"#,
    )
}

#[test]
fn record_1() -> TestResult {
    run_test(r#"{'a': 'b'} | get a"#, "b")
}

#[test]
fn record_2() -> TestResult {
    run_test(r#"{'b': 'c'}.b"#, "c")
}

#[test]
fn where_on_ranges() -> TestResult {
    run_test(r#"1..10 | where $it > 8 | math sum"#, "19")
}

#[test]
fn index_on_list() -> TestResult {
    run_test(r#"[1, 2, 3].1"#, "2")
}

#[test]
fn update_cell_path_1() -> TestResult {
    run_test(
        r#"[[name, size]; [a, 1.1]] | into int size | get size.0"#,
        "1",
    )
}

#[test]
fn missing_column_fills_in_nothing() -> TestResult {
    // The empty value will be replaced with $nothing when fetching a column
    run_test(
        r#"[ { name: ABC, size: 20 }, { name: HIJ } ].size.1 == $nothing"#,
        "true",
    )
}

#[test]
fn string_cell_path() -> TestResult {
    run_test(
        r#"let x = "name"; [["name", "score"]; [a, b], [c, d]] | get $x | get 1"#,
        "c",
    )
}

#[test]
fn split_row() -> TestResult {
    run_test(r#""hello world" | split row " " | get 1"#, "world")
}

#[test]
fn split_column() -> TestResult {
    run_test(
        r#""hello world" | split column " " | get "column1".0"#,
        "hello",
    )
}

#[test]
fn wrap() -> TestResult {
    run_test(r#"([1, 2, 3] | wrap foo).foo.1"#, "2")
}

#[test]
fn get() -> TestResult {
    run_test(
        r#"[[name, grade]; [Alice, A], [Betty, B]] | get grade.1"#,
        "B",
    )
}

#[test]
fn select_1() -> TestResult {
    run_test(
        r#"([[name, age]; [a, 1], [b, 2]]) | select name | get 1 | get name"#,
        "b",
    )
}

#[test]
fn select_2() -> TestResult {
    run_test(
        r#"[[name, age]; [a, 1] [b, 2]] | get 1 | select age | get age"#,
        "2",
    )
}

#[test]
fn update_will_insert() -> TestResult {
    run_test(r#"{} | upsert a b | get a"#, "b")
}

#[test]
fn length_for_columns() -> TestResult {
    run_test(
        r#"[[name,age,grade]; [bill,20,a] [a b c]] | length -c"#,
        "3",
    )
}

#[test]
fn length_for_rows() -> TestResult {
    run_test(r#"[[name,age,grade]; [bill,20,a] [a b c]] | length"#, "2")
}

#[test]
fn length_defaulted_columns() -> TestResult {
    run_test(
        r#"[[name, age]; [test, 10]] | default 11 age | get 0 | columns | length"#,
        "2",
    )
}

#[test]
fn get_fuzzy() -> TestResult {
    run_test("(ls | get -i foo) == $nothing", "true")
}

#[test]
fn get_insensitive() -> TestResult {
    run_test(
        r#"[[name, age]; [a, 1] [b, 2]] | get NAmE | select 0 | get 0"#,
        "a",
    )
}
