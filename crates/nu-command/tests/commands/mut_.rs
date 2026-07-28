use rstest::rstest;

use nu_experimental::ENFORCE_RUNTIME_ANNOTATIONS;
use nu_test_support::{fs::Stub::EmptyFile, playground::Playground, prelude::*};

#[test]
fn mut_variable() -> Result {
    test().run("mut x = 3; $x = $x + 1; $x").expect_value_eq(4)
}

#[rstest]
#[case("mut in = 3")]
#[case("mut in: int = 3")]
fn mut_name_builtin_var(#[case] assignment: &str) -> Result {
    test()
        .run(assignment)
        .expect_error_code_eq("nu::parser::name_is_builtin_var")
}

#[test]
fn mut_name_builtin_var_with_dollar() -> Result {
    test()
        .run("mut $env = 3")
        .expect_error_code_eq("nu::parser::name_is_builtin_var")
}

#[test]
fn mut_variable_in_loop() -> Result {
    test()
        .run("mut x = 1; for i in 1..10 { $x = $x + $i }; $x")
        .expect_value_eq(56)
}

#[test]
fn capture_of_mutable_var() -> Result {
    let err = test().run("mut x = 123; {|| $x }").expect_parse_error()?;
    assert_eq!(err.to_string(), ("Capture of mutable variable."));
    Ok(())
}

#[test]
fn mut_add_assign() -> Result {
    test().run("mut y = 3; $y += 2; $y").expect_value_eq(5)
}

#[test]
fn mut_minus_assign() -> Result {
    test().run("mut y = 3; $y -= 2; $y").expect_value_eq(1)
}

#[test]
fn mut_multiply_assign() -> Result {
    test().run("mut y = 3; $y *= 2; $y").expect_value_eq(6)
}

#[test]
fn mut_divide_assign() -> Result {
    test()
        .run("mut y: number = 8; $y /= 2; $y")
        .expect_value_eq(4.0)
}

#[test]
fn mut_divide_assign_should_error() -> Result {
    test()
        .run("mut y = 8; $y /= 2; $y")
        .expect_error_code_eq("nu::parser::operator_incompatible_types")
}

#[test]
fn mut_subtract_assign_should_error() -> Result {
    test()
        .run("mut x = (date now); $x -= 2019-05-10")
        .expect_error_code_eq("nu::parser::operator_incompatible_types")
}

#[test]
fn mut_assign_number() -> Result {
    test()
        .run("mut x: number = 1; $x = 2.0; $x")
        .expect_value_eq(2.0)
}

#[test]
fn mut_assign_glob() -> Result {
    test()
        .run(r#"mut x: glob = ""; $x = "meow"; $x"#)
        .expect_value_eq("meow")
}

#[test]
fn mut_path_insert() -> Result {
    test()
        .run("mut y = {abc: 123}; $y.abc = 456; $y.abc")
        .expect_value_eq(456)
}

#[test]
fn mut_path_insert_list() -> Result {
    test()
        .run("mut a = [0 1 2]; $a.3 = 3; $a")
        .expect_value_eq([0, 1, 2, 3])
}

#[test]
fn mut_path_upsert() -> Result {
    test()
        .run("mut a = {b:[{c:1}]}; $a.b.0.d = 11; $a.b.0.d")
        .expect_value_eq(11)
}

#[test]
fn mut_path_upsert_list() -> Result {
    test()
        .run("mut a = [[[3] 2] 1]; $a.0.0.1 = 0; $a.0.2 = 0; $a.2 = 0; $a")
        .expect_value_eq(test_value!([[[3, 0], 2, 0], 1, 0]))
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn mut_path_operator_assign_should_error_enforce_runtime() -> Result {
    // should error on the division
    test()
        .run("mut a: record<b: int> = {b:1}; $a.b += 3; $a.b -= 2; $a.b *= 10; $a.b /= 4; $a.b")
        .expect_error_code_eq("nu::shell::type_mismatch")
}

#[test]
fn mut_records_update_properly() -> Result {
    test()
        .run("mut a = {}; $a.b.c = 100; $a.b.c")
        .expect_value_eq(100)
}

#[test]
fn mut_value_with_if() -> Result {
    test()
        .run("mut a = 3; $a = if 3 == 3 { 10 }; echo $a")
        .expect_value_eq(10)
}

#[test]
fn mut_value_with_match() -> Result {
    test()
        .run("mut a = 'maybe?'; $a = match 3 { 1 => { 'yes!' }, _ => { 'no!' } }; echo $a")
        .expect_value_eq("no!")
}

#[test]
fn mut_glob_type() -> Result {
    test()
        .run("mut x: glob = 'aa'; $x | describe")
        .expect_value_eq("glob")
}

#[test]
fn mut_typed_glob_expands_in_ls() -> Result {
    Playground::setup("mut_glob_ls", |dirs, sandbox| {
        sandbox.with_files(&[EmptyFile("a.toml"), EmptyFile("b.toml"), EmptyFile("c.txt")]);

        test()
            .cwd(dirs.test())
            .run(r#"mut x: glob = "*.toml"; ls $x | length"#)
            .expect_value_eq(2)
    })
}

#[rstest]
#[case("r#'abc'#", "abc")]
#[case(r#"r#'abcde""fghi"''''jkl'#"#, r#"abcde""fghi"''''jkl"#)]
#[case(r#"r##'abcde""fghi"''''#jkl'##"#, r#"abcde""fghi"''''#jkl"#)]
#[case(r#"r###'abcde""fghi"'''##'#jkl'###"#, r#"abcde""fghi"'''##'#jkl"#)]
fn mut_raw_string(#[case] input: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!("mut x = {input}; $x"))
        .expect_value_eq(expected)
}

#[test]
fn def_should_not_mutate_mut() -> Result {
    let err = test()
        .run("mut a = 3; def foo [] { $a = 4}")
        .expect_parse_error()?;
    assert_eq!(err.to_string(), ("Capture of mutable variable."));
    Ok(())
}

#[test]
fn assign_to_non_mut_variable_raises_parse_error() -> Result {
    test()
        .run("let x = 3; $x = 4")
        .expect_error_code_eq("nu::parser::assignment_requires_mutable_variable")
}

#[test]
fn assign_to_non_variable_raises_parse_error() -> Result {
    test()
        .run("mut x = 3; x = 5")
        .expect_error_code_eq("nu::parser::assignment_requires_variable")
}
