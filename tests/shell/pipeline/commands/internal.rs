use itertools::Itertools;
use nu_test_support::playground::Playground;
use nu_test_support::prelude::*;
use pretty_assertions::{assert_eq, assert_matches};
use rstest::rstest;

const NU_RUNNER: &str = "let commands = $in; nu -n -c $commands | complete";

#[test]
#[deps(TESTBIN_CHOP, TESTBIN_COCOCO)]
fn takes_rows_of_nu_value_strings_and_pipes_it_to_stdin_of_external() -> Result {
    let code = r###"
        [
            [name rusty_luck origin];
            [Jason 1 Canada]
            [JT 1 "New Zealand"]
            [Andres 1 Ecuador]
            [AndKitKatz 1 "Estados Unidos"]
        ]
        | get origin
        | each {|it| cococo $it | chop}
        | get 2
    "###;

    // chop removes the last character from Ecuador.
    test().run(code).expect_value_eq("Ecuado")
}

#[test]
fn treats_dot_dot_as_path_not_range(playground: Playground) -> Result {
    playground.file(
        "nu_times.csv",
        indoc::indoc! {"
        name,rusty_luck,origin
        Jason,1,Canada
    "},
    )?;

    let code = "
        mkdir temp
        cd temp
        let name = (open ../nu_times.csv).name.0
        cd ..
        rm temp
        $name
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("Jason")
}

#[test]
#[deps(TESTBIN_COCOCO)]
fn subexpression_properly_redirects() -> Result {
    test()
        .run(r#"echo (cococo "hello") | str join"#)
        .expect_value_eq("hello")
}

#[test]
fn argument_subexpression() -> Result {
    test()
        .run(r#"["foo"] | each { |it| echo (echo $it) } | get 0"#)
        .expect_value_eq("foo")
}

#[test]
fn for_loop() -> Result {
    test()
        .run("mut out = ''; for i in 1..3 { $out = $out + ($i | into string) }; $out")
        .expect_value_eq("123")
}

#[test]
#[deps(TESTBIN_CHOP)]
fn subexpression_handles_dot(playground: Playground) -> Result {
    playground.file(
        "nu_times.csv",
        indoc::indoc! {"
        name,rusty_luck,origin
        Jason,1,Canada
        JT,1,New Zealand
        Andres,1,Ecuador
        AndKitKatz,1,Estados Unidos
    "},
    )?;

    let code = "
        echo (open nu_times.csv)
        | get name
        | each { |it| chop $it }
        | get 3
    ";

    test()
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("AndKitKat")
}

#[test]
fn string_interpolation_with_it() -> Result {
    test()
        .run(r#"["foo"] | each { |it| echo $"($it)" } | get 0"#)
        .expect_value_eq("foo")
}

#[test]
fn string_interpolation_with_it_column_path() -> Result {
    test()
        .run(r#"echo [[name]; [sammie]] | each { |it| echo $"($it.name)" } | get 0"#)
        .expect_value_eq("sammie")
}

#[test]
fn string_interpolation_shorthand_overlap() -> Result {
    test()
        .run(r#"$"3 + 4 = (3 + 4)""#)
        .expect_value_eq("3 + 4 = 7")
}

#[test]
fn string_interpolation_and_paren() -> Result {
    test()
        .run(r#"$"a paren is ('(')""#)
        .expect_value_eq("a paren is (")
}

#[test]
fn string_interpolation_with_unicode() -> Result {
    // カ = U+30AB : KATAKANA LETTER KA
    test().run(r#"$"カ""#).expect_value_eq("カ")
}

#[test]
fn run_custom_command() -> Result {
    test()
        .run("def add-me [x y] { $x + $y}; add-me 10 5")
        .expect_value_eq(15)
}

#[test]
fn run_custom_command_with_flag() -> Result {
    test()
        .run(r#"def foo [--bar:number] { if ($bar | is-empty) { echo "empty" } else { echo $bar } }; foo --bar 10"#)
        .expect_value_eq(10)
}

#[test]
fn run_custom_command_with_flag_missing() -> Result {
    test()
        .run(r#"def foo [--bar:number] { if ($bar | is-empty) { echo "empty" } else { echo $bar } }; foo"#)
        .expect_value_eq("empty")
}

#[test]
fn run_custom_subcommand() -> Result {
    test()
        .run(r#"def "str double" [x] { echo $x $x | str join }; str double bob"#)
        .expect_value_eq("bobbob")
}

#[test]
fn run_inner_custom_command() -> Result {
    test()
        .run("def outer [x] { def inner [y] { echo $y }; inner $x }; outer 10")
        .expect_value_eq(10)
}

#[test]
fn run_broken_inner_custom_command() -> Result {
    test()
        .run("def outer [x] { def inner [y] { echo $y }; inner $x }; inner 10")
        .expect_error()?;
    Ok(())
}

#[test]
fn run_custom_command_with_rest() -> Result {
    test()
        .run(r#"def rest-me [...rest: string] { echo $rest.1 $rest.0}; rest-me "hello" "world""#)
        .expect_value_eq(["world", "hello"])
}

#[test]
fn run_custom_command_with_rest_and_arg() -> Result {
    test()
        .run(r#"def rest-me-with-arg [name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-arg "hello" "world" "yay""#)
        .expect_value_eq(["yay", "world", "hello"])
}

#[test]
fn run_custom_command_with_rest_and_flag() -> Result {
    test()
        .run(r#"def rest-me-with-flag [--name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-flag "hello" "world" --name "yay""#)
        .expect_value_eq(["world", "hello", "yay"])
}

#[test]
fn run_custom_command_with_empty_rest() -> Result {
    test()
        .run("def rest-me-with-empty-rest [...rest: string] { $rest }; rest-me-with-empty-rest | is-empty")
        .expect_value_eq(true)
}

#[test]
fn run_custom_command_with_rest_other_name() -> Result {
    let code = r#"
        def say-hello [
            greeting:string,
            ...names:string # All of the names
        ] {
            echo $"($greeting), ($names | sort | str join)"
        }
        say-hello Salutations E D C A B
    "#;

    test().run(code).expect_value_eq("Salutations, ABCDE")
}

#[test]
fn alias_a_load_env() -> Result {
    let code = "
        def activate-helper [] { {BOB: SAM} }
        alias activate = load-env (activate-helper)
        activate
        $env.BOB
    ";

    test().run(code).expect_value_eq("SAM")
}

#[test]
fn let_variable() -> Result {
    test()
        .run("let x = 5; let y = 12; $x + $y")
        .expect_value_eq(17)
}

#[test]
fn let_doesnt_leak() -> Result {
    test()
        .run("do { let x = 5 }; echo $x")
        .expect_error_code_eq("nu::parser::variable_not_found")
}

#[test]
fn mutate_env_variable() -> Result {
    test()
        .run(r#"$env.TESTENVVAR = "hello world"; $env.TESTENVVAR"#)
        .expect_value_eq("hello world")
}

#[test]
#[deps(NU)]
fn mutate_env_hides_variable() -> Result {
    let code = r#"
        $env.TEST_ENV_VAR = "hello world"
        print $env.TEST_ENV_VAR
        hide-env TEST_ENV_VAR
        print $env.TEST_ENV_VAR
    "#;

    let result: CompleteResult = test().run_with_data(NU_RUNNER, code)?;
    assert_eq!(result.stdout, "hello world\n");
    assert_contains("Cannot find column 'TEST_ENV_VAR'", result.stderr);
    Ok(())
}

#[test]
#[deps(NU)]
fn mutate_env_hides_variable_in_parent_scope() -> Result {
    let code = r#"
        $env.TEST_ENV_VAR = "hello world"
        print $env.TEST_ENV_VAR
        do {
            hide-env TEST_ENV_VAR
            print $env.TEST_ENV_VAR
        }
        print $env.TEST_ENV_VAR
    "#;

    let result: CompleteResult = test().run_with_data(NU_RUNNER, code)?;
    assert_eq!(result.stdout, "hello world\n");
    assert_contains("Cannot find column 'TEST_ENV_VAR", result.stderr);
    Ok(())
}

#[test]
fn unlet_env_variable() -> Result {
    let err = test()
        .run(r#"$env.TEST_VAR = "hello world"; hide-env TEST_VAR; $env.TEST_VAR"#)
        .expect_error()?;
    assert_matches!(err, ShellError::CantFindColumn {col_name, ..} if col_name == "TEST_VAR");
    Ok(())
}

#[test]
fn unlet_nonexistent_variable() -> Result {
    test()
        .run("hide-env NONEXISTENT_VARIABLE")
        .expect_error_code_eq("nu::shell::env_variable_not_found")
}

#[test]
fn unlet_variable_in_parent_scope() -> Result {
    let code = r#"
        $env.DEBUG = "1"
        let inner = (do {
            $env.DEBUG = "2"
            hide-env DEBUG
            $env.DEBUG
        })
        [$env.DEBUG, "2", $inner, $env.DEBUG] | str join
    "#;

    test().run(code).expect_value_eq("1211")
}

#[test]
fn mutate_env_doesnt_leak() -> Result {
    let err = test()
        .run(r#"do { $env.xyz = "my message" }; $env.xyz"#)
        .expect_error()?;
    assert_matches!(err, ShellError::CantFindColumn { col_name, .. } if col_name == "xyz");
    Ok(())
}

#[test]
fn proper_shadow_mutate_env_aliases() -> Result {
    let code = r#"
        $env.DEBUG = "true"
        let inner = (do { $env.DEBUG = "false"; $env.DEBUG })
        [$env.DEBUG, $inner, $env.DEBUG] | str join
    "#;

    test().run(code).expect_value_eq("truefalsetrue")
}

#[test]
fn load_env_variable() -> Result {
    test()
        .run(r#"echo {TESTENVVAR: "hello world"} | load-env; $env.TESTENVVAR"#)
        .expect_value_eq("hello world")
}

#[test]
fn load_env_variable_arg() -> Result {
    test()
        .run(r#"load-env {TESTENVVAR: "hello world"}; $env.TESTENVVAR"#)
        .expect_value_eq("hello world")
}

#[test]
fn load_env_doesnt_leak() -> Result {
    let err = test()
        .run(r#"do { echo { name: xyz, value: "my message" } | load-env }; $env.xyz"#)
        .expect_error()?;
    assert_matches!(err, ShellError::CantFindColumn { col_name, .. } if col_name == "xyz");
    Ok(())
}

#[test]
fn proper_shadow_load_env_aliases() -> Result {
    let code = r#"
        $env.DEBUG = "true"
        let inner = (do { echo {DEBUG: "false"} | load-env; $env.DEBUG })
        [$env.DEBUG, $inner, $env.DEBUG] | str join
    "#;

    test().run(code).expect_value_eq("truefalsetrue")
}

//FIXME: jt: load-env can not currently hide variables because null no longer hides
#[ignore]
#[test]
#[deps(NU)]
fn load_env_can_hide_var_envs() -> Result {
    let runner = "let commands = $in; nu -n -c $commands | complete";
    let code = r#"
        $env.DEBUG = "1"
        echo $env.DEBUG
        load-env [[name, value]; [DEBUG null]]
        echo $env.DEBUG
    "#;

    let result: CompleteResult = test().run_with_data(runner, code)?;
    assert_eq!(result.stdout, "1");
    assert_contains("error", &result.stderr);
    assert_contains("Unknown column", result.stderr);
    Ok(())
}

//FIXME: jt: load-env can not currently hide variables because null no longer hides
#[ignore]
#[test]
#[deps(NU)]
fn load_env_can_hide_var_envs_in_parent_scope() -> Result {
    let code = r#"
        $env.DEBUG = "1"
        echo $env.DEBUG
        do {
            load-env [[name, value]; [DEBUG null]]
            echo $env.DEBUG
        }
        echo $env.DEBUG
    "#;

    let result: CompleteResult = test().run_with_data(NU_RUNNER, code)?;
    assert_eq!(result.stdout, "11");
    assert_contains("error", &result.stderr);
    assert_contains("Unknown column", result.stderr);
    Ok(())
}

#[test]
fn proper_shadow_let_aliases() -> Result {
    let code = "
        let DEBUG = false
        let inner = (do { let DEBUG = true; $DEBUG })
        [$DEBUG, $inner, $DEBUG] | str join
    ";

    test().run(code).expect_value_eq("falsetruefalse")
}

#[test]
fn block_params_override() -> Result {
    test()
        .run("[1, 2, 3] | each { |a| echo $it }")
        .expect_error_code_eq("nu::parser::variable_not_found")
}

#[test]
fn alias_reuse() -> Result {
    test()
        .run("alias foo = echo bob; foo; foo")
        .expect_value_eq("bob")
}

#[test]
fn block_params_override_correct() -> Result {
    test()
        .run("[1, 2, 3] | each { |a| echo $a }")
        .expect_value_eq([1, 2, 3])
}

#[test]
fn hex_number() -> Result {
    test().run("0x10").expect_value_eq(16)
}

#[test]
fn binary_number() -> Result {
    test().run("0b10").expect_value_eq(2)
}

#[test]
fn octal_number() -> Result {
    test().run("0o10").expect_value_eq(8)
}

#[test]
fn run_dynamic_closures() -> Result {
    test()
        .run(r#"let closure = {|| echo "holaaaa" }; do $closure"#)
        .expect_value_eq("holaaaa")
}

#[test]
fn dynamic_closure_type_check() -> Result {
    let err = test()
        .run(r#"let closure = {|x: int| echo $x}; do $closure "aa""#)
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "int" && from_type == "string"
    );
    Ok(())
}

#[test]
fn dynamic_closure_optional_arg() -> Result {
    test()
        .run("let closure = {|x: int = 3| echo $x}; do $closure")
        .expect_value_eq(3)?;
    test()
        .run("let closure = {|x: int = 3| echo $x}; do $closure 10")
        .expect_value_eq(10)
}

#[test]
fn dynamic_closure_rest_args() -> Result {
    test()
        .run(r#"let closure = {|...args| $args | str join ""}; do $closure 1 2 3"#)
        .expect_value_eq("123")?;
    test()
        .run(r#"let closure = {|required, ...args| $"($required), ($args | str join "")"}; do $closure 1 2 3"#)
        .expect_value_eq("1, 23")?;
    test()
        .run(r#"let closure = {|required, optional?, ...args| $"($required), ($optional), ($args | str join "")"}; do $closure 1 2 3"#)
        .expect_value_eq("1, 2, 3")
}

#[test]
fn argument_subexpression_reports_errors() -> Result {
    test().run("echo (ferris_is_not_here.exe)").expect_error()?;
    Ok(())
}

#[test]
#[deps(TESTBIN_CHOP)]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() -> Result {
    test()
        .run(r#""nushelll" | chop"#)
        .expect_value_eq("nushell")
}

#[test]
fn bad_operator() -> Result {
    let err = test().run("2 $ 2").expect_parse_error()?;
    assert_matches!(err, ParseError::Expected("operator", ..));
    Ok(())
}

#[test]
fn index_out_of_bounds() -> Result {
    test()
        .run("let foo = [1, 2, 3]; echo $foo.5")
        .expect_error_code_eq("nu::shell::access_beyond_end")
}

#[test]
fn negative_float_start() -> Result {
    test().run("-1.3 + 4").expect_value_eq(2.7)
}

#[test]
fn string_inside_of() -> Result {
    test().run(r#""bob" in "bobby""#).expect_value_eq(true)
}

#[test]
fn string_not_inside_of() -> Result {
    test().run(r#""bob" not-in "bobby""#).expect_value_eq(false)
}

#[test]
fn index_row() -> Result {
    test()
        .run("let foo = [[name]; [joe] [bob]]; echo $foo.1")
        .expect_value_eq(test_record! { "name" => "bob" })
}

#[test]
fn index_cell() -> Result {
    test()
        .run("let foo = [[name]; [joe] [bob]]; echo $foo.name.1")
        .expect_value_eq("bob")
}

#[test]
fn index_cell_alt() -> Result {
    test()
        .run("let foo = [[name]; [joe] [bob]]; echo $foo.1.name")
        .expect_value_eq("bob")
}

#[test]
fn not_echoing_ranges_without_numbers() -> Result {
    test().run("echo ..").expect_value_eq("..")
}

#[test]
fn not_echoing_exclusive_ranges_without_numbers() -> Result {
    test().run("echo ..<").expect_value_eq("..<")
}

#[test]
fn echoing_ranges() -> Result {
    test().run("echo 1..3 | math sum").expect_value_eq(6)
}

#[test]
fn echoing_exclusive_ranges() -> Result {
    test().run("echo 1..<4 | math sum").expect_value_eq(6)
}

#[test]
fn table_literals1() -> Result {
    test()
        .run("echo [[name age]; [foo 13]] | get age.0")
        .expect_value_eq(13)
}

#[test]
fn table_literals2() -> Result {
    test()
        .run("echo [[name age] ; [bob 13] [sally 20]] | get age | math sum")
        .expect_value_eq(33)
}

#[test]
fn list_with_commas() -> Result {
    test().run("echo [1, 2, 3] | math sum").expect_value_eq(6)
}

#[test]
fn range_with_left_var() -> Result {
    test()
        .run("({ size: 3}.size)..10 | math sum")
        .expect_value_eq(52)
}

#[test]
fn range_with_right_var() -> Result {
    test()
        .run("4..({ size: 30}.size) | math sum")
        .expect_value_eq(459)
}

#[test]
fn range_with_open_left() -> Result {
    test().run("echo ..30 | math sum").expect_value_eq(465)
}

#[test]
fn exclusive_range_with_open_left() -> Result {
    test().run("echo ..<31 | math sum").expect_value_eq(465)
}

#[test]
fn range_with_open_right() -> Result {
    test()
        .run("echo 5.. | first 10 | math sum")
        .expect_value_eq(95)
}

#[test]
fn exclusive_range_with_open_right() -> Result {
    test()
        .run("echo 5..< | first 10 | math sum")
        .expect_value_eq(95)
}

#[test]
fn range_with_mixed_types() -> Result {
    test().run("echo 1..10.5 | math sum").expect_value_eq(55.0)
}

#[rstest]
#[case::int_mul_filesize("100 * 10kb", 1_000_000)]
#[case::filesize_div_int("100kB / 10", 10_000)]
#[case::filesize_mul_int("100kB * 5", 500_000)]
fn filesize_math(#[case] code: &str, #[case] filesize: u32) -> Result {
    test()
        .run(code)
        .expect_value_eq(Value::test_filesize(filesize))
}

#[test]
fn cannot_divide_by_filesize() -> Result {
    test()
        .run("100 / 10kB")
        .expect_error_code_eq("nu::parser::operator_incompatible_types")
}

#[test]
fn exclusive_range_with_mixed_types() -> Result {
    test().run("echo 1..<10.5 | math sum").expect_value_eq(55.0)
}

#[test]
fn table_with_commas() -> Result {
    test()
        .run("echo [[name, age, height]; [JT, 42, 185] [Unknown, 99, 99]] | get age | math sum")
        .expect_value_eq(141)
}

#[test]
fn duration_overflow() -> Result {
    let err = test()
        .run("ls | get modified | each { |it| $it + 10000000000000000day }")
        .expect_compile_error()?;
    assert_matches!(err, CompileError::InvalidLiteral { msg, .. } if msg == "duration too large");
    Ok(())
}

#[test]
fn date_and_duration_overflow() -> Result {
    let err = test()
        .run("ls | get modified | each { |it| $it + 1000000000day }")
        .expect_compile_error()?;
    assert_matches!(err, CompileError::InvalidLiteral { msg, .. } if msg == "duration too large");
    Ok(())
}

#[test]
fn pipeline_params_simple() -> Result {
    test().run("echo 1 2 3 | $in.1 * $in.2").expect_value_eq(6)
}

#[test]
fn pipeline_params_inner() -> Result {
    test()
        .run("echo 1 2 3 | (echo $in.2 6 7 | $in.0 * $in.1 * $in.2)")
        .expect_value_eq(126)
}

#[test]
fn better_table_lex() -> Result {
    let code = "
        let table = [
            [name, size];
            [small, 7]
            [medium, 10]
            [large, 12]
        ]
        $table.1.size
    ";

    test().run(code).expect_value_eq(10)
}

#[test]
fn better_subexpr_lex() -> Result {
    test()
        .run("(echo boo sam | str length | math sum)")
        .expect_value_eq(6)
}

#[test]
fn subsubcommand() -> Result {
    test()
        .run(r#"def "aws s3 rb" [url] { $url + " loaded" }; aws s3 rb localhost"#)
        .expect_value_eq("localhost loaded")
}

#[test]
fn manysubcommand() -> Result {
    test()
        .run(r#"def "aws s3 rb ax vf qqqq rrrr" [url] { $url + " loaded" }; aws s3 rb ax vf qqqq rrrr localhost"#)
        .expect_value_eq("localhost loaded")
}

#[test]
fn nothing_string_1() -> Result {
    test().run(r#"null == "foo""#).expect_value_eq(false)
}

#[test]
fn hide_alias_shadowing() -> Result {
    let code = "
        def test-shadowing [] {
            alias greet = echo hello
            let xyz = {|| greet }
            hide greet
            do $xyz
        }
        test-shadowing
    ";

    test().run(code).expect_value_eq("hello")
}

// FIXME: Seems like subexpression are no longer scoped. Should we remove this test?
#[ignore]
#[test]
fn hide_alias_does_not_escape_scope() -> Result {
    let code = "
        def test-alias [] {
            alias greet = echo hello
            (hide greet)
            greet
        }
        test-alias
    ";

    test().run(code).expect_value_eq("hello")
}

#[test]
fn hide_alias_hides_alias() -> Result {
    let code = "
        def test-alias [] {
            alias ll = ls -l
            hide ll
            ll
        }
        test-alias
    ";

    let err = test().run(code).expect_shell_error()?;
    assert_matches!(err, ShellError::ExternalCommand { label, .. } if label == "Command `ll` not found");
    Ok(())
}

mod parse {
    use nu_test_support::prelude::*;
    use pretty_assertions::assert_matches;

    /*
        The debug command's signature is:

        Usage:
        > debug {flags}

        flags:
        -h, --help: Display the help message for this command
        -r, --raw: Prints the raw value representation.
    */

    #[test]
    fn errors_if_flag_passed_is_not_exact() -> Result {
        let err = test().run("debug -ra").expect_parse_error()?;
        assert_matches!(err, ParseError::UnknownFlag(name, flag, ..) if name == "debug" && flag == "-a");

        let err = test().run("debug --rawx").expect_parse_error()?;
        assert_matches!(err, ParseError::UnknownFlag(name, flag, ..) if name == "debug" && flag == "rawx");

        Ok(())
    }

    #[test]
    fn errors_if_flag_is_not_supported() -> Result {
        let err = test().run("debug --ferris").expect_parse_error()?;
        assert_matches!(err, ParseError::UnknownFlag(name, flag, ..) if name == "debug" && flag == "ferris");
        Ok(())
    }

    #[test]
    fn errors_if_passed_an_unexpected_argument() -> Result {
        let err = test().run("debug ferris").expect_parse_error()?;
        assert_matches!(err, ParseError::ExtraPositional(..));
        Ok(())
    }

    #[test]
    fn ensure_backticks_are_bareword_command() -> Result {
        test()
            .run("`8abc123`")
            .expect_error_code_eq("nu::shell::external_command")
    }
}

mod tilde_expansion {
    use nu_test_support::prelude::*;

    #[test]
    #[should_panic]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() -> Result {
        let actual: String = test().run("echo ~")?;
        assert_contains_not("~", actual);
        Ok(())
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() -> Result {
        test().run(r#"echo "1~1""#).expect_value_eq("1~1")
    }
}

mod variable_scoping {
    use nu_test_support::prelude::*;
    use rstest::rstest;

    #[rustfmt::skip]
    #[rstest]
    #[case("[0 1 2] | do { do { $input } }", "ZZZ")]
    #[case(r#"[0 1 2] | do { do { if $input == "ZZZ" { $input } else { $input } } }"#, "ZZZ")]
    #[case(r#"[0 1 2] | do { do { if $input == "ZZZ" { $input } else { $input } } }"#, "ZZZ")]
    #[case("[0 1 2] | do { $input }", "ZZZ")]
    #[case("[0 1 2] | do { if $input == $input { $input } else { $input } }", "ZZZ")]
    #[case("[0 1 2] | each { |_| $input }", ["ZZZ", "ZZZ", "ZZZ"])]
    #[case("[0 1 2] | each { |it| if $it > 0 { $input } else { $input } }", ["ZZZ", "ZZZ", "ZZZ"])]
    #[case("[0 1 2] | each { |_| if $input == $input { $input } else { $input } }", ["ZZZ", "ZZZ", "ZZZ"])]
    fn test_variable_scope(#[case] body: &str, #[case] expected: impl IntoValue) -> Result {
        let code = format!("def test [input] {{ {body} }}; test ZZZ");
        test().run(code).expect_value_eq(expected)
    }
}

#[test]
#[deps(NU)]
fn pipe_input_to_print() -> Result {
    let result: CompleteResult = test().run_with_data(NU_RUNNER, r#""foo" | print | describe"#)?;
    // "foo" got printed, but the output type is "nothing"
    assert_eq!(result.stdout, "foo\nnothing\n");
    Ok(())
}

#[rstest]
#[case::err("e>")]
#[case::out_err("o+e>")]
fn err_pipe_input_to_print(#[case] redirect: &str) -> Result {
    let err = test()
        .run(format!(r#""foo" {redirect}| print"#))
        .expect_shell_error()?;
    assert_eq!(
        "piping stderr only works on external commands",
        err.generic_msg()?
    );
    Ok(())
}

#[test]
fn command_not_found_error_shows_not_found_2() -> Result {
    let err = test()
        .run("export def --wrapped my-foo [...rest] { foo }; my-foo")
        .expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::ExternalCommand {label, help, ..}
            if label == "Command `foo` not found" && help == "Did you mean `for`?"
    );
    Ok(())
}

#[test]
fn error_on_out_greater_pipe() -> Result {
    let err = test().run(r#""foo" o>| print"#).expect_parse_error()?;
    assert_matches!(
        err,
        ParseError::Expected(msg, ..)
            if msg == "`|`.  Redirecting stdout to a pipe is the same as normal piping."
    );
    Ok(())
}

#[test]
#[deps(TESTBIN_FAIL)]
fn error_with_backtrace() -> Result {
    let code = "
        def a [x] {
            if $x == 3 {
                error make {
                    msg: 'a custom error'
                }
            }
        }

        a 3
    ";

    let err = test()
        .env("NU_BACKTRACE", 1)
        .run(code)
        .expect_shell_error()?
        .into_inner()? // we go 2 layers deep
        .into_inner()?
        .into_labeled()?;
    assert_eq!(err.msg, "a custom error");

    // without NU_BACKTRACE, the labeled error is there immediately
    let err = test().run(code).expect_shell_error()?.into_labeled()?;
    assert_eq!(err.msg, "a custom error");

    let err = test()
        .env("NU_BACKTRACE", 1)
        .run("fail")
        .expect_shell_error()?;
    // calling externals does not chain errors
    assert_matches!(err, ShellError::NonZeroExitCode { .. });

    Ok(())
}

#[test]
fn liststream_error_with_backtrace_custom() -> Result {
    let code = "
        def a [x] {
            if $x == 3 {
                [1] | each {
                    error make {
                        'msg': 'a custom error'
                    }
                }
            }
        }
        
        a 3
    ";

    let err = test()
        .env("NU_BACKTRACE", 1)
        .run(code)
        .expect_shell_error()?;
    let [err] = match err {
        ShellError::EvalBlockWithInput { sources, .. } => sources.try_into().unwrap(),
        _ => return Err(err.into()),
    };

    // is this enough to test that backtracing is on?
    assert_eq!(err.into_inner()?.into_labeled()?.msg, "a custom error");
    Ok(())
}

#[test]
fn liststream_error_with_backtrace_function() -> Result {
    let code = "
        def a [x] {
            if $x == 3 {
                [1] | each {
                    error make {
                        msg: 'a custom error'
                    }
                }
            }
        }
        
        def b [] {
            a 1
            a 3
            a 2
        }
        
        b
    ";

    let [err, _] = test()
        .env("NU_BACKTRACE", 1)
        .run(code)
        .expect_shell_error()?
        .into_inner()?
        .into_chained_iter()?
        .collect_array()
        .unwrap();

    assert_matches!(err, ShellError::EvalBlockWithInput { .. });
    let err = err.into_inner()?;
    assert_matches!(err, ShellError::EvalBlockWithInput { .. });
    let err = err.into_inner()?;
    assert_matches!(err, ShellError::ChainedError(..));
    assert_eq!(err.into_inner()?.into_labeled()?.msg, "a custom error");
    Ok(())
}

#[test]
fn liststream_error_with_backtrace_single_stream() -> Result {
    let code = "[1] | each { error make {msg: 'a custom err'} }";
    let err = test()
        .env("NU_BACKTRACE", 1)
        .run(code)
        .expect_shell_error()?;

    assert_matches!(err, ShellError::EvalBlockWithInput { .. });
    let err = err.into_inner()?;
    assert_matches!(err, ShellError::ChainedError(..));
    assert_eq!(err.into_inner()?.into_labeled()?.msg, "a custom err");
    Ok(())
}
