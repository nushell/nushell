use crate::tests::{fail_test, run_test, TestResult};

#[test]
fn concrete_variable_assignment() -> TestResult {
    run_test(
        "let x = (1..100 | each { |y| $y + 100 }); let y = ($x | length); $x | length",
        "100",
    )
}

#[test]
fn proper_shadow() -> TestResult {
    run_test("let x = 10; let x = $x + 9; $x", "19")
}

#[test]
fn in_variable_1() -> TestResult {
    run_test(r#"[3] | if $in.0 > 4 { "yay!" } else { "boo" }"#, "boo")
}

#[test]
fn in_variable_2() -> TestResult {
    run_test(r#"3 | if $in > 2 { "yay!" } else { "boo" }"#, "yay!")
}

#[test]
fn in_variable_3() -> TestResult {
    run_test(r#"3 | if $in > 4 { "yay!" } else { $in }"#, "3")
}

#[test]
fn in_variable_4() -> TestResult {
    run_test(r#"3 | do { $in }"#, "3")
}

#[test]
fn in_variable_5() -> TestResult {
    run_test(r#"3 | if $in > 2 { $in - 10 } else { $in * 10 }"#, "-7")
}

#[test]
fn in_variable_6() -> TestResult {
    run_test(r#"3 | if $in > 6 { $in - 10 } else { $in * 10 }"#, "30")
}

#[test]
fn in_and_if_else() -> TestResult {
    run_test(
        r#"[1, 2, 3] | if false {} else if true { $in | length }"#,
        "3",
    )
}

#[test]
fn help_works_with_missing_requirements() -> TestResult {
    run_test(r#"each --help | lines | length"#, "65")
}

#[test]
fn scope_variable() -> TestResult {
    run_test(
        r#"let x = 3; $nu.scope.vars | where name == "$x" | get type.0"#,
        "int",
    )
}

#[test]
fn earlier_errors() -> TestResult {
    fail_test(
        r#"[1, "bob"] | each { |it| $it + 3 } | each { |it| $it / $it } | table"#,
        "int",
    )
}

#[test]
fn missing_flags_are_nothing() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo"#,
        "110",
    )
}

#[test]
fn missing_flags_are_nothing2() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -a 90"#,
        "190",
    )
}

#[test]
fn missing_flags_are_nothing3() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -b 45"#,
        "55",
    )
}

#[test]
fn missing_flags_are_nothing4() -> TestResult {
    run_test(
        r#"def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -a 3 -b 10000"#,
        "10003",
    )
}

#[test]
fn proper_variable_captures() -> TestResult {
    run_test(
        r#"def foo [x] { let y = 100; { || $y + $x } }; do (foo 23)"#,
        "123",
    )
}

#[test]
fn proper_variable_captures_with_calls() -> TestResult {
    run_test(
        r#"def foo [] { let y = 60; def bar [] { $y }; {|| bar } }; do (foo)"#,
        "60",
    )
}

#[test]
fn proper_variable_captures_with_nesting() -> TestResult {
    run_test(
        r#"def foo [x] { let z = 100; def bar [y] { $y - $x + $z } ; { |z| bar $z } }; do (foo 11) 13"#,
        "102",
    )
}

#[test]
fn divide_duration() -> TestResult {
    run_test(r#"4ms / 4ms"#, "1")
}

#[test]
fn divide_filesize() -> TestResult {
    run_test(r#"4mb / 4mb"#, "1")
}

#[test]
fn date_comparison() -> TestResult {
    run_test(r#"(date now) < ((date now) + 2min)"#, "true")
}

#[test]
fn let_sees_input() -> TestResult {
    run_test(
        r#"def c [] { let x = (str length); $x }; "hello world" | c"#,
        "11",
    )
}

#[test]
fn let_sees_in_variable() -> TestResult {
    run_test(
        r#"def c [] { let x = $in.name; $x | str length }; {name: bob, size: 100 } | c"#,
        "3",
    )
}

#[test]
fn let_sees_in_variable2() -> TestResult {
    run_test(
        r#"def c [] { let x = ($in | str length); $x }; 'bob' | c"#,
        "3",
    )
}

#[test]
fn def_env() -> TestResult {
    run_test(
        r#"def-env bob [] { let-env BAR = "BAZ" }; bob; $env.BAR"#,
        "BAZ",
    )
}

#[test]
fn not_def_env() -> TestResult {
    fail_test(r#"def bob [] { let-env BAR = "BAZ" }; bob; $env.BAR"#, "")
}

#[test]
fn def_env_hiding_something() -> TestResult {
    fail_test(
        r#"let-env FOO = "foo"; def-env bob [] { hide-env FOO }; bob; $env.FOO"#,
        "",
    )
}

#[test]
fn def_env_then_hide() -> TestResult {
    fail_test(
        r#"def-env bob [] { let-env BOB = "bob" }; def-env un-bob [] { hide-env BOB }; bob; un-bob; $env.BOB"#,
        "",
    )
}

#[test]
fn export_def_env() -> TestResult {
    run_test(
        r#"module foo { export def-env bob [] { let-env BAR = "BAZ" } }; use foo bob; bob; $env.BAR"#,
        "BAZ",
    )
}

#[test]
fn dynamic_let_env() -> TestResult {
    run_test(r#"let x = "FOO"; let-env $x = "BAZ"; $env.FOO"#, "BAZ")
}

#[test]
fn reduce_spans() -> TestResult {
    fail_test(
        r#"let x = ([1, 2, 3] | reduce -f 0 { $it.item + 2 * $it.acc }); error make {msg: "oh that hurts", label: {text: "right here", start: (metadata $x).span.start, end: (metadata $x).span.end } }"#,
        "right here",
    )
}

#[test]
fn with_env_shorthand_nested_quotes() -> TestResult {
    run_test(
        r#"FOO='-arg "hello world"' echo $env | get FOO"#,
        "-arg \"hello world\"",
    )
}

#[test]
fn test_redirection_stderr() -> TestResult {
    // try a nonsense binary
    run_test(r#"do -i { asdjw4j5cnaabw44rd }; echo done"#, "done")
}

#[test]
fn datetime_literal() -> TestResult {
    run_test(r#"(date now) - 2019-08-23 > 1hr"#, "true")
}

#[test]
fn shortcircuiting_and() -> TestResult {
    run_test(r#"false and (5 / 0; false)"#, "false")
}

#[test]
fn shortcircuiting_or() -> TestResult {
    run_test(r#"true or (5 / 0; false)"#, "true")
}

#[test]
fn nonshortcircuiting_xor() -> TestResult {
    run_test(r#"true xor (print "hello"; false) | ignore"#, "hello")
}

#[test]
fn open_ended_range() -> TestResult {
    run_test(r#"1.. | first 100000 | length"#, "100000")
}

#[test]
fn default_value1() -> TestResult {
    run_test(r#"def foo [x = 3] { $x }; foo"#, "3")
}

#[test]
fn default_value2() -> TestResult {
    run_test(r#"def foo [x: int = 3] { $x }; foo"#, "3")
}

#[test]
fn default_value3() -> TestResult {
    run_test(r#"def foo [--x = 3] { $x }; foo"#, "3")
}

#[test]
fn default_value4() -> TestResult {
    run_test(r#"def foo [--x: int = 3] { $x }; foo"#, "3")
}

#[test]
fn default_value5() -> TestResult {
    run_test(r#"def foo [x = 3] { $x }; foo 10"#, "10")
}

#[test]
fn default_value6() -> TestResult {
    run_test(r#"def foo [x: int = 3] { $x }; foo 10"#, "10")
}

#[test]
fn default_value7() -> TestResult {
    run_test(r#"def foo [--x = 3] { $x }; foo --x 10"#, "10")
}

#[test]
fn default_value8() -> TestResult {
    run_test(r#"def foo [--x: int = 3] { $x }; foo --x 10"#, "10")
}

#[test]
fn default_value9() -> TestResult {
    fail_test(r#"def foo [--x = 3] { $x }; foo --x a"#, "expected int")
}

#[test]
fn default_value10() -> TestResult {
    fail_test(r#"def foo [x = 3] { $x }; foo a"#, "expected int")
}

#[test]
fn default_value11() -> TestResult {
    fail_test(
        r#"def foo [x = 3, y] { $x }; foo a"#,
        "after optional parameter",
    )
}

#[test]
fn default_value12() -> TestResult {
    fail_test(
        r#"def foo [--x:int = "a"] { $x }"#,
        "expected default value to be `int`",
    )
}

#[test]
fn default_value_constant() -> TestResult {
    run_test(r#"def foo [x = "foo"] { $x }; foo"#, "foo")
}

#[test]
fn default_value_not_constant1() -> TestResult {
    fail_test(
        r#"def foo [x = ("foo" | str length)] { $x }; foo"#,
        "expected a constant",
    )
}

#[test]
fn default_value_not_constant2() -> TestResult {
    fail_test(
        r#"def foo [--x = ("foo" | str length)] { $x }; foo"#,
        "expected a constant",
    )
}

#[test]
fn loose_each() -> TestResult {
    run_test(
        r#"[[1, 2, 3], [4, 5, 6]] | each {|| $in.1 } | math sum"#,
        "7",
    )
}

#[test]
fn in_means_input() -> TestResult {
    run_test(r#"def shl [] { $in * 2 }; 2 | shl"#, "4")
}

#[test]
fn in_iteration() -> TestResult {
    run_test(
        r#"[3, 4, 5] | each {|| echo $"hi ($in)" } | str join"#,
        "hi 3hi 4hi 5",
    )
}

#[test]
fn reuseable_in() -> TestResult {
    run_test(
        r#"[1, 2, 3, 4] | take (($in | length) - 1) | math sum"#,
        "6",
    )
}

#[test]
fn better_operator_spans() -> TestResult {
    run_test(
        r#"metadata ({foo: 10} | (20 - $in.foo)) | get span | $in.start < $in.end"#,
        "true",
    )
}

#[test]
fn range_right_exclusive() -> TestResult {
    run_test(r#"[1, 4, 5, 8, 9] | range 1..<3 | math sum"#, "9")
}

/// Issue #7872
#[test]
fn assignment_to_in_var_no_panic() -> TestResult {
    fail_test(r#"$in = 3"#, "needs to be a mutable variable")
}

#[test]
fn assignment_to_env_no_panic() -> TestResult {
    fail_test(r#"$env = 3"#, "cannot_replace_env")
}

#[test]
fn short_flags() -> TestResult {
    run_test(
        r#"def foobar [-a: int, -b: string, -c: string] { echo $'($a) ($c) ($b)' }; foobar -b "balh balh" -a 1543  -c "FALSE123""#,
        "1543 FALSE123 balh balh",
    )
}

#[test]
fn short_flags_1() -> TestResult {
    run_test(
        r#"def foobar [-a: string, -b: string, -s: int] { if ( $s == 0 ) { echo $'($b)($a)' }}; foobar -a test -b case -s 0  "#,
        "casetest",
    )
}

#[test]
fn short_flags_2() -> TestResult {
    run_test(
        r#"def foobar [-a: int, -b: string, -c: int] { $a + $c };foobar -b "balh balh" -a 10  -c 1 "#,
        "11",
    )
}
