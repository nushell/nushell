use nu_experimental::{CELL_PATH_TYPES, ENFORCE_RUNTIME_ANNOTATIONS};
use nu_protocol::{ParseError, Type};
use nu_test_support::{
    fs::Stub::{self, FileWithContent},
    prelude::*,
};
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn env_shorthand() -> Result {
    test()
        .run("FOO=BAR if false { 3 } else { 4 }")
        .expect_value_eq(4)
}

#[test]
fn subcommand() -> Result {
    test()
        .run(r#"def foo [] {}; def "foo bar" [] {3}; foo bar"#)
        .expect_value_eq(3)
}

#[test]
fn alias_1() -> Result {
    test()
        .run("def foo [$x] { $x + 10 }; alias f = foo; f 100")
        .expect_value_eq(110)
}

#[test]
fn ints_with_underscores() -> Result {
    test()
        .run("1_0000_0000_0000 + 10")
        .expect_value_eq(1_000_000_000_010_i64)
}

#[test]
fn floats_with_underscores() -> Result {
    test()
        .run("3.1415_9265_3589_793 * 2")
        .expect_value_eq(6.283185307179586)
}

#[test]
fn bin_ints_with_underscores() -> Result {
    test().run("0b_10100_11101_10010").expect_value_eq(21426)
}

#[test]
fn oct_ints_with_underscores() -> Result {
    test()
        .run("0o2443_6442_7652_0044")
        .expect_value_eq(90_422_533_333_028_i64)
}

#[test]
fn hex_ints_with_underscores() -> Result {
    test().run("0x68__9d__6a").expect_value_eq(6_856_042)
}

#[test]
fn alias_2() -> Result {
    test()
        .run("def foo [$x $y] { $x + $y + 10 }; alias f = foo 33; f 100")
        .expect_value_eq(143)
}

#[test]
fn alias_2_multi_word() -> Result {
    test()
        .run(r#"def "foo bar" [$x $y] { $x + $y + 10 }; alias f = foo bar 33; f 100"#)
        .expect_value_eq(143)
}

#[ignore = "TODO: Allow alias to alias existing command with the same name"]
#[test]
fn alias_recursion() -> Result {
    let _: Value = test().run("alias ls = ls -a; ls")?;
    Ok(())
}

#[test]
fn block_param1() -> Result {
    test()
        .run("[3] | each { |it| $it + 10 } | get 0")
        .expect_value_eq(13)
}

#[test]
fn block_param2() -> Result {
    test()
        .run("[3] | each { |y| $y + 10 } | get 0")
        .expect_value_eq(13)
}

#[test]
fn block_param3_list_iteration() -> Result {
    test()
        .run("[1,2,3] | each { |it| $it + 10 } | get 1")
        .expect_value_eq(12)
}

#[test]
fn block_param4_list_iteration() -> Result {
    test()
        .run("[1,2,3] | each { |y| $y + 10 } | get 2")
        .expect_value_eq(13)
}

#[test]
fn range_iteration1() -> Result {
    test()
        .run("1..4 | each { |y| $y + 10 } | get 0")
        .expect_value_eq(11)
}

#[test]
fn range_iteration2() -> Result {
    test()
        .run("4..1 | each { |y| $y + 100 } | get 3")
        .expect_value_eq(101)
}

#[test]
fn range_ends_with_duration_suffix_variable_name() -> Result {
    test()
        .run("let runs = 10; 1..$runs | math sum")
        .expect_value_eq(55)
}

#[test]
fn range_ends_with_filesize_suffix_variable_name() -> Result {
    test()
        .run("let sizekb = 10; 1..$sizekb | math sum")
        .expect_value_eq(55)
}

#[test]
fn simple_value_iteration() -> Result {
    test().run("4 | each { |it| $it + 10 }").expect_value_eq(14)
}

#[test]
fn comment_multiline() -> Result {
    let code = "
        def foo [] {
            let x = 1 + 2 # comment
            let y = 3 + 4 # another comment
            $x + $y
        }; foo
    ";

    test().run(code).expect_value_eq(10)
}

#[test]
fn comment_skipping_1() -> Result {
    let code = "
        let x = {
            y: 20
            # foo
        }; $x.y
    ";

    test().run(code).expect_value_eq(20)
}

#[test]
fn comment_skipping_2() -> Result {
    let code = "
        let x = {
            y: 20
            # foo
            z: 40
        }; $x.z
    ";

    test().run(code).expect_value_eq(40)
}

#[test]
fn comment_skipping_in_pipeline_1() -> Result {
    let code = "
        [1,2,3] | #comment
        each { |$it| $it + 2 } | # foo
        math sum #bar
    ";

    test().run(code).expect_value_eq(12)
}

#[test]
fn comment_skipping_in_pipeline_2() -> Result {
    let code = "
        [1,2,3] #comment
        | #comment2
        each { |$it| $it + 2 } #foo
        | # bar
        math sum #baz
    ";

    test().run(code).expect_value_eq(12)
}

#[test]
fn comment_skipping_in_pipeline_3() -> Result {
    let code = "
        [1,2,3] | #comment
        #comment2
        each { |$it| $it + 2 } #foo
        | # bar
        #baz
        math sum #foobar
    ";

    test().run(code).expect_value_eq(12)
}

#[test]
fn still_string_if_hashtag_is_middle_of_string() -> Result {
    test()
        .run("echo test#testing")
        .expect_value_eq("test#testing")
}

#[test]
fn non_comment_hashtag_in_comment_does_not_stop_comment() -> Result {
    test()
        .run("# command_bar_text: { fg: '#C4C9C6' },")
        .expect_value_eq(())
}

#[test]
fn non_comment_hashtag_in_comment_does_not_stop_comment_in_block() -> Result {
    let code = "
        {
            explore: {
                # command_bar_text: { fg: '#C4C9C6' },
            }
        } | get explore | is-empty
    ";

    test().run(code).expect_value_eq(true)
}

#[test]
fn still_string_if_hashtag_is_middle_of_string_inside_each() -> Result {
    test()
        .run("1..1 | each {echo test#testing } | get 0")
        .expect_value_eq("test#testing")
}

#[test]
fn still_string_if_hashtag_is_middle_of_string_inside_each_also_with_dot() -> Result {
    test()
        .run("1..1 | each {echo '.#testing' } | get 0")
        .expect_value_eq(".#testing")
}

#[test]
fn bad_var_name() -> Result {
    let err = test().run(r#"let $"foo bar" = 4"#).expect_parse_error()?;
    assert_matches!(err, ParseError::VariableNotValid(_));
    Ok(())
}

#[test]
fn bad_var_name2() -> Result {
    let err = test().run("let $foo-bar = 4").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "valid variable name");

    let err = test().run("foo-bar=4 true").expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `foo-bar=4` not found");
    Ok(())
}

#[test]
fn bad_var_name3() -> Result {
    let err = test().run("let $=foo = 4").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "valid variable name");

    let err = test().run("=foo=4 true").expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `=foo=4` not found");
    Ok(())
}

#[rstest]
#[case::let_assignment("let = if $", "var_name")]
#[case::mut_assignment("mut = if $", "var_name")]
#[case::const_assignment("const = if $", "const_name")]
#[case::piped_let_assignment("let = 'foo' | $in; $x | describe", "var_name")]
#[case::piped_mut_assignment("mut = 'foo' | $in; $x | describe", "var_name")]
fn assignment_with_no_var(#[case] code: &str, #[case] expected_name: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::MissingPositional(name, _, _) if name == expected_name);
    Ok(())
}

// Test for https://github.com/nushell/nushell/issues/9072
#[rstest]
#[case::five_params("def a [b: bool, c: bool, d: float, e: float, f: float] {}; a true true 1 1")]
#[case::six_params(
    "def a [b: bool, c: bool, d: float, e: float, f: float, g: float] {}; a true true 1 1"
)]
#[case::seven_params(
    "def a [b: bool, c: bool, d: float, e: float, f: float, g: float, h: float] {}; a true true 1 1"
)]
fn too_few_arguments(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::MissingPositional(name, _, _) if name == "f");
    Ok(())
}

