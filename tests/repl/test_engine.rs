use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::concrete_variable_assignment(
    "let x = (1..100 | each { |y| $y + 100 }); let y = ($x | length); $x | length",
    100
)]
#[case::proper_shadow("let x = 10; let x = $x + 9; $x", 19)]
#[case::param_default_value_does_not_shadow_const(
    "const foo = 123; def bar [foo = $foo] { $foo }; bar",
    123
)]
#[case::flag_default_value_does_not_shadow_const(
    "const foo = 123; def bar [--foo = $foo] { $foo }; bar",
    123
)]
#[case::param_default_value_overridden_by_argument(
    "const foo = 123; def bar [foo = $foo] { $foo }; bar 456",
    456
)]
#[case::sibling_param_default_does_not_shadow_const(
    "const foo = 123; def bar [foo = $foo, a: int = $foo] { $foo + $a }; bar",
    246
)]
#[case::in_variable_3(r#"3 | if $in > 4 { "yay!" } else { $in }"#, 3)]
#[case::in_variable_4("3 | do { $in }", 3)]
#[case::in_variable_5("3 | if $in > 2 { $in - 10 } else { $in * 10 }", -7)]
#[case::in_variable_6("3 | if $in > 6 { $in - 10 } else { $in * 10 }", 30)]
#[case::in_and_if_else("[1, 2, 3] | if false {} else if true { $in | length }", 3)]
#[case::in_with_closure("3 | do { let x = $in; let y = $in; $x + $y }", 6)]
#[case::in_with_custom_command("def foo [] { let x = $in; let y = $in; $x + $y }; 3 | foo", 6)]
#[case::in_used_twice_and_also_in_pipeline(
    "3 | do { let x = $in; let y = $in; $x + $y | $in * 4 }",
    24
)]
#[case::in_used_in_range_from("6 | $in..10 | math sum", 40)]
#[case::in_used_in_range_to("6 | 3..$in | math sum", 18)]
#[case::missing_flags_are_nothing(
    "def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo",
    110
)]
#[case::missing_flags_are_nothing2(
    "def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -a 90",
    190
)]
#[case::missing_flags_are_nothing3(
    "def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -b 45",
    55
)]
#[case::missing_flags_are_nothing4(
    "def foo [--aaa(-a): int, --bbb(-b): int] { (if $aaa == null { 10 } else { $aaa }) + (if $bbb == null { 100 } else { $bbb }) }; foo -a 3 -b 10000",
    10003
)]
#[case::proper_variable_captures("def foo [x] { let y = 100; { || $y + $x } }; do (foo 23)", 123)]
#[case::proper_variable_captures_with_calls(
    "def foo [] { let y = 60; def bar [] { $y }; {|| bar } }; do (foo)",
    60
)]
#[case::proper_variable_captures_with_nesting(
    "def foo [x] { let z = 100; def bar [y] { $y - $x + $z } ; { |z| bar $z } }; do (foo 11) 13",
    102
)]
#[case::let_sees_input(r#"def c [] { let x = (str length); $x }; "hello world" | c"#, 11)]
#[case::let_sees_in_variable(
    "def c [] { let x = $in.name; $x | str length }; {name: bob, size: 100 } | c",
    3
)]
#[case::let_sees_in_variable2("def c [] { let x = ($in | str length); $x }; 'bob' | c", 3)]
#[case::open_ended_range("1.. | first 100000 | length", 100000)]
#[case::default_value1("def foo [x = 3] { $x }; foo", 3)]
#[case::default_value2("def foo [x: int = 3] { $x }; foo", 3)]
#[case::default_value3("def foo [--x = 3] { $x }; foo", 3)]
#[case::default_value4("def foo [--x: int = 3] { $x }; foo", 3)]
#[case::default_value5("def foo [x = 3] { $x }; foo 10", 10)]
#[case::default_value6("def foo [x: int = 3] { $x }; foo 10", 10)]
#[case::default_value7("def foo [--x = 3] { $x }; foo --x 10", 10)]
#[case::default_value8("def foo [--x: int = 3] { $x }; foo --x 10", 10)]
#[case::default_value_constant3(r#"def foo [x = ("foo" | str length)] { $x }; foo"#, 3)]
#[case::loose_each("[[1, 2, 3], [4, 5, 6]] | each {|| $in.1 } | math sum", 7)]
#[case::in_means_input("def shl [] { $in * 2 }; 2 | shl", 4)]
#[case::reusable_in("[1, 2, 3, 4] | take (($in | length) - 1) | math sum", 6)]
#[case::range_right_exclusive("[1, 4, 5, 8, 9] | slice 1..<3 | math sum", 9)]
#[case::short_flags_2(
    r#"def foobar [-a: int, -b: string, -c: int] { $a + $c };foobar -b "balh balh" -a 10  -c 1 "#,
    11
)]
fn integer_successes(#[case] code: &str, #[case] expected: i64) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::in_variable_1(r#"[3] | if $in.0 > 4 { "yay!" } else { "boo" }"#, "boo")]
#[case::in_variable_2(r#"3 | if $in > 2 { "yay!" } else { "boo" }"#, "yay!")]
#[case::def_env(r#"def --env bob [] { $env.BAR = "BAZ" }; bob; $env.BAR"#, "BAZ")]
#[case::export_def_env(
    r#"module foo { export def --env bob [] { $env.BAR = "BAZ" } }; use foo bob; bob; $env.BAR"#,
    "BAZ"
)]
#[case::dynamic_load_env(r#"let x = "FOO"; load-env {$x: "BAZ"}; $env.FOO"#, "BAZ")]
#[case::with_env_shorthand_nested_quotes(
    r#"FOO='-arg "hello world"' echo $env | get FOO"#,
    "-arg \"hello world\""
)]
#[case::default_value_glob("def foo [--x:glob = *.nu] { $x | describe }; foo", "glob")]
#[case::default_value_constant1(r#"def foo [x = "foo"] { $x }; foo"#, "foo")]
#[case::in_iteration(
    r#"[3, 4, 5] | each {|| echo $"hi ($in)" } | str join"#,
    "hi 3hi 4hi 5"
)]
#[case::call_rest_arg_span(
    "let l = [2, 3]; def foo [...rest] { metadata $rest | view span $in.span.start $in.span.end }; foo 1 ...$l",
    "1 ...$l"
)]
#[case::short_flags(
    r#"def foobar [-a: int, -b: string, -c: string] { echo $'($a) ($c) ($b)' }; foobar -b "balh balh" -a 1543  -c "FALSE123""#,
    "1543 FALSE123 balh balh"
)]
#[case::short_flags_1(
    "def foobar [-a: string, -b: string, -s: int] { if ( $s == 0 ) { echo $'($b)($a)' }}; foobar -a test -b case -s 0  ",
    "casetest"
)]
fn string_successes(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn default_value_constant2() -> Result {
    test()
        .run("def foo [secs = 1sec] { $secs }; foo")
        .expect_value_eq(std::time::Duration::from_secs(1))
}

#[rstest]
#[case::divide_duration("4ms / 4ms", 1.0)]
#[case::divide_filesize("4mb / 4mb", 1.0)]
fn float_successes(#[case] code: &str, #[case] expected: f64) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::date_comparison("(date now) < ((date now) + 2min)", true)]
#[case::datetime_literal("(date now) - 2019-08-23 > 1hr", true)]
#[case::shortcircuiting_and("false and (5 / 0; false)", false)]
#[case::shortcircuiting_or("true or (5 / 0; false)", true)]
#[case::better_operator_spans(
    "metadata ({foo: 10} | (20 - $in.foo)) | get span | $in.start < $in.end",
    true
)]
fn bool_successes(#[case] code: &str, #[case] expected: bool) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn test_redirection_stderr() -> Result {
    test()
        .run("do -i { asdjw4j5cnaabw44rd }; 'done'")
        .expect_value_eq("done")
}

