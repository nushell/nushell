use nu_protocol::Type;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[rstest]
#[case("let in = 3")]
#[case("let in: int = 3")]
fn let_name_builtin_var(#[case] assignment: &str) -> Result {
    let err = test().run(assignment).expect_parse_error()?;
    assert_contains("`in` used as variable name", err.to_string());
    Ok(())
}

#[test]
fn let_doesnt_mutate() -> Result {
    let err = test().run("let i = 3; $i = 4").expect_parse_error()?;
    assert_contains("immutable", err.to_string());
    Ok(())
}

#[test]
fn let_takes_pipeline() -> Result {
    test()
        .run(r#"let x = "hello world" | str length; $x"#)
        .expect_value_eq(11)
}

#[test]
fn let_takes_pipeline_with_declared_type() -> Result {
    test()
        .run(r#"let x: list<string> = [] | append "hello world"; $x.0"#)
        .expect_value_eq("hello world")
}

#[test]
fn let_pipeline_allows_in() -> Result {
    test()
        .run(r#"def foo [] { let x = $in | str length; $x + 10 }; "hello world" | foo"#)
        .expect_value_eq(21)
}

#[test]
fn mut_takes_pipeline() -> Result {
    test()
        .run(r#"mut x = "hello world" | str length; $x"#)
        .expect_value_eq(11)
}

#[test]
fn mut_takes_pipeline_with_declared_type() -> Result {
    test()
        .run(r#"mut x: list<string> = [] | append "hello world"; $x.0"#)
        .expect_value_eq("hello world")
}

#[test]
fn mut_pipeline_allows_in() -> Result {
    test()
        .run(r#"def foo [] { mut x = $in | str length; $x + 10 }; "hello world" | foo"#)
        .expect_value_eq(21)
}

#[test]
fn let_pipeline_redirects_internals() -> Result {
    test()
        .run("let x = echo 'bar'; $x | str length")
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn let_pipeline_redirects_externals() -> Result {
    test()
        .run("let x = cococo 'bar'; $x | str length")
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn let_err_pipeline_redirects_externals() -> Result {
    test()
        .run(r#"let x = with-env { FOO: "foo" } { echo_env_stderr FOO e>| str length }; $x"#)
        .expect_value_eq(3)
}

#[test]
#[deps(TESTBIN_ECHO_ENV_STDERR)]
fn let_outerr_pipeline_redirects_externals() -> Result {
    test()
        .run(r#"let x = with-env { FOO: "foo" } { echo_env_stderr FOO o+e>| str length }; $x"#)
        .expect_value_eq(3)
}

#[ignore]
#[test]
#[deps(TESTBIN_FAIL)]
fn let_with_external_failed() -> Result {
    // FIXME: this test hasn't run successfully for a long time. We should
    // bring it back to life at some point.
    test()
        .run("let x = fail; echo fail")
        .expect_error_code_eq("nu::shell::non_zero_exit_code")
}

#[test]
fn let_glob_type() -> Result {
    test()
        .run("let x: glob = 'aa'; $x | describe")
        .expect_value_eq("glob")
}

#[test]
fn let_typed_glob_expands_in_ls(playground: Playground) -> Result {
    playground.empty_file("a.toml")?;
    playground.empty_file("b.toml")?;
    playground.empty_file("c.txt")?;

    test()
        .cwd(playground.path())
        .run(r#"let x: glob = "*.toml"; ls $x | length"#)
        .expect_value_eq(2)
}

#[test]
fn let_raw_string() -> Result {
    test()
        .run(r#"let x = r#'abcde""fghi"''''jkl'#; $x"#)
        .expect_value_eq(r#"abcde""fghi"''''jkl"#)?;
    test()
        .run(r#"let x = r##'abcde""fghi"''''#jkl'##; $x"#)
        .expect_value_eq(r#"abcde""fghi"''''#jkl"#)?;
    test()
        .run(r#"let x = r###'abcde""fghi"'''##'#jkl'###; $x"#)
        .expect_value_eq(r#"abcde""fghi"'''##'#jkl"#)?;
    test().run("let x = r#'abc'#; $x").expect_value_eq("abc")
}

#[test]
fn let_malformed_type() -> Result {
    // Unexpected `)` — unbalanced delimiter.
    let err = test().run("let foo: )a").expect_parse_error()?;
    let msg = err.to_string();
    assert!(
        msg.contains("unbalanced with `(`") || msg.contains("Unbalanced delimiter"),
        "unexpected err: {msg}"
    );

    let err = test().run("let foo: }a").expect_parse_error()?;
    let msg = err.to_string();
    assert!(
        msg.contains("unbalanced with `{`") || msg.contains("Unbalanced delimiter"),
        "unexpected err: {msg}"
    );

    let err = test().run("mut : , a").expect_parse_error()?;
    assert_contains("Unknown type", err.to_string());
    Ok(())
}

// ============================================================
// Tests for the three pipeline positions of `let`
// ============================================================

// --- LHS (beginning): `let x = expr` produces NO output ---

#[test]
fn let_lhs_produces_no_output() -> Result {
    test().run("let x = 5").expect_value_eq(())
}

#[test]
fn let_lhs_expression_produces_no_output() -> Result {
    test().run("let x = 10 + 100").expect_value_eq(())
}

#[test]
fn let_lhs_stores_value_correctly() -> Result {
    test().run("let x = 10; $x").expect_value_eq(10)
}

#[test]
fn let_lhs_pipeline_expr_produces_no_output() -> Result {
    test()
        .run(r#"let x = "hello world" | str length"#)
        .expect_value_eq(())
}

#[test]
fn let_lhs_invalid_pipeline() -> Result {
    let _ = test().run("let x = 5 | echo done").expect_parse_error()?;
    Ok(())
}

// --- Middle: `input | let var | next` passes value through ---

#[test]
fn let_mid_passes_value_through() -> Result {
    test().run("10 | let x | $x + 5").expect_value_eq(15)
}

#[test]
fn let_mid_allows_in_variable() -> Result {
    test().run("10 | let x | $in + 5").expect_value_eq(15)
}

#[test]
fn let_mid_passes_list_through() -> Result {
    test().run("[2 3 4] | let nums | first").expect_value_eq(2)
}

#[test]
fn let_mid_passes_string_through() -> Result {
    test()
        .run(r#""hello" | let msg | str length"#)
        .expect_value_eq(5)
}

#[test]
fn let_mid_pipeline_equivalence() -> Result {
    let actual1: i64 = test().run("[2 3 4] | let nums | first")?;
    let actual2: i64 = test().run("[2 3 4] | let nums; $nums | first")?;
    assert_eq!(actual1, actual2);
    Ok(())
}

// --- End: `input | let var` outputs the assigned value ---

#[test]
fn let_end_outputs_value() -> Result {
    test().run("5 | let x").expect_value_eq(5)
}

#[test]
fn let_end_outputs_computed_value() -> Result {
    test()
        .run(r#""hello" | str length | let n"#)
        .expect_value_eq(5)
}

#[test]
fn let_end_stores_value_correctly() -> Result {
    test().run("5 | let x; $x + 10").expect_value_eq(15)
}

#[test]
#[ignore = "TODO: Need to detect at parse time that 'let x | echo done' is invalid"]
fn let_var_at_beginning_error() -> Result {
    let err = test().run("let x | echo done").expect_error()?;
    let msg = err.to_string();
    assert!(msg.contains("doesn't support") || msg.contains("invalid `let` keyword call"));
    Ok(())
}

// --- Destructuring: `let {$a} = ...` and `let [$a, $b] = ...` ---

#[rstest]
#[case::record("let {$a, b: $c} = {a: 1, b: 2, d: 3}; [$a $c]", test_value!([1, 2]))]
#[case::list("let [$x, $y] = [1 2]; [$x $y]", test_value!([1, 2]))]
#[case::rest("let [$head, ..$tail] = [1 2 3]; [$head $tail]", test_value!([1, [2, 3]]))]
#[case::rest_first("let [..$init, $last] = [1 2 3]; [$init $last]", test_value!([[1, 2], 3]))]
#[case::ignore("let [_, $b, ..] = [1 2 3]; $b", test_value!(2))]
#[case::nested("let [[$a, $b], {c: {$d}}] = [[1 2] {c: {d: 3}}]; [$a $b $d]", test_value!([1, 2, 3]))]
#[case::quoted_key(r#"let {"a b": $x} = {"a b": 1}; $x"#, test_value!(1))]
#[case::table_row("let [{$name}, ..] = [[name]; [x] [y]]; $name", test_value!("x"))]
#[case::empty_list("let [] = []; 1", test_value!(1))]
#[case::stream("let [$first, ..] = (1..3 | each { $in * 2 }); $first", test_value!(2))]
fn let_destructures(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn let_destructure_passes_input_through() -> Result {
    test()
        .run("{a: 1, b: 2} | let {$a} | [$a $in.b]")
        .expect_value_eq(test_value!([1, 2]))
}

#[test]
fn let_destructure_input_type_mismatch() -> Result {
    let err = test().run("5 | let {$a}").expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::Record(_), Type::Int, _));
    Ok(())
}

#[rstest]
#[case::def("def f [] { let {$a} = {a: 5}; $a }; f")]
#[case::closure_capture("let {$a} = {a: 5}; do { $a }")]
#[case::nested_closure("let c = {|| let [$a] = [5]; {|| $a } }; do (do $c)")]
#[case::closure_called_twice("let f = {|| let [$a] = [5]; $a }; do $f; do $f")]
#[case::loop_body("mut s = 0; for x in [2 3] { let [$a] = [$x]; $s += $a }; $s")]
fn let_destructure_scopes(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(5)
}

#[rstest]
#[case::let_("let x = 1; let {$x} = {x: 2}; $x")]
#[case::const_("const x = 1; let {$x} = {x: 2}; $x")]
fn let_destructure_shadows(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq(2)
}

#[rstest]
#[case::missing_field(
    "let {$name, $mtime} = {name: a, size: 1}",
    "record has no field `mtime`",
    Some("The record has name, size. Add `?` after the pattern if the field can be missing.")
)]
#[case::empty_record(
    "let {$a} = {}",
    "record has no field `a`",
    Some("The record is empty. Add `?` after the pattern if the field can be missing.")
)]
#[case::too_many_items(
    "let [$a, $b] = [1 2 3]",
    "list has 3 items, the pattern takes 2",
    Some("Add `..` to ignore the rest.")
)]
#[case::too_few_items("let [$a, $b] = [1]", "list has 1 item, the pattern takes 2", None)]
#[case::too_few_with_rest(
    "let [$a, $b, ..] = [1]",
    "list has 1 item, the pattern needs at least 2",
    None
)]
#[case::wrong_type(
    "def f [] { [1] }; let {$a} = (f)",
    "expected a record, got list",
    None
)]
#[case::nested_wrong_type("let {a: [$b]} = {a: 1}", "expected a list, got int", None)]
fn let_destructure_mismatch(
    #[case] code: &str,
    #[case] expected_msg: &str,
    #[case] expected_help: Option<&str>,
) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::PatternMismatch { msg, help, .. }
            if msg == expected_msg && help.as_deref() == expected_help
    );
    Ok(())
}

#[test]
fn let_destructure_value_type_mismatch() -> Result {
    let err = test().run("let {$a} = 5").expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::Record(_), Type::Int, _));
    Ok(())
}

#[rstest]
#[case::bare_word(
    "let [a, $b] = [1 2]",
    "A bare word in a pattern is a string, not a variable"
)]
#[case::literal("let [$a, 1] = [1 2]", "A destructuring pattern can't hold a literal")]
#[case::range(
    "let [$a, 1..2] = [1 2]",
    "A destructuring pattern can't hold a literal"
)]
#[case::bound_twice("let [$a, {$a}] = [1 {a: 2}]", "`$a` is bound twice in this pattern")]
#[case::bound_twice_rest("let [$a, ..$a] = [1 2]", "`$a` is bound twice in this pattern")]
fn let_destructure_invalid_pattern(#[case] code: &str, #[case] expected: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledErrorWithHelp { error, .. } if error == expected);
    Ok(())
}

#[rstest]
#[case::missing_field("let {$a, $b?} = {a: 1}; [$a $b]", test_value!([1, ()]))]
#[case::null_field("let {$a?} = {a: null}; $a", test_value!(()))]
#[case::present_field("let {a: $x?} = {a: 1}; $x", test_value!(1))]
#[case::nested_missing("let {foo: {$bar, $baz}?} = {}; [$bar $baz]", test_value!([(), ()]))]
#[case::missing_item("let [$x, $y?] = [1]; [$x $y]", test_value!([1, ()]))]
#[case::present_item("let [$x, $y?] = [1 2]; [$x $y]", test_value!([1, 2]))]
#[case::wildcard("let {a: _?, $b} = {b: 2}; $b", test_value!(2))]
#[case::record_rest(
    "let {$a, ..$rest} = {a: 1, b: 2, c: 3}; $rest",
    test_value!({b: 2, c: 3})
)]
#[case::record_rest_empty("let {$a, ..$rest} = {a: 1}; $rest", test_value!({}))]
#[case::record_rest_only("let {..$all} = {a: 1}; $all", test_value!({a: 1}))]
fn let_destructure_optional_and_rest(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case::optional_wrong_type("let {foo: {$bar}?} = {foo: 5}", "expected a record, got int", None)]
#[case::optional_too_long(
    "let [$x, $y?] = [1 2 3]",
    "list has 3 items, the pattern takes 1 to 2",
    None
)]
fn let_destructure_optional_mismatch(
    #[case] code: &str,
    #[case] expected_msg: &str,
    #[case] expected_help: Option<&str>,
) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::PatternMismatch { msg, help, .. }
            if msg == expected_msg && help.as_deref() == expected_help
    );
    Ok(())
}