#[test]
fn long_flag() -> Result {
    test()
        .run("([a, b, c] | enumerate | each --keep-empty { |e| if $e.index != 1 { 100 }}).1")
        .expect_value_eq(())
}

#[test]
fn for_in_missing_var_name() -> Result {
    let err = test().run("for in").expect_parse_error()?;
    assert_matches!(err, ParseError::MissingPositional(name, _, _) if name == "var_name");
    Ok(())
}

#[test]
fn multiline_pipe_in_block() -> Result {
    let code = "
        do {
            echo hello |
            str length
        }
    ";

    test().run(code).expect_value_eq(5)
}

#[test]
fn bad_short_flag() -> Result {
    let err = test()
        .run("def foo3 [-l?:int] { $l }")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "short flag");
    Ok(())
}

#[test]
fn quotes_with_equals() -> Result {
    test()
        .run(r#"let query_prefix = "https://api.github.com/search/issues?q=repo:nushell/"; $query_prefix"#)
        .expect_value_eq("https://api.github.com/search/issues?q=repo:nushell/")
}

#[test]
fn string_interp_with_equals() -> Result {
    test()
        .run(r#"let query_prefix = $"https://api.github.com/search/issues?q=repo:nushell/"; $query_prefix"#)
        .expect_value_eq("https://api.github.com/search/issues?q=repo:nushell/")
}

#[test]
fn raw_string_with_equals() -> Result {
    test()
        .run("let query_prefix = r#'https://api.github.com/search/issues?q=repo:nushell/'#; $query_prefix")
        .expect_value_eq("https://api.github.com/search/issues?q=repo:nushell/")
}

#[test]
fn raw_string_with_hashtag() -> Result {
    test()
        .run("r##' one # two '##")
        .expect_value_eq(" one # two ")
}

#[test]
fn list_quotes_with_equals() -> Result {
    test()
        .run(r#"["https://api.github.com/search/issues?q=repo:nushell/"] | get 0"#)
        .expect_value_eq("https://api.github.com/search/issues?q=repo:nushell/")
}

#[test]
fn list_raw_string_unit_value_like() -> Result {
    test().run("[.foons] | get 0").expect_value_eq(".foons")
}

#[rstest]
#[case::key_ends_with_equals(r#"{"a=":b} | get a="#, "b")]
#[case::key_starts_with_equals(r#"{"=a":b} | get =a"#, "b")]
#[case::value_starts_with_equals(r#"{a:"=b"} | get a"#, "=b")]
#[case::value_ends_with_equals(r#"{a:"b="} | get a"#, "b=")]
#[case::second_key_starts_with_equals(r#"{a:b,"=c":d} | get =c"#, "d")]
#[case::second_key_ends_with_equals(r#"{a:b,"c=":d} | get c="#, "d")]
fn record_quotes_with_equals(#[case] code: &str, #[case] expected: &str) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[test]
fn recursive_parse() -> Result {
    test()
        .run("def c [] { c }; echo done")
        .expect_value_eq("done")
}

#[test]
fn commands_have_description() -> Result {
    let code = "
        # This is a test
        #
        # To see if I have cool description
        def foo [] {}
        help foo
    ";

    let actual: String = test().run(code)?;
    assert_contains("cool description", actual);
    Ok(())
}

#[test]
fn commands_from_crlf_source_have_short_description() -> Result {
    let code = "
        # This is a test
        #
        # To see if I have cool description
        def foo [] {}
        scope commands | where name == foo | get description.0
    ";

    test()
        .run(code.replace('\n', "\r\n"))
        .expect_value_eq("This is a test")
}

#[test]
fn commands_from_crlf_source_have_extra_description() -> Result {
    let code = "
        # This is a test
        #
        # To see if I have cool description
        def foo [] {}
        scope commands | where name == foo | get extra_description.0
    ";

    let actual: String = test().run(code.replace('\n', "\r\n"))?;
    assert_eq!(
        actual.trim_end_matches('\r'),
        "To see if I have cool description"
    );
    Ok(())
}

#[test]
fn equals_separates_long_flag() -> Result {
    test()
        .run("'nushell' | fill --alignment right --width=10 --character='-'")
        .expect_value_eq("---nushell")
}

#[test]
fn assign_expressions() -> Result {
    test()
        .env("VENV_OLD_PATH", "Foobar")
        .env("Path", "Quux")
        .run(r#"$env.Path = (if ($env | columns | "VENV_OLD_PATH" in $in) { $env.VENV_OLD_PATH } else { $env.Path }); $env.Path"#)
        .expect_value_eq("Foobar")
}

#[test]
fn assign_takes_pipeline() -> Result {
    test()
        .run("mut foo = 'bar'; $foo = $foo | str upcase | str reverse; $foo")
        .expect_value_eq("RAB")
}

#[test]
fn append_assign_takes_pipeline() -> Result {
    test()
        .run("mut foo = 'bar'; $foo ++= $foo | str upcase; $foo")
        .expect_value_eq("barBAR")
}

#[rstest]
#[case::bare("cococo")]
#[case::quoted("`cococo`")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn assign_external_fails(#[case] external: &str) -> Result {
    let code = format!("$env.FOO = {external}");
    let err = test().run(code).expect_parse_error()?;

    match err {
        ParseError::LabeledErrorWithHelp { error, .. } => {
            assert_contains("must be explicit", error);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[rstest]
#[case::with_caret("^cococo")]
#[case::quoted_with_caret("^`cococo`")]
#[nu_test_support::test]
#[deps(TESTBIN_COCOCO)]
fn assign_external_works(#[case] external: &str) -> Result {
    let code = format!("$env.FOO = {external}; $env.FOO");
    test().run(code).expect_value_eq("cococo")
}

#[test]
fn percent_forces_builtin_command() -> Result {
    test().run("%echo ok").expect_value_eq("ok")
}

#[test]
fn percent_prefers_builtin_when_custom_shadows_name() -> Result {
    let mut tester = test();
    let () = tester.run("def ls [] { 'hi' }")?;
    tester
        .run("(%ls | describe) == 'string'")
        .expect_value_eq(false)
}

#[test]
fn percent_prefers_builtin_inside_same_name_wrapper() -> Result {
    let actual: String = test().run("def ls [] { %ls | move name --last }; ls | describe")?;
    assert_contains("table", actual);
    Ok(())
}

#[test]
fn percent_help_prefers_builtin_when_alias_shadows_name() -> Result {
    let actual: String = test().run("alias cd = echo; %cd --help")?;
    assert_contains("Change the current working directory.", actual);
    Ok(())
}

#[test]
fn help_percent_prefers_builtin_without_alias() -> Result {
    let actual: String = test().run("help %cd")?;
    assert_contains("Change the current working directory.", actual);
    Ok(())
}

#[test]
fn help_percent_prefers_builtin_when_alias_shadows_name() -> Result {
    let actual: String = test().run("alias cd = echo; help %cd")?;
    assert_contains("Change the current working directory.", actual);
    Ok(())
}

#[test]
fn help_plain_keeps_alias_resolution_behavior() -> Result {
    let actual: String = test().run("alias cd = echo; help cd")?;
    assert_contains(
        "Returns its arguments, ignoring the piped-in value.",
        actual,
    );
    Ok(())
}

#[test]
fn plain_help_keeps_alias_resolution_behavior() -> Result {
    let actual: String = test().run("alias cd = echo; cd --help")?;
    assert_contains(
        "Returns its arguments, ignoring the piped-in value.",
        actual,
    );
    Ok(())
}

#[rstest]
#[case::unknown_command("%nu --version")]
#[case::custom_command("def foo [] { 'ok' }; %foo")]
#[case::alias_to_internal("def foo [] { 'ok' }; alias bar = foo; %bar")]
#[case::alias_to_external("alias ext = ^nu --version; %ext")]
fn percent_requires_builtin(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    match err {
        ParseError::LabeledErrorWithHelp { error, .. } => {
            assert_contains("percent sigil requires a built-in command", error);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[test]
fn percent_dynamic_dispatch_with_builtin() -> Result {
    let code = "let cmd = 'echo'; %($cmd) 'hello'";
    test().run(code).expect_value_eq("hello")
}

#[test]
fn percent_dynamic_dispatch_bare_var() -> Result {
    let code = "let cmd = 'echo'; %$cmd 'hello'";
    test().run(code).expect_value_eq("hello")
}

#[test]
fn percent_dynamic_dispatch_with_paren_expr() -> Result {
    let code = "%('echo') 'world'";
    test().run(code).expect_value_eq("world")
}

#[test]
fn percent_dynamic_dispatch_with_non_builtin() -> Result {
    let code = "let cmd = 'my_nonexistent_cmd'; %($cmd)";
    let err = test().run(code).expect_error()?;
    assert_matches!(err, ShellError::CommandNotFound { .. });
    Ok(())
}

#[test]
fn percent_dynamic_dispatch_with_custom_command() -> Result {
    let code = "def custom_cmd [] { 'nope' }; let cmd = 'custom_cmd'; %($cmd)";
    let err = test().run(code).expect_error()?;
    assert_matches!(err, ShellError::CommandNotFound { .. });
    Ok(())
}

#[test]
fn percent_dynamic_dispatch_prefers_builtin_when_custom_shadows_name() -> Result {
    let code = "def echo [] { 'shadowed' }; let cmd = 'echo'; %($cmd) 'hello'";
    test().run(code).expect_value_eq("hello")
}

#[test]
fn percent_dynamic_dispatch_prefers_builtin_inside_same_name_wrapper() -> Result {
    let code = "def echo [] { 'shadowed' }; def wrapper [cmd] { %($cmd) 'hello' }; wrapper 'echo'";
    test().run(code).expect_value_eq("hello")
}

#[test]
fn percent_dynamic_dispatch_alias_to_custom_command_is_not_builtin() -> Result {
    let code = "def custom_cmd [] { 'shadowed' }; alias maybe_builtin = custom_cmd; let cmd = 'maybe_builtin'; %($cmd)";
    let err = test().run(code).expect_error()?;
    assert_matches!(err, ShellError::CommandNotFound { .. });
    Ok(())
}

#[test]
fn percent_dynamic_dispatch_with_spread_args() -> Result {
    let code = "let cmd = 'echo'; let args = ['hello' 'world']; %($cmd) ...$args";
    test().run(code).expect_value_eq(["hello", "world"])
}

#[test]
fn percent_dynamic_dispatch_with_mixed_positional_and_spread_args() -> Result {
    let code = "let cmd = 'echo'; let args = ['middle' 'end']; %($cmd) 'start' ...$args";
    test().run(code).expect_value_eq(["start", "middle", "end"])
}

#[test]
fn percent_dynamic_dispatch_in_wrapped_command_forwards_rest_args() -> Result {
    let code = "export def --wrapped builtin [arg1, ...args] { %($arg1) ...$args }; builtin echo hello world";
    test().run(code).expect_value_eq(["hello", "world"])
}

#[test]
fn percent_dynamic_dispatch_in_wrapped_command_preserves_no_arg_builtin_defaults() -> Result {
    Playground::setup(
        "percent_dynamic_dispatch_in_wrapped_command_preserves_no_arg_builtin_defaults",
        |dirs, play| {
            play.with_files(&[Stub::EmptyFile("probe.txt")]);

            let code = "
                export def --wrapped builtin [arg1, ...args] { %($arg1) ...$args }
                let direct = (ls | where name =~ 'probe.txt' | length)
                let wrapped = (builtin ls | where name =~ 'probe.txt' | length)
                [$direct $wrapped]
            ";

            test().cwd(dirs.test()).run(code).expect_value_eq([1, 1])
        },
    )
}

#[rstest]
#[case::subexpression("(do {0})-str")]
#[case::closure("({|| })-str")]
#[case::ambiguous_block(r#"(if true { "T" } else { "F" })-str"#)]
#[case::spaced_subexpression("(do {0} )-str")]
#[case::spaced_closure("({|| } )-str")]
#[case::spaced_ambiguous_block(r#"(if true { "T" } else { "F" } )-str"#)]
#[case::unambiguous_block(r#"(if true { "T" })-str"#)]
fn bare_interpolation_does_not_hide_redefined_command(#[case] body: &str) -> Result {
    let mut tester = test();
    let () = tester.run(r#"def cmd [] { "fallback" }"#)?;
    let same_entry: String = tester.run(format!("def cmd [] {{ {body} }}; cmd"))?;
    let later_entry: String = tester.run("cmd")?;

    assert_ne!(same_entry, "fallback");
    assert_eq!(same_entry, later_entry);
    Ok(())
}

#[test]
fn string_interpolation_paren_test() -> Result {
    test().run(r#"$"('(')(')')""#).expect_value_eq("()")
}

#[test]
fn string_interpolation_paren_test2() -> Result {
    test().run(r#"$"('(')test(')')""#).expect_value_eq("(test)")
}

#[test]
fn string_interpolation_paren_test3() -> Result {
    test()
        .run(r#"$"('(')("test")test(')')""#)
        .expect_value_eq("(testtest)")
}

#[test]
fn string_interpolation_escaping() -> Result {
    test()
        .run(r#"$"hello\nworld" | lines | length"#)
        .expect_value_eq(2)
}

#[test]
fn capture_multiple_commands() -> Result {
    let code = "
        let CONST_A = 'Hello'

        def 'say-hi' [] {
            echo (call-me)
        }

        def 'call-me' [] {
            echo $CONST_A
        }

        [(say-hi) (call-me)] | str join

    ";

    test().run(code).expect_value_eq("HelloHello")
}

#[test]
fn capture_multiple_commands2() -> Result {
    let code = "
        let CONST_A = 'Hello'

        def 'call-me' [] {
            echo $CONST_A
        }

        def 'say-hi' [] {
            echo (call-me)
        }

        [(say-hi) (call-me)] | str join

    ";

    test().run(code).expect_value_eq("HelloHello")
}

#[test]
fn capture_multiple_commands3() -> Result {
    let code = "
        let CONST_A = 'Hello'

        def 'say-hi' [] {
            echo (call-me)
        }

        def 'call-me' [] {
            echo $CONST_A
        }

        [(call-me) (say-hi)] | str join

    ";

    test().run(code).expect_value_eq("HelloHello")
}

#[test]
fn capture_multiple_commands4() -> Result {
    let code = "
        let CONST_A = 'Hello'

        def 'call-me' [] {
            echo $CONST_A
        }

        def 'say-hi' [] {
            echo (call-me)
        }

        [(call-me) (say-hi)] | str join

    ";

    test().run(code).expect_value_eq("HelloHello")
}

#[test]
fn capture_row_condition() -> Result {
    test()
        .run(r#"let name = "foo"; [foo] | where $'($name)' =~ $it | str join"#)
        .expect_value_eq("foo")
}

#[test]
fn starts_with_operator_succeeds() -> Result {
    test()
        .run("[Moe Larry Curly] | where $it starts-with L | str join")
        .expect_value_eq("Larry")
}

#[test]
fn not_starts_with_operator_succeeds() -> Result {
    test()
        .run("[Moe Larry Curly] | where $it not-starts-with L | str join")
        .expect_value_eq("MoeCurly")
}

#[test]
fn ends_with_operator_succeeds() -> Result {
    test()
        .run("[Moe Larry Curly] | where $it ends-with ly | str join")
        .expect_value_eq("Curly")
}

#[test]
fn not_ends_with_operator_succeeds() -> Result {
    test()
        .run("[Moe Larry Curly] | where $it not-ends-with y | str join")
        .expect_value_eq("Moe")
}

#[test]
fn proper_missing_param() -> Result {
    let err = test()
        .run("def foo [x y z w] { }; foo a b c")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::MissingPositional(name, _, _) if name == "w");
    Ok(())
}

#[test]
fn block_arity_check1() -> Result {
    let err = test().run("ls | each { |x, y| 1}").expect_parse_error()?;
    assert_matches!(err, ParseError::ExpectedWithStringMsg(expected, _) if expected == "1 closure parameter");
    Ok(())
}

// deprecating former support for escapes like `/uNNNN`, dropping test.
#[test]
fn string_escape_unicode_extended() -> Result {
    test().run(r#""\u{015B}\u{1f10b}""#).expect_value_eq("ś🄋")
}

#[test]
fn string_escape_interpolation() -> Result {
    test()
        .run(r#"$"\u{015B}(char hamburger)abc""#)
        .expect_value_eq("ś≡abc")
}

#[test]
fn string_escape_interpolation2() -> Result {
    test()
        .run(r#"$"2 + 2 is \(2 + 2)""#)
        .expect_value_eq("2 + 2 is (2 + 2)")
}

#[test]
fn proper_rest_types() -> Result {
    test()
        .run(
            r#"def foo [--verbose(-v), # my test flag
                   ...rest: int # my rest comment
                ] { if $verbose { "verbose!" } else { "not verbose!" } }; foo"#,
        )
        .expect_value_eq("not verbose!")
}

#[test]
fn single_value_row_condition() -> Result {
    test()
        .run("[[a, b]; [true, false], [true, true]] | where a | length")
        .expect_value_eq(2)
}

#[test]
fn row_condition_non_boolean() -> Result {
    let err = test().run("[1 2 3] | where 1").expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::Bool, Type::Int, _));
    Ok(())
}

#[test]
fn performance_nested_lists() -> Result {
    // Parser used to be exponential on deeply nested lists
    // TODO: Add a timeout
    let err = test()
        .run("[[[[[[[[[[[[[[[[[[[[[[[[[[[[")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedEof(delimiter, _) if delimiter == "]");
    Ok(())
}

#[test]
fn performance_nested_modules() -> Result {
    // Parser used to be exponential on deeply nested modules
    // TODO: Add a timeout
    let code = "
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        module foo { module foo { module foo { module foo {
        use bar.nu }}}}}}}}}}}}}}}}}}}}}}}}}}}}
    ";
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::ModuleNotFound(_, module) if module == "bar.nu");
    Ok(())
}