#[test]
fn nonshortcircuiting_xor() -> Result {
    test()
        .run(r#"true xor (print "hello"; false) | ignore"#)
        .expect_value_eq(())
}

#[test]
fn help_works_with_missing_requirements() -> Result {
    test()
        .run("each")
        .expect_error_code_eq("nu::parser::missing_positional")?;

    let help: String = test().run("each --help")?;
    assert_contains("Usage", help);
    Ok(())
}

#[rstest]
#[case("let x = 3", "$x", "int", 3)]
#[case("const x = 3", "$x", "int", 3)]
fn scope_variable(
    #[case] var_decl: &str,
    #[case] exp_name: &str,
    #[case] exp_type: &str,
    #[case] exp_value: i64,
) -> Result {
    let get_var_info =
        format!(r#"{var_decl}; scope variables | where name == "{exp_name}" | first"#);

    test()
        .run(format!("{get_var_info} | get type"))
        .expect_value_eq(exp_type)?;
    test()
        .run(format!("{get_var_info} | get value"))
        .expect_value_eq(exp_value)
}

#[rstest]
#[case("a", "<> nothing")]
#[case("b", "<1.23> float")]
#[case("flag1", "<> nothing")]
#[case("flag2", "<4.56> float")]
fn scope_command_defaults(#[case] var: &str, #[case] expected: &str) -> Result {
    test()
        .run(format!(
            r#"def t1 [a:int b?:float=1.23 --flag1:string --flag2:float=4.56] {{ true }};
            let rslt = (scope commands | where name == 't1' | get signatures.0.any | where parameter_name == '{var}' | get parameter_default.0);
            $"<($rslt)> ($rslt | describe)""#
        ))
        .expect_value_eq(expected)
}

#[rstest]
#[case::earlier_errors(
    r#"[1, "bob"] | each { |it| $it + 3 } | each { |it| $it / $it } | table"#,
    "OperatorIncompatibleTypes"
)]
#[case::not_def_env(r#"def bob [] { $env.BAR = "BAZ" }; bob; $env.BAR"#, "")]
#[case::def_env_hiding_something(
    r#"$env.FOO = "foo"; def --env bob [] { hide-env FOO }; bob; $env.FOO"#,
    ""
)]
#[case::def_env_then_hide(
    r#"def --env bob [] { $env.BOB = "bob" }; def --env un-bob [] { hide-env BOB }; bob; un-bob; $env.BOB"#,
    ""
)]
#[case::reduce_spans(
    r#"
        let x = ([1, 2, 3] | reduce --fold 0 {|it, acc| $it + 2 * $acc })
        let span = (metadata $x).span
        error make {
            msg: "oh that hurts"
            label: {
                text: "right here"
                span: $span
            }
        }
    "#,
    "right here"
)]
fn failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_shell_error()?;

    if !expected.is_empty() {
        assert_contains(expected, format!("{error:?}"));
    }

    Ok(())
}

