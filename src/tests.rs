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
fn block_param1() -> TestResult {
    run_test("[3] | each { $it + 10 }", "[13]")
}

#[test]
fn block_param2() -> TestResult {
    run_test("[3] | each { |y| $y + 10 }", "[13]")
}

#[test]
fn block_param3_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { $it + 10 }", "[11, 12, 13]")
}

#[test]
fn block_param4_list_iteration() -> TestResult {
    run_test("[1,2,3] | each { |y| $y + 10 }", "[11, 12, 13]")
}

#[test]
fn range_iteration1() -> TestResult {
    run_test("1..4 | each { |y| $y + 10 }", "[11, 12, 13, 14]")
}

#[test]
fn range_iteration2() -> TestResult {
    run_test("4..1 | each { |y| $y + 100 }", "[104, 103, 102, 101]")
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
        "['sam','rick','pete'] | each { build-string $it ' is studying'}",
        "[sam is studying, rick is studying, pete is studying]",
    )
}

#[test]
fn build_string5() -> TestResult {
    run_test(
        "['sam','rick','pete'] | each { |x| build-string $x ' is studying'}",
        "[sam is studying, rick is studying, pete is studying]",
    )
}

#[test]
fn cell_path_subexpr1() -> TestResult {
    run_test("([[lang, gems]; [nu, 100]]).lang", "[nu]")
}

#[test]
fn cell_path_subexpr2() -> TestResult {
    run_test("([[lang, gems]; [nu, 100]]).lang.0", "nu")
}

#[test]
fn cell_path_var1() -> TestResult {
    run_test("let x = [[lang, gems]; [nu, 100]]; $x.lang", "[nu]")
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
        "[[name, size]; [tj, 100], [rl, 200]] | each { $it.size * 8 }",
        "[800, 1600]",
    )
}

#[test]
fn record_iteration() -> TestResult {
    run_test("([[name, level]; [aa, 100], [bb, 200]] | each { $it | each { |x| if $x.column == \"level\" { $x.value + 100 } else { $x.value } } }).level", "[200, 300]")
}

#[test]
fn row_condition1() -> TestResult {
    run_test(
        "([[name, size]; [a, 1], [b, 2], [c, 3]] | where size < 3).name",
        "[a, b]",
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
        r#"module foo { def a [] { 1 }; def b [] { 2 } }; use foo; foo.a"#,
        "1",
    )
}

#[test]
fn module_imports_2() -> TestResult {
    run_test(
        r#"module foo { def a [] { 1 }; def b [] { 2 } }; use foo.a; a"#,
        "1",
    )
}

#[test]
fn module_imports_3() -> TestResult {
    run_test(
        r#"module foo { def a [] { 1 }; def b [] { 2 } }; use foo.*; b"#,
        "2",
    )
}

#[test]
fn module_imports_4() -> TestResult {
    fail_test(
        r#"module foo { def a [] { 1 }; def b [] { 2 } }; use foo.c"#,
        "not find import",
    )
}

#[test]
fn module_imports_5() -> TestResult {
    run_test(
        r#"module foo { def a [] { 1 }; def b [] { 2 }; def c [] { 3 } }; use foo.[a, c]; c"#,
        "3",
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