#[rstest]
#[case::required_after_optional(
    "let [$a?, $b] = [1]",
    "A required item can't follow an optional one"
)]
#[case::optional_with_rest(
    "let [$a, $b?, ..] = [1]",
    "A list pattern can't have both `..` and optional items"
)]
#[case::record_ignore_rest(
    "let {$a, ..} = {a: 1}",
    "A record pattern already ignores the fields it doesn't name"
)]
#[case::record_rest_twice(
    "let {..$a, ..$b} = {a: 1}",
    "`..$name` can only be used once per record pattern"
)]
fn let_destructure_invalid_optional_or_rest(#[case] code: &str, #[case] expected: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledErrorWithHelp { error, .. } if error == expected);
    Ok(())
}

#[rstest]
#[case::missing_field("let {$host, $port = 8080} = {host: h}; [$host $port]", test_value!(["h", 8080]))]
#[case::null_field("let {$port = 8080} = {port: null}; $port", test_value!(8080))]
#[case::present_field("let {$port = 8080} = {port: 1}; $port", test_value!(1))]
#[case::renamed("let {port: $p = 8080} = {}; $p", test_value!(8080))]
#[case::missing_items("let [$a, $b = 0, $c = 5] = [1 2]; [$a $b $c]", test_value!([1, 2, 5]))]
#[case::nested("let {cfg: {$x, $y} = {x: 1, y: 2}} = {}; [$x $y]", test_value!([1, 2]))]
#[case::const_default("const n = 3; let {$a = $n} = {}; $a", test_value!(3))]
#[case::closure_parameter("do {|{$x = 5}| $x } {}", test_value!(5))]
fn let_destructure_default(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn let_destructure_default_must_fit() -> Result {
    let err = test()
        .run("let {cfg: {$x} = {y: 2}} = {}")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::PatternMismatch { msg, .. } if msg == "record has no field `x`");
    Ok(())
}

