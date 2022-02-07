<<<<<<< HEAD
#[cfg(feature = "which")]
use nu_test_support::fs::Stub::FileWithContent;
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;
use nu_test_support::nu;
use nu_test_support::pipeline;
use nu_test_support::playground::Playground;

#[test]
fn takes_rows_of_nu_value_strings_and_pipes_it_to_stdin_of_external() {
    Playground::setup("internal_to_external_pipe_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
                Jonathan,1,New Zealand
                Andrés,1,Ecuador
                AndKitKatz,1,Estados Unidos
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            open nu_times.csv
            | get origin
            | each { ^echo $it | nu --testbin chop | lines }
<<<<<<< HEAD
            | nth 2
=======
            | get 2
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert_eq!(actual.out, "Ecuado");
    })
}

#[test]
fn treats_dot_dot_as_path_not_range() {
    Playground::setup("dot_dot_dir", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            mkdir temp;
            cd temp;
<<<<<<< HEAD
            echo (open ../nu_times.csv).name | autoview;
=======
            echo (open ../nu_times.csv).name.0 | table;
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            cd ..;
            rmdir temp
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert_eq!(actual.out, "Jason");
    })
}

#[test]
<<<<<<< HEAD
fn tags_dont_persist_through_column_path() {
    Playground::setup("dot_dot_dir", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            mkdir temp;
            cd temp;
            let x = (open ../nu_times.csv).name;
            $x | tags | get anchor | autoview;
            cd ..;
            rmdir temp
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert!(actual.err.contains("isn't a column"));
    })
}

#[test]
fn tags_persist_through_vars() {
    Playground::setup("dot_dot_dir", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            mkdir temp;
            cd temp;
            let x = (open ../nu_times.csv);
            $x | tags | get anchor.file | autoview;
            cd ..;
            rmdir temp
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert!(actual.out.contains("nu_times.csv"));
    })
}

#[test]
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
fn subexpression_properly_redirects() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo (nu --testbin cococo "hello") | str collect
        "#
    );

    assert_eq!(actual.out, "hello");
}

#[test]
fn argument_subexpression() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo "foo" | each { echo (echo $it) }
        "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn subexpression_handles_dot() {
    Playground::setup("subexpression_handles_dot", |dirs, sandbox| {
        sandbox.with_files(vec![FileWithContentToBeTrimmed(
            "nu_times.csv",
            r#"
                name,rusty_luck,origin
                Jason,1,Canada
                Jonathan,1,New Zealand
                Andrés,1,Ecuador
                AndKitKatz,1,Estados Unidos
            "#,
        )]);

        let actual = nu!(
        cwd: dirs.test(), pipeline(
        r#"
            echo (open nu_times.csv)
            | get name
            | each { nu --testbin chop $it | lines }
<<<<<<< HEAD
            | nth 3
=======
            | get 3
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
            "#
        ));

        assert_eq!(actual.out, "AndKitKat");
    })
}

#[test]
fn string_interpolation_with_it() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo "foo" | each { echo $"($it)" }
            "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn string_interpolation_with_it_column_path() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo [[name]; [sammie]] | each { echo $"($it.name)" }
        "#
    );

    assert_eq!(actual.out, "sammie");
}

#[test]
fn string_interpolation_shorthand_overlap() {
    let actual = nu!(
        cwd: ".",
        r#"
                    $"3 + 4 = (3 + 4)"
        "#
    );

    assert_eq!(actual.out, "3 + 4 = 7");
}

<<<<<<< HEAD
=======
// FIXME: jt - we don't currently have a way to escape the single ticks easily
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn string_interpolation_and_paren() {
    let actual = nu!(
        cwd: ".",
        r#"
                    $"a paren is ('(')"
        "#
    );

    assert_eq!(actual.out, "a paren is (");
}

#[test]
fn string_interpolation_with_unicode() {
    //カ = U+30AB : KATAKANA LETTER KA
    let actual = nu!(
        cwd: ".",
        r#"
            $"カ"
        "#
    );

    assert_eq!(actual.out, "カ");
}