#[test]
fn unary_not_1() -> Result {
    test().run("not false").expect_value_eq(true)
}

#[test]
fn unary_not_2() -> Result {
    test().run("not (false)").expect_value_eq(true)
}

#[test]
fn unary_not_3() -> Result {
    test().run("(not false)").expect_value_eq(true)
}

#[test]
fn unary_not_4() -> Result {
    test()
        .run(r#"if not false { "hello" } else { "world" }"#)
        .expect_value_eq("hello")
}

#[test]
fn unary_not_5() -> Result {
    test()
        .run(r#"if not not not not false { "hello" } else { "world" }"#)
        .expect_value_eq("world")
}

#[test]
fn unary_not_6() -> Result {
    test()
        .run("[[name, present]; [abc, true], [def, false]] | where not present | get name.0")
        .expect_value_eq("def")
}

#[test]
fn comment_in_multiple_pipelines() -> Result {
    let code = "
        [[name, present]; [abc, true], [def, false]]
        # | where not present
        | get name.0
    ";

    test().run(code).expect_value_eq("abc")
}

#[test]
fn date_literal() -> Result {
    test()
        .run("2022-09-10 | into record | get day")
        .expect_value_eq(10)
}

#[test]
fn and_and_or() -> Result {
    test().run("true and false or true").expect_value_eq(true)
}

#[test]
fn and_and_xor() -> Result {
    // Assumes the precedence NOT > AND > XOR > OR
    test()
        .run("true and true xor true and false")
        .expect_value_eq(true)
}

#[test]
fn or_and_xor() -> Result {
    // Assumes the precedence NOT > AND > XOR > OR
    test()
        .run("true or false xor true or false")
        .expect_value_eq(true)
}

#[test]
fn unbalanced_delimiter() -> Result {
    let err = test().run("{a:{b:5}}}").expect_parse_error()?;
    assert_matches!(err, ParseError::Unbalanced("{", "}", _));
    Ok(())
}

#[test]
fn unbalanced_delimiter2() -> Result {
    let err = test().run("{}#.}").expect_parse_error()?;
    assert_matches!(err, ParseError::Unbalanced("{", "}", _));
    Ok(())
}

#[test]
fn unbalanced_delimiter3() -> Result {
    let err = test().run("{").expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedEof(delimiter, _) if delimiter == "}");
    Ok(())
}

#[test]
fn unbalanced_delimiter4() -> Result {
    let err = test().run("}").expect_parse_error()?;
    assert_matches!(err, ParseError::Unbalanced("{", "}", _));
    Ok(())
}

#[test]
fn unbalanced_parens1() -> Result {
    let err = test().run(")").expect_parse_error()?;
    assert_matches!(err, ParseError::Unbalanced("(", ")", _));
    Ok(())
}

#[test]
fn unbalanced_parens2() -> Result {
    let err = test().run(r#"("("))"#).expect_parse_error()?;
    assert_matches!(err, ParseError::Unbalanced("(", ")", _));
    Ok(())
}

#[cfg(feature = "plugin")]
mod plugin_tests {
    use super::*;

    #[test]
    fn plugin_use_with_string_literal() -> Result {
        let err = test()
            .run("plugin use 'nu-plugin-math'")
            .expect_parse_error()?;
        assert_matches!(err, ParseError::LabeledErrorWithHelp { error, .. } if error == "Plugin registry file not set");
        Ok(())
    }

    #[test]
    fn plugin_use_with_string_constant() -> Result {
        let input = "
            const file = 'nu-plugin-math'
            plugin use $file
        ";
        // should not fail with `not a constant`
        let err = test().run(input).expect_parse_error()?;
        assert_matches!(err, ParseError::LabeledErrorWithHelp { error, .. } if error == "Plugin registry file not set");
        Ok(())
    }

    #[test]
    fn plugin_use_with_string_variable() -> Result {
        let input = "
            let file = 'nu-plugin-math'
            plugin use $file
        ";
        let err = test().run(input).expect_parse_error()?;
        assert_matches!(
            err,
            ParseError::LabeledError(_, label, _)
                if label == "Encountered error during parse-time evaluation"
        );
        Ok(())
    }

    #[test]
    fn plugin_use_with_non_string_constant() -> Result {
        let input = "
            const file = 6
            plugin use $file
        ";
        let err = test().run(input).expect_parse_error()?;
        assert_matches!(err, ParseError::TypeMismatch(Type::String, Type::Int, _));
        Ok(())
    }
}

#[test]
fn extern_errors_with_no_space_between_params_and_name_1() -> Result {
    let err = test().run("extern cmd[]").expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledErrorWithHelp { label, .. } if label == "expected space");
    Ok(())
}

#[test]
fn extern_errors_with_no_space_between_params_and_name_2() -> Result {
    let err = test().run("extern cmd(--flag)").expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledErrorWithHelp { label, .. } if label == "expected space");
    Ok(())
}

