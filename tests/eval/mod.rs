use fancy_regex::Regex;
use nu_protocol::PipelineData;
use nu_test_support::{fs::Stub::FileWithContent, prelude::*, tester::TestError};
use rstest::rstest;

#[test]
fn record_with_redefined_key() -> Result {
    test()
        .run("{x: 1, x: 2}")
        .expect_error_code_eq("nu::shell::column_defined_twice")
}

#[test]
#[deps(NU)]
fn run_file_parse_error() -> Result {
    let result: CompleteResult = test()
        .cwd("tests/fixtures/eval")
        .run("nu script.nu | complete")?;

    assert_ne!(result.exit_code, 0);
    assert_contains("unknown type", result.stderr);
    Ok(())
}

#[rstest]
#[case::literal_bool("true", true)]
#[case::literal_int("1", 1)]
#[case::literal_float("1.5", 1.5)]
#[case::literal_filesize("30MB", Value::test_filesize(30_000_000))]
#[case::literal_duration("30ms", Value::test_duration(30_000_000))]
#[case::literal_closure_to_nuon("{||} | to nuon --serialize", "\"{||}\"")]
#[case::literal_closure_to_json("{||} | to json --serialize", "\"{||}\"")]
#[case::literal_closure_to_toml("{a: {||}} | to toml --serialize", "a = \"{||}\"\n")]
#[case::literal_closure_to_yaml("{||} | to yaml --serialize", "!closure \"{||}\"\n")]
#[case::literal_string(r#""foobar""#, "foobar")]
#[case::literal_raw_string("r#'bazquux'#", "bazquux")]
// https://github.com/nushell/nushell/issues/18807
#[case::interp_quote_inside_subexpr(r#"$"('" "')""#, "\" \"")]
#[case::interp_double_quote_inside_single_interp(r#"$'(", ")'"#, ", ")]
#[case::interp_nested_parens(r#"$"(1 + (2 * 3))""#, "7")]
#[case::interp_escaped_paren_is_literal(r#"$"\('a')""#, "('a')")]
#[case::interp_escaped_quote_in_nested_string(r#"$"("a\"b")""#, "a\"b")]
#[case::interp_escaped_quote_then_tail(r#"$"("a\"b")c""#, "a\"bc")]
#[case::interp_sequential_subexprs(r#"$"a(1)b(2)""#, "a1b2")]
#[case::interp_nested_interp(r#"$"($"in(2)ner")""#, "in2ner")]
#[case::literal_nothing("null", ())]
#[case::list_spread("[foo bar ...[baz quux]] | length", 4)]
#[case::record_spread("{foo: bar ...{baz: quux}} | columns | length", 2)]
#[case::binary_op_example(
    "(([1 2] ++ [3 4]) == [1 2 3 4]) and (([1] ++ [2 3 4]) == [1 2 3 4])",
    true
)]
#[case::range_from_expressions("(1 + 1)..(2 + 2) | each { |x| $x }", vec![2, 3, 4])]
#[case::list_from_expressions("[('foo' | str upcase) ('BAR' | str downcase)]", vec!["FOO", "bar"])]
#[case::record_from_expressions("{('foo' | str upcase): 42}", test_record! { "FOO" => 42 })]
#[case::call_flag("def flag-test [--flag] { $flag }; flag-test --flag", true)]
#[case::call_named("10.123 | into string --decimals 1", "10.1")]
#[case::let_variable("let foo = 'test'; $foo", "test")]
#[case::constant("const foo = 1 + 2; $foo", 3)]
#[case::mut_variable("mut foo = 'test'; $foo = 'bar'; $foo", "bar")]
#[case::mut_variable_append_assign("mut foo = 'test'; $foo ++= 'bar'; $foo", "testbar")]
#[case::bind_in_variable_to_input("3 | (4 + $in)", 7)]
#[case::if_true("if true { 'foo' }", "foo")]
#[case::if_false("if false { 'foo' }", ())]
#[case::if_else_true("if 5 > 3 { 'foo' } else { 'bar' }", "foo")]
#[case::if_else_false("if 5 < 3 { 'foo' } else { 'bar' }", "bar")]
#[case::match_empty_fallthrough("match 42 { }; 'pass'", "pass")]
#[case::match_value("match 1 { 1 => 'pass', 2 => 'fail' }", "pass")]
#[case::match_value_default("match 3 { 1 => 'fail1', 2 => 'fail2', _ => 'pass' }", "pass")]
#[case::match_value_fallthrough("match 3 { 1 => 'fail1', 2 => 'fail2' }", ())]
#[case::match_variable("match 'pass' { $s => { $s }, _ => { 'fail' } }", "pass")]
#[case::match_variable_in_list("match [fail pass] { [$f, $p] => { $p } }", "pass")]
#[case::match_passthrough_input(
    "'yes' | match [pass fail] { [$p, ..] => (collect { |y| $y ++ $p }) }",
    "yespass"
)]
#[case::while_mutate_var(
    "mut out = ''; mut x = 2; while $x > 0 { $out ++= ($x | into string); $x -= 1 }; $out",
    "21"
)]
#[case::for_list(
    "mut out = ''; for v in [1 2 3] { $out ++= (($v * 2) | into string) }; $out",
    "246"
)]
#[case::for_seq(
    "mut out = ''; for v in (seq 1 4) { $out ++= (($v * 2) | into string) }; $out",
    "2468"
)]
#[case::early_return("do { return 'foo'; 'bar' }", "foo")]
#[case::early_return_from_if("do { if true { return 'pass' }; 'fail' }", "pass")]
#[case::early_return_from_loop("do { loop { return 'pass' } }", "pass")]
#[case::early_return_from_while("do { let x = true; while $x { return 'pass' } }", "pass")]
#[case::early_return_from_for("do { for x in [pass fail] { return $x } }", "pass")]
#[case::try_no_catch("try { error make { msg: foo } }; 'pass'", "pass")]
#[case::try_catch_no_var("try { error make { msg: foo } } catch { 'pass' }", "pass")]
#[case::try_catch_var("try { error make { msg: foo } } catch { |err| $err.msg }", "foo")]
#[case::try_catch_with_non_literal_closure_no_var(
    r#"
        let error_handler = { || "pass" }
        try { error make { msg: foobar } } catch $error_handler
    "#,
    "pass"
)]
#[case::try_catch_with_non_literal_closure(
    "
        let error_handler = { |err| $err.msg }
        try { error make { msg: foobar } } catch $error_handler
    ",
    "foobar"
)]
#[case::row_condition("[[a b]; [1 2] [3 4]] | where a < 3", test_table![
    ["a", "b"];
    [1, 2],
])]
#[case::custom_command(
    r#"
        def cmd [a: int, b: string = 'fail', ...c: string, --x: int] { $"($a)($b)($c)($x)" }
        cmd 42 pass foo --x 30
    "#,
    "42pass[foo]30"
)]
fn eval_value_eq(#[case] source: &str, #[case] expected: impl IntoValue) -> Result {
    test().run(source).expect_value_eq(expected)
}

