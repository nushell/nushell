use nu_test_support::fs::{Stub, file_contents};
use nu_test_support::nu;
use nu_test_support::playground::Playground;
use std::io::Write;

#[test]
fn writes_out_csv() {
    Playground::setup("save_test_2", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("cargo_sample.csv");

        nu!(
            cwd: dirs.root(),
            r#"[[name, version, description, license, edition]; [nu, "0.14", "A new type of shell", "MIT", "2018"]] | save save_test_2/cargo_sample.csv"#,
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert!(actual.contains("nu,0.14,A new type of shell,MIT,2018"));
    })
}

#[test]
fn writes_out_list() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.txt");

        nu!(
            cwd: dirs.root(),
            "[a b c d] | save save_test_3/list_sample.txt",
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "a\nb\nc\nd\n")
    })
}

#[test]
fn save_append_will_create_file_if_not_exists() {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("new-file.txt");

        nu!(
            cwd: dirs.root(),
            r#"'hello' | save --raw --append save_test_3/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "hello");
    })
}

#[test]
fn save_append_will_not_overwrite_content() {
    Playground::setup("save_test_4", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("new-file.txt");

        {
            let mut file =
                std::fs::File::create(&expected_file).expect("Failed to create test file");
            file.write_all("hello ".as_bytes())
                .expect("Failed to write to test file");
            file.flush().expect("Failed to flush io")
        }

        nu!(
            cwd: dirs.root(),
            r#"'world' | save --append save_test_4/new-file.txt"#,
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "hello world");
    })
}

#[test]
fn save_stderr_and_stdout_to_same_file() {
    Playground::setup("save_test_5", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let actual = nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = "bar";
            $env.BAZ = "ZZZ";
            do -c {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -r save_test_5/new-file.txt --stderr save_test_5/new-file.txt
            "#,
        );
        assert!(
            actual
                .err
                .contains("can't save both input and stderr input to the same file")
        );
    })
}

#[test]
fn save_stderr_and_stdout_to_diff_file() {
    Playground::setup("save_test_6", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = "bar";
            $env.BAZ = "ZZZ";
            do -c {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -r save_test_6/log.txt --stderr save_test_6/err.txt
            "#,
        );

        let actual = file_contents(expected_file);
        assert!(actual.contains("bar"));
        assert!(!actual.contains("ZZZ"));

        let actual = file_contents(expected_stderr_file);
        assert!(actual.contains("ZZZ"));
        assert!(!actual.contains("bar"));
    })
}

#[test]
fn save_string_and_stream_as_raw() {
    Playground::setup("save_test_7", |dirs, sandbox| {
        sandbox.with_files(&[]);
        let expected_file = dirs.test().join("temp.html");
        nu!(
            cwd: dirs.root(),
            r#"
            "<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>" | save save_test_7/temp.html
            "#,
        );
        let actual = file_contents(expected_file);
        assert_eq!(
            actual,
            r#"<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>"#
        )
    })
}

#[test]
fn save_not_override_file_by_default() {
    Playground::setup("save_test_8", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("log.txt")]);

        let actual = nu!(
            cwd: dirs.root(),
            r#""abcd" | save save_test_8/log.txt"#
        );
        assert!(actual.err.contains("Destination file already exists"));
    })
}

#[test]
fn save_override_works() {
    Playground::setup("save_test_9", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("log.txt")]);

        let expected_file = dirs.test().join("log.txt");
        nu!(
            cwd: dirs.root(),
            r#""abcd" | save save_test_9/log.txt -f"#
        );
        let actual = file_contents(expected_file);
        assert_eq!(actual, "abcd");
    })
}

#[test]
fn save_failure_not_overrides() {
    Playground::setup("save_test_10", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("result.toml", "Old content")]);

        let expected_file = dirs.test().join("result.toml");
        nu!(
            cwd: dirs.root(),
            // Writing number to file as toml fails
            "3 | save save_test_10/result.toml -f"
        );
        let actual = file_contents(expected_file);
        assert_eq!(actual, "Old content");
    })
}

#[test]
fn save_append_works_on_stderr() {
    Playground::setup("save_test_11", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            do -i {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -a -r save_test_11/log.txt --stderr save_test_11/err.txt"#,
        );

        let actual = file_contents(expected_file);
        assert_eq!(actual, "Old New\n");

        let actual = file_contents(expected_stderr_file);
        assert_eq!(actual, "Old Err New Err\n");
    })
}

#[test]
fn save_not_overrides_err_by_default() {
    Playground::setup("save_test_12", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("err.txt", "Old Err")]);

        let actual = nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            do -i {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -r save_test_12/log.txt --stderr save_test_12/err.txt"#,
        );

        assert!(actual.err.contains("Destination file already exists"));
    })
}

#[test]
fn save_override_works_stderr() {
    Playground::setup("save_test_13", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = "New";
            $env.BAZ = "New Err";
            do -i {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -f -r save_test_13/log.txt --stderr save_test_13/err.txt"#,
        );

        let actual = file_contents(expected_file);
        assert_eq!(actual, "New\n");

        let actual = file_contents(expected_stderr_file);
        assert_eq!(actual, "New Err\n");
    })
}

#[test]
fn save_list_stream() {
    Playground::setup("save_test_13", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.txt");

        nu!(
            cwd: dirs.root(),
            "[a b c d] | each {|i| $i} | save -r save_test_13/list_sample.txt",
        );

        let actual = file_contents(expected_file);
        assert_eq!(actual, "a\nb\nc\nd\n")
    })
}

