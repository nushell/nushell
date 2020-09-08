use nu_test_support::fs::Stub::EmptyFile;
#[cfg(feature = "which")]
use nu_test_support::fs::Stub::FileWithContent;
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
            | ^echo $it
            | nu --testbin chop
            | lines
            | nth 2
            | echo $it
            "#
        ));

        // chop will remove the last escaped double quote from \"Estados Unidos\"
        assert_eq!(actual.out, "Ecuado");
    })
}

#[cfg(feature = "directories-support")]
#[cfg(feature = "which-support")]
#[test]
fn autoenv() {
    use nu_test_support::fs::Stub::FileWithContent;
    Playground::setup("autoenv_test", |dirs, sandbox| {
        sandbox.mkdir("foo/bar");
        sandbox.mkdir("bizz/buzz");
        sandbox.mkdir("foob");

        // Windows uses a different command to create an empty file so we need to have different content on windows.
        let full_nu_env = if cfg!(target_os = "windows") {
            r#"[env]
                testkey = "testvalue"

                [scriptvars]
                myscript = "echo myval"

                [scripts]
                entryscripts = ["echo nul > hello.txt"]
                exitscripts = ["echo nul > bye.txt"]"#
        } else {
            r#"[env]
                testkey = "testvalue"

                [scriptvars]
                myscript = "echo myval"

                [scripts]
                entryscripts = ["touch hello.txt"]
                exitscripts = ["touch bye.txt"]"#
        };

        sandbox.with_files(vec![
            FileWithContent(".nu-env", full_nu_env),
            FileWithContent(
                "foo/.nu-env",
                r#"[env]
                    overwrite_me = "set_in_foo"
                    fookey = "fooval" "#,
            ),
            FileWithContent(
                "foo/bar/.nu-env",
                r#"[env]
                    overwrite_me = "set_in_bar""#,
            ),
            FileWithContent("bizz/.nu-env", full_nu_env),
        ]);

        //Make sure basic keys are set
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust
               echo $nu.env.testkey"#
        );
        assert!(actual.out.ends_with("testvalue"));

        // Make sure exitscripts are run in the directory they were specified.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust
               cd ..
               cd autoenv_test
               ls
               ls | where name == "bye.txt" | get name"#
        );
        assert!(actual.out.contains("bye.txt"));

        // Make sure entry scripts are run
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd ..
               autoenv trust autoenv_test
               cd autoenv_test
               ls | where name == "hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        // If inside a directory with exitscripts, entering a subdirectory should not trigger the exitscripts.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust
               cd foob
               ls | where name == "bye.txt" | get name"#
        );
        assert!(!actual.out.contains("bye.txt"));

        // Make sure entryscripts are run when re-visiting a directory
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust bizz
               cd bizz
               rm hello.txt
               cd ..
               cd bizz
               ls | where name == "hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        // Entryscripts should not run after changing to a subdirectory.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust bizz
               cd bizz
               cd buzz
               ls | where name == hello.txt | get name"#
        );
        assert!(!actual.out.ends_with("hello.txt"));

        //Backing out of the directory should unset the keys
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd ..
               echo $nu.env.testkey"#
        );
        assert!(!actual.out.ends_with("testvalue"));

        // Make sure script keys are set
        let actual = nu!(
            cwd: dirs.test(),
            r#"echo $nu.env.myscript"#
        );
        assert!(actual.out.ends_with("myval"));

        //Going to sibling directory without passing parent should work.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust foo
               cd foob
               cd ../foo
               echo $nu.env.fookey
               cd .."#
        );
        assert!(actual.out.ends_with("fooval"));

        //Going to sibling directory should unset keys
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd foo
               cd ../foob
               echo $nu.env.fookey
               cd .."#
        );
        assert!(!actual.out.ends_with("fooval"));

        // Make sure entry scripts are run
        let actual = nu!(
            cwd: dirs.test(),
            r#"ls | where name == "hello.txt" | get name"#
        );
        assert!(actual.out.contains("hello.txt"));

        //Variables set in parent directories should be set even if you directly cd to a subdir
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust foo
                   cd foo/bar
                   autoenv trust
                   echo $nu.env.fookey"#
        );
        assert!(actual.out.ends_with("fooval"));

        //Subdirectories should overwrite the values of parent directories.
        let actual = nu!(
            cwd: dirs.test(),
            r#"autoenv trust foo
                   cd foo/bar
                   autoenv trust
                   echo $nu.env.overwrite_me"#
        );
        assert!(actual.out.ends_with("set_in_bar"));

        //Make sure that overwritten values are restored.
        //By deleting foo/.nu-env, we make sure that the value is actually restored and not just set again by autoenv when we re-visit foo.
        let actual = nu!(
            cwd: dirs.test(),
            r#"cd foo
                   cd bar
                   rm ../.nu-env
                   cd ..
                   echo $nu.env.overwrite_me"#
        );
        assert!(actual.out.ends_with("set_in_foo"))
    })
}

