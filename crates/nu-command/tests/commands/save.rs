use nu_test_support::prelude::*;
use nu_utils::consts::LINE_SEPARATOR_STR;
use std::{fs, io::Write};

#[test]
fn writes_out_csv(playground: Playground) -> Result {
    let expected_file = playground.path().join("cargo_sample.csv");

    let () = test()
        .cwd(playground.path())
        .run(r#"[[name, version, description, license, edition]; [nu, "0.14", "A new type of shell", "MIT", "2018"]] | save cargo_sample.csv"#)?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("nu,0.14,A new type of shell,MIT,2018"));
    Ok(())
}

#[test]
fn writes_out_list(playground: Playground) -> Result {
    let expected_file = playground.path().join("list_sample.txt");

    let () = test()
        .cwd(playground.path())
        .run("[a b c d] | save list_sample.txt")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, ["a", "b", "c", "d", ""].join(LINE_SEPARATOR_STR));
    Ok(())
}

#[test]
fn writes_structured_data_as_text(playground: Playground) -> Result {
    let expected_file = playground.path().join("structured.txt");

    let () = test()
        .cwd(playground.path())
        .run("[[a, b]; [1, 2]] | save structured.txt")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, ["a: 1, b: 2", ""].join(LINE_SEPARATOR_STR));
    Ok(())
}

#[test]
fn saves_explicit_table_rendering_as_is(playground: Playground) -> Result {
    let expected_file = playground.path().join("rendered.txt");

    let () = test().cwd(playground.path()).run(
        "
            $env.config.use_ansi_coloring = true
            [[name]; [value]] | table | save rendered.txt
        ",
    )?;

    let actual = fs::read_to_string(expected_file)?;
    assert_contains("\u{1b}[", &actual);
    assert_contains("value", actual);
    Ok(())
}

#[test]
fn custom_txt_converter_takes_precedence(playground: Playground) -> Result {
    let expected_file = playground.path().join("custom.txt");

    let () = test().cwd(playground.path()).run(
        r#"
            def "to txt" []: any -> string { "custom txt" }
            [[a, b]; [1, 2]] | save custom.txt
        "#,
    )?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "custom txt");
    Ok(())
}

#[test]
fn unknown_extension_does_not_default_to_text(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("[[a, b]; [1, 2]] | save structured.unknown")
        .expect_error()?;

    // No serializer for the extension: report unsupported input with an
    // actionable message pointing at the explicit conversions.
    assert_contains("Unsupported input", err.to_string());
    assert_contains("to json", format!("{err:?}"));
    assert!(!playground.path().join("structured.unknown").exists());
    Ok(())
}

#[test]
fn no_extension_gives_actionable_error_for_structured_data(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("[[a, b]; [1, 2]] | save structured")
        .expect_error()?;

    assert_contains("Unsupported input", err.to_string());
    assert_contains("to json", format!("{err:?}"));
    Ok(())
}

#[test]
fn save_append_will_create_file_if_not_exists(playground: Playground) -> Result {
    let expected_file = playground.path().join("new-file.txt");

    let () = test()
        .cwd(playground.path())
        .run("'hello' | save --raw --append new-file.txt")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "hello");
    Ok(())
}

#[test]
fn save_append_will_not_overwrite_content(playground: Playground) -> Result {
    let expected_file = playground.path().join("new-file.txt");

    {
        let mut file = std::fs::File::create(&expected_file).expect("Failed to create test file");
        file.write_all("hello ".as_bytes())
            .expect("Failed to write to test file");
        file.flush().expect("Failed to flush io")
    }

    let () = test()
        .cwd(playground.path())
        .run("'world' | save --append new-file.txt")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "hello world");
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_stderr_and_stdout_to_same_file(playground: Playground) -> Result {
    let code = r#"
        $env.FOO = "bar";
        $env.BAZ = "ZZZ";
        echo_env_mixed out-err FOO BAZ | save -r new-file.txt --stderr new-file.txt
    "#;

    let err = test().cwd(playground.path()).run(code).expect_error()?;
    assert_contains("input and stderr input to same file", err.to_string());
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_stderr_and_stdout_to_diff_file(playground: Playground) -> Result {
    let expected_file = playground.path().join("log.txt");
    let expected_stderr_file = playground.path().join("err.txt");

    let code = r#"
        $env.FOO = "bar";
        $env.BAZ = "ZZZ";
        echo_env_mixed out-err FOO BAZ | save -r log.txt --stderr err.txt
    "#;

    let () = test().cwd(playground.path()).run(code)?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("bar"));
    assert!(!actual.contains("ZZZ"));

    let actual = fs::read_to_string(expected_stderr_file)?;
    assert!(actual.contains("ZZZ"));
    assert!(!actual.contains("bar"));
    Ok(())
}

