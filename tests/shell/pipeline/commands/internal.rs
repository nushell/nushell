use nu_test_support::fs::Stub::{FileWithContent, FileWithContentToBeTrimmed};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use pretty_assertions::assert_eq;

#[test]
fn takes_rows_of_nu_value_strings_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(r###"
        [
            [name rusty_luck origin];
            [Jason 1 Canada]
            [JT 1 "New Zealand"]
            [Andrés 1 Ecuador]
            [AndKitKatz 1 "Estados Unidos"]
        ]
        | get origin
        | each {|it| nu --testbin cococo $it | nu --testbin chop}
        | get 2
    "###);

    // chop will remove the last escaped double quote from \"Estados Unidos\"
    assert_eq!(actual.out, "Ecuado");
}

#[test]
fn treats_dot_dot_as_path_not_range() {
    Playground::setup("dot_dot_dir", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "nu_times.csv",
            "
                name,rusty_luck,origin
                Jason,1,Canada
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), "
            mkdir temp;
            cd temp;
            print (open ../nu_times.csv).name.0 | table;
            cd ..;
            rmdir temp
            ");

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert_eq!(actual.out, "Jason");
    })
}

#[test]
fn subexpression_properly_redirects() {
    let actual = nu!(r#"
            echo (nu --testbin cococo "hello") | str join
        "#);

    assert_eq!(actual.out, "hello");
}

#[test]
fn argument_subexpression() {
    let actual = nu!(r#"
            echo "foo" | each { |it| echo (echo $it) }
        "#);

    assert_eq!(actual.out, "foo");
}

#[test]
fn for_loop() {
    let actual = nu!("
            for i in 1..3 { print $i }
        ");

    assert_eq!(actual.out, "123");
}

#[test]
fn subexpression_handles_dot() {
    Playground::setup("subexpression_handles_dot", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContentToBeTrimmed(
            "nu_times.csv",
            "
                name,rusty_luck,origin
                Jason,1,Canada
                JT,1,New Zealand
                Andrés,1,Ecuador
                AndKitKatz,1,Estados Unidos
            ",
        )]);

        let actual = nu!(cwd: dirs.test(), "
            echo (open nu_times.csv)
            | get name
            | each { |it| nu --testbin chop $it }
            | get 3
            ");

        assert_eq!(actual.out, "AndKitKat");
    });
}

#[test]
fn string_interpolation_with_it() {
    let actual = nu!(r#"
                    echo "foo" | each { |it| echo $"($it)" }
            "#);

    assert_eq!(actual.out, "foo");
}

#[test]
fn string_interpolation_with_it_column_path() {
    let actual = nu!(r#"
                    echo [[name]; [sammie]] | each { |it| echo $"($it.name)" } | get 0
        "#);

    assert_eq!(actual.out, "sammie");
}

#[test]
fn string_interpolation_shorthand_overlap() {
    let actual = nu!(r#"
                    $"3 + 4 = (3 + 4)"
        "#);

    assert_eq!(actual.out, "3 + 4 = 7");
}

// FIXME: jt - we don't currently have a way to escape the single ticks easily
#[ignore]
#[test]
fn string_interpolation_and_paren() {
    let actual = nu!(r#"
                    $"a paren is ('(')"
        "#);

    assert_eq!(actual.out, "a paren is (");
}

#[test]
fn string_interpolation_with_unicode() {
    //カ = U+30AB : KATAKANA LETTER KA
    let actual = nu!(r#"
            $"カ"
        "#);

    assert_eq!(actual.out, "カ");
}

#[test]
fn run_custom_command() {
    let actual = nu!("
            def add-me [x y] { $x + $y}; add-me 10 5
        ");

    assert_eq!(actual.out, "15");
}

#[test]
fn run_custom_command_with_flag() {
    let actual = nu!(r#"
        def foo [--bar:number] { if ($bar | is-empty) { echo "empty" } else { echo $bar } }; foo --bar 10
        "#);

    assert_eq!(actual.out, "10");
}

#[test]
fn run_custom_command_with_flag_missing() {
    let actual = nu!(r#"
        def foo [--bar:number] { if ($bar | is-empty) { echo "empty" } else { echo $bar } }; foo
        "#);

    assert_eq!(actual.out, "empty");
}

#[test]
fn run_custom_subcommand() {
    let actual = nu!(r#"
        def "str double" [x] { echo $x $x | str join }; str double bob
        "#);

    assert_eq!(actual.out, "bobbob");
}

#[test]
fn run_inner_custom_command() {
    let actual = nu!("
          def outer [x] { def inner [y] { echo $y }; inner $x }; outer 10
        ");

    assert_eq!(actual.out, "10");
}

#[test]
fn run_broken_inner_custom_command() {
    let actual = nu!("
        def outer [x] { def inner [y] { echo $y }; inner $x }; inner 10
        ");

    assert!(!actual.err.is_empty());
}

#[test]
fn run_custom_command_with_rest() {
    let actual = nu!(r#"
            def rest-me [...rest: string] { echo $rest.1 $rest.0}; rest-me "hello" "world" | to json --raw
        "#);

    assert_eq!(actual.out, r#"["world","hello"]"#);
}

#[test]
fn run_custom_command_with_rest_and_arg() {
    let actual = nu!(r#"
            def rest-me-with-arg [name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-arg "hello" "world" "yay" | to json --raw
        "#);

    assert_eq!(actual.out, r#"["yay","world","hello"]"#);
}

#[test]
fn run_custom_command_with_rest_and_flag() {
    let actual = nu!(r#"
            def rest-me-with-flag [--name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-flag "hello" "world" --name "yay" | to json --raw
        "#);

    assert_eq!(actual.out, r#"["world","hello","yay"]"#);
}

#[test]
fn run_custom_command_with_empty_rest() {
    let actual = nu!("
            def rest-me-with-empty-rest [...rest: string] { $rest }; rest-me-with-empty-rest | is-empty
        ");

    assert_eq!(actual.out, "true");
    assert_eq!(actual.err, "");
}

//FIXME: jt: blocked on https://github.com/nushell/engine-q/issues/912
#[ignore]
#[test]
fn run_custom_command_with_rest_other_name() {
    let actual = nu!(r#"
            def say-hello [
                greeting:string,
                ...names:string # All of the names
                ] {
                    echo $"($greeting), ($names | sort-by | str join)"
                }
            say-hello Salutations E D C A B
        "#);

    assert_eq!(actual.out, "Salutations, ABCDE");
    assert_eq!(actual.err, "");
}

#[test]
fn alias_a_load_env() {
    let actual = nu!("
            def activate-helper [] { {BOB: SAM} }; alias activate = load-env (activate-helper); activate; $env.BOB
        ");

    assert_eq!(actual.out, "SAM");
}

#[test]
fn let_variable() {
    let actual = nu!("
            let x = 5
            let y = 12
            $x + $y
        ");

    assert_eq!(actual.out, "17");
}

#[test]
fn let_doesnt_leak() {
    let actual = nu!("
        do { let x = 5 }; echo $x
        ");

    assert!(actual.err.contains("variable not found"));
}

#[test]
fn mutate_env_variable() {
    let actual = nu!(r#"
            $env.TESTENVVAR = "hello world"
            echo $env.TESTENVVAR
        "#);

    assert_eq!(actual.out, "hello world");
}

#[test]
fn mutate_env_hides_variable() {
    let actual = nu!(r#"
            $env.TESTENVVAR = "hello world"
            print $env.TESTENVVAR
            hide-env TESTENVVAR
            print $env.TESTENVVAR
        "#);

    assert_eq!(actual.out, "hello world");
    assert!(actual.err.contains("not_found"));
}

#[test]
fn mutate_env_hides_variable_in_parent_scope() {
    let actual = nu!(r#"
            $env.TESTENVVAR = "hello world"
            print $env.TESTENVVAR
            do {
                hide-env TESTENVVAR
                print $env.TESTENVVAR
            }
            print $env.TESTENVVAR
        "#);

    assert_eq!(actual.out, "hello world");
    assert!(actual.err.contains("not_found"));
}

#[test]
fn unlet_env_variable() {
    let actual = nu!(r#"
            $env.TEST_VAR = "hello world"
            hide-env TEST_VAR
            echo $env.TEST_VAR
        "#);
    assert!(actual.err.contains("not_found"));
}

#[test]
#[ignore]
fn unlet_nonexistent_variable() {
    let actual = nu!("
            hide-env NONEXISTENT_VARIABLE
        ");

    assert!(actual.err.contains("did not find"));
}

#[test]
fn unlet_variable_in_parent_scope() {
    let actual = nu!(r#"
            $env.DEBUG = "1"
            print $env.DEBUG
            do {
                $env.DEBUG = "2"
                print $env.DEBUG
                hide-env DEBUG
                print $env.DEBUG
            }
            print $env.DEBUG
        "#);

    assert_eq!(actual.out, "1211");
}

#[test]
fn mutate_env_doesnt_leak() {
    let actual = nu!(r#"
        do { $env.xyz = "my message" }; echo $env.xyz
        "#);

    assert!(actual.err.contains("not_found"));
}

#[test]
fn proper_shadow_mutate_env_aliases() {
    let actual = nu!(r#"
        $env.DEBUG = "true"; print $env.DEBUG | table; do { $env.DEBUG = "false"; print $env.DEBUG } | table; print $env.DEBUG
        "#);
    assert_eq!(actual.out, "truefalsetrue");
}

#[test]
fn load_env_variable() {
    let actual = nu!(r#"
            echo {TESTENVVAR: "hello world"} | load-env
            echo $env.TESTENVVAR
        "#);

    assert_eq!(actual.out, "hello world");
}

#[test]
fn load_env_variable_arg() {
    let actual = nu!(r#"
            load-env {TESTENVVAR: "hello world"}
            echo $env.TESTENVVAR
        "#);

    assert_eq!(actual.out, "hello world");
}

#[test]
fn load_env_doesnt_leak() {
    let actual = nu!(r#"
        do { echo { name: xyz, value: "my message" } | load-env }; echo $env.xyz
        "#);

    assert!(actual.err.contains("not_found"));
}

#[test]
fn proper_shadow_load_env_aliases() {
    let actual = nu!(r#"
        $env.DEBUG = "true"; print $env.DEBUG | table; do { echo {DEBUG: "false"} | load-env; print $env.DEBUG } | table; print $env.DEBUG
        "#);
    assert_eq!(actual.out, "truefalsetrue");
}

//FIXME: jt: load-env can not currently hide variables because null no longer hides
#[ignore]
#[test]
fn load_env_can_hide_var_envs() {
    let actual = nu!(r#"
        $env.DEBUG = "1"
        echo $env.DEBUG
        load-env [[name, value]; [DEBUG null]]
        echo $env.DEBUG
        "#);
    assert_eq!(actual.out, "1");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
}

//FIXME: jt: load-env can not currently hide variables because null no longer hides
#[ignore]
#[test]
fn load_env_can_hide_var_envs_in_parent_scope() {
    let actual = nu!(r#"
        $env.DEBUG = "1"
        echo $env.DEBUG
        do {
            load-env [[name, value]; [DEBUG null]]
            echo $env.DEBUG
        }
        echo $env.DEBUG
        "#);
    assert_eq!(actual.out, "11");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
}

#[test]
fn proper_shadow_let_aliases() {
    let actual = nu!("
        let DEBUG = false; print $DEBUG | table; do { let DEBUG = true; print $DEBUG } | table; print $DEBUG
        ");
    assert_eq!(actual.out, "falsetruefalse");
}

#[test]
fn block_params_override() {
    let actual = nu!("
        [1, 2, 3] | each { |a| echo $it }
        ");
    assert!(actual.err.contains("variable not found"));
}

#[test]
fn alias_reuse() {
    let actual = nu!("alias foo = echo bob; foo; foo");

    assert!(actual.out.contains("bob"));
    assert!(actual.err.is_empty());
}

#[test]
fn block_params_override_correct() {
    let actual = nu!("
        [1, 2, 3] | each { |a| echo $a } | to json --raw
        ");
    assert_eq!(actual.out, "[1,2,3]");
}

#[test]
fn hex_number() {
    let actual = nu!("
        0x10
        ");
    assert_eq!(actual.out, "16");
}

#[test]
fn binary_number() {
    let actual = nu!("
        0b10
        ");
    assert_eq!(actual.out, "2");
}

#[test]
fn octal_number() {
    let actual = nu!("
        0o10
        ");
    assert_eq!(actual.out, "8");
}

#[test]
fn run_dynamic_closures() {
    let actual = nu!(r#"
        let closure = {|| echo "holaaaa" }; do $closure
        "#);
    assert_eq!(actual.out, "holaaaa");
}

#[test]
fn dynamic_closure_type_check() {
    let actual = nu!(r#"let closure = {|x: int| echo $x}; do $closure "aa""#);
    assert!(actual.err.contains("can't convert string to int"))
}

#[test]
fn dynamic_closure_optional_arg() {
    let actual = nu!(r#"let closure = {|x: int = 3| echo $x}; do $closure"#);
    assert_eq!(actual.out, "3");
    let actual = nu!(r#"let closure = {|x: int = 3| echo $x}; do $closure 10"#);
    assert_eq!(actual.out, "10");
}

#[test]
fn dynamic_closure_rest_args() {
    let actual = nu!(r#"let closure = {|...args| $args | str join ""}; do $closure 1 2 3"#);
    assert_eq!(actual.out, "123");

    let actual = nu!(
        r#"let closure = {|required, ...args| $"($required), ($args | str join "")"}; do $closure 1 2 3"#
    );
    assert_eq!(actual.out, "1, 23");
    let actual = nu!(
        r#"let closure = {|required, optional?, ...args| $"($required), ($optional), ($args | str join "")"}; do $closure 1 2 3"#
    );
    assert_eq!(actual.out, "1, 2, 3");
}

#[test]
fn argument_subexpression_reports_errors() {
    let actual = nu!("echo (ferris_is_not_here.exe)");

    assert!(!actual.err.is_empty());
}

#[test]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(r#""nushelll" | nu --testbin chop"#);

    assert_eq!(actual.out, "nushell");
}

#[test]
fn bad_operator() {
    let actual = nu!("
            2 $ 2
        ");

    assert!(actual.err.contains("operator"));
}

#[test]
fn index_out_of_bounds() {
    let actual = nu!("
            let foo = [1, 2, 3]; echo $foo.5
        ");

    assert!(actual.err.contains("too large"));
}

#[test]
fn negative_float_start() {
    let actual = nu!("
            -1.3 + 4
        ");

    assert_eq!(actual.out, "2.7");
}

#[test]
fn string_inside_of() {
    let actual = nu!(r#"
            "bob" in "bobby"
        "#);

    assert_eq!(actual.out, "true");
}

#[test]
fn string_not_inside_of() {
    let actual = nu!(r#"
            "bob" not-in "bobby"
        "#);

    assert_eq!(actual.out, "false");
}

#[test]
fn index_row() {
    let actual = nu!("
        let foo = [[name]; [joe] [bob]]; echo $foo.1 | to json --raw
        ");

    assert_eq!(actual.out, r#"{"name":"bob"}"#);
}

#[test]
fn index_cell() {
    let actual = nu!("
        let foo = [[name]; [joe] [bob]]; echo $foo.name.1
        ");

    assert_eq!(actual.out, "bob");
}

#[test]
fn index_cell_alt() {
    let actual = nu!("
        let foo = [[name]; [joe] [bob]]; echo $foo.1.name
        ");

    assert_eq!(actual.out, "bob");
}

#[test]
fn not_echoing_ranges_without_numbers() {
    let actual = nu!("
            echo ..
        ");

    assert_eq!(actual.out, "..");
}

#[test]
fn not_echoing_exclusive_ranges_without_numbers() {
    let actual = nu!("
            echo ..<
        ");

    assert_eq!(actual.out, "..<");
}

#[test]
fn echoing_ranges() {
    let actual = nu!("
            echo 1..3 | math sum
        ");

    assert_eq!(actual.out, "6");
}

#[test]
fn echoing_exclusive_ranges() {
    let actual = nu!("
            echo 1..<4 | math sum
        ");

    assert_eq!(actual.out, "6");
}

#[test]
fn table_literals1() {
    let actual = nu!("
            echo [[name age]; [foo 13]] | get age.0
        ");

    assert_eq!(actual.out, "13");
}

#[test]
fn table_literals2() {
    let actual = nu!("
        echo [[name age] ; [bob 13] [sally 20]] | get age | math sum
        ");

    assert_eq!(actual.out, "33");
}

#[test]
fn list_with_commas() {
    let actual = nu!("
        echo [1, 2, 3] | math sum
        ");

    assert_eq!(actual.out, "6");
}

#[test]
fn range_with_left_var() {
    let actual = nu!("
        ({ size: 3}.size)..10 | math sum
        ");

    assert_eq!(actual.out, "52");
}

#[test]
fn range_with_right_var() {
    let actual = nu!("
        4..({ size: 30}.size) | math sum
        ");

    assert_eq!(actual.out, "459");
}

#[test]
fn range_with_open_left() {
    let actual = nu!("
        echo ..30 | math sum
        ");

    assert_eq!(actual.out, "465");
}

#[test]
fn exclusive_range_with_open_left() {
    let actual = nu!("
        echo ..<31 | math sum
        ");

    assert_eq!(actual.out, "465");
}

#[test]
fn range_with_open_right() {
    let actual = nu!("
        echo 5.. | first 10 | math sum
        ");

    assert_eq!(actual.out, "95");
}

#[test]
fn exclusive_range_with_open_right() {
    let actual = nu!("
        echo 5..< | first 10 | math sum
        ");

    assert_eq!(actual.out, "95");
}

#[test]
fn range_with_mixed_types() {
    let actual = nu!("
        echo 1..10.5 | math sum
        ");

    assert_eq!(actual.out, "55.0");
}

#[test]
fn filesize_math() {
    let actual = nu!("100 * 10kB");
    assert_eq!(actual.out, "1.0 MB");
}

#[test]
fn filesize_math2() {
    let actual = nu!("100 / 10kB");
    assert!(
        actual
            .err
            .contains("nu::parser::operator_incompatible_types")
    );
}

#[test]
fn filesize_math3() {
    let actual = nu!("100kB / 10");
    assert_eq!(actual.out, "10.0 kB");
}

#[test]
fn filesize_math4() {
    let actual = nu!("100kB * 5");
    assert_eq!(actual.out, "500.0 kB");
}

#[test]
fn filesize_math5() {
    let actual = nu!("100 * 1kB");
    assert_eq!(actual.out, "100.0 kB");
}

#[test]
fn exclusive_range_with_mixed_types() {
    let actual = nu!("
        echo 1..<10.5 | math sum
        ");

    assert_eq!(actual.out, "55.0");
}

#[test]
fn table_with_commas() {
    let actual = nu!("
        echo [[name, age, height]; [JT, 42, 185] [Unknown, 99, 99]] | get age | math sum
        ");

    assert_eq!(actual.out, "141");
}

#[test]
fn duration_overflow() {
    let actual = nu!("
    ls | get modified | each { |it| $it + 10000000000000000day }
    ");

    assert!(actual.err.contains("duration too large"));
}

#[test]
fn date_and_duration_overflow() {
    let actual = nu!("
    ls | get modified | each { |it| $it + 1000000000day }
    ");

    // assert_eq!(actual.err, "overflow");
    assert!(actual.err.contains("duration too large"));
}

#[test]
fn pipeline_params_simple() {
    let actual = nu!("
    echo 1 2 3 | $in.1 * $in.2
    ");

    assert_eq!(actual.out, "6");
}

#[test]
fn pipeline_params_inner() {
    let actual = nu!("
    echo 1 2 3 | (echo $in.2 6 7 | $in.0 * $in.1 * $in.2)
    ");

    assert_eq!(actual.out, "126");
}

#[test]
fn better_table_lex() {
    let actual = nu!("
    let table = [
        [name, size];
        [small, 7]
        [medium, 10]
        [large, 12]
    ];
    $table.1.size
    ");

    assert_eq!(actual.out, "10");
}

#[test]
fn better_subexpr_lex() {
    let actual = nu!("
    (echo boo
    sam | str length | math sum)
    ");

    assert_eq!(actual.out, "6");
}

#[test]
fn subsubcommand() {
    let actual = nu!(r#"
    def "aws s3 rb" [url] { $url + " loaded" }; aws s3 rb localhost
    "#);

    assert_eq!(actual.out, "localhost loaded");
}

#[test]
fn manysubcommand() {
    let actual = nu!(r#"
    def "aws s3 rb ax vf qqqq rrrr" [url] { $url + " loaded" }; aws s3 rb ax vf qqqq rrrr localhost
    "#);

    assert_eq!(actual.out, "localhost loaded");
}

#[test]
fn nothing_string_1() {
    let actual = nu!(r#"
    null == "foo"
    "#);

    assert_eq!(actual.out, "false");
}

#[test]
fn hide_alias_shadowing() {
    let actual = nu!("
    def test-shadowing [] {
        alias greet = echo hello;
        let xyz = {|| greet };
        hide greet;
        do $xyz
    };
    test-shadowing
    ");
    assert_eq!(actual.out, "hello");
}

// FIXME: Seems like subexpression are no longer scoped. Should we remove this test?
#[ignore]
#[test]
fn hide_alias_does_not_escape_scope() {
    let actual = nu!("
    def test-alias [] {
        alias greet = echo hello;
        (hide greet);
        greet
    };
    test-alias
    ");
    assert_eq!(actual.out, "hello");
}

#[test]
fn hide_alias_hides_alias() {
    let actual = nu!("
    def test-alias [] {
        alias ll = ls -l;
        hide ll;
        ll
    };
    test-alias
    ");

    assert!(
        actual.err.contains("Command `ll` not found") && actual.err.contains("Did you mean `all`?")
    );
}

mod parse {
    use nu_test_support::nu;

    /*
        The debug command's signature is:

        Usage:
        > debug {flags}

        flags:
        -h, --help: Display the help message for this command
        -r, --raw: Prints the raw value representation.
    */

    #[test]
    fn errors_if_flag_passed_is_not_exact() {
        let actual = nu!("debug -ra");

        assert!(actual.err.contains("unknown flag"),);

        let actual = nu!("debug --rawx");

        assert!(actual.err.contains("unknown flag"),);
    }

    #[test]
    fn errors_if_flag_is_not_supported() {
        let actual = nu!("debug --ferris");

        assert!(actual.err.contains("unknown flag"),);
    }

    #[test]
    fn errors_if_passed_an_unexpected_argument() {
        let actual = nu!("debug ferris");

        assert!(actual.err.contains("extra positional argument"),);
    }

    #[test]
    fn ensure_backticks_are_bareword_command() {
        let actual = nu!("`8abc123`");

        assert!(actual.err.contains("Command `8abc123` not found"),);
    }
}

mod tilde_expansion {
    use nu_test_support::nu;

    #[test]
    #[should_panic]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!("
            echo ~
        ");

        assert!(!actual.out.contains('~'),);
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(r#"
                    echo "1~1"
                "#);

        assert_eq!(actual.out, "1~1");
    }
}

mod variable_scoping {
    use nu_test_support::nu;

    fn test_variable_scope(code: &str, expected: &str) {
        let actual = nu!(code);
        assert_eq!(actual.out, expected);
    }

    fn test_variable_scope_list(code: &str, expected: &[&str]) {
        let actual = nu!(code);
        let result: Vec<&str> = actual.out.matches("ZZZ").collect();
        assert_eq!(result, expected);
    }

    #[test]
    fn access_variables_in_scopes() {
        test_variable_scope(
            " def test [input] { echo [0 1 2] | do { do { echo $input } } }
                test ZZZ ",
            "ZZZ",
        );
        test_variable_scope(
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } else { echo $input } } } }
                test ZZZ "#,
            "ZZZ",
        );
        test_variable_scope(
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } else { echo $input } } } }
                test ZZZ "#,
            "ZZZ",
        );
        test_variable_scope(
            " def test [input] { echo [0 1 2] | do { echo $input } }
                test ZZZ ",
            "ZZZ",
        );
        test_variable_scope(
            " def test [input] { echo [0 1 2] | do { if $input == $input { echo $input } else { echo $input } } }
                test ZZZ ",
                "ZZZ"
        );
        test_variable_scope_list(
            " def test [input] { echo [0 1 2] | each { |_| echo $input } }
                test ZZZ ",
            &["ZZZ", "ZZZ", "ZZZ"],
        );
        test_variable_scope_list(
            " def test [input] { echo [0 1 2] | each { |it| if $it > 0 {echo $input} else {echo $input}} }
                test ZZZ ",
            &["ZZZ", "ZZZ", "ZZZ"],
        );
        test_variable_scope_list(
            " def test [input] { echo [0 1 2] | each { |_| if $input == $input {echo $input} else {echo $input}} }
                test ZZZ ",
            &["ZZZ", "ZZZ", "ZZZ"],
        );
    }
}

#[test]
fn pipe_input_to_print() {
    let actual = nu!(r#""foo" | print"#);
    assert_eq!(actual.out, "foo");
    assert!(actual.err.is_empty());
}

#[test]
fn err_pipe_input_to_print() {
    let actual = nu!(r#""foo" e>| print"#);
    assert!(actual.err.contains("only works on external commands"));
}

#[test]
fn outerr_pipe_input_to_print() {
    let actual = nu!(r#""foo" o+e>| print"#);
    assert!(actual.err.contains("only works on external commands"));
}

#[test]
fn command_not_found_error_shows_not_found_2() {
    let actual = nu!(r#"
            export def --wrapped my-foo [...rest] { foo };
            my-foo
        "#);
    assert!(
        actual.err.contains("Command `foo` not found")
            && actual.err.contains("Did you mean `for`?")
    );
}

#[test]
fn error_on_out_greater_pipe() {
    let actual = nu!(r#""foo" o>| print"#);
    assert!(
        actual
            .err
            .contains("Redirecting stdout to a pipe is the same as normal piping")
    )
}

#[test]
fn error_with_backtrace() {
    Playground::setup("error with backtrace", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("tmp_env.nu", "$env.NU_BACKTRACE = 1")]);

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"def a [x] { if $x == 3 { error make {msg: 'a custom error'}}};a 3"#);
        let chained_error_cnt: Vec<&str> = actual
            .err
            .matches("diagnostic code: chained_error")
            .collect();
        // run `a 3`, and it raises error, so there should be 1.
        assert_eq!(chained_error_cnt.len(), 1);
        assert!(actual.err.contains("a custom error"));

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"def a [x] { if $x == 3 { error make {msg: 'a custom error'}}};def b [] { a 1; a 3; a 2 };b"#);

        let chained_error_cnt: Vec<&str> = actual
            .err
            .matches("diagnostic code: chained_error")
            .collect();
        // run `b`, it runs `a 3`, and it raises error, so there should be 2.
        assert_eq!(chained_error_cnt.len(), 2);
        assert!(actual.err.contains("a custom error"));

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"error make {msg: 'a custom err'}"#);
        let chained_error_cnt: Vec<&str> = actual
            .err
            .matches("diagnostic code: chained_error")
            .collect();
        // run error make directly, show no backtrace is available
        assert_eq!(chained_error_cnt.len(), 0);
    });
}

#[test]
fn liststream_error_with_backtrace() {
    Playground::setup("liststream error with backtrace", |dirs, sandbox| {
        sandbox.with_files(&[FileWithContent("tmp_env.nu", "$env.NU_BACKTRACE = 1")]);

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"def a [x] { if $x == 3 { [1] | each {error make {'msg': 'a custom error'}}}};a 3"#);
        assert!(actual.err.contains("a custom error"));

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"def a [x] { if $x == 3 { [1] | each {error make {'msg': 'a custom error'}}}};def b [] { a 1; a 3; a 2 };b"#);
        let chained_error_cnt: Vec<&str> = actual
            .err
            .matches("diagnostic code: chained_error")
            .collect();
        assert_eq!(chained_error_cnt.len(), 1);
        assert!(actual.err.contains("a custom error"));
        let eval_with_input_cnt: Vec<&str> = actual.err.matches("eval_block_with_input").collect();
        assert_eq!(eval_with_input_cnt.len(), 2);

        let actual = nu!(
            env_config: "tmp_env.nu",
            cwd: dirs.test(),
            r#"[1] | each { error make {msg: 'a custom err'} }"#);
        let chained_error_cnt: Vec<&str> = actual
            .err
            .matches("diagnostic code: chained_error")
            .collect();
        // run error make directly, show no backtrace is available
        assert_eq!(chained_error_cnt.len(), 0);
    });
}
