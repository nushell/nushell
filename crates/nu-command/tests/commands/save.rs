use nu_test_support::{fs::Stub, prelude::*};
use std::{fs, io::Write};

#[test]
fn writes_out_csv() -> Result {
    Playground::setup("save_test_2", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("cargo_sample.csv");

        let () = test()
            .cwd(dirs.root())
            .run(r#"[[name, version, description, license, edition]; [nu, "0.14", "A new type of shell", "MIT", "2018"]] | save save_test_2/cargo_sample.csv"#)?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("nu,0.14,A new type of shell,MIT,2018"));
        Ok(())
    })
}

#[test]
fn writes_out_list() -> Result {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.txt");

        let () = test()
            .cwd(dirs.root())
            .run("[a b c d] | save save_test_3/list_sample.txt")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "a\nb\nc\nd\n");
        Ok(())
    })
}

#[test]
fn save_append_will_create_file_if_not_exists() -> Result {
    Playground::setup("save_test_3", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("new-file.txt");

        let () = test()
            .cwd(dirs.root())
            .run("'hello' | save --raw --append save_test_3/new-file.txt")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "hello");
        Ok(())
    })
}

#[test]
fn save_append_will_not_overwrite_content() -> Result {
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

        let () = test()
            .cwd(dirs.root())
            .run("'world' | save --append save_test_4/new-file.txt")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "hello world");
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_stderr_and_stdout_to_same_file() -> Result {
    Playground::setup("save_test_5", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let code = r#"
            $env.FOO = "bar";
            $env.BAZ = "ZZZ";
            echo_env_mixed out-err FOO BAZ | save -r save_test_5/new-file.txt --stderr save_test_5/new-file.txt
        "#;

        let err = test().cwd(dirs.root()).run(code).expect_error()?;
        assert_contains("input and stderr input to same file", err.to_string());
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_stderr_and_stdout_to_diff_file() -> Result {
    Playground::setup("save_test_6", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let code = r#"
            $env.FOO = "bar";
            $env.BAZ = "ZZZ";
            echo_env_mixed out-err FOO BAZ | save -r save_test_6/log.txt --stderr save_test_6/err.txt
        "#;

        let () = test().cwd(dirs.root()).run(code)?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("bar"));
        assert!(!actual.contains("ZZZ"));

        let actual = fs::read_to_string(expected_stderr_file)?;
        assert!(actual.contains("ZZZ"));
        assert!(!actual.contains("bar"));
        Ok(())
    })
}

#[test]
fn save_string_and_stream_as_raw() -> Result {
    Playground::setup("save_test_7", |dirs, sandbox| {
        sandbox.with_files(&[]);
        let expected_file = dirs.test().join("temp.html");
        let () = test()
            .cwd(dirs.root())
            .run(r#"
            "<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>" | save save_test_7/temp.html
            "#)?;
        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(
            actual,
            "<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>"
        );
        Ok(())
    })
}

#[test]
fn save_not_override_file_by_default() -> Result {
    Playground::setup("save_test_8", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("log.txt")]);

        let err = test()
            .cwd(dirs.root())
            .run(r#""abcd" | save save_test_8/log.txt"#)
            .expect_error()?;
        assert_contains("Destination file already exists", err.to_string());
        Ok(())
    })
}

#[test]
fn save_override_works() -> Result {
    Playground::setup("save_test_9", |dirs, sandbox| {
        sandbox.with_files(&[Stub::EmptyFile("log.txt")]);

        let expected_file = dirs.test().join("log.txt");
        let () = test()
            .cwd(dirs.root())
            .run(r#""abcd" | save save_test_9/log.txt -f"#)?;
        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "abcd");
        Ok(())
    })
}

#[test]
fn save_failure_not_overrides() -> Result {
    Playground::setup("save_test_10", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("result.toml", "Old content")]);

        let expected_file = dirs.test().join("result.toml");
        let _ = test()
            .cwd(dirs.root())
            // Writing number to file as toml fails
            .run("3 | save save_test_10/result.toml -f")
            .expect_error()?;
        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "Old content");
        Ok(())
    })
}