#[test]
fn let_destructure_default_must_be_const() -> Result {
    let err = test()
        .run("let {$a = $env.HOME} = {}")
        .expect_parse_error()?;
    assert_contains("not a parse-time constant", err.to_string());
    Ok(())
}

#[test]
fn let_destructure_default_and_optional() -> Result {
    let err = test().run("let {$a? = 1} = {}").expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::LabeledErrorWithHelp { error, .. }
            if error == "A pattern can have `?` or a default, not both"
    );
    Ok(())
}

#[test]
fn let_destructure_across_source_units() -> Result {
    let mut tester = test();
    let () = tester.run("let {$a, ..$rest} = {a: 1, b: 2}")?;
    tester
        .run("[$a $rest.b]")
        .expect_value_eq(test_value!([1, 2]))
}

#[test]
fn let_destructure_in_sourced_file(playground: Playground) -> Result {
    playground.file(
        "vars.nu",
        "let {$a, ..$rest} = {a: 1, b: 2}\nconst [$k] = [7]\n",
    )?;
    test()
        .cwd(playground.path())
        .run("source vars.nu; [1] | each {|x| [$x $a $rest.b $k] } | first")
        .expect_value_eq(test_value!([1, 1, 2, 7]))
}

#[rstest]
#[case::let_value("let {$a} = (error make {msg: boom})")]
#[case::for_item("for {$a} in ([1] | each { error make {msg: boom} }) { }")]
fn let_destructure_raises_error_values(#[case] code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_contains("boom", err.to_string());
    Ok(())
}