#[test]
<<<<<<< HEAD
fn bignum_large_integer() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo 91231720741731287123917
        "#
    );

    assert_eq!(actual.out, "91231720741731287123917");
}

#[test]
fn bignum_large_decimal() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo 91231720741731287123917.1
        "#
    );

    assert_eq!(actual.out, "91231720741731287123917.1");
}

#[test]
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
fn run_custom_command() {
    let actual = nu!(
        cwd: ".",
        r#"
            def add-me [x y] { $x + $y}; add-me 10 5
        "#
    );

    assert_eq!(actual.out, "15");
}

#[test]
fn run_custom_command_with_flag() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        def foo [--bar:number] { if ($bar | empty?) { echo "empty" } { echo $bar } }; foo --bar 10
=======
        def foo [--bar:number] { if ($bar | empty?) { echo "empty" } else { echo $bar } }; foo --bar 10
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "10");
}

#[test]
fn run_custom_command_with_flag_missing() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        def foo [--bar:number] { if ($bar | empty?) { echo "empty" } { echo $bar } }; foo
=======
        def foo [--bar:number] { if ($bar | empty?) { echo "empty" } else { echo $bar } }; foo
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "empty");
}

#[test]
fn run_custom_subcommand() {
    let actual = nu!(
        cwd: ".",
        r#"
        def "str double" [x] { echo $x $x | str collect }; str double bob
        "#
    );

    assert_eq!(actual.out, "bobbob");
}

#[test]
fn run_inner_custom_command() {
    let actual = nu!(
        cwd: ".",
        r#"
          def outer [x] { def inner [y] { echo $y }; inner $x }; outer 10
        "#
    );

    assert_eq!(actual.out, "10");
}

#[test]
fn run_broken_inner_custom_command() {
    let actual = nu!(
        cwd: ".",
        r#"
        def outer [x] { def inner [y] { echo $y }; inner $x }; inner 10
        "#
    );

<<<<<<< HEAD
    assert!(actual.err.contains("not found"));
=======
    assert!(!actual.err.is_empty());
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn run_custom_command_with_rest() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            def rest-me [...rest: string] { echo $rest.1 $rest.0}; rest-me "hello" "world" | to json
=======
            def rest-me [...rest: string] { echo $rest.1 $rest.0}; rest-me "hello" "world" | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, r#"["world","hello"]"#);
}