#[test]
fn duration_with_float_number() -> Result {
    test()
        .run(".6min")
        .expect_value_eq(std::time::Duration::from_secs(36))
}

#[test]
fn extern_with_reserved_variable_name_1() -> Result {
    test().run("extern cmd [in, --env]").expect_value_eq(())
}

#[test]
fn extern_with_reserved_variable_name_2() -> Result {
    test()
        .run("export extern cmd (in, --env, ...nu)")
        .expect_value_eq(())
}

#[test]
fn extern_allows_default_value_in_signature() -> Result {
    test().run("extern cmd [in: bool=true]").expect_value_eq(())
}

#[test]
fn duration_with_underscores_1() -> Result {
    test()
        .run("420_min")
        .expect_value_eq(std::time::Duration::from_secs(7 * 60 * 60))
}

#[test]
fn duration_with_underscores_2() -> Result {
    test()
        .run("1_000_000sec")
        .expect_value_eq(std::time::Duration::from_secs(1_000_000))
}

#[test]
fn duration_with_underscores_3() -> Result {
    let err = test().run("1_000_d_ay").expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `1_000_d_ay` not found");
    Ok(())
}

#[test]
fn duration_with_faulty_number() -> Result {
    let err = test().run("sleep 4-ms").expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledError(error, _, _) if error == "duration value must be a number");
    Ok(())
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.utf8")]
#[env(LANG = "en_US.UTF-8")]
#[env(LANGUAGE = "en")]
fn filesize_with_underscores_1() -> Result {
    test()
        .run("420_MB")
        .expect_value_eq(Value::test_filesize(420_000_000))
}

