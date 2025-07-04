use fancy_regex::Regex;
use nu_test_support::{nu, playground::Playground};

#[test]
fn record_with_redefined_key() {
    let actual = nu!("{x: 1, x: 2}");

    assert!(actual.err.contains("redefined"));
}

#[test]
fn run_file_parse_error() {
    let actual = nu!(
        cwd: "tests/fixtures/eval",
        "nu script.nu"
    );

    assert!(actual.err.contains("unknown type"));
}

enum ExpectedOut<'a> {
    /// Equals a string exactly
    Eq(&'a str),
    /// Matches a regex
    Matches(&'a str),
    /// Produces an error (match regex)
    Error(&'a str),
    /// Drops a file that contains these contents
    FileEq(&'a str, &'a str),
}
use self::ExpectedOut::*;

fn test_eval(source: &str, expected_out: ExpectedOut) {
    Playground::setup("test_eval", |dirs, _playground| {
        let actual = nu!(
            cwd: dirs.test(),
            source,
        );

        match expected_out {
            Eq(eq) => {
                assert_eq!(actual.out, eq);
                assert!(actual.status.success());
            }
            Matches(regex) => {
                let compiled_regex = Regex::new(regex).expect("regex failed to compile");
                assert!(
                    compiled_regex.is_match(&actual.out).unwrap_or(false),
                    "eval out does not match: {}\n{}",
                    regex,
                    actual.out,
                );
                assert!(actual.status.success());
            }
            Error(regex) => {
                let compiled_regex = Regex::new(regex).expect("regex failed to compile");
                assert!(
                    compiled_regex.is_match(&actual.err).unwrap_or(false),
                    "eval err does not match: {regex}"
                );
                assert!(!actual.status.success());
            }
            FileEq(path, contents) => {
                let read_contents =
                    std::fs::read_to_string(dirs.test().join(path)).expect("failed to read file");
                assert_eq!(read_contents.trim(), contents);
                assert!(actual.status.success());
            }
        }
    });
}

#[test]
fn literal_bool() {
    test_eval("true", Eq("true"))
}

#[test]
fn literal_int() {
    test_eval("1", Eq("1"))
}

#[test]
fn literal_float() {
    test_eval("1.5", Eq("1.5"))
}

#[test]
fn literal_filesize() {
    test_eval("30MB", Eq("30.0 MB"))
}

#[test]
fn literal_duration() {
    test_eval("30ms", Eq("30ms"))
}

#[test]
fn literal_binary() {
    test_eval("0x[1f 2f f0]", Matches("Length.*1f.*2f.*f0"))
}

#[test]
fn literal_closure() {
    test_eval("{||}", Matches("closure_"))
}

#[test]
fn literal_closure_to_nuon() {
    test_eval("{||} | to nuon --serialize", Eq("\"{||}\""))
}

#[test]
fn literal_closure_to_json() {
    test_eval("{||} | to json --serialize", Eq("\"{||}\""))
}

#[test]
fn literal_closure_to_toml() {
    test_eval("{a: {||}} | to toml --serialize", Eq("a = \"{||}\""))
}

#[test]
fn literal_closure_to_yaml() {
    test_eval("{||} | to yaml --serialize", Eq("'{||}'"))
}

#[test]
fn literal_range() {
    test_eval("0..2..10", Matches("10"))
}

#[test]
fn literal_list() {
    test_eval("[foo bar baz]", Matches("foo.*bar.*baz"))
}

#[test]
fn literal_record() {
    test_eval("{foo: bar, baz: quux}", Matches("foo.*bar.*baz.*quux"))
}

#[test]
fn literal_table() {
    test_eval("[[a b]; [1 2] [3 4]]", Matches("a.*b.*1.*2.*3.*4"))
}

#[test]
fn literal_string() {
    test_eval(r#""foobar""#, Eq("foobar"))
}

#[test]
fn literal_raw_string() {
    test_eval(r#"r#'bazquux'#"#, Eq("bazquux"))
}

#[test]
fn literal_date() {
    test_eval("2020-01-01T00:00:00Z", Matches("2020"))
}

#[test]
fn literal_nothing() {
    test_eval("null", Eq(""))
}

#[test]
fn list_spread() {
    test_eval("[foo bar ...[baz quux]] | length", Eq("4"))
}

#[test]
fn record_spread() {
    test_eval("{foo: bar ...{baz: quux}} | columns | length", Eq("2"))
}

#[test]
fn binary_op_example() {
    test_eval(
        "(([1 2] ++ [3 4]) == [1 2 3 4]) and (([1] ++ [2 3 4]) == [1 2 3 4])",
        Eq("true"),
    )
}

#[test]
fn range_from_expressions() {
    test_eval("(1 + 1)..(2 + 2)", Matches("2.*3.*4"))
}

#[test]
fn list_from_expressions() {
    test_eval(
        "[('foo' | str upcase) ('BAR' | str downcase)]",
        Matches("FOO.*bar"),
    )
}

#[test]
fn record_from_expressions() {
    test_eval("{('foo' | str upcase): 42}", Matches("FOO.*42"))
}

#[test]
fn call_spread() {
    test_eval(
        "echo foo bar ...[baz quux nushell]",
        Matches("foo.*bar.*baz.*quux.*nushell"),
    )
}

#[test]
fn call_flag() {
    test_eval("print -e message", Eq("")) // should not be visible on stdout
}

#[test]
fn call_named() {
    test_eval("10.123 | into string --decimals 1", Eq("10.1"))
}

#[test]
fn external_call() {
    test_eval("nu --testbin cococo foo=bar baz", Eq("foo=bar baz"))
}

#[test]
fn external_call_redirect_pipe() {
    test_eval(
        "nu --testbin cococo foo=bar baz | str upcase",
        Eq("FOO=BAR BAZ"),
    )
}

#[test]
fn external_call_redirect_capture() {
    test_eval(
        "echo (nu --testbin cococo foo=bar baz) | str upcase",
        Eq("FOO=BAR BAZ"),
    )
}

#[test]
fn external_call_redirect_file() {
    test_eval(
        "nu --testbin cococo hello out> hello.txt",
        FileEq("hello.txt", "hello"),
    )
}

#[test]
fn let_variable() {
    test_eval("let foo = 'test'; print $foo", Eq("test"))
}

#[test]
fn let_variable_mutate_error() {
    test_eval(
        "let foo = 'test'; $foo = 'bar'; print $foo",
        Error("immutable"),
    )
}

#[test]
fn constant() {
    test_eval("const foo = 1 + 2; print $foo", Eq("3"))
}

#[test]
fn constant_assign_error() {
    test_eval(
        "const foo = 1 + 2; $foo = 4; print $foo",
        Error("immutable"),
    )
}

#[test]
fn mut_variable() {
    test_eval("mut foo = 'test'; $foo = 'bar'; print $foo", Eq("bar"))
}

#[test]
fn mut_variable_append_assign() {
    test_eval(
        "mut foo = 'test'; $foo ++= 'bar'; print $foo",
        Eq("testbar"),
    )
}

#[test]
fn bind_in_variable_to_input() {
    test_eval("3 | (4 + $in)", Eq("7"))
}

#[test]
fn if_true() {
    test_eval("if true { 'foo' }", Eq("foo"))
}

#[test]
fn if_false() {
    test_eval("if false { 'foo' } | describe", Eq("nothing"))
}

#[test]
fn if_else_true() {
    test_eval("if 5 > 3 { 'foo' } else { 'bar' }", Eq("foo"))
}

#[test]
fn if_else_false() {
    test_eval("if 5 < 3 { 'foo' } else { 'bar' }", Eq("bar"))
}

#[test]
fn match_empty_fallthrough() {
    test_eval("match 42 { }; 'pass'", Eq("pass"))
}

#[test]
fn match_value() {
    test_eval("match 1 { 1 => 'pass', 2 => 'fail' }", Eq("pass"))
}

#[test]
fn match_value_default() {
    test_eval(
        "match 3 { 1 => 'fail1', 2 => 'fail2', _ => 'pass' }",
        Eq("pass"),
    )
}

#[test]
fn match_value_fallthrough() {
    test_eval("match 3 { 1 => 'fail1', 2 => 'fail2' }", Eq(""))
}

#[test]
fn match_variable() {
    test_eval(
        "match 'pass' { $s => { print $s }, _ => { print 'fail' } }",
        Eq("pass"),
    )
}

#[test]
fn match_variable_in_list() {
    test_eval("match [fail pass] { [$f, $p] => { print $p } }", Eq("pass"))
}

#[test]
fn match_passthrough_input() {
    test_eval(
        "'yes' | match [pass fail] { [$p, ..] => (collect { |y| $y ++ $p }) }",
        Eq("yespass"),
    )
}

#[test]
fn while_mutate_var() {
    test_eval("mut x = 2; while $x > 0 { print $x; $x -= 1 }", Eq("21"))
}

#[test]
fn for_list() {
    test_eval("for v in [1 2 3] { print ($v * 2) }", Eq(r"246"))
}

#[test]
fn for_seq() {
    test_eval("for v in (seq 1 4) { print ($v * 2) }", Eq("2468"))
}

#[test]
fn early_return() {
    test_eval("do { return 'foo'; 'bar' }", Eq("foo"))
}

#[test]
fn early_return_from_if() {
    test_eval("do { if true { return 'pass' }; 'fail' }", Eq("pass"))
}

#[test]
fn early_return_from_loop() {
    test_eval("do { loop { return 'pass' } }", Eq("pass"))
}

#[test]
fn early_return_from_while() {
    test_eval(
        "do { let x = true; while $x { return 'pass' } }",
        Eq("pass"),
    )
}

#[test]
fn early_return_from_for() {
    test_eval("do { for x in [pass fail] { return $x } }", Eq("pass"))
}

#[test]
fn try_no_catch() {
    test_eval("try { error make { msg: foo } }; 'pass'", Eq("pass"))
}

#[test]
fn try_catch_no_var() {
    test_eval(
        "try { error make { msg: foo } } catch { 'pass' }",
        Eq("pass"),
    )
}

#[test]
fn try_catch_var() {
    test_eval(
        "try { error make { msg: foo } } catch { |err| $err.msg }",
        Eq("foo"),
    )
}

#[test]
fn try_catch_with_non_literal_closure_no_var() {
    test_eval(
        r#"
            let error_handler = { || "pass" }
            try { error make { msg: foobar } } catch $error_handler
        "#,
        Eq("pass"),
    )
}

#[test]
fn try_catch_with_non_literal_closure() {
    test_eval(
        r#"
            let error_handler = { |err| $err.msg }
            try { error make { msg: foobar } } catch $error_handler
        "#,
        Eq("foobar"),
    )
}

#[test]
fn try_catch_external() {
    test_eval(
        r#"try { nu -c 'exit 1' } catch { $env.LAST_EXIT_CODE }"#,
        Eq("1"),
    )
}

#[test]
fn row_condition() {
    test_eval(
        "[[a b]; [1 2] [3 4]] | where a < 3 | to nuon",
        Eq("[[a, b]; [1, 2]]"),
    )
}

#[test]
fn custom_command() {
    test_eval(
        r#"
            def cmd [a: int, b: string = 'fail', ...c: string, --x: int] { $"($a)($b)($c)($x)" }
            cmd 42 pass foo --x 30
        "#,
        Eq("42pass[foo]30"),
    )
}