#[test]
fn run_custom_command_with_rest_and_arg() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            def rest-me-with-arg [name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-arg "hello" "world" "yay" | to json
=======
            def rest-me-with-arg [name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-arg "hello" "world" "yay" | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, r#"["yay","world","hello"]"#);
}

#[test]
fn run_custom_command_with_rest_and_flag() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            def rest-me-with-flag [--name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-flag "hello" "world" --name "yay" | to json
=======
            def rest-me-with-flag [--name: string, ...rest: string] { echo $rest.1 $rest.0 $name}; rest-me-with-flag "hello" "world" --name "yay" | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, r#"["world","hello","yay"]"#);
}

#[test]
fn run_custom_command_with_empty_rest() {
    let actual = nu!(
        cwd: ".",
        r#"
            def rest-me-with-empty-rest [...rest: string] { echo $rest }; rest-me-with-empty-rest
        "#
    );

    assert_eq!(actual.out, r#""#);
    assert_eq!(actual.err, r#""#);
}

<<<<<<< HEAD
=======
//FIXME: jt: blocked on https://github.com/nushell/engine-q/issues/912
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn run_custom_command_with_rest_other_name() {
    let actual = nu!(
        cwd: ".",
        r#"
            def say-hello [
                greeting:string,
                ...names:string # All of the names
                ] {
<<<<<<< HEAD
                    echo $"($greeting), ($names | sort-by | str collect ' ')"
=======
                    echo $"($greeting), ($names | sort-by | str collect)"
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                }
            say-hello Salutations E D C A B
        "#
    );

<<<<<<< HEAD
    assert_eq!(actual.out, r#"Salutations, A B C D E"#);
=======
    assert_eq!(actual.out, r#"Salutations, ABCDE"#);
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    assert_eq!(actual.err, r#""#);
}

#[test]
fn alias_a_load_env() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            def activate-helper [] { [[name, value]; [BOB, SAM]] }; alias activate = load-env (activate-helper); activate; $nu.env.BOB
=======
            def activate-helper [] { {BOB: SAM} }; alias activate = load-env (activate-helper); activate; $env.BOB
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, r#"SAM"#);
}

#[test]
fn let_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
            let x = 5
            let y = 12
            $x + $y
        "#
    );

    assert_eq!(actual.out, "17");
}

#[test]
fn let_doesnt_leak() {
    let actual = nu!(
        cwd: ".",
        r#"
        do { let x = 5 }; echo $x
        "#
    );

<<<<<<< HEAD
    assert!(actual.err.contains("unknown variable"));
=======
    assert!(actual.err.contains("variable not found"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn let_env_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
            let-env TESTENVVAR = "hello world"
<<<<<<< HEAD
            echo $nu.env.TESTENVVAR
=======
            echo $env.TESTENVVAR
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "hello world");
}

#[test]
fn let_env_hides_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
            let-env TESTENVVAR = "hello world"
<<<<<<< HEAD
            echo $nu.env.TESTENVVAR
            let-env TESTENVVAR = $nothing
            echo $nu.env.TESTENVVAR
=======
            echo $env.TESTENVVAR
            hide TESTENVVAR
            echo $env.TESTENVVAR
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "hello world");
<<<<<<< HEAD
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
=======
    assert!(actual.err.contains("did you mean"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn let_env_hides_variable_in_parent_scope() {
    let actual = nu!(
        cwd: ".",
        r#"
            let-env TESTENVVAR = "hello world"
<<<<<<< HEAD
            echo $nu.env.TESTENVVAR
            do {
                let-env TESTENVVAR = $nothing
                echo $nu.env.TESTENVVAR
            }
            echo $nu.env.TESTENVVAR
        "#
    );

    assert_eq!(actual.out, "hello worldhello world");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
=======
            echo $env.TESTENVVAR
            do {
                hide TESTENVVAR
                echo $env.TESTENVVAR
            }
            echo $env.TESTENVVAR
        "#
    );

    assert_eq!(actual.out, "hello world");
    assert!(actual.err.contains("did you mean"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn unlet_env_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
            let-env TEST_VAR = "hello world"
<<<<<<< HEAD
            unlet-env TEST_VAR
            echo $nu.env.TEST_VAR
=======
            hide TEST_VAR
            echo $env.TEST_VAR
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert!(actual.err.contains("did you mean"));
}

#[test]
fn unlet_nonexistent_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            unlet-env NONEXISTENT_VARIABLE
        "#
    );

    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Not an environment variable"));
=======
            hide NONEXISTENT_VARIABLE
        "#
    );

    assert!(actual.err.contains("did not find"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn unlet_variable_in_parent_scope() {
    let actual = nu!(
        cwd: ".",
        r#"
            let-env DEBUG = "1"
<<<<<<< HEAD
            echo $nu.env.DEBUG
            do {
                let-env DEBUG = "2"
                echo $nu.env.DEBUG
                unlet-env DEBUG
                echo $nu.env.DEBUG
            }
            echo $nu.env.DEBUG
=======
            echo $env.DEBUG
            do {
                let-env DEBUG = "2"
                echo $env.DEBUG
                hide DEBUG
                echo $env.DEBUG
            }
            echo $env.DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "1211");
}

#[test]
fn let_env_doesnt_leak() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        do { let-env xyz = "my message" }; echo $nu.env.xyz
=======
        do { let-env xyz = "my message" }; echo $env.xyz
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert!(actual.err.contains("did you mean"));
}

#[test]
fn proper_shadow_let_env_aliases() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        let-env DEBUG = true; echo $nu.env.DEBUG | autoview; do { let-env DEBUG = false; echo $nu.env.DEBUG } | autoview; echo $nu.env.DEBUG
=======
        let-env DEBUG = true; echo $env.DEBUG | table; do { let-env DEBUG = false; echo $env.DEBUG } | table; echo $env.DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "truefalsetrue");
}

#[test]
fn load_env_variable() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            echo [[name, value]; [TESTENVVAR, "hello world"]] | load-env
            echo $nu.env.TESTENVVAR
=======
            echo {TESTENVVAR: "hello world"} | load-env
            echo $env.TESTENVVAR
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "hello world");
}

#[test]
fn load_env_variable_arg() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
            load-env [[name, value]; [TESTENVVAR, "hello world"]]
            echo $nu.env.TESTENVVAR
=======
            load-env {TESTENVVAR: "hello world"}
            echo $env.TESTENVVAR
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "hello world");
}