#[test]
#[env(NU_TEST_LOCALE_OVERRIDE = "en_US.utf8")]
#[env(LANG = "en_US.UTF-8")]
#[env(LANGUAGE = "en")]
fn filesize_with_underscores_2() -> Result {
    test()
        .run("1_000_000B")
        .expect_value_eq(Value::test_filesize(1_000_000))
}

#[test]
fn filesize_with_underscores_3() -> Result {
    let err = test().run("42m_b").expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `42m_b` not found");
    Ok(())
}

#[test]
fn filesize_is_not_hex() -> Result {
    test().run("0x42b").expect_value_eq(1067)
}

#[test]
fn let_variable_type_mismatch() -> Result {
    let err = test().run(r#"let x: int = "foo""#).expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::Int, Type::String, _));
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn let_variable_table_runtime_cast() -> Result {
    let actual: String =
        test().run("let x: table = ([[a]; [1]] | to nuon | from nuon); $x | describe")?;

    // Type::Any should be accepted by compatible types (record can convert to table)
    assert_contains("table<a: int>", actual);
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn let_variable_table_runtime_mismatch() -> Result {
    let code = "mut x: table<b: int> = ([[b]; [1]]  | to nuon | from nuon); $x = [[a]; [1]]";
    let err = test().run(code).expect_parse_error()?;

    // This conversion should fail due to a key mismatch
    assert_matches!(
        err,
        ParseError::OperatorIncompatibleTypes { op, lhs, rhs, .. }
            if op == "=" && lhs.to_string() == "table<b: int>" && rhs.to_string() == "table<a: int>"
    );
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn mut_variable_table_runtime_mismatch() -> Result {
    let code = "mut x: table<b: int> = ([[b]; [1]]  | to nuon | from nuon); $x = [[a]; [1]]";
    let err = test().run(code).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::OperatorIncompatibleTypes { op, lhs, rhs, .. }
            if op == "=" && lhs.to_string() == "table<b: int>" && rhs.to_string() == "table<a: int>"
    );
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn let_variable_record_runtime_cast() -> Result {
    let actual: String =
        test().run("let x: record<a: int> = ({a: 1} | to nuon | from nuon); $x | describe")?;

    // Records from Type::Any sources should be convertible to tables when field types match
    assert_contains("record<a: int>", actual);
    Ok(())
}

#[test]
#[exp(ENFORCE_RUNTIME_ANNOTATIONS)]
fn let_variable_record_runtime_mismatch() -> Result {
    let code = "let x: record<b: int> = ({a: 1} | to nuon | from nuon); $x | describe";
    let err = test().run(code).expect_shell_error()?;

    // This conversion should fail due to a key mismatch
    let err = err.into_labeled()?;
    assert_eq!(err.code.as_deref(), Some("nu::shell::type_mismatch"));
    let labels = err
        .labels
        .iter()
        .map(|label| label.text.as_str())
        .collect::<Vec<_>>();
    assert_contains("expected record<b: int>, got record<a: int>", labels);
    Ok(())
}

#[rstest]
#[case::string("a", "list<record<b: int>>")]
#[case::string_int("a.0", "record<b: int>")]
#[case::string_int_string("a.0.b", "int")]
#[case::string_string_int("a.b.0", "int")]
#[nu_test_support::test]
#[exp(CELL_PATH_TYPES)]
fn let_assign_record_cell_path_to_wrong_type(
    #[case] cell_path: &str,
    #[case] inferred_type: &str,
) -> Result {
    let code = format!("let foo = {{a: [{{b: 1}}]}}; let bar: string = $foo.{cell_path}");
    let err = test().run(code).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::TypeMismatch(Type::String, found, _) if found.to_string() == inferred_type
    );
    Ok(())
}

#[rstest]
#[case::int("0", "record<a: record<b: list<int>>>")]
#[case::int("0.a", "record<b: list<int>>")]
#[case::int("0.a.b", "list<int>")]
#[case::int("0.a.b.0", "int")]
#[case::string("a", "list<record<b: list<int>>>")]
#[case::string_int("a.0", "record<b: list<int>>")]
#[case::string_int_string("a.0.b", "list<int>")]
#[case::string_string("a.b", "list<list<int>>")]
#[case::string_string_int("a.b.0", "list<int>")]
#[case::string_string_int_int("a.b.0.0", "int")]
#[nu_test_support::test]
#[exp(CELL_PATH_TYPES)]
fn let_assign_list_cell_path_to_wrong_type(
    #[case] cell_path: &str,
    #[case] inferred_type: &str,
) -> Result {
    let code = format!("let foo = [{{a: {{b: [1]}}}}]; let bar: string = $foo.{cell_path}");
    let err = test().run(code).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::TypeMismatch(Type::String, found, _) if found.to_string() == inferred_type
    );
    Ok(())
}

#[rstest]
#[case::int("0", "record<a: record<b: list<int>>>")]
#[case::int_string("0.a", "record<b: list<int>>")]
#[case::int_string_string("0.a.b", "list<int>")]
#[case::int_string_string_int("0.a.b.0", "int")]
#[case::string("a", "list<record<b: list<int>>>")]
#[case::string_int("a.0", "record<b: list<int>>")]
#[case::string_int_string("a.0.b", "list<int>")]
#[case::string_string("a.b", "list<list<int>>")]
#[case::string_string_int("a.b.0", "list<int>")]
#[case::string_string_int_int("a.b.0.0", "int")]
#[nu_test_support::test]
#[exp(CELL_PATH_TYPES)]
fn let_assign_table_cell_path_to_wrong_type(
    #[case] cell_path: &str,
    #[case] inferred_type: &str,
) -> Result {
    let code = format!("let foo = [[a]; [{{b: [1]}}]]; let bar: string = $foo.{cell_path}");
    let err = test().run(code).expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::TypeMismatch(Type::String, found, _) if found.to_string() == inferred_type
    );
    Ok(())
}

#[test]
fn let_variable_disallows_completer() -> Result {
    let err = test()
        .run("let x: int@completer = 42")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledError(error, _, _) if error == "Unexpected custom completer in type spec");
    Ok(())
}

