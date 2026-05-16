use nu_test_support::{fs::Stub::FileWithContentToBeTrimmed, prelude::*};

fn assert_parse_error_contains(code: &str, expected: &str) -> Result {
    let err = format!("{:?}", test().run(code).expect_parse_error()?);
    assert!(
        err.contains(expected),
        "expected parse error containing {expected:?}, got {err:?}"
    );
    Ok(())
}

#[test]
fn basic_escape_sequences_work() -> Result {
    test()
        .run("\"line1\\nline2\"")
        .expect_value_eq("line1\nline2")?;
    test()
        .run("\"col1\\tcol2\"")
        .expect_value_eq("col1\tcol2")?;
    test()
        .run("\"test\\rmore\"")
        .expect_value_eq("test\rmore")?;
    test()
        .run("\"path\\\\file\"")
        .expect_value_eq("path\\file")?;
    test()
        .run("\"say \\\"hello\\\"\"")
        .expect_value_eq("say \"hello\"")?;
    test()
        .run("\"it's \\\"quoted\\\"\"")
        .expect_value_eq("it's \"quoted\"")?;

    Ok(())
}

#[test]
fn posix_escape_sequences_work() -> Result {
    test().run("\"x\\0y\"").expect_value_eq("x\0y")?;
    test().run("\"x\\ay\"").expect_value_eq("x\u{7}y")?;
    test().run("\"x\\by\"").expect_value_eq("x\u{8}y")?;
    test().run("\"x\\ey\"").expect_value_eq("x\u{1b}y")?;
    test().run("\"x\\fy\"").expect_value_eq("x\u{c}y")?;
    test().run("\"\\x41\\x42\\x43\"").expect_value_eq("ABC")?;
    test()
        .run("\"hello\\x20world\"")
        .expect_value_eq("hello world")?;

    Ok(())
}

#[test]
fn unicode_escape_sequences_work() -> Result {
    test().run(r#""\u{0041}""#).expect_value_eq("A")?;
    test().run(r#""\u{1F600}""#).expect_value_eq("😀")?;
    test().run(r#""\u{1234}""#).expect_value_eq("ሴ")?;

    Ok(())
}

#[test]
fn invalid_escape_sequences_report_parse_errors() -> Result {
    assert_parse_error_contains(r#""hello \u{6e""#, "missing closing '}'")?;
    assert_parse_error_contains(r#""\u{110000}""#, "max codepoint 0x10FFFF")?;
    assert_parse_error_contains(
        r#""\u{000000000000000000000000000000000000000000000037}""#,
        "must be 1-6 hex digits",
    )?;
    assert_parse_error_contains(r#""\x4""#, "incomplete hex escape")?;
    assert_parse_error_contains(r#""\x4z""#, "invalid hex escape")?;
    assert_parse_error_contains(r#""\q""#, "unrecognized escape sequence")?;

    Ok(())
}

#[test]
fn interpolation_escape_sequences_work() -> Result {
    test()
        .run(
            r#"
                let name = "world"
                $"hello\n($name)"
            "#,
        )
        .expect_value_eq("hello\nworld")?;

    test().run(r#"$"tab\there""#).expect_value_eq("tab\there")?;

    Ok(())
}

#[test]
fn external_command_escape_sequences_work() -> Result {
    test()
        .add_nu_to_path()
        .run(
            r#"
                let quoted_text = "hello\nworld"
                nu --testbin cococo $quoted_text | str contains (char newline)
            "#,
        )
        .expect_value_eq(true)
}

#[test]
fn escaped_glob_pattern_reports_current_error() -> Result {
    Playground::setup(
        "escaped_glob_pattern_reports_current_error",
        |dirs, sandbox| {
            sandbox.with_files(&[
                FileWithContentToBeTrimmed("file[1].txt", "literal"),
                FileWithContentToBeTrimmed("file1.txt", "pattern"),
            ]);

            let mut tester = test().cwd(dirs.test());
            let err = match tester.run::<Value>(r#"ls file\[1\].txt"#) {
                Ok(value) => panic!("expected escaped glob path to fail, got {value:?}"),
                Err(err) => err.to_string(),
            };

            assert!(
                err.contains("unrecognized escape")
                    || (err.contains("No matches found for Expand(")
                        && err.contains("Pattern, file or folder not found"))
                    || (err.contains("Error extracting glob pattern")
                        && err.contains("invalid range pattern")),
                "expected escaped glob error, got {err:?}"
            );

            Ok(())
        },
    )
}

#[test]
fn quoted_glob_path_stays_literal() -> Result {
    Playground::setup("quoted_glob_path_stays_literal", |dirs, sandbox| {
        sandbox.with_files(&[
            FileWithContentToBeTrimmed("file[1].txt", "literal"),
            FileWithContentToBeTrimmed("file1.txt", "pattern"),
        ]);

        test()
            .cwd(dirs.test())
            .run(
                r#"
                    let quoted = "file[1].txt"
                    ls $quoted | get name | path basename
                "#,
            )
            .expect_value_eq(["file[1].txt"])
    })
}

#[test]
fn backward_compatible_escape_sequences_still_work() -> Result {
    test()
        .run("\"line1\\nline2\"")
        .expect_value_eq("line1\nline2")?;
    test()
        .run("\"col1\\tcol2\"")
        .expect_value_eq("col1\tcol2")?;
    test()
        .run("\"path\\\\file\"")
        .expect_value_eq("path\\file")?;
    test()
        .run("\"say \\\"hello\\\"\"")
        .expect_value_eq("say \"hello\"")?;

    Ok(())
}

#[test]
fn special_syntax_character_escaping_works() -> Result {
    test()
        .run("\"ignore\\$this\"")
        .expect_value_eq("ignore$this")?;
    test()
        .run("\"not\\(expanded\\)\"")
        .expect_value_eq("not(expanded)")?;
    test()
        .run("\"literal\\{curly\\}\"")
        .expect_value_eq("literal{curly}")?;

    Ok(())
}