#[test]
<<<<<<< HEAD
fn load_env_variable_arg_and_stream() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo [[name, value]; [TESTVARSTREAM, "true"]] | load-env [[name, value]; [TESTVARARG, "false"]]
            echo $nu.env | format "{TESTVARSTREAM} {TESTVARARG}"
        "#
    );

    assert_eq!(actual.out, "true false");
}

#[test]
=======
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
fn load_env_doesnt_leak() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        do { echo [[name, value]; [xyz, "my message"]] | load-env }; echo $nu.env.xyz
=======
        do { echo { name: xyz, value: "my message" } | load-env }; echo $env.xyz
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert!(actual.err.contains("did you mean"));
}

#[test]
fn proper_shadow_load_env_aliases() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        let-env DEBUG = true; echo $nu.env.DEBUG | autoview; do { echo [[name, value]; [DEBUG, false]] | load-env; echo $nu.env.DEBUG } | autoview; echo $nu.env.DEBUG
=======
        let-env DEBUG = true; echo $env.DEBUG | table; do { echo {DEBUG: "false"} | load-env; echo $env.DEBUG } | table; echo $env.DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "truefalsetrue");
}

<<<<<<< HEAD
=======
//FIXME: jt: load-env can not currently hide variables because $nothing no longer hides
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn load_env_can_hide_var_envs() {
    let actual = nu!(
        cwd: ".",
        r#"
        let-env DEBUG = "1"
<<<<<<< HEAD
        echo $nu.env.DEBUG
        load-env [[name, value]; [DEBUG $nothing]]
        echo $nu.env.DEBUG
=======
        echo $env.DEBUG
        load-env [[name, value]; [DEBUG $nothing]]
        echo $env.DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "1");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
}

<<<<<<< HEAD
=======
//FIXME: jt: load-env can not currently hide variables because $nothing no longer hides
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn load_env_can_hide_var_envs_in_parent_scope() {
    let actual = nu!(
        cwd: ".",
        r#"
        let-env DEBUG = "1"
<<<<<<< HEAD
        echo $nu.env.DEBUG
        do {
            load-env [[name, value]; [DEBUG $nothing]]
            echo $nu.env.DEBUG
        }
        echo $nu.env.DEBUG
=======
        echo $env.DEBUG
        do {
            load-env [[name, value]; [DEBUG $nothing]]
            echo $env.DEBUG
        }
        echo $env.DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "11");
    assert!(actual.err.contains("error"));
    assert!(actual.err.contains("Unknown column"));
}

#[test]
fn proper_shadow_let_aliases() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        let DEBUG = false; echo $DEBUG | autoview; do { let DEBUG = true; echo $DEBUG } | autoview; echo $DEBUG
=======
        let DEBUG = $false; echo $DEBUG | table; do { let DEBUG = $true; echo $DEBUG } | table; echo $DEBUG
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "falsetruefalse");
}

