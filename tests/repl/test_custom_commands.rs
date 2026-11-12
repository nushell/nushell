use nu_protocol::{ParseError, ShellError, Type};
use nu_test_support::{prelude::*, tester::NuTester};
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn no_scope_leak1() -> Result {
    let err = test()
        .run("if false { let $x = 10 } else { let $x = 20 }; $x")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::VariableNotFound(_, _));
    Ok(())
}

#[test]
fn no_scope_leak2() -> Result {
    let err = test()
        .run("def foo [] { $x }; def bar [] { let $x = 10; foo }; bar")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::VariableNotFound(_, _));
    Ok(())
}

#[test]
fn no_scope_leak3() -> Result {
    test()
        .run("def foo [$x] { $x }; def bar [] { let $x = 10; foo 20}; bar")
        .expect_value_eq(20)
}

#[test]
fn no_scope_leak4() -> Result {
    test()
        .run("def foo [$x] { $x }; def bar [] { let $x = 10; (foo 20) + $x}; bar")
        .expect_value_eq(30)
}

#[test]
fn custom_rest_var() -> Result {
    test()
        .run("def foo [...x] { $x.0 + $x.1 }; foo 10 80")
        .expect_value_eq(90)
}

#[test]
fn def_twice_should_fail() -> Result {
    let err = test()
        .run(r#"def foo [] { "foo" }; def foo [] { "bar" }"#)
        .expect_parse_error()?;
    assert_matches!(err, ParseError::DuplicateCommandDef(_));
    Ok(())
}

#[test]
fn missing_parameters() -> Result {
    let err = test().run("def foo {}").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected("[ or (", _));
    Ok(())
}

#[test]
fn flag_param_value() -> Result {
    test()
        .run("def foo [--bob: int] { $bob + 100 }; foo --bob 55")
        .expect_value_eq(155)
}

#[test]
fn do_rest_args() -> Result {
    test()
        .run("(do { |...rest| $rest } 1 2).1 + 10")
        .expect_value_eq(12)
}