#[test]
fn save_string_and_stream_as_raw(playground: Playground) -> Result {
    let expected_file = playground.path().join("temp.html");
    let () = test()
        .cwd(playground.path())
        .run(r#"
            "<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>" | save temp.html
        "#)?;
    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(
        actual,
        "<!DOCTYPE html><html><body><a href='http://example.org/'>Example</a></body></html>"
    );
    Ok(())
}

#[test]
fn save_not_override_file_by_default(playground: Playground) -> Result {
    playground.empty_file("log.txt")?;

    let err = test()
        .cwd(playground.path())
        .run(r#""abcd" | save log.txt"#)
        .expect_error()?;
    assert_contains("Destination file already exists", err.to_string());
    Ok(())
}

#[test]
fn save_override_works(playground: Playground) -> Result {
    playground.empty_file("log.txt")?;

    let expected_file = playground.path().join("log.txt");
    let () = test()
        .cwd(playground.path())
        .run(r#""abcd" | save log.txt -f"#)?;
    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "abcd");
    Ok(())
}

#[test]
fn save_failure_not_overrides(playground: Playground) -> Result {
    playground.file("result.toml", "Old content")?;

    let expected_file = playground.path().join("result.toml");
    let _ = test()
        .cwd(playground.path())
        // Writing number to file as toml fails
        .run("3 | save result.toml -f")
        .expect_error()?;
    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "Old content");
    Ok(())
}

#[test]
fn save_preserves_toml_comment_and_inline_table_after_update(playground: Playground) -> Result {
    playground.file(
        "sample.toml",
        r#"# keep this comment
            [package]
            name = "demo"
            version = "0.1.0"
            metadata = { repo = "https://example.com", keywords = ["alpha", "beta"] }
        "#,
    )?;

    let expected_file = playground.path().join("out.toml");

    let () = test()
        .cwd(playground.path())
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
}

#[test]
fn save_preserves_toml_array_of_tables_comments(playground: Playground) -> Result {
    playground.file(
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
    )?;

    let expected_file = playground.path().join("out.toml");

    let () = test()
        .cwd(playground.path())
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
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_append_works_on_stderr(playground: Playground) -> Result {
    playground.file("log.txt", "Old")?;
    playground.file("err.txt", "Old Err")?;

    let expected_file = playground.path().join("log.txt");
    let expected_stderr_file = playground.path().join("err.txt");

    let () = test().cwd(playground.path()).run(
        r#"
            $env.FOO = " New";
            $env.BAZ = " New Err";
            echo_env_mixed out-err FOO BAZ | save -a -r log.txt --stderr err.txt"#,
    )?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "Old New\n");

    let actual = fs::read_to_string(expected_stderr_file)?;
    assert_eq!(actual, "Old Err New Err\n");
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_not_overrides_err_by_default(playground: Playground) -> Result {
    playground.file("err.txt", "Old Err")?;

    let code = r#"
        $env.FOO = " New";
        $env.BAZ = " New Err";
        echo_env_mixed out-err FOO BAZ | save -r log.txt --stderr err.txt
    "#;

    let err = test().cwd(playground.path()).run(code).expect_error()?;

    assert_contains("Destination file already exists", err.to_string());
    Ok(())
}

#[test]
#[deps(TESTBIN_ECHO_ENV_MIXED)]
fn save_override_works_stderr(playground: Playground) -> Result {
    playground.file("log.txt", "Old")?;
    playground.file("err.txt", "Old Err")?;

    let expected_file = playground.path().join("log.txt");
    let expected_stderr_file = playground.path().join("err.txt");

    let code = r#"
        $env.FOO = "New";
        $env.BAZ = "New Err";
        echo_env_mixed out-err FOO BAZ | save -f -r log.txt --stderr err.txt
    "#;

    let () = test().cwd(playground.path()).run(code)?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "New\n");

    let actual = fs::read_to_string(expected_stderr_file)?;
    assert_eq!(actual, "New Err\n");
    Ok(())
}

#[test]
fn save_list_stream(playground: Playground) -> Result {
    let expected_file = playground.path().join("list_sample.txt");

    let () = test()
        .cwd(playground.path())
        .run("[a b c d] | each {|i| $i} | save -r list_sample.txt")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "a\nb\nc\nd\n");
    Ok(())
}