#[test]
fn def_with_input_output() -> Result {
    test()
        .run("def foo []: nothing -> int { 3 }; foo")
        .expect_value_eq(3)
}

#[test]
fn def_with_input_output_with_line_breaks() -> Result {
    let code = "
        def foo []: [
          nothing -> int
        ] { 3 }; foo
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
fn def_with_multi_input_output_with_line_breaks() -> Result {
    let code = "
        def foo []: [
          nothing -> int
          string -> int
        ] { 3 }; foo
    ";

    test().run(code).expect_value_eq(3)
}

#[test]
fn def_with_multi_input_output_without_commas() -> Result {
    test()
        .run("def foo []: [nothing -> int string -> int] { 3 }; foo")
        .expect_value_eq(3)
}

#[test]
fn def_with_multi_input_output_called_with_first_sig() -> Result {
    test()
        .run("def foo []: [int -> int, string -> int] { 3 }; 10 | foo")
        .expect_value_eq(3)
}

#[test]
fn def_with_multi_input_output_called_with_second_sig() -> Result {
    test()
        .run(r#"def foo []: [int -> int, string -> int] { 3 }; "bob" | foo"#)
        .expect_value_eq(3)
}

#[test]
fn def_with_input_output_mismatch_1() -> Result {
    let err = test()
        .run("def foo []: [int -> int, string -> int] { 3 }; foo")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::InputMismatch(input, _) if input == "nothing");
    Ok(())
}

#[test]
fn def_with_input_output_mismatch_2() -> Result {
    let err = test()
        .run("def foo []: [int -> int, string -> int] { 3 }; {x: 2} | foo")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::InputMismatch(input, _) if input == "record<x: int>");
    Ok(())
}