#[test]
fn save_preserves_toml_comment_and_inline_table_after_update() -> Result {
    Playground::setup("save_test_10_toml_preservation", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "sample.toml",
            r#"# keep this comment
[package]
name = "demo"
version = "0.1.0"
metadata = { repo = "https://example.com", keywords = ["alpha", "beta"] }
"#,
        )]);

        let expected_file = dirs.test().join("out.toml");

        let () = test()
            .cwd(dirs.test())
            .run("open sample.toml | update package.version '0.2.0' | save -f out.toml")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(
            actual,
            r#"# keep this comment
[package]
name = "demo"
version = "0.2.0"
metadata = { repo = "https://example.com", keywords = ["alpha", "beta"] }
"#
        );
        Ok(())
    })
}

#[test]
fn save_preserves_toml_array_of_tables_comments() -> Result {
    Playground::setup("save_test_toml_aot_preservation", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent(
            "sample.toml",
            r#"# project config
[settings]
verbose = true

# first item
[[items]]
name = "alpha"
value = 1

# second item
[[items]]
name = "beta"
value = 2
"#,
        )]);

        let expected_file = dirs.test().join("out.toml");

        let () = test()
            .cwd(dirs.test())
            .run("open sample.toml | update items.0.value 99 | save -f out.toml")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(
            actual,
            r#"# project config
[settings]
verbose = true

# first item
[[items]]
name = "alpha"
value = 99

# second item
[[items]]
name = "beta"
value = 2
"#
        );
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_append_works_on_stderr() -> Result {
    Playground::setup("save_test_11", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let () = test()
            .cwd(dirs.root())
            .run(r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            echo_env_mixed out-err FOO BAZ | save -a -r save_test_11/log.txt --stderr save_test_11/err.txt"#)?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "Old New\n");

        let actual = fs::read_to_string(expected_stderr_file)?;
        assert_eq!(actual, "Old Err New Err\n");
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_not_overrides_err_by_default() -> Result {
    Playground::setup("save_test_12", |dirs, sandbox| {
        sandbox.with_files(&[Stub::FileWithContent("err.txt", "Old Err")]);

        let code = r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            echo_env_mixed out-err FOO BAZ | save -r save_test_12/log.txt --stderr save_test_12/err.txt
        "#;

        let err = test().cwd(dirs.root()).run(code).expect_error()?;

        assert_contains("Destination file already exists", err.to_string());
        Ok(())
    })
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_override_works_stderr() -> Result {
    Playground::setup("save_test_13", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let code = r#"
            $env.FOO = "New";
            $env.BAZ = "New Err";
            echo_env_mixed out-err FOO BAZ | save -f -r save_test_13/log.txt --stderr save_test_13/err.txt
        "#;

        let () = test().cwd(dirs.root()).run(code)?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "New\n");

        let actual = fs::read_to_string(expected_stderr_file)?;
        assert_eq!(actual, "New Err\n");
        Ok(())
    })
}

#[test]
fn save_list_stream() -> Result {
    Playground::setup("save_test_13", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.txt");

        let () = test()
            .cwd(dirs.root())
            .run("[a b c d] | each {|i| $i} | save -r save_test_13/list_sample.txt")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "a\nb\nc\nd\n");
        Ok(())
    })
}

#[test]
fn writes_out_range() -> Result {
    Playground::setup("save_test_14", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("list_sample.json");

        let () = test()
            .cwd(dirs.root())
            .run("1..3 | save save_test_14/list_sample.json")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "[\n  1,\n  2,\n  3\n]");
        Ok(())
    })
}

// https://github.com/nushell/nushell/issues/10044
#[test]
fn save_file_correct_relative_path() -> Result {
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

        let () = test().cwd(dirs.test()).run("use test.nu; test")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual, "foo!");
        Ok(())
    })
}