#[rstest]
#[case::default_value9("def foo [--x = 3] { $x }; foo --x a", "Expected")]
#[case::default_value10("def foo [x = 3] { $x }; foo a", "Expected")]
#[case::default_value11("def foo [x = 3, y] { $x }; foo a", "RequiredAfterOptional")]
#[case::default_value12(r#"def foo [--x:int = "a"] { $x }"#, "Expected")]
#[case::default_value_not_constant2(
    "def foo [x = (loop { break })] { $x }; foo",
    "NonConstantDefaultValue"
)]
#[case::assignment_to_in_var_no_panic("$in = 3", "AssignmentRequiresMutableVar")]
fn parse_failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_parse_error()?;
    assert_contains(expected, format!("{error:?}"));
    Ok(())
}

#[rstest]
#[case::assignment_to_env_no_panic("$env = 3", "CannotReplaceEnv")]
fn compile_failures(#[case] code: &str, #[case] expected: &str) -> Result {
    let error = test().run(code).expect_compile_error()?;
    assert_contains(expected, format!("{error:?}"));
    Ok(())
}

#[test]
fn shadowed_variables_in_aliases() -> Result {
    let mut tester = test();
    tester
        .run("let x = 10; alias foo = echo $x; foo")
        .expect_value_eq(10)?;
    let () = tester.run("let x = 20")?;
    tester
        .engine_state
        .cleanup_stack_variables(&mut tester.stack);
    tester.run("foo").expect_value_eq(10)?;
    Ok(())
}