#[test]
fn custom_switch1() -> Result {
    test()
        .run(r#"def florb [ --dry-run ] { if ($dry_run) { "foo" } else { "bar" } }; florb --dry-run"#)
        .expect_value_eq("foo")
}

#[rstest]
fn custom_flag_with_type_checking(
    #[values(
        ("int", "\"3\""),
        ("record<i: int>", "{i: \"\"}"),
        ("list<int>", "[\"\"]")
    )]
    (type_sig, value): (&str, &str),
    #[values("--dry-run", "-d")] flag: &str,
) -> Result {
    let code = format! {"
        def florb [{flag}: {type_sig}] {{}}
        let y = {value}
        florb {flag} $y
    "};

    test()
        .run(code)
        .expect_error_code_eq("nu::parser::type_mismatch")
}

/// `null` is intentionally allowed for optional named flags: it omits the flag
/// (same as not passing it) rather than type-mismatching.
#[rstest]
fn custom_flag_null_is_omitted(#[values("--dry-run", "-d")] flag: &str) -> Result {
    let code = format! {"
        def florb [--dry-run (-d): int] {{ $dry_run }}
        let y = null
        florb {flag} $y
    "};

    test().run(code).expect_value_eq(())
}

#[test]
fn custom_switch2() -> Result {
    test()
        .run(r#"def florb [ --dry-run ] { if ($dry_run) { "foo" } else { "bar" } }; florb"#)
        .expect_value_eq("bar")
}

#[test]
fn custom_switch3() -> Result {
    test()
        .run("def florb [ --dry-run ] { $dry_run }; florb --dry-run=false")
        .expect_value_eq(false)
}

#[test]
fn custom_switch4() -> Result {
    test()
        .run("def florb [ --dry-run ] { $dry_run }; florb --dry-run=true")
        .expect_value_eq(true)
}

#[test]
fn custom_switch5() -> Result {
    test()
        .run("def florb [ --dry-run ] { $dry_run }; florb")
        .expect_value_eq(false)
}

#[test]
fn custom_switch6() -> Result {
    test()
        .run("def florb [ --dry-run ] { $dry_run }; florb --dry-run")
        .expect_value_eq(true)
}

#[test]
fn custom_flag1() -> Result {
    let code = r#"
        def florb [
            --age: int = 0
            --name = "foobar"
        ] {
            ($age | into string) + $name
        }

        florb
    "#;

    test().run(code).expect_value_eq("0foobar")
}

#[test]
fn custom_flag2() -> Result {
    let code = r#"
        def florb [
            --age: int
            --name = "foobar"
        ] {
            ($age | into string) + $name
        }
        
        florb --age 3
    "#;

    test().run(code).expect_value_eq("3foobar")
}

#[test]
fn deprecated_boolean_flag() -> Result {
    let err = test()
        .run(r#"def florb [--dry-run: bool, --another-flag] { "aaa" };  florb"#)
        .expect_parse_error()?;
    assert_contains("not allowed", std::dbg!(err).to_string());
    Ok(())
}

#[test]
fn simple_var_closing() -> Result {
    test()
        .run("let $x = 10; def foo [] { $x }; foo")
        .expect_value_eq(10)
}

#[test]
fn predecl_check() -> Result {
    test()
        .run("def bob [] { sam }; def sam [] { 3 }; bob")
        .expect_value_eq(3)
}

#[test]
fn def_with_no_dollar() -> Result {
    test()
        .run("def bob [x] { $x + 3 }; bob 4")
        .expect_value_eq(7)
}

#[test]
fn allow_missing_optional_params() -> Result {
    test()
        .run("def foo [x?:int] { if $x != null { $x + 10 } else { 5 } }; foo")
        .expect_value_eq(5)
}

#[test]
fn help_present_in_def() -> Result {
    let actual: String = test().run("def foo [] {}; help foo")?;
    assert_contains("Display the help message for this command", actual);
    Ok(())
}

#[test]
fn help_not_present_in_extern() -> Result {
    let code = r#"
        module test {export extern "git fetch" []};
        use test `git fetch`;
        help git fetch | find help | to text | ansi strip
    "#;

    test().run(code).expect_value_eq("")
}

#[test]
fn override_table() -> Result {
    test()
        .run(r#"def table [-e] { "hi" }; table"#)
        .expect_value_eq("hi")
}

#[test]
fn override_table_eval_file() -> Result {
    test()
        .run(r#"def table [-e] { "hi" }; table"#)
        .expect_value_eq("hi")
}

/// A tester whose engine config has `recursion_limit` set to `limit`.
fn test_with_recursion_limit(limit: i64) -> NuTester {
    let mut tester = test();
    let mut config = tester.engine_state.get_config().as_ref().clone();
    config.recursion_limit = limit;
    tester.engine_state.set_config(config);
    tester
}

#[test]
fn infinite_recursion_does_not_panic() -> Result {
    let err = test_with_recursion_limit(5)
        .run("def bang [] { bang }; bang")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::RecursionLimitReached {
            recursion_limit: 5,
            ..
        }
    );
    Ok(())
}

#[test]
fn infinite_mutual_recursion_does_not_panic() -> Result {
    let err = test_with_recursion_limit(5)
        .run("def bang [] { def boom [] { bang }; boom }; bang")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::RecursionLimitReached {
            recursion_limit: 5,
            ..
        }
    );
    Ok(())
}

#[test]
fn infinite_closure_recursion_does_not_panic() -> Result {
    let err = test_with_recursion_limit(5)
        .run("let M = {|x| do $x $x}; do $M $M")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::RecursionLimitReached {
            recursion_limit: 5,
            ..
        }
    );
    Ok(())
}

#[rstest]
#[case::within_limit(2, true)]
#[case::past_limit(3, false)]
fn closure_calls_count_toward_recursion_limit(#[case] depth: i64, #[case] ok: bool) -> Result {
    // Each level of `f` is one custom command call plus one closure call, so depth 2 nests
    // 5 calls (within the limit of 5) and depth 3 nests 7.
    let mut tester = test_with_recursion_limit(5);
    let code =
        format!("def f [n] {{ if $n > 0 {{ do {{ f ($n - 1) }} }} else {{ 'ok' }} }}; f {depth}");
    if ok {
        return tester.run(code).expect_value_eq("ok");
    }
    let err = tester.run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::RecursionLimitReached {
            recursion_limit: 5,
            ..
        }
    );
    Ok(())
}

#[test]
fn infinite_closure_recursion_in_par_each_does_not_panic() -> Result {
    // A par-each worker that recurses without end must hit the recursion
    // limit like the main thread does. This only holds when the worker
    // threads have a stack big enough for the limit; an overflow would
    // abort the whole process instead of raising this error.
    // The default limit (50) is the case that needs the deeper worker stack:
    // with Rayon's smaller default stack the worker overflows and aborts the
    // process before the limit check can fire.
    let err = test_with_recursion_limit(50)
        .run("[1] | par-each {|_| let M = {|x| do $x $x}; do $M $M}")
        .expect_shell_error()?;
    let hit_limit = matches!(
        &err,
        ShellError::RecursionLimitReached {
            recursion_limit: 50,
            ..
        }
    ) || matches!(
        &err,
        ShellError::EvalBlockWithInput(wrapped)
            if matches!(
                wrapped.error.as_ref(),
                ShellError::RecursionLimitReached { recursion_limit: 50, .. }
            )
    );
    assert!(hit_limit, "expected the recursion limit, got: {err}");
    Ok(())
}

#[test]
fn recursion_limit_set_in_script_applies() -> Result {
    // The limit is read from the stack's config, so setting it earlier in the same script
    // takes effect without going through `config.nu`.
    let err = test_with_recursion_limit(50)
        .run("$env.config.recursion_limit = 3; def bang [] { bang }; bang")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::RecursionLimitReached {
            recursion_limit: 3,
            ..
        }
    );
    Ok(())
}

#[test]
fn closure_called_per_item_does_not_count_toward_recursion_limit() -> Result {
    // `each` evaluates one closure call per item on the same callee stack, so a long
    // stream must not add one recursion level per item.
    test_with_recursion_limit(5)
        .run("1..100 | each {|x| $x } | length")
        .expect_value_eq(100)
}

#[test]
fn par_each_closure_counts_one_recursion_level() -> Result {
    // Each streaming `par-each` closure call is one level, so two nested `par-each` calls
    // fit in a limit of 2.
    test_with_recursion_limit(2)
        .run("[1 2] | par-each { [1 2] | par-each { 1 } | math sum } | math sum")
        .expect_value_eq(4)
}

#[test]
fn type_check_for_during_eval() -> Result {
    let err = test()
        .run("def spam [foo: string] { $foo | describe }; def outer [--foo: string] { spam $foo }; outer")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "string" && from_type == "nothing"
    );
    Ok(())
}
#[test]
fn type_check_for_during_eval2() -> Result {
    let err = test()
        .run("def spam [foo: string] { $foo | describe }; def outer [--foo: any] { spam $foo }; outer")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "string" && from_type == "nothing"
    );
    Ok(())
}