#[test]
fn def_with_input_output_broken_1() -> Result {
    let err = test().run("def foo []: int { 3 }").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "arrow (->)");
    Ok(())
}

#[test]
fn def_with_input_output_broken_2() -> Result {
    let err = test()
        .run("def foo []: int -> { 3 }")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::MissingType(_));
    Ok(())
}

#[test]
fn def_with_input_output_broken_3() -> Result {
    let err = test()
        .run("def foo []: int -> int@completer {}")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledError(error, _, _) if error == "Unexpected custom completer in type spec");
    Ok(())
}

#[test]
fn def_with_input_output_broken_4() -> Result {
    let err = test()
        .run("def foo []: int -> list<int@completer> {}")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::LabeledError(error, _, _) if error == "Unexpected custom completer in type spec");
    Ok(())
}

#[test]
fn def_with_in_var_let_1() -> Result {
    test()
        .run(r#"def foo []: [int -> int, string -> int] { let x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; "100" | foo"#)
        .expect_value_eq(4)
}

#[test]
fn def_with_in_var_let_2() -> Result {
    test()
        .run(r#"def foo []: [int -> int, string -> int] { let x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; 100 | foo"#)
        .expect_value_eq(3)
}

#[test]
fn def_with_in_var_mut_1() -> Result {
    test()
        .run(r#"def foo []: [int -> int, string -> int] { mut x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; "100" | foo"#)
        .expect_value_eq(4)
}

#[test]
fn def_with_in_var_mut_2() -> Result {
    test()
        .run(r#"def foo []: [int -> int, string -> int] { mut x = $in; if ($x | describe) == "int" { 3 } else { 4 } }; 100 | foo"#)
        .expect_value_eq(3)
}

#[test]
fn properly_nest_captures() -> Result {
    test()
        .run("do { let b = 3; def c [] { $b }; c }")
        .expect_value_eq(3)
}

#[test]
fn properly_nest_captures_call_first() -> Result {
    test()
        .run("do { let b = 3; c; def c [] { $b }; c }")
        .expect_value_eq(3)
}

#[test]
fn properly_typecheck_rest_param() -> Result {
    test()
        .run(r#"def foo [...rest: string] { $rest | length }; foo "a" "b" "c""#)
        .expect_value_eq(3)
}

#[test]
fn implied_collect_has_compatible_type() -> Result {
    test()
        .run("let idx = 3 | $in; $idx < 1")
        .expect_value_eq(false)
}

#[test]
fn record_expected_colon() -> Result {
    let err = test().run("{ a: 2 b }").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "':'");

    let err = test().run("{ a: 2 b 3 }").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "':'");
    Ok(())
}

#[test]
fn record_missing_value() -> Result {
    let err = test().run("{ a: 2 b: }").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "value for record field");
    Ok(())
}

#[test]
fn record_type_inferred() -> Result {
    let err = test()
        .run("let foo: string = { 1: 1 }")
        .expect_parse_error()?;
    assert_matches!(err, ParseError::TypeMismatch(Type::String, found, _) if found.to_string() == "record<1: int>");
    Ok(())
}

#[test]
fn record_force_string_key_names() -> Result {
    test().run("{1kb: 1}.1kb").expect_value_eq(1)
}

#[test]
fn def_requires_body_closure() -> Result {
    let err = test().run("def a [] (echo 4)").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected(expected, _) if expected == "definition body closure { ... }");
    Ok(())
}

#[test]
#[deps(NU)]
fn not_panic_with_recursive_call() -> Result {
    test()
        .run_multiple([
            "def px [] { if true { 3 } else { px } }",
            "let x = 1",
            "$x | px",
        ])
        .expect_value_eq(3)?;

    test()
        .run_multiple([
            "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
            "let x = 1",
            "$x | px",
        ])
        .expect_value_eq(false)?;

    test()
        .run_multiple([
            "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
            "let x = 1",
            "def foo [] { $x }",
            "foo | px",
        ])
        .expect_value_eq(false)?;

    test()
        .run_multiple([
            "def px [n=0] { let l = $in; if $n == 0 { return false } else { $l | px ($n - 1) } }",
            "let x = 1",
            "do {|| $x } | px",
        ])
        .expect_value_eq(false)?;

    let result: CompleteResult = test()
        .cwd("tests/parsing/samples")
        .run("nu recursive_func_with_alias.nu | complete")?;
    assert_eq!(result.exit_code, 0);
    assert_eq!(result.stderr, "");

    Ok(())
}

// https://github.com/nushell/nushell/issues/16040
#[test]
#[deps(TESTBIN_COCOCO)]
fn external_argument_with_subexpressions() -> Result {
    test()
        .run("^cococo foo( ('bar') | $in ++ 'baz' )")
        .expect_value_eq("foobarbaz")?;
    test()
        .run("^cococo foo( 'bar' )('baz')")
        .expect_value_eq("foobarbaz")?;
    test()
        .run(r#"^cococo ")('foo')(""#)
        .expect_value_eq(")('foo')(")?;

    let err = test().run("^cococo foo( 'bar'").expect_parse_error()?;
    assert_matches!(err, ParseError::UnexpectedEof(delimiter, _) if delimiter == ")");
    Ok(())
}

// https://github.com/nushell/nushell/issues/16332
#[test]
fn quote_escape_but_not_env_shorthand() -> Result {
    test().run(r#""\"=foo""#).expect_value_eq("\"=foo")
}

// https://github.com/nushell/nushell/issues/16586
// Shadowing the `def` keyword used to panic in the REPL; it is now rejected cleanly.
#[rstest]
#[case("def def (=a|s)>")]
#[case("def def [] {}")]
fn redefine_def_should_not_panic(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::NameIsKeyword(..));
    Ok(())
}

#[test]
fn table_literal_column_var() -> Result {
    let code = "
            let column_name = 'column0'
            let tbl = [[ $column_name column1 ]; [ foo bar ] [ baz car ] [ far fit ]]
            $tbl.column0.0
        ";

    test().run(code).expect_value_eq("foo")
}

#[test]
fn table_literal_column_var_parse_err() -> Result {
    let code = "
            let column_name = {a: 123}
            let tbl = [[ $column_name column1 ]; [ foo bar ] [ baz car ] [ far fit ]]
            $tbl.column0.0
        ";
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::LabeledErrorWithHelp { error, label, .. }
            if error == "Table column name not string" && label == "must be a string"
    );
    Ok(())
}

#[test]
fn table_literal_column_var_shell_err() -> Result {
    let code = "
            let column_name = echo {a: 123}
            let tbl = [[ $column_name column1 ]; [ foo bar ] [ baz car ] [ far fit ]]
            $tbl.column0.0
        ";
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "string" && from_type == "record<a: int>"
    );
    Ok(())
}