#[test]
fn block_params_override() {
    let actual = nu!(
        cwd: ".",
        r#"
        [1, 2, 3] | each { |a| echo $it }
        "#
    );
<<<<<<< HEAD
    assert!(actual.err.contains("unknown variable"));
=======
    assert!(actual.err.contains("variable not found"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn block_params_override_correct() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        [1, 2, 3] | each { |a| echo $a } | to json
=======
        [1, 2, 3] | each { |a| echo $a } | to json --raw
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );
    assert_eq!(actual.out, "[1,2,3]");
}

#[test]
fn hex_number() {
    let actual = nu!(
        cwd: ".",
        r#"
        0x10
        "#
    );
    assert_eq!(actual.out, "16");
}

#[test]
fn binary_number() {
    let actual = nu!(
        cwd: ".",
        r#"
        0b10
        "#
    );
    assert_eq!(actual.out, "2");
}

#[test]
fn octal_number() {
    let actual = nu!(
        cwd: ".",
        r#"
        0o10
        "#
    );
    assert_eq!(actual.out, "8");
}

#[test]
fn run_dynamic_blocks() {
    let actual = nu!(
        cwd: ".",
        r#"
        let block = { echo "holaaaa" }; do $block
        "#
    );
    assert_eq!(actual.out, "holaaaa");
}

#[cfg(feature = "which")]
#[test]
fn argument_subexpression_reports_errors() {
    let actual = nu!(
        cwd: ".",
        "echo (ferris_is_not_here.exe)"
    );

<<<<<<< HEAD
    assert!(actual.err.contains("Command not found"));
=======
    assert!(!actual.err.is_empty());
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn can_process_one_row_from_internal_and_pipes_it_to_stdin_of_external() {
    let actual = nu!(
        cwd: ".",
        r#"echo "nushelll" | nu --testbin chop"#
    );

    assert_eq!(actual.out, "nushell");
}

#[test]
fn bad_operator() {
    let actual = nu!(
        cwd: ".",
        r#"
            2 $ 2
        "#
    );

    assert!(actual.err.contains("operator"));
}

#[test]
fn index_out_of_bounds() {
    let actual = nu!(
        cwd: ".",
        r#"
            let foo = [1, 2, 3]; echo $foo.5
        "#
    );

<<<<<<< HEAD
    assert!(actual.err.contains("unknown row"));
}

=======
    assert!(actual.err.contains("too large"));
}

//FIXME: jt - umm, do we actually want to support this?
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn dash_def() {
    let actual = nu!(
        cwd: ".",
        r#"
            def - [x, y] { $x - $y }; - 4 1
        "#
    );

    assert_eq!(actual.out, "3");
}

#[test]
fn negative_decimal_start() {
    let actual = nu!(
        cwd: ".",
        r#"
            -1.3 + 4
        "#
    );

    assert_eq!(actual.out, "2.7");
}

#[test]
fn string_inside_of() {
    let actual = nu!(
        cwd: ".",
        r#"
            "bob" in "bobby"
        "#
    );

    assert_eq!(actual.out, "true");
}

#[test]
fn string_not_inside_of() {
    let actual = nu!(
        cwd: ".",
        r#"
            "bob" not-in "bobby"
        "#
    );

    assert_eq!(actual.out, "false");
}

#[test]
fn index_row() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        let foo = [[name]; [joe] [bob]]; echo $foo.1 | to json
        "#
    );

    assert_eq!(actual.out, r#"{"name":"bob"}"#);
=======
        let foo = [[name]; [joe] [bob]]; echo $foo.1 | to json --raw
        "#
    );

    assert_eq!(actual.out, r#"{"name": "bob"}"#);
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn index_cell() {
    let actual = nu!(
        cwd: ".",
        r#"
        let foo = [[name]; [joe] [bob]]; echo $foo.name.1
        "#
    );

    assert_eq!(actual.out, "bob");
}

#[test]
fn index_cell_alt() {
    let actual = nu!(
        cwd: ".",
        r#"
        let foo = [[name]; [joe] [bob]]; echo $foo.1.name
        "#
    );

    assert_eq!(actual.out, "bob");
}

#[test]
fn not_echoing_ranges_without_numbers() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo ..
        "#
    );

    assert_eq!(actual.out, "..");
}

