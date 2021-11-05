use assert_cmd::prelude::*;
use pretty_assertions::assert_eq;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(test)]
fn run_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(output.status.success());

    assert_eq!(stdout.trim(), expected);

    Ok(())
}

#[cfg(test)]
fn fail_test(input: &str, expected: &str) -> TestResult {
    let mut file = NamedTempFile::new()?;
    let name = file.path();

    let mut cmd = Command::cargo_bin("engine-q")?;
    cmd.arg(name);

    writeln!(file, "{}", input)?;

    let output = cmd.output()?;

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();

    println!("stdout: {}", stdout);
    println!("stderr: {}", stderr);

    assert!(stderr.contains(expected));

    Ok(())
}

fn not_found_msg() -> &'static str {
    if cfg!(windows) {
        "not recognized"
    } else {
        "not found"
    }
}

#[test]
fn add_simple() -> TestResult {
    run_test("3 + 4", "7")
}

#[test]
fn add_simple2() -> TestResult {
    run_test("3 + 4 + 9", "16")
}

#[test]
fn broken_math() -> TestResult {
    fail_test("3 + ", "incomplete")
}

#[test]
fn modulo1() -> TestResult {
    run_test("5 mod 2", "1")
}

#[test]
fn modulo2() -> TestResult {
    run_test("5.25 mod 2", "1.25")
}

#[test]
fn and() -> TestResult {
    run_test("$true && $false", "false")
}

#[test]
fn or() -> TestResult {
    run_test("$true || $false", "true")
}

#[test]
fn pow() -> TestResult {
    run_test("3 ** 3", "27")
}

#[test]
fn contains() -> TestResult {
    run_test("'testme' =~ 'test'", "true")
}

#[test]
fn not_contains() -> TestResult {
    run_test("'testme' !~ 'test'", "false")
}

#[test]
fn if_test1() -> TestResult {
    run_test("if $true { 10 } else { 20 } ", "10")
}

#[test]
fn if_test2() -> TestResult {
    run_test("if $false { 10 } else { 20 } ", "20")
}

#[test]
fn simple_if() -> TestResult {
    run_test("if $true { 10 } ", "10")
}

#[test]
fn simple_if2() -> TestResult {
    run_test("if $false { 10 } ", "")
}

#[test]
fn if_cond() -> TestResult {
    run_test("if 2 < 3 { 3 } ", "3")
}

#[test]
fn if_cond2() -> TestResult {
    run_test("if 2 > 3 { 3 } ", "")
}

#[test]
fn if_cond3() -> TestResult {
    run_test("if 2 < 3 { 5 } else { 4 } ", "5")
}

#[test]
fn if_cond4() -> TestResult {
    run_test("if 2 > 3 { 5 } else { 4 } ", "4")
}

#[test]
fn if_elseif1() -> TestResult {
    run_test("if 2 > 3 { 5 } else if 6 < 7 { 4 } ", "4")
}

#[test]
fn if_elseif2() -> TestResult {
    run_test("if 2 < 3 { 5 } else if 6 < 7 { 4 } else { 8 } ", "5")
}

#[test]
fn if_elseif3() -> TestResult {
    run_test("if 2 > 3 { 5 } else if 6 > 7 { 4 } else { 8 } ", "8")
}

#[test]
fn if_elseif4() -> TestResult {
    run_test("if 2 > 3 { 5 } else if 6 < 7 { 4 } else { 8 } ", "4")
}

#[test]
fn no_scope_leak1() -> TestResult {
    fail_test(
        "if $false { let $x = 10 } else { let $x = 20 }; $x",
        "Variable not found",
    )
}

#[test]
fn no_scope_leak2() -> TestResult {
    fail_test(
        "def foo [] { $x }; def bar [] { let $x = 10; foo }; bar",
        "Variable not found",
    )
}

#[test]
fn no_scope_leak3() -> TestResult {
    run_test(
        "def foo [$x] { $x }; def bar [] { let $x = 10; foo 20}; bar",
        "20",
    )
}

#[test]
fn no_scope_leak4() -> TestResult {
    run_test(
        "def foo [$x] { $x }; def bar [] { let $x = 10; (foo 20) + $x}; bar",
        "30",
    )
}

#[test]
fn simple_var_closing() -> TestResult {
    run_test("let $x = 10; def foo [] { $x }; foo", "10")
}

#[test]
fn predecl_check() -> TestResult {
    run_test("def bob [] { sam }; def sam [] { 3 }; bob", "3")
}

#[test]
fn def_with_no_dollar() -> TestResult {
    run_test("def bob [x] { $x + 3 }; bob 4", "7")
}