#[test]
fn proper_it_expansion() {
    Playground::setup("ls_test_1", |dirs, sandbox| {
        sandbox.with_files(vec![
            EmptyFile("andres.txt"),
            EmptyFile("gedge.txt"),
            EmptyFile("jonathan.txt"),
            EmptyFile("yehuda.txt"),
        ]);

        let actual = nu!(
            cwd: dirs.test(), pipeline(
            r#"
                    ls | sort-by name | group-by type | each { get File.name | echo $it } | to json
                "#
        ));

        assert_eq!(
            actual.out,
            r#"["andres.txt","gedge.txt","jonathan.txt","yehuda.txt"]"#
        );
    })
}

#[test]
fn it_expansion_of_list() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo "foo" | echo [bar $it] | to json
        "#
    );

    assert_eq!(actual.out, "[\"bar\",\"foo\"]");
}

#[test]
fn it_expansion_of_invocation() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo $(echo "4" | echo $it | str to-int )
        "#
    );

    assert_eq!(actual.out, "4");
}

#[test]
fn invocation_properly_redirects() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo $(nu --testbin cococo "hello") | str collect
        "#
    );

    assert_eq!(actual.out, "hello");
}

#[test]
fn argument_invocation() {
    let actual = nu!(
        cwd: ".",
        r#"
            echo "foo" | echo $(echo $it)
        "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn invocation_handles_dot() {
    Playground::setup("invocation_handles_dot", |dirs, sandbox| {
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
            echo $(open nu_times.csv)
            | get name
            | nu --testbin chop $it
            | nth 3
            | echo $it
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
                    echo "foo" | echo `{{$it}}`
            "#
    );

    assert_eq!(actual.out, "foo");
}

#[test]
fn string_interpolation_with_column() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "bob"}' | from json | echo `{{name}} is cool`
            "#
    );

    assert_eq!(actual.out, "bob is cool");
}

#[test]
fn string_interpolation_with_column2() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "fred"}' | from json | echo `also {{name}} is cool`
            "#
    );

    assert_eq!(actual.out, "also fred is cool");
}

#[test]
fn string_interpolation_with_column3() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "sally"}' | from json | echo `also {{name}}`
            "#
    );

    assert_eq!(actual.out, "also sally");
}

#[test]
fn string_interpolation_with_it_column_path() {
    let actual = nu!(
        cwd: ".",
        r#"
                    echo '{"name": "sammie"}' | from json | echo `{{$it.name}}`
        "#
    );

    assert_eq!(actual.out, "sammie");
}

#[cfg(feature = "which")]
#[test]
fn argument_invocation_reports_errors() {
    let actual = nu!(
        cwd: ".",
        "echo $(ferris_is_not_here.exe)"
    );

    assert!(actual.err.contains("Command not found"));
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
        echo [[size]; [3]] | echo $it.size..10 | math sum
        "#
    );

    assert_eq!(actual.out, "52");
}

#[test]
fn range_with_right_var() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo [[size]; [30]] | echo 4..$it.size | math sum
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
fn it_expansion_of_tables() {
    let actual = nu!(
        cwd: ".",
        r#"
        echo foo | echo [[`foo {{$it}} bar`]; [`{{$it}} foo`]] | get "foo foo bar"
        "#
    );

    assert_eq!(actual.out, "foo foo");
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

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );

        let actual = nu!(cwd: ".", "debug --rawx");

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );
    }

    #[test]
    fn errors_if_flag_is_not_supported() {
        let actual = nu!(cwd: ".", "debug --ferris");

        assert!(
            actual.err.contains("unexpected flag"),
            format!(
                "error message '{}' should contain 'unexpected flag'",
                actual.err
            )
        );
    }

    #[test]
    fn errors_if_passed_an_unexpected_argument() {
        let actual = nu!(cwd: ".", "debug ferris");

        assert!(
            actual.err.contains("unexpected argument"),
            format!(
                "error message '{}' should contain 'unexpected argument'",
                actual.err
            )
        );
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

        assert!(
            !actual.out.contains('~'),
            format!("'{}' should not contain ~", actual.out)
        );
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