#[test]
fn not_echoing_exclusive_ranges_without_numbers() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo ..<
        "#
    );

    assert_eq!(actual.out, "..<");
}

#[test]
fn echoing_ranges() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo 1..3 | math sum
        "#
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn echoing_exclusive_ranges() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo 1..<4 | math sum
        "#
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn table_literals1() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo [[name age]; [foo 13]] | get age
        "#
    );

    assert_eq!(actual.out, "13");
}

#[test]
fn table_literals2() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo [[name age] ; [bob 13] [sally 20]] | get age | math sum
        "#
    );

    assert_eq!(actual.out, "33");
}

#[test]
fn list_with_commas() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo [1, 2, 3] | math sum
        "#
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn range_with_left_var() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        echo [[size]; [3]] | each { echo $it.size..10 } | math sum
=======
        ({ size: 3}.size)..10 | math sum
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "52");
}

#[test]
fn range_with_right_var() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        echo [[size]; [30]] | each { echo 4..$it.size } | math sum
=======
        4..({ size: 30}.size) | math sum
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
        "#
    );

    assert_eq!(actual.out, "459");
}

#[test]
fn range_with_open_left() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo ..30 | math sum
        "#
    );

    assert_eq!(actual.out, "465");
}

#[test]
fn exclusive_range_with_open_left() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo ..<31 | math sum
        "#
    );

    assert_eq!(actual.out, "465");
}

#[test]
fn range_with_open_right() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo 5.. | first 10 | math sum
        "#
    );

    assert_eq!(actual.out, "95");
}

#[test]
fn exclusive_range_with_open_right() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo 5..< | first 10 | math sum
        "#
    );

    assert_eq!(actual.out, "95");
}

#[test]
fn range_with_mixed_types() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo 1..10.5 | math sum
        "#
    );

    assert_eq!(actual.out, "55");
}

#[test]
fn filesize_math() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        100 * 10kb
        "#
    );

    assert_eq!(actual.out, "1000.0 KB");
=======
        100 * 10kib
        "#
    );

    assert_eq!(actual.out, "1000.0 KiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    // why 1000.0 KB instead of 1.0 MB?
    // looks like `byte.get_appropriate_unit(false)` behaves this way
}

#[test]
fn filesize_math2() {
    let actual = nu!(
        cwd: ".",
        r#"
        100 / 10kb
        "#
    );

<<<<<<< HEAD
    assert!(actual.err.contains("Coercion"));
=======
    assert!(actual.err.contains("doesn't support"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn filesize_math3() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        100kb / 10
        "#
    );

    assert_eq!(actual.out, "10.0 KB");
=======
        100kib / 10
        "#
    );

    assert_eq!(actual.out, "10.0 KiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}
#[test]
fn filesize_math4() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        100kb * 5
        "#
    );

    assert_eq!(actual.out, "500.0 KB");
=======
        100kib * 5
        "#
    );

    assert_eq!(actual.out, "500.0 KiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn filesize_math5() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        1001 * 1kb
        "#
    );

    assert_eq!(actual.out, "1.0 MB");
=======
        1000 * 1kib
        "#
    );

    assert_eq!(actual.out, "1000.0 KiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn filesize_math6() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        1001 * 1mb
        "#
    );

    assert_eq!(actual.out, "1.0 GB");
=======
        1000 * 1mib
        "#
    );

    assert_eq!(actual.out, "1000.0 MiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn filesize_math7() {
    let actual = nu!(
        cwd: ".",
        r#"
<<<<<<< HEAD
        1001 * 1gb
        "#
    );

    assert_eq!(actual.out, "1.0 TB");
=======
        1000 * 1gib
        "#
    );

    assert_eq!(actual.out, "1000.0 GiB");
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn exclusive_range_with_mixed_types() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo 1..<10.5 | math sum
        "#
    );

    assert_eq!(actual.out, "55");
}