#[test]
fn env_shorthand() -> TestResult {
    run_test("FOO=BAR if $false { 3 } else { 4 }", "4")
}

#[test]
fn floating_add() -> TestResult {
    run_test("10.1 + 0.8", "10.9")
}

#[test]
fn subcommand() -> TestResult {
    run_test("def foo [] {}; def \"foo bar\" [] {3}; foo bar", "3")
}

#[test]
fn alias_1() -> TestResult {
    run_test("def foo [$x] { $x + 10 }; alias f = foo; f 100", "110")
}

#[test]
fn alias_2() -> TestResult {
    run_test(
        "def foo [$x $y] { $x + $y + 10 }; alias f = foo 33; f 100",
        "143",
    )
}

#[test]
fn alias_2_multi_word() -> TestResult {
    run_test(
        r#"def "foo bar" [$x $y] { $x + $y + 10 }; alias f = foo bar 33; f 100"#,
        "143",
    )
}

#[test]
fn block_param1() -> TestResult {
    run_test("[3] | each { $it + 10 } | get 0", "13")
}

#[test]
fn block_param2() -> TestResult {
    run_test("[3] | each { |y| $y + 10 } | get 0", "13")
}

#[test]
fn block_param3_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { $it + 10 } | get 1", "12")
}

#[test]
fn block_param4_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { |y| $y + 10 } | get 2", "13")
}

#[test]
fn range_iteration1() -> TestResult {
    run_test("1..4 | each { |y| $y + 10 } | get 0", "11")
}

#[test]
fn range_iteration2() -> TestResult {
    run_test("4..1 | each { |y| $y + 100 } | get 3", "101")
}

#[test]
fn simple_value_iteration() -> TestResult {
    run_test("4 | each { $it + 10 }", "14")
}

#[test]
fn concrete_variable_assignment() -> TestResult {
    run_test(
        "let x = (1..100 | each { |y| $y + 100 }); $x | length; $x | length",
        "100",
    )
}

#[test]
fn build_string1() -> TestResult {
    run_test("build-string 'nu' 'shell'", "nushell")
}

#[test]
fn build_string2() -> TestResult {
    run_test("'nu' | each {build-string $it 'shell'}", "nushell")
}

#[test]
fn build_string3() -> TestResult {
    run_test(
        "build-string 'nu' 'shell' | each {build-string $it ' rocks'}",
        "nushell rocks",
    )
}

#[test]
fn build_string4() -> TestResult {
    run_test(
        "['sam','rick','pete'] | each { build-string $it ' is studying'} | get 2",
        "pete is studying",
    )
}

#[test]
fn build_string5() -> TestResult {
    run_test(
        "['sam','rick','pete'] | each { |x| build-string $x ' is studying'} | get 1",
        "rick is studying",
    )
}

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
fn custom_rest_var() -> TestResult {
    run_test("def foo [...x] { $x.0 + $x.1 }; foo 10 80", "90")
}

#[test]
fn row_iteration() -> TestResult {
    run_test(
        "[[name, size]; [tj, 100], [rl, 200]] | each { $it.size * 8 } | get 1",
        "1600",
    )
}

#[test]
fn record_iteration() -> TestResult {
    run_test("([[name, level]; [aa, 100], [bb, 200]] | each { $it | each { |x| if $x.column == \"level\" { $x.value + 100 } else { $x.value } } }).level | get 1", "300")
}

#[test]
fn row_condition1() -> TestResult {
    run_test(
        "([[name, size]; [a, 1], [b, 2], [c, 3]] | where size < 3).name | get 1",
        "b",
    )
}

#[test]
fn row_condition2() -> TestResult {
    run_test(
        "[[name, size]; [a, 1], [b, 2], [c, 3]] | where $it.size > 2 | length",
        "1",
    )
}

#[test]
fn better_block_types() -> TestResult {
    run_test(
        r#"([1, 2, 3] | each -n { $"($it.index) is ($it.item)" }).1"#,
        "1 is 2",
    )
}

#[test]
fn module_imports_1() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { 2 } }; use foo; foo a"#,
        "1",
    )
}

#[test]
fn module_imports_2() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { 2 } }; use foo a; a"#,
        "1",
    )
}

#[test]
fn module_imports_3() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; export def b [] { 2 } }; use foo *; b"#,
        "2",
    )
}

#[test]
fn module_imports_4() -> TestResult {
    fail_test(
        r#"module foo { export def a [] { 1 }; export def b [] { 2 } }; use foo c"#,
        "not find import",
    )
}

#[test]
fn module_imports_5() -> TestResult {
    run_test(
        r#"module foo { export def a [] { 1 }; def b [] { 2 }; export def c [] { 3 } }; use foo [a, c]; c"#,
        "3",
    )
}