#[test]
fn empty_list_matches_list_type() -> Result {
    test()
        .run("def spam [foo: list<int>] { echo $foo }; spam [] | length")
        .expect_value_eq(0)?;
    test()
        .run("def spam [foo: list<string>] { echo $foo }; spam [] | length")
        .expect_value_eq(0)
}

#[test]
fn path_argument_dont_auto_expand_if_single_quoted() -> Result {
    test()
        .run("def spam [foo: path] { echo $foo }; spam '~/aa'")
        .expect_value_eq("~/aa")
}

#[test]
fn path_argument_dont_auto_expand_if_double_quoted() -> Result {
    test()
        .run(r#"def spam [foo: path] { echo $foo }; spam "~/aa""#)
        .expect_value_eq("~/aa")
}

#[test]
fn path_argument_dont_make_absolute_if_unquoted() -> Result {
    test()
        .run("def spam [foo: path] { echo $foo }; spam foo/.../bar")
        .expect_value_eq(cfg_select! {
            windows => "..\\bar",
            _ => "../bar",
        })
}

#[test]
fn dont_allow_implicit_casting_between_glob_and_string() -> Result {
    let err = test()
        .run("def spam [foo: string] { echo $foo }; let f: glob = 'aa'; spam $f")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::String, Type::Glob, _));
    test()
        .run("def spam [foo: glob] { echo $foo }; let f = 'aa'; spam $f")
        .expect_value_eq("aa")
}

#[test]
fn allow_pass_negative_float() -> Result {
    test()
        .run("def spam [val: float] { $val }; spam -1.4")
        .expect_value_eq(-1.4)?;
    test()
        .run("def spam [val: float] { $val }; spam -2")
        .expect_value_eq(-2.0)
}

#[test]
fn glob_bare_word_with_interpolation() -> Result {
    test()
        .run("def spam [foo: glob] { $foo | describe }; let var = 'val'; spam ~/($var)/test")
        .expect_value_eq("glob")?;
    test()
        .run("def spam [--foo: glob] { $foo | describe }; let var = 'val'; spam --foo ~/($var)")
        .expect_value_eq("glob")?;
    test()
        .run("def spam [foo: glob] { $foo | describe }; let var = 'val'; spam ($var)/test")
        .expect_value_eq("glob")
}

#[test]
fn glob_string_interpolation() -> Result {
    test()
        .run("def spam [--foo: glob] { $foo | describe }; let var = 'val'; spam --foo $\"/path/($var)\"")
        .expect_value_eq("glob")
}

#[test]
fn glob_no_interpolation() -> Result {
    test()
        .run("def spam [foo: glob] { $foo | describe }; spam *.nu")
        .expect_value_eq("glob")?;
    test()
        .run("def spam [--foo: glob] { $foo | describe }; spam --foo '*.nu'")
        .expect_value_eq("glob")?;
    test()
        .run("def spam [foo: glob] { $foo | describe }; spam `*.nu`")
        .expect_value_eq("glob")
}

#[test]
fn glob_literal_string_interpolation() -> Result {
    test()
        .run("def spam [foo: glob] { $foo }; spam $\"/path/to/file\"")
        .expect_value_eq("/path/to/file")
}

#[test]
fn glob_literal_string_interpolation_with_metachars() -> Result {
    test()
        .run("def spam [foo: glob] { $foo }; spam $\"/path/[foo]*.txt\"")
        .expect_value_eq("/path/[foo]*.txt")
}