#[test]
fn writes_out_range(playground: Playground) -> Result {
    let expected_file = playground.path().join("list_sample.json");

    let () = test()
        .cwd(playground.path())
        .run("1..3 | save list_sample.json")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "[\n  1,\n  2,\n  3\n]");
    Ok(())
}

// https://github.com/nushell/nushell/issues/10044
#[test]
fn save_file_correct_relative_path(playground: Playground) -> Result {
    playground.file(
        "test.nu",
        r#"
            export def main [] {
                let foo = "foo"
                mkdir bar
                cd bar
                'foo!' | save $foo
            }
        "#,
    )?;

    let expected_file = playground.path().join("bar/foo");

    let () = test().cwd(playground.path()).run("use test.nu; test")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual, "foo!");
    Ok(())
}

#[test]
fn save_same_file_with_extension(playground: Playground) -> Result {
    let code = "
        echo 'world'
        | save --raw hello.md;
        open --raw hello.md
        | save --raw --force hello.md
    ";

    let err = test().cwd(playground.path()).run(code).expect_error()?;

    assert_contains(
        "pipeline input and output are the same file",
        err.to_string(),
    );
    Ok(())
}

#[test]
fn save_same_file_with_extension_pipeline(playground: Playground) -> Result {
    let code = "
        echo 'world'
        | save --raw hello.md;
        open --raw hello.md
        | prepend 'hello'
        | save --raw --force hello.md
    ";

    let err = test().cwd(playground.path()).run(code).expect_error()?;

    assert_contains(
        "pipeline input and output are the same file",
        err.to_string(),
    );
    Ok(())
}

#[test]
fn save_same_file_without_extension(playground: Playground) -> Result {
    let code = "
        echo 'world'
        | save hello;
        open hello
        | save --force hello
    ";

    let err = test().cwd(playground.path()).run(code).expect_error()?;

    assert_contains(
        "pipeline input and output are the same file",
        err.to_string(),
    );
    Ok(())
}

#[test]
fn save_same_file_without_extension_pipeline(playground: Playground) -> Result {
    let code = "
        echo 'world'
        | save hello;
        open hello
        | prepend 'hello'
        | save --force hello
    ";

    let err = test().cwd(playground.path()).run(code).expect_error()?;

    assert_contains(
        "pipeline input and output are the same file",
        err.to_string(),
    );
    Ok(())
}