#[test]
fn module_import_uses_internal_command() -> TestResult {
    run_test(
        r#"module foo { def b [] { 2 }; export def a [] { b }  }; use foo; foo a"#,
        "2",
    )
}

// TODO: Test the use/hide tests also as separate lines in REPL (i.e., with  merging the delta in between)
#[test]
fn hides_def() -> TestResult {
    fail_test(r#"def foo [] { "foo" }; hide foo; foo"#, not_found_msg())
}

#[test]
fn hides_def_then_redefines() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; hide foo; def foo [] { "bar" }; foo"#,
        "defined more than once",
    )
}

#[test]
fn hides_def_in_scope_1() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { hide foo; foo }"#,
        not_found_msg(),
    )
}

#[test]
fn hides_def_in_scope_2() -> TestResult {
    run_test(
        r#"def foo [] { "foo" }; do { def foo [] { "bar" }; hide foo; foo }"#,
        "foo",
    )
}

#[test]
fn hides_def_in_scope_3() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { hide foo; def foo [] { "bar" }; hide foo; foo }"#,
        not_found_msg(),
    )
}

#[test]
fn hides_def_in_scope_4() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; do { def foo [] { "bar" }; hide foo; hide foo; foo }"#,
        not_found_msg(),
    )
}

#[test]
fn hide_twice_not_allowed() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; hide foo; hide foo"#,
        "unknown command",
    )
}

#[test]
fn hides_import_1() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam foo; foo"#,
        not_found_msg(),
    )
}

#[test]
fn hides_import_2() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam *; foo"#,
        not_found_msg(),
    )
}

#[test]
fn hides_import_3() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam [foo]; foo"#,
        not_found_msg(),
    )
}

#[test]
fn hides_import_4() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam foo; hide foo; foo"#,
        not_found_msg(),
    )
}

#[test]
fn hides_import_5() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam *; hide foo; foo"#,
        not_found_msg(),
    )
}

#[test]
fn hides_import_6() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "foo" } }; use spam; hide spam; foo"#,
        not_found_msg(),
    )
}

#[test]
fn def_twice_should_fail() -> TestResult {
    fail_test(
        r#"def foo [] { "foo" }; def foo [] { "bar" }"#,
        "defined more than once",
    )
}

#[test]
fn use_import_after_hide() -> TestResult {
    run_test(
        r#"module spam { export def foo [] { "foo" } }; use spam foo; hide foo; use spam foo; foo"#,
        "foo",
    )
}

#[test]
fn hide_shadowed_decl() -> TestResult {
    run_test(
        r#"module spam { export def foo [] { "bar" } }; def foo [] { "foo" }; do { use spam foo; hide foo; foo }"#,
        "foo",
    )
}

#[test]
fn hides_all_decls_within_scope() -> TestResult {
    fail_test(
        r#"module spam { export def foo [] { "bar" } }; def foo [] { "foo" }; use spam foo; hide foo; foo"#,
        not_found_msg(),
    )
}