#[rstest]
#[case::literal_binary("0x[1f 2f f0] | table", "(?s)Length.*1f.*2f.*f0")]
#[case::literal_closure("{||} | to nuon --serialize", r#""\{\|\|\}""#)]
#[case::literal_range("0..2..10 | table", "10")]
#[case::literal_list("[foo bar baz] | table", "(?s)foo.*bar.*baz")]
#[case::literal_record("{foo: bar, baz: quux} | table", "(?s)foo.*bar.*baz.*quux")]
#[case::literal_table("[[a b]; [1 2] [3 4]] | table", "(?s)a.*b.*1.*2.*3.*4")]
#[case::literal_date("2020-01-01T00:00:00Z | format date '%Y'", "2020")]
#[case::call_spread(
    "echo foo bar ...[baz quux nushell] | table",
    "(?s)foo.*bar.*baz.*quux.*nushell"
)]
fn eval_rendered_matches(#[case] source: &str, #[case] regex: &str) -> Result {
    let actual: String = test().run(source)?;
    let compiled_regex = Regex::new(regex).expect("regex failed to compile");
    assert!(
        compiled_regex.is_match(&actual).unwrap_or(false),
        "eval out does not match: {regex}\n{actual}",
    );
    Ok(())
}

#[test]
fn binary_op_rhs_collects_in_variable() -> Result {
    // Regression test for #18323: a binary op whose RHS collects `$in` (e.g. through `not`,
    // a list literal, or a subexpression) used to clobber the LHS register and emit a
    // `register_uninitialized` compiler error.
    //
    // `$in` (not `$it`) is deliberate: it is the form that triggers the bug. The `$it`
    // equivalent compiles fine on `main`, so it would not guard this regression.
    test()
        .run("[[v]; [1] [2] [6]] | where $in.v > 0 and not ($in.v > 5) | get v")
        .expect_value_eq([1, 2])?;
    test()
        .run("[[v]; [1] [2] [6]] | where ($in.v == 1) or (0..($in.v) | is-empty) | get v")
        .expect_value_eq([1])
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn external_call() -> Result {
    test()
        .run("cococo foo=bar baz")
        .expect_value_eq("foo=bar baz")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn external_call_redirect_pipe() -> Result {
    test()
        .run("cococo foo=bar baz | str upcase")
        .expect_value_eq("FOO=BAR BAZ")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn external_call_redirect_capture() -> Result {
    test()
        .run("echo (cococo foo=bar baz) | str upcase")
        .expect_value_eq("FOO=BAR BAZ")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn external_call_redirect_file() -> Result {
    Playground::setup("external_call_redirect_file", |dirs, _sandbox| {
        let () = test().cwd(dirs.test()).run("cococo hello out> hello.txt")?;
        let read_contents = std::fs::read_to_string(dirs.test().join("hello.txt"))?;
        assert_eq!(read_contents.trim(), "hello");
        Ok(())
    })
}

#[test]
fn let_variable_mutate_error() -> Result {
    test()
        .run("let foo = 'test'; $foo = 'bar'; $foo")
        .expect_error_code_eq("nu::parser::assignment_requires_mutable_variable")
}

#[test]
fn constant_assign_error() -> Result {
    test()
        .run("const foo = 1 + 2; $foo = 4; $foo")
        .expect_error_code_eq("nu::parser::assignment_requires_mutable_variable")
}

#[test]
#[deps(NU)]
fn try_catch_external() -> Result {
    test()
        .run("try { nu -c 'exit 1' } catch { $env.LAST_EXIT_CODE }")
        .expect_value_eq(1)
}

#[test]
fn early_return_keeps_metadata() -> Result {
    // An early `return` used to drop pipeline metadata that a value in tail position kept.
    // https://github.com/nushell/nushell/issues/18552
    let code = r#"
        def foo [] { if true { return ("body" | metadata set { merge {my: 302} }) } }
        foo | metadata | get my
    "#;

    test().run(code).expect_value_eq(302)
}

#[test]
fn early_return_keeps_stream() -> Result {
    // An early `return` used to collect its value; it should stay a stream like a value in
    // tail position does. Assert on the pipeline structure rather than `describe` output, so a
    // regression that collects the stream into a list is caught directly.
    let output = test().run_raw("def foo [] { return (1..3 | each { |x| $x }) }; foo")?;
    let PipelineData::ListStream(stream, _) = output.body else {
        panic!("early return should stay a stream")
    };
    stream
        .into_value()
        .map_err(TestError::from)
        .expect_value_eq(vec![1i64, 2, 3])
}

#[test]
fn early_return_with_finally_runs_cleanup_and_keeps_value() -> Result {
    // In-process `print` output isn't captured, so the `finally` block reports through the root
    // job's mailbox (`job send 0`) instead. The recovered message and the returned value confirm
    // the cleanup ran and the early-return value survived it. The pipeline is single-threaded, so
    // by the time `job recv` runs the message is already queued and no timeout is needed.
    let code = r#"
        def foo [] { try { return 1 } finally { "cleanup" | job send 0 } }
        let val = foo;
        {
            finally: (job recv --timeout 0sec),
            returned: $val,
        }
    "#;

    test().run(code).expect_value_eq(test_value!({
        finally: "cleanup",
        returned: 1,
    }))
}

#[test]
fn early_return_with_finally_keeps_metadata() -> Result {
    let code = r#"
        def foo [] { try { return ("body" | metadata set { merge {my: 302} }) } finally { } }
        foo | metadata | get my
    "#;

    test().run(code).expect_value_eq(302)
}

#[test]
fn early_return_not_intercepted_by_catch() -> Result {
    test()
        .run("def foo [] { try { return early } catch { 'caught' } }; foo")
        .expect_value_eq("early")
}

#[test]
fn early_return_in_export_env_stays_in_env_block() -> Result {
    // `return` inside `export-env` ends the environment block; it used to unwind further and
    // abort the enclosing command.
    test()
        .run("def foo [] { export-env { return }; 'after' }; foo")
        .expect_value_eq("after")
}

#[test]
fn early_return_in_export_env_guard_skips_rest_of_env_block() -> Result {
    let code = "
        def foo [] { export-env { if true { return }; $env.FOO = 'set' }; $env.FOO? }
        foo
    ";

    test().run(code).expect_value_eq(())
}

#[test]
#[deps(NU)]
fn early_return_inside_command_does_not_skip_main() -> Result {
    // A `return` inside a command called at the top level of a script is consumed where the
    // command is called; only a top-level `return` should prevent `main` from running. This runs
    // the `nu` binary because that "skip main" decision lives in file evaluation, not in the
    // in-process engine.
    Playground::setup(
        "early_return_inside_command_does_not_skip_main",
        |dirs, sandbox| -> Result {
            sandbox.with_files(&[FileWithContent(
                "script.nu",
                "def helper [] { return 1 }\nhelper\ndef main [] { print 'main ran' }",
            )]);

            let result: CompleteResult =
                test().cwd(dirs.test()).run("nu -n script.nu | complete")?;
            assert_eq!(result.exit_code, 0);
            assert_contains("main ran", result.stdout);
            Ok(())
        },
    )
}

#[test]
fn early_return_in_module_export_env_does_not_abort_caller() -> Result {
    Playground::setup(
        "early_return_in_module_export_env_does_not_abort_caller",
        |dirs, sandbox| -> Result {
            sandbox.with_files(&[FileWithContent(
                "mod.nu",
                "export-env { return }\nexport def hi [] { 'hi' }",
            )]);
            test()
                .cwd(dirs.test())
                .run("def foo [] { use mod.nu *; hi }; foo")
                .expect_value_eq("hi")
        },
    )
}

// Regression tests for issue #18951

#[rstest]
#[case::mutable_record_existing_field(
    "
        mut r = {a: 1}
        $r.a = 2
        $r.a
    ",
    2
)]
#[case::mutable_record_new_field(
    "
        mut r = {a: 1}
        $r.b = 1
        $r.b = 2
        $r.b
    ",
    2
)]
#[case::mutable_nested_record_existing_field(
    "
        mut r = {a: {b: 1}}
        $r.a.b = 2
        $r.a.b
    ",
    2
)]
#[case::mutable_nested_record_new_field(
    "
        mut r = {a: {b: 1}}
        $r.a.c = 1
        $r.a.c = 2
        $r.a.c
    ",
    2
)]
#[case::mutable_table_existing_column(
    "
        mut t = [[a]; [1]]
        $t.0.a = 2
        $t.0.a
    ",
    2
)]
#[case::mutable_table_new_column(
    "
        mut t = [[a]; [1]]
        $t.0.b = 1
        $t.0.b = 2
        $t.0.b
    ",
    2
)]
#[case::mutable_heterogeneous_list_cell_path_assignment(
    r#"
        mut l = [1 "a"]
        $l.0 = "hello"
        $l.0
    "#,
    "hello"
)]
#[case::mutable_list_of_records_existing_field(
    "
        mut l = [{a: 1}]
        $l.0.a = 2
        $l.0.a
    ",
    2
)]
#[case::mutable_list_of_records_new_field(
    "
        mut l = [{a: 1}]
        $l.0.b = 1
        $l.0.b = 2
        $l.0.b
    ",
    2
)]
fn mutable_aggregate_cell_path_assignment(
    #[case] code: &str,
    #[case] expected: impl IntoValue,
) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::mutable_record_existing_field(
    r#"
        mut r = {a: 1}
        $r.a = "hello"
    "#
)]
#[case::mutable_record_new_field(
    r#"
        mut r = {a: 1}
        $r.b = 1
        $r.b = "hello"
    "#
)]
#[case::mutable_list_existing_element(
    r#"
        mut l = [1 2 3]
        $l.0 = "hello"
    "#
)]
#[case::mutable_list_out_of_bounds_element(
    r#"
        mut l = [1 2 3]
        $l.3 = "hello"
    "#
)]
#[case::mutable_nested_record_existing_field(
    r#"
        mut r = {a: {b: 1}}
        $r.a.b = "hello"
    "#
)]
#[case::mutable_nested_record_new_field(
    r#"
        mut r = {a: {b: 1}}
        $r.a.c = 1
        $r.a.c = "hello"
    "#
)]
#[case::mutable_table_existing_column(
    r#"
        mut t = [[a]; [1]]
        $t.0.a = "hello"
    "#
)]
#[case::mutable_table_new_column(
    r#"
        mut t = [[a]; [1]]
        $t.0.b = 1
        $t.0.b = "hello"
    "#
)]
#[case::mutable_list_of_records_existing_field(
    r#"
        mut l = [{a: 1}]
        $l.0.a = "hello"
    "#
)]
#[case::mutable_list_of_records_new_field(
    r#"
        mut l = [{a: 1}]
        $l.0.b = 1
        $l.0.b = "hello"
    "#
)]
fn mutable_aggregate_cell_path_type_mismatch(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::shell::type_mismatch")
}