#[rstest]
#[case::record("let {$a, $b}: record<a: int, b: string> = {a: 1, b: x}; [$a $b]", test_value!([1, "x"]))]
#[case::extra_fields("let {$a}: record<a: int> = {a: 1, c: 2}; $a", test_value!(1))]
#[case::list("let [$x, ..$rest]: list<int> = [1 2 3]; $rest", test_value!([2, 3]))]
#[case::pass_through("{a: 1} | let {$a}: record<a: int> | $a", test_value!(1))]
#[case::mut_("mut {$a}: record<a: int> = {a: 1}; $a += 1; $a", test_value!(2))]
#[case::const_("const {$a}: record<a: int> = {a: 1}; $a", test_value!(1))]
fn let_destructure_annotated(#[case] code: &str, #[case] expected: Value) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn let_destructure_annotation_types_variables() -> Result {
    let err = test()
        .run("let {$a}: record<a: int> = {a: 1}; $a | str length")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::InputMismatch { .. });
    Ok(())
}

#[rstest]
#[case::value("let {$a}: record<a: int> = 5")]
#[case::field("let r = {a: s}; let {$a}: record<a: int> = $r")]
#[case::shape("let {$a}: int = {a: 1}")]
#[case::pass_through("5 | let {$a}: record<a: int>")]
#[case::const_("const {$a}: record<a: int> = {a: s}")]
fn let_destructure_annotation_mismatch(#[case] code: &str) -> Result {
    test()
        .run(code)
        .expect_error_code_eq("nu::parser::type_mismatch")
}

#[rstest]
#[case::let_("let {$p}: record<p: glob> = {p: '*.md'}; $p | describe")]
#[case::const_("const {$p}: record<p: glob> = {p: '*.md'}; $p | describe")]
fn let_destructure_annotated_glob(#[case] code: &str) -> Result {
    test().run(code).expect_value_eq("glob")
}

#[test]
#[exp(nu_experimental::ENFORCE_RUNTIME_ANNOTATIONS)]
fn let_destructure_annotation_checked_at_run_time() -> Result {
    test()
        .run("def f [] { {a: s} }; let {$a}: record<a: int> = (f)")
        .expect_error_code_eq("nu::shell::type_mismatch")
}