#[test]
fn from_json_1() -> TestResult {
    run_test(r#"('{"name": "Fred"}' | from json).name"#, "Fred")
}

#[test]
fn from_json_2() -> TestResult {
    run_test(
        r#"('{"name": "Fred"}
                   {"name": "Sally"}' | from json -o).name.1"#,
        "Sally",
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
fn select() -> TestResult {
    run_test(
        r#"([[name, age]; [a, 1], [b, 2]]) | select name | get 1 | get name"#,
        "b",
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
        r#""hello world" | split column " " | get "Column1".0"#,
        "hello",
    )
}

#[test]
fn for_loops() -> TestResult {
    run_test(r#"(for x in [1, 2, 3] { $x + 10 }).1"#, "12")
}

#[test]
fn par_each() -> TestResult {
    run_test(
        r#"1..10 | par-each --numbered { ([[index, item]; [$it.index, ($it.item > 5)]]).0 } | where index == 4 | get item.0"#,
        "false",
    )
}

#[test]
fn type_in_list_of_this_type() -> TestResult {
    run_test(r#"42 in [41 42 43]"#, "true")
}

#[test]
fn type_in_list_of_non_this_type() -> TestResult {
    fail_test(r#"'hello' in [41 42 43]"#, "mismatched for operation")
}

#[test]
fn string_in_string() -> TestResult {
    run_test(r#"'z' in 'abc'"#, "false")
}

#[test]
fn non_string_in_string() -> TestResult {
    fail_test(r#"42 in 'abc'"#, "mismatched for operation")
}

#[test]
fn int_in_inc_range() -> TestResult {
    run_test(r#"1 in -4..9.42"#, "true")
}

#[test]
fn int_in_dec_range() -> TestResult {
    run_test(r#"1 in 9.42..-4"#, "true")
}

#[test]
fn int_in_exclusive_range() -> TestResult {
    run_test(r#"3 in 0..<3"#, "false")
}

#[test]
fn non_number_in_range() -> TestResult {
    fail_test(r#"'a' in 1..3"#, "mismatched for operation")
}

#[test]
fn string_in_record() -> TestResult {
    run_test(r#""a" in ('{ "a": 13, "b": 14 }' | from json)"#, "true")
}

#[test]
fn non_string_in_record() -> TestResult {
    fail_test(
        r#"4 in ('{ "a": 13, "b": 14 }' | from json)"#,
        "mismatch during operation",
    )
}

#[test]
fn string_in_valuestream() -> TestResult {
    run_test(
        r#"
    'Hello' in ("Hello
    World" | lines)"#,
        "true",
    )
}

#[test]
fn string_not_in_string() -> TestResult {
    run_test(r#"'d' not-in 'abc'"#, "true")
}

#[test]
fn float_not_in_inc_range() -> TestResult {
    run_test(r#"1.4 not-in 2..9.42"#, "true")
}

#[test]
fn earlier_errors() -> TestResult {
    fail_test(
        r#"[1, "bob"] | each { $it + 3 } | each { $it / $it } | table"#,
        "int",
    )
}

#[test]
fn missing_column_error() -> TestResult {
    fail_test(
        r#"([([[name, size]; [ABC, 10], [DEF, 20]]).1, ([[name]; [HIJ]]).0]).size | table"#,
        "cannot find column",
    )
}

#[test]
fn missing_parameters() -> TestResult {
    fail_test(r#"def foo {}"#, "expected [")
}

#[test]
fn flag_param_value() -> TestResult {
    run_test(
        r#"def foo [--bob: int] { $bob + 100 }; foo --bob 55"#,
        "155",
    )
}

#[test]
fn do_rest_args() -> TestResult {
    run_test(r#"(do { |...rest| $rest } 1 2).1 + 10"#, "12")
}

#[test]
fn custom_switch1() -> TestResult {
    run_test(
        r#"def florb [ --dry-run: bool ] { if ($dry-run) { "foo" } else { "bar" } }; florb --dry-run"#,
        "foo",
    )
}

#[test]
fn custom_switch2() -> TestResult {
    run_test(
        r#"def florb [ --dry-run: bool ] { if ($dry-run) { "foo" } else { "bar" } }; florb"#,
        "bar",
    )
}

#[test]
fn custom_switch3() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { if ($dry-run) { "foo" } else { "bar" } }; florb --dry-run"#,
        "foo",
    )
}

#[test]
fn custom_switch4() -> TestResult {
    run_test(
        r#"def florb [ --dry-run ] { if ($dry-run) { "foo" } else { "bar" } }; florb"#,
        "bar",
    )
}

#[test]
fn bad_var_name() -> TestResult {
    fail_test(r#"let $"foo bar" = 4"#, "can't contain")
}

#[test]
fn long_flag() -> TestResult {
    run_test(
        r#"([a, b, c] | each --numbered { if $it.index == 1 { 100 } else { 0 } }).1"#,
        "100",
    )
}

#[test]
fn help_works_with_missing_requirements() -> TestResult {
    run_test(r#"each --help | lines | length"#, "10")
}

#[test]
fn scope_variable() -> TestResult {
    run_test(r#"let x = 3; $scope.vars.0"#, "$x")
}

#[test]
fn zip_ranges() -> TestResult {
    run_test(r#"1..3 | zip 4..6 | get 2.1"#, "6")
}

#[test]
fn shorthand_env_1() -> TestResult {
    run_test(r#"FOO=BAZ $nu.env.FOO"#, "BAZ")
}

#[test]
fn shorthand_env_2() -> TestResult {
    run_test(r#"FOO=BAZ FOO=MOO $nu.env.FOO"#, "MOO")
}

#[test]
fn shorthand_env_3() -> TestResult {
    run_test(r#"FOO=BAZ BAR=MOO $nu.env.FOO"#, "BAZ")
}

#[test]
fn shorthand_env_4() -> TestResult {
    fail_test(r#"FOO=BAZ FOO= $nu.env.FOO"#, "cannot find column")
}

#[test]
fn update_cell_path_1() -> TestResult {
    run_test(
        r#"[[name, size]; [a, 1.1]] | into int size | get size.0"#,
        "1",
    )
}