#[test]
fn table_with_commas() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo [[name, age, height]; [JT, 42, 185] [Unknown, 99, 99]] | get age | math sum
        "#
    );

    assert_eq!(actual.out, "141");
}

#[test]
fn duration_overflow() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        ls | get modified | each { $it + 10000000000000000day }
        "#)
    );

<<<<<<< HEAD
    assert!(actual.err.contains("Duration overflow"));
=======
    assert!(actual.err.contains("duration too large"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn date_and_duration_overflow() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        ls | get modified | each { $it + 1000000000day }
        "#)
    );

    // assert_eq!(actual.err, "overflow");
<<<<<<< HEAD
    assert!(actual.err.contains("Duration and date addition overflow"));
=======
    assert!(actual.err.contains("duration too large"));
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
}

#[test]
fn pipeline_params_simple() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1 2 3 | $in.1 * $in.2
        "#)
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn pipeline_params_inner() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        echo 1 2 3 | (echo $in.2 6 7 | $in.0 * $in.1 * $in.2)
        "#)
    );

    assert_eq!(actual.out, "126");
}

#[test]
fn better_table_lex() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        let table = [
            [name, size];
            [small, 7]
            [medium, 10]
            [large, 12]
        ];
        $table.1.size
        "#)
    );

    assert_eq!(actual.out, "10");
}

#[test]
fn better_subexpr_lex() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        (echo boo
        sam | str length | math sum)
        "#)
    );

    assert_eq!(actual.out, "6");
}

#[test]
fn subsubcommand() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def "aws s3 rb" [url] { $url + " loaded" }; aws s3 rb localhost
        "#)
    );

    assert_eq!(actual.out, "localhost loaded");
}

#[test]
fn manysubcommand() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def "aws s3 rb ax vf qqqq rrrr" [url] { $url + " loaded" }; aws s3 rb ax vf qqqq rrrr localhost
        "#)
    );

    assert_eq!(actual.out, "localhost loaded");
}

#[test]
fn nothing_string_1() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        $nothing == "foo"
        "#)
    );

    assert_eq!(actual.out, "false");
}

<<<<<<< HEAD
#[test]
fn nothing_string_2() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        "" == $nothing
        "#)
    );

    assert_eq!(actual.out, "true");
}

=======
// FIXME: no current way to hide aliases
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn unalias_shadowing() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-shadowing [] {
            alias greet = echo hello;
            let xyz = { greet };
            unalias greet;
            do $xyz
        };
        test-shadowing
        "#)
    );
    assert_eq!(actual.out, "hello");
}

<<<<<<< HEAD
=======
// FIXME: no current way to hide aliases
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn unalias_does_not_escape_scope() {
    let actual = nu!(
        cwd: ".", pipeline(
        r#"
        def test-alias [] {
            alias greet = echo hello;
            (unalias greet);
            greet
        };
        test-alias
        "#)
    );
    assert_eq!(actual.out, "hello");
}

<<<<<<< HEAD
=======
// FIXME: no current way to hide aliases
#[ignore]
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
#[test]
fn unalias_hides_alias() {
    let actual = nu!(cwd: ".", pipeline(
        r#"
        def test-alias [] {
            alias ll = ls -l;
            unalias ll;
            ll
        };
        test-alias
        "#)
    );

    assert!(actual.err.contains("not found"));
}

mod parse {
    use nu_test_support::nu;

    /*
        The debug command's signature is:

        Usage:
        > debug {flags}

        flags:
        -h, --help: Display this help message
        -r, --raw: Prints the raw value representation.
    */

    #[test]
    fn errors_if_flag_passed_is_not_exact() {
        let actual = nu!(cwd: ".", "debug -ra");

<<<<<<< HEAD
        assert!(actual.err.contains("unexpected flag"),);

        let actual = nu!(cwd: ".", "debug --rawx");

        assert!(actual.err.contains("unexpected flag"),);
=======
        assert!(actual.err.contains("unknown flag"),);

        let actual = nu!(cwd: ".", "debug --rawx");

        assert!(actual.err.contains("unknown flag"),);
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }

    #[test]
    fn errors_if_flag_is_not_supported() {
        let actual = nu!(cwd: ".", "debug --ferris");

<<<<<<< HEAD
        assert!(actual.err.contains("unexpected flag"),);
=======
        assert!(actual.err.contains("unknown flag"),);
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }

    #[test]
    fn errors_if_passed_an_unexpected_argument() {
        let actual = nu!(cwd: ".", "debug ferris");

<<<<<<< HEAD
        assert!(actual.err.contains("unexpected argument"),);
=======
        assert!(actual.err.contains("extra positional argument"),);
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
    }
}

mod tilde_expansion {
    use nu_test_support::nu;

    #[test]
    #[should_panic]
    fn as_home_directory_when_passed_as_argument_and_begins_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
            echo ~
        "#
        );

        assert!(!actual.out.contains('~'),);
    }

    #[test]
    fn does_not_expand_when_passed_as_argument_and_does_not_start_with_tilde() {
        let actual = nu!(
            cwd: ".",
            r#"
                    echo "1~1"
                "#
        );

        assert_eq!(actual.out, "1~1");
    }
}