#[test]
fn writes_out_range() {
    Playground::setup("save_test_14", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.json");

        nu!(
            cwd: dirs.root(),
            "1..3 | save save_test_14/list_sample.json",
        );

        let actual = file_contents(expected_file);
        println!("{actual}");
        assert_eq!(actual, "[\n  1,\n  2,\n  3\n]")
    })
}

// https://github.com/nushell/nushell/issues/10044
#[test]
fn save_file_correct_relative_path() {
    Playground::setup("save_test_15", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "test.nu",
            r#"
                export def main [] {
                    let foo = "foo"
                    mkdir bar
                    cd bar
                    'foo!' | save $foo
                }
            "#,
        )]);

        let expected_file = dirs.test().join("bar/foo");

        nu!(
            cwd: dirs.test(),
            r#"use test.nu; test"#
        );

        let actual = file_contents(expected_file);
        assert_eq!(actual, "foo!");
    })
}

#[test]
fn save_same_file_with_extension() {
    Playground::setup("save_test_16", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save --raw hello.md;
            open --raw hello.md
            | save --raw --force hello.md
        ");

        assert!(
            actual
                .err
                .contains("pipeline input and output are the same file")
        );
    })
}

#[test]
fn save_same_file_with_extension_pipeline() {
    Playground::setup("save_test_17", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save --raw hello.md;
            open --raw hello.md
            | prepend 'hello'
            | save --raw --force hello.md
        ");

        assert!(
            actual
                .err
                .contains("pipeline input and output are the same file")
        );
    })
}

#[test]
fn save_same_file_without_extension() {
    Playground::setup("save_test_18", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save hello;
            open hello
            | save --force hello
        ");

        assert!(
            actual
                .err
                .contains("pipeline input and output are the same file")
        );
    })
}

#[test]
fn save_same_file_without_extension_pipeline() {
    Playground::setup("save_test_19", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | save --force hello
        ");

        assert!(
            actual
                .err
                .contains("pipeline input and output are the same file")
        );
    })
}

#[test]
fn save_with_custom_converter() {
    Playground::setup("save_with_custom_converter", |dirs, _| {
        let file = dirs.test().join("test.ndjson");

        nu!(cwd: dirs.test(), r#"
            def "to ndjson" []: any -> string { each { to json --raw } | to text --no-newline } ;
            {a: 1, b: 2} | save test.ndjson
        "#);

        let actual = file_contents(file);
        assert_eq!(actual, r#"{"a":1,"b":2}"#);
    })
}

#[test]
fn save_same_file_with_collect() {
    Playground::setup("save_test_20", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | collect
            | save --force hello;
            open hello
        ");
        assert!(actual.status.success());
        assert_eq!("helloworld", actual.out);
    })
}

#[test]
fn save_same_file_with_collect_and_filter() {
    Playground::setup("save_test_21", |dirs, _sandbox| {
        let actual = nu!(cwd: dirs.test(), "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | collect
            | filter { true }
            | save --force hello;
            open hello
        ");
        assert!(actual.status.success());
        assert_eq!("helloworld", actual.out);
    })
}

#[test]
fn save_from_child_process_dont_sink_stderr() {
    Playground::setup("save_test_22", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let actual = nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            do -i {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -a -r save_test_22/log.txt"#,
        );
        assert_eq!(actual.err.trim_end(), " New Err");

        let actual = file_contents(expected_file);
        assert_eq!(actual.trim_end(), "Old New");

        let actual = file_contents(expected_stderr_file);
        assert_eq!(actual.trim_end(), "Old Err");
    })
}

#[test]
fn parent_redirection_doesnt_affect_save() {
    Playground::setup("save_test_23", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let actual = nu!(
            cwd: dirs.root(),
            r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            def tttt [] {
                do -i {nu -n -c 'nu --testbin echo_env FOO; nu --testbin echo_env_stderr BAZ'} | save -a -r save_test_23/log.txt
            };
            tttt e> ("save_test_23" | path join empty_file)"#
        );
        assert_eq!(actual.err.trim_end(), " New Err");

        let actual = file_contents(expected_file);
        assert_eq!(actual.trim_end(), "Old New");

        let actual = file_contents(expected_stderr_file);
        assert_eq!(actual.trim_end(), "Old Err");

        let actual = file_contents(dirs.test().join("empty_file"));
        assert_eq!(actual.trim_end(), "");
    })
}

#[test]
fn save_missing_parent_dir() {
    Playground::setup("save_test_24", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let actual = nu!(
            cwd: dirs.root(),
            r#"'hello' | save save_test_24/foobar/hello.txt"#,
        );

        assert!(actual.err.contains("nu::shell::io::directory_not_found"));
        assert!(actual.err.contains("foobar' does not exist"));
    })
}

#[test]
fn save_missing_ancestor_dir() {
    Playground::setup("save_test_24", |dirs, sandbox| {
        sandbox.with_files(&[]);

        std::fs::create_dir(dirs.test().join("foo"))
            .expect("should have been able to create subdir for test");

        let actual = nu!(
            cwd: dirs.root(),
            r#"'hello' | save save_test_24/foo/bar/baz/hello.txt"#,
        );

        assert!(actual.err.contains("nu::shell::io::directory_not_found"));
        assert!(actual.err.contains("bar' does not exist"));
    })
}

#[test]
fn force_save_to_dir() {
    let actual = nu!(cwd: "crates/nu-command/tests/commands", r#"
        "aaa" | save -f ..
        "#);

    assert!(actual.err.contains("nu::shell::io::is_a_directory"));
}