#[test]
fn save_same_file_with_extension() -> Result {
    Playground::setup("save_test_16", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save --raw hello.md;
            open --raw hello.md
            | save --raw --force hello.md
        ";

        let err = test().cwd(dirs.test()).run(code).expect_error()?;

        assert_contains(
            "pipeline input and output are the same file",
            err.to_string(),
        );
        Ok(())
    })
}

#[test]
fn save_same_file_with_extension_pipeline() -> Result {
    Playground::setup("save_test_17", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save --raw hello.md;
            open --raw hello.md
            | prepend 'hello'
            | save --raw --force hello.md
        ";

        let err = test().cwd(dirs.test()).run(code).expect_error()?;

        assert_contains(
            "pipeline input and output are the same file",
            err.to_string(),
        );
        Ok(())
    })
}

#[test]
fn save_same_file_without_extension() -> Result {
    Playground::setup("save_test_18", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save hello;
            open hello
            | save --force hello
        ";

        let err = test().cwd(dirs.test()).run(code).expect_error()?;

        assert_contains(
            "pipeline input and output are the same file",
            err.to_string(),
        );
        Ok(())
    })
}

#[test]
fn save_same_file_without_extension_pipeline() -> Result {
    Playground::setup("save_test_19", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | save --force hello
        ";

        let err = test().cwd(dirs.test()).run(code).expect_error()?;

        assert_contains(
            "pipeline input and output are the same file",
            err.to_string(),
        );
        Ok(())
    })
}

#[test]
fn save_with_custom_converter() -> Result {
    Playground::setup("save_with_custom_converter", |dirs, _| {
        let file = dirs.test().join("test.ndjson");

        let code = r#"
            def "to ndjson" []: any -> string { each { to json --raw } | to text --no-newline } ;
            {a: 1, b: 2} | save test.ndjson
        "#;

        let () = test().cwd(dirs.test()).run(code)?;

        let actual = fs::read_to_string(file)?;
        assert_eq!(actual, r#"{"a":1,"b":2}"#);
        Ok(())
    })
}

#[test]
fn save_same_file_with_collect() -> Result {
    Playground::setup("save_test_20", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | collect
            | save --force hello;
            open hello
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("hello\nworld\n")
    })
}

#[test]
fn save_same_file_with_collect_and_filter() -> Result {
    Playground::setup("save_test_21", |dirs, _sandbox| {
        let code = "
            echo 'world'
            | save hello;
            open hello
            | prepend 'hello'
            | collect
            | filter { true }
            | save --force hello;
            open hello
        ";

        test()
            .cwd(dirs.test())
            .run(code)
            .expect_value_eq("hello\nworld\n")
    })
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn save_from_child_process_dont_sink_stderr() -> Result {
    Playground::setup("save_test_22", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let code = r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            echo_env_mixed out-err FOO BAZ | save -a -r save_test_22/log.txt
        "#;

        let result: CompleteResult = test()
            .cwd(dirs.root())
            .run_with_data("let code; nu -n -c $code | complete", code)?;
        assert_eq!(result.stderr.trim_end(), " New Err");

        let actual = fs::read_to_string(expected_file)?;
        assert_eq!(actual.trim_end(), "Old New");

        let actual = fs::read_to_string(expected_stderr_file)?;
        assert_eq!(actual.trim_end(), "Old Err");
        Ok(())
    })
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn parent_redirection_doesnt_affect_save() -> Result {
    Playground::setup("save_test_23", |dirs, sandbox| {
        sandbox.with_files(&[
            Stub::FileWithContent("log.txt", "Old"),
            Stub::FileWithContent("err.txt", "Old Err"),
        ]);

        let expected_file = dirs.test().join("log.txt");
        let expected_stderr_file = dirs.test().join("err.txt");

        let code = r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            def tttt [] {
                echo_env_mixed out-err FOO BAZ | save -a -r save_test_23/log.txt
            };
            tttt e> ("save_test_23" | path join empty_file)
        "#;

        let result: CompleteResult = test()
            .cwd(dirs.root())
            .run_with_data("let code; nu -n -c $code | complete", code)?;
        assert_eq!(result.stderr.trim_end(), " New Err");

        assert_eq!(fs::read_to_string(expected_file)?.trim_end(), "Old New");
        assert_eq!(
            fs::read_to_string(expected_stderr_file)?.trim_end(),
            "Old Err"
        );
        assert_eq!(
            fs::read_to_string(dirs.test().join("empty_file"))?.trim_end(),
            ""
        );
        Ok(())
    })
}

#[test]
fn save_missing_parent_dir() -> Result {
    Playground::setup("save_test_24", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let err = test()
            .cwd(dirs.root())
            .run("'hello' | save save_test_24/foobar/hello.txt")
            .expect_error()?;

        assert_contains("Directory not found", err.to_string());
        Ok(())
    })
}

