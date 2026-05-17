use std::sync::LazyLock;

use nu_protocol::Record;
use nu_test_support::{fs::Stub::EmptyFile, prelude::*};

#[derive(Debug, Clone, IntoValue)]
struct Amigo {
    name: &'static str,
    rusty_luck: Option<Value>,
}

#[rustfmt::skip]
static AMIGOS: LazyLock<[Amigo; 7]> = LazyLock::new(|| [
    Amigo { name: "Yehuda",    rusty_luck: None },
    Amigo { name: "Jt",        rusty_luck: Some(Value::test_int(0)) },
    Amigo { name: "Andres",    rusty_luck: Some(Value::test_int(0)) },
    Amigo { name: "Michael",   rusty_luck: Some(Value::test_list(Vec::default())) },
    Amigo { name: "Darren",    rusty_luck: Some(Value::test_record(Record::default())) },
    Amigo { name: "Stefan",    rusty_luck: Some(Value::test_string(String::default())) },
    Amigo { name: "GorbyPuff", rusty_luck: None },
]);

#[test]
fn adds_row_data_if_column_missing() -> Result {
    let code = "
        $in
        | default 1 rusty_luck
        | where rusty_luck == 1
        | length
    ";

    test()
        .run_with_data(code, AMIGOS.clone())
        .expect_value_eq(2)
}

#[test]
fn default_after_empty_filter() -> Result {
    test()
        .run("[a b] | where $it == 'c' | get -o 0 | default 'd'")
        .expect_value_eq("d")
}

#[test]
fn keeps_nulls_in_lists() -> Result {
    test()
        .run("[null, 2, 3] | default []")
        .expect_value_eq(((), 2, 3))
}

#[test]
fn replaces_null() -> Result {
    test().run("null | default 1").expect_value_eq(1)
}

#[test]
fn adds_row_data_if_column_missing_or_empty() -> Result {
    let code = "
        $in
        | default -e 1 rusty_luck
        | where rusty_luck == 1
        | length
    ";

    test()
        .run_with_data(code, AMIGOS.clone())
        .expect_value_eq(5)
}

#[test]
fn replace_empty_string() -> Result {
    test().run("'' | default -e foo").expect_value_eq("foo")
}

#[test]
fn do_not_replace_empty_string() -> Result {
    test().run("'' | default 1").expect_value_eq("")
}

#[test]
fn replace_empty_list() -> Result {
    test().run("[] | default -e foo").expect_value_eq("foo")
}

#[test]
fn do_not_replace_empty_list() -> Result {
    test().run("[] | default 1 | length").expect_value_eq(0)
}

#[test]
fn replace_empty_record() -> Result {
    test().run("{} | default -e foo").expect_value_eq("foo")
}

#[test]
fn do_not_replace_empty_record() -> Result {
    test()
        .run("{} | default {a:5} | columns | length")
        .expect_value_eq(0)
}

#[test]
fn replace_empty_list_stream() -> Result {
    // This is specific for testing ListStreams when empty behave like other empty values
    Playground::setup("glob_empty_list", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jttxt"),
            EmptyFile("andres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob ? | default -e void")
            .expect_value_eq("void")
    })
}

#[test]
fn do_not_replace_non_empty_list_stream() -> Result {
    // This is specific for testing ListStreams when empty behave like other empty values
    Playground::setup("glob_non_empty_list", |dirs, sandbox| {
        sandbox.with_files(&[
            EmptyFile("yehuda.txt"),
            EmptyFile("jt.rs"),
            EmptyFile("andres.txt"),
        ]);

        test()
            .cwd(dirs.test())
            .run("glob '*.txt' | default -e void | length")
            .expect_value_eq(2)
    })
}

#[test]
fn closure_eval_simple() -> Result {
    test().run("null | default { 1 }").expect_value_eq(1)
}

#[test]
fn closure_eval_complex() -> Result {
    test()
        .run("null | default { seq 1 5 | math sum }")
        .expect_value_eq(15)
}

#[test]
fn closure_eval_is_lazy() -> Result {
    test()
        .run("1 | default { error make -u {msg: foo} }")
        .expect_value_eq(1)
}

#[test]
fn column_closure_eval_is_lazy() -> Result {
    test()
        .run("{a: 1} | default { error make -u {msg: foo} } a | get a")
        .expect_value_eq(1)
}

#[test]
fn closure_eval_replace_empty_string() -> Result {
    test().run("'' | default --empty { 1 }").expect_value_eq(1)
}

#[test]
fn closure_eval_do_not_replace_empty_string() -> Result {
    test().run("'' | default { 1 }").expect_value_eq("")
}

#[test]
fn closure_eval_replace_empty_list() -> Result {
    test().run("[] | default --empty { 1 }").expect_value_eq(1)
}

#[test]
fn closure_eval_do_not_replace_empty_list() -> Result {
    test().run("[] | default { 1 } | length").expect_value_eq(0)
}

#[test]
fn closure_eval_replace_empty_record() -> Result {
    test().run("{} | default --empty { 1 }").expect_value_eq(1)
}

#[test]
fn closure_eval_do_not_replace_empty_record() -> Result {
    test()
        .run("{} | default { 1 } | columns | length")
        .expect_value_eq(0)
}

#[test]
fn closure_eval_add_missing_column_record() -> Result {
    test()
        .run("{a: 1} | default { 2 } b | get b")
        .expect_value_eq(2)
}

#[test]
fn closure_eval_add_missing_column_table() -> Result {
    test()
        .run("[{a: 1, b: 2}, {b: 4}] | default { 3 } a | get a")
        .expect_value_eq([1, 3])
}

#[test]
fn closure_eval_replace_empty_column() -> Result {
    test()
        .run("{a: ''} | default -e { 1 } a | get a")
        .expect_value_eq(1)
}

#[test]
fn replace_multiple_columns() -> Result {
    test()
        .run("{a: ''} | default -e 1 a b | values")
        .expect_value_eq([1, 1])
}

#[test]
fn return_closure_value() -> Result {
    let outcome: Value = test().run("null | default { {||} }")?;
    assert!(matches!(outcome, Value::Closure { .. }));
    Ok(())
}

#[test]
fn lazy_output_streams() -> Result {
    let code = "default { nu --testbin cococo 'hello' } | describe";
    let actual: String = test().add_nu_to_path().run(code)?;
    assert_contains("byte stream", actual);
    Ok(())
}