mod variable_scoping {
    use nu_test_support::nu;

    macro_rules! test_variable_scope {
        ($func:literal == $res:literal $(,)*) => {
            let actual = nu!(
                cwd: ".",
                $func
            );

            assert_eq!(actual.out, $res);
        };
    }
    macro_rules! test_variable_scope_list {
        ($func:literal == $res:expr $(,)*) => {
            let actual = nu!(
                cwd: ".",
                $func
            );

            let result: Vec<&str> = actual.out.matches("ZZZ").collect();
            assert_eq!(result, $res);
        };
    }

    #[test]
    fn access_variables_in_scopes() {
        test_variable_scope!(
            r#" def test [input] { echo [0 1 2] | do { do { echo $input } } }
                test ZZZ "#
                == "ZZZ"
        );
        test_variable_scope!(
<<<<<<< HEAD
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } { echo $input } } } }
=======
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } else { echo $input } } } }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                test ZZZ "#
                == "ZZZ"
        );
        test_variable_scope!(
<<<<<<< HEAD
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } { echo $input } } } }
=======
            r#" def test [input] { echo [0 1 2] | do { do { if $input == "ZZZ" { echo $input } else { echo $input } } } }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                test ZZZ "#
                == "ZZZ"
        );
        test_variable_scope!(
            r#" def test [input] { echo [0 1 2] | do { echo $input } }
                test ZZZ "#
                == "ZZZ"
        );
        test_variable_scope!(
<<<<<<< HEAD
            r#" def test [input] { echo [0 1 2] | do { if $input == $input { echo $input } { echo $input } } }
=======
            r#" def test [input] { echo [0 1 2] | do { if $input == $input { echo $input } else { echo $input } } }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                test ZZZ "#
                == "ZZZ"
        );
        test_variable_scope_list!(
            r#" def test [input] { echo [0 1 2] | each { echo $input } }
                test ZZZ "#
                == ["ZZZ", "ZZZ", "ZZZ"]
        );
        test_variable_scope_list!(
<<<<<<< HEAD
            r#" def test [input] { echo [0 1 2] | each { if $it > 0 {echo $input} {echo $input}} }
=======
            r#" def test [input] { echo [0 1 2] | each { if $it > 0 {echo $input} else {echo $input}} }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                test ZZZ "#
                == ["ZZZ", "ZZZ", "ZZZ"]
        );
        test_variable_scope_list!(
<<<<<<< HEAD
            r#" def test [input] { echo [0 1 2] | each { if $input == $input {echo $input} {echo $input}} }
=======
            r#" def test [input] { echo [0 1 2] | each { if $input == $input {echo $input} else {echo $input}} }
>>>>>>> 9259a56a28f1dd3a4b720ad815aa19c6eaf6adce
                test ZZZ "#
                == ["ZZZ", "ZZZ", "ZZZ"]
        );
    }
}