#[rstest]
#[case::piped_let_assignment("null | let nu: nothing")]
#[case::piped_let_assignment("null | let $in")]
#[case::const_assignment("const env: nothing = null")]
#[case::mut_assignment("mut nu = null")]
#[case::for_loop("for nu in [] {}")]
#[case::for_loop_with_type("for $in: int in [] {}")]
#[case::match_pattern_list("match [1] {[$in] => $in}")]
#[case::match_pattern_list_rest("match [1] {[..$env] => $in}")]
#[case::match_pattern_record("{a: {b: 3}} | match $in {{a: { $nu }} => 10 }")]
fn reserved_variable_name_checking(#[case] code: &str) -> Result {
    let err = test().run(code).expect_parse_error()?;
    assert_matches!(err, ParseError::NameIsBuiltinVar(_, _));
    Ok(())
}

#[test]
fn allow_it_as_variable_name() -> Result {
    test()
        .run("let it = 3; [1 2 3 4] | where $it > 2 | length")
        .expect_value_eq(2)
}

#[test]
fn keep_variable_it_after_where() -> Result {
    // Test for https://github.com/nushell/nushell/issues/17380
    test()
        .run("let it = 3; [1 2 3 4] | where $it > 2; $it")
        .expect_value_eq(3)
}

#[test]
#[deps(NU)]
fn external_arg_correctness() -> Result {
    Playground::setup("external_arg_correctness", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent(
            "script.nu",
            "
            def main [
                --flag: external_arg
                --flag2: external_arg
                --flag3: external_arg
                arg: external_arg
                arg2: external_arg
                arg3: external_arg
                ...rest: external_arg
            ] {
                [
                    [label value type];
                    [flag $flag ($flag | describe)]
                    [flag2 $flag2 ($flag2 | describe)]
                    [flag3 $flag3 ($flag3 | describe)]
                    [arg $arg ($arg | describe)]
                    [arg2 $arg2 ($arg2 | describe)]
                    [arg3 $arg3 ($arg3 | describe)]
                    [rest $rest ($rest | describe)]
                ]
                | to nuon
            }
            ",
        )]);

        let code = "
            nu script.nu --flag=false --flag2=0001 --flag3={fake: null} false 0001 {fake: null} false 0001 {fake: null}
            | from nuon
        ";

        test().cwd(dirs.test()).run(code).expect_value_eq(test_table![
            ["label", "value", "type"];
            ["flag", "false", "glob"],
            ["flag2", "0001", "glob"],
            ["flag3", "{fake: null}", "string"],
            ["arg", "false", "glob"],
            ["arg2", "0001", "glob"],
            ["arg3", "{fake: null}", "string"],
            ["rest", test_value!(["false", "0001", "{fake: null}"]), "list<oneof<glob, string>>"],
        ])
    })
}