#[test]
fn save_missing_ancestor_dir() -> Result {
    Playground::setup("save_test_24", |dirs, sandbox| {
        sandbox.with_files(&[]);

        std::fs::create_dir(dirs.test().join("foo"))
            .expect("should have been able to create subdir for test");

        let err = test()
            .cwd(dirs.root())
            .run("'hello' | save save_test_24/foo/bar/baz/hello.txt")
            .expect_error()?;

        assert_contains("Directory not found", err.to_string());
        Ok(())
    })
}

#[test]
fn force_save_to_dir() -> Result {
    let err = test()
        .cwd("crates/nu-command/tests/commands")
        .run(
            r#"
        "aaa" | save -f ..
        "#,
        )
        .expect_error()?;

    assert_contains("I/O error", err.to_string());
    Ok(())
}

#[test]
fn save_table_to_csv_with_explicit_columns() -> Result {
    Playground::setup("save_table_csv", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("test.csv");

        let () = test().cwd(dirs.root()).run(
            "[[a b]; [1 2] [3 4]] | to csv --columns [a b] | save -f save_table_csv/test.csv",
        )?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("a,b"));
        assert!(actual.contains("1,2"));
        assert!(actual.contains("3,4"));
        Ok(())
    })
}

#[test]
fn save_table_to_csv_without_explicit_columns() -> Result {
    Playground::setup("save_table_csv_auto", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("test.csv");

        let () = test()
            .cwd(dirs.root())
            .run("[[a b]; [1 2] [3 4]] | to csv | save -f save_table_csv_auto/test.csv")?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("a,b"));
        assert!(actual.contains("1,2"));
        assert!(actual.contains("3,4"));
        Ok(())
    })
}

#[test]
fn save_record_to_csv() -> Result {
    Playground::setup("save_record_csv", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("test.csv");

        let () = test()
            .cwd(dirs.root())
            .run("{a: 1, b: 2} | to csv | save -f save_record_csv/test.csv")?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("a,b"));
        assert!(actual.contains("1,2"));
        Ok(())
    })
}

#[test]
fn save_table_to_tsv() -> Result {
    Playground::setup("save_table_tsv", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("test.tsv");

        let () = test()
            .cwd(dirs.root())
            .run("[[a b]; [1 2] [3 4]] | to tsv | save -f save_table_tsv/test.tsv")?;

        let actual = fs::read_to_string(expected_file)?;
        assert!(actual.contains("a\tb"));
        assert!(actual.contains("1\t2"));
        assert!(actual.contains("3\t4"));
        Ok(())
    })
}

#[test]
fn save_streaming_list_stream_to_csv() -> Result {
    // Exercises the streaming path (ListStream -> ByteStream -> save) rather than
    // the materialized table path, ensuring rows are streamed to disk progressively.
    Playground::setup("save_streaming_csv", |dirs, sandbox| {
        sandbox.with_files(&[]);

        let expected_file = dirs.test().join("test.csv");

        let () = test()
            .cwd(dirs.root())
            .run("1..5 | each { |i| {a: $i, b: ($i * 10)} } | to csv | save -f save_streaming_csv/test.csv")?;

        let actual = fs::read_to_string(expected_file)?;
        assert_contains("a,b", &actual);
        assert_contains("1,10", &actual);
        assert_contains("5,50", &actual);
        Ok(())
    })
}