#[test]
fn save_with_custom_converter(playground: Playground) -> Result {
    let file = playground.path().join("test.ndjson");

    let code = r#"
        def "to ndjson" []: any -> string { each { to json --raw } | to text --no-newline } ;
        {a: 1, b: 2} | save test.ndjson
    "#;

    let () = test().cwd(playground.path()).run(code)?;

    let actual = fs::read_to_string(file)?;
    assert_eq!(actual, r#"{"a":1,"b":2}"#);
    Ok(())
}

#[test]
fn save_same_file_with_collect(playground: Playground) -> Result {
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
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("hello\nworld\n")
}

#[test]
fn save_same_file_with_collect_and_filter(playground: Playground) -> Result {
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
        .cwd(playground.path())
        .run(code)
        .expect_value_eq("hello\nworld\n")
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn save_from_child_process_dont_sink_stderr(playground: Playground) -> Result {
    playground.file("log.txt", "Old")?;
    playground.file("err.txt", "Old Err")?;

    let expected_file = playground.path().join("log.txt");
    let expected_stderr_file = playground.path().join("err.txt");

    let code = r#"
        $env.FOO = " New";
        $env.BAZ = " New Err";
        echo_env_mixed out-err FOO BAZ | save -a -r log.txt
    "#;

    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_eq!(result.stderr.trim_end(), " New Err");

    let actual = fs::read_to_string(expected_file)?;
    assert_eq!(actual.trim_end(), "Old New");

    let actual = fs::read_to_string(expected_stderr_file)?;
    assert_eq!(actual.trim_end(), "Old Err");
    Ok(())
}

#[test]
#[deps(NU, TESTBIN_ECHO_ENV_MIXED)]
fn parent_redirection_doesnt_affect_save(playground: Playground) -> Result {
    playground.file("log.txt", "Old")?;
    playground.file("err.txt", "Old Err")?;

    let expected_file = playground.path().join("log.txt");
    let expected_stderr_file = playground.path().join("err.txt");

    let code = r#"
        $env.FOO = " New";
        $env.BAZ = " New Err";
        def tttt [] {
            echo_env_mixed out-err FOO BAZ | save -a -r log.txt
        };
        tttt e> empty_file
    "#;

    let result: CompleteResult = test()
        .cwd(playground.path())
        .run_with_data("let code; nu -n -c $code | complete", code)?;
    assert_eq!(result.stderr.trim_end(), " New Err");

    assert_eq!(fs::read_to_string(expected_file)?.trim_end(), "Old New");
    assert_eq!(
        fs::read_to_string(expected_stderr_file)?.trim_end(),
        "Old Err"
    );
    assert_eq!(
        fs::read_to_string(playground.path().join("empty_file"))?.trim_end(),
        ""
    );
    Ok(())
}

#[test]
fn save_missing_parent_dir(playground: Playground) -> Result {
    let err = test()
        .cwd(playground.path())
        .run("'hello' | save foobar/hello.txt")
        .expect_error()?;

    assert_contains("Directory not found", err.to_string());
    Ok(())
}

#[test]
fn save_missing_ancestor_dir(playground: Playground) -> Result {
    std::fs::create_dir(playground.path().join("foo"))
        .expect("should have been able to create subdir for test");

    let err = test()
        .cwd(playground.path())
        .run("'hello' | save foo/bar/baz/hello.txt")
        .expect_error()?;

    assert_contains("Directory not found", err.to_string());
    Ok(())
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
fn save_table_to_csv_with_explicit_columns(playground: Playground) -> Result {
    let expected_file = playground.path().join("test.csv");

    let () = test()
        .cwd(playground.path())
        .run("[[a b]; [1 2] [3 4]] | to csv --columns [a b] | save -f test.csv")?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("a,b"));
    assert!(actual.contains("1,2"));
    assert!(actual.contains("3,4"));
    Ok(())
}

#[test]
fn save_table_to_csv_without_explicit_columns(playground: Playground) -> Result {
    let expected_file = playground.path().join("test.csv");

    let () = test()
        .cwd(playground.path())
        .run("[[a b]; [1 2] [3 4]] | to csv | save -f test.csv")?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("a,b"));
    assert!(actual.contains("1,2"));
    assert!(actual.contains("3,4"));
    Ok(())
}

#[test]
fn save_record_to_csv(playground: Playground) -> Result {
    let expected_file = playground.path().join("test.csv");

    let () = test()
        .cwd(playground.path())
        .run("{a: 1, b: 2} | to csv | save -f test.csv")?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("a,b"));
    assert!(actual.contains("1,2"));
    Ok(())
}

#[test]
fn save_table_to_tsv(playground: Playground) -> Result {
    let expected_file = playground.path().join("test.tsv");

    let () = test()
        .cwd(playground.path())
        .run("[[a b]; [1 2] [3 4]] | to tsv | save -f test.tsv")?;

    let actual = fs::read_to_string(expected_file)?;
    assert!(actual.contains("a\tb"));
    assert!(actual.contains("1\t2"));
    assert!(actual.contains("3\t4"));
    Ok(())
}

#[test]
fn save_streaming_list_stream_to_csv(playground: Playground) -> Result {
    // Exercises the streaming path (ListStream -> ByteStream -> save) rather than
    // the materialized table path, ensuring rows are streamed to disk progressively.
    let expected_file = playground.path().join("test.csv");

    let () = test()
        .cwd(playground.path())
        .run("1..5 | each { |i| {a: $i, b: ($i * 10)} } | to csv | save -f test.csv")?;

    let actual = fs::read_to_string(expected_file)?;
    assert_contains("a,b", &actual);
    assert_contains("1,10", &actual);
    assert_contains("5,50", &actual);
    Ok(())
}
