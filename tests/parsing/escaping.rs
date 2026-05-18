use nu_test_support::prelude::*;
use rstest::rstest;

use nu_test_support::fs::Stub::FileWithContentToBeTrimmed;

#[rstest]
#[case::basic(r#""line1\nline2""#, "line1\nline2")]
#[case::basic(r#""col1\tcol2""#, "col1\tcol2")]
#[case::basic(r#""test\rmore""#, "test\rmore")]
#[case::basic(r#""path\\file""#, "path\\file")]
#[case::basic(r#""say \"hello\"""#, "say \"hello\"")]
#[case::basic(r#""it's \"quoted\"""#, "it's \"quoted\"")]
#[case::posix("\"x\\0y\"", "x\0y")]
#[case::posix("\"x\\ay\"", "x\u{7}y")]
#[case::posix("\"x\\by\"", "x\u{8}y")]
#[case::posix("\"x\\ey\"", "x\u{1b}y")]
#[case::posix("\"x\\fy\"", "x\u{c}y")]
#[case::posix("\"\\x41\\x42\\x43\"", "ABC")]
#[case::posix("\"hello\\x20world\"", "hello world")]
#[case::unicode(r#""\u{0041}""#, "A")]
#[case::unicode(r#""\u{1F600}""#, "😀")]
#[case::unicode(r#""\u{1234}""#, "ሴ")]
#[case::interpolation(
    r#"
        let name = "world"
        $"hello\n($name)"
    "#,
    "hello\nworld"
)]
#[case::interpolation(r#"$"tab\there""#, "tab\there")]
#[case::backward_compatible("\"line1\\nline2\"", "line1\nline2")]
#[case::backward_compatible("\"col1\\tcol2\"", "col1\tcol2")]
#[case::backward_compatible("\"path\\\\file\"", "path\\file")]
#[case::backward_compatible("\"say \\\"hello\\\"\"", "say \"hello\"")]
#[case::special_syntax_character("\"ignore\\$this\"", "ignore$this")]
#[case::special_syntax_character("\"not\\(expanded\\)\"", "not(expanded)")]
#[case::special_syntax_character("\"literal\\{curly\\}\"", "literal{curly}")]
fn escape_sequences_work(#[case] code: &str, #[case] expected: impl IntoValue) -> Result {
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case(r#""hello \u{6e""#, "missing closing '}'")]
#[case(r#""\u{110000}""#, "max codepoint 0x10FFFF")]
#[case(
    r#""\u{000000000000000000000000000000000000000000000037}""#,
    "must be 1-6 hex digits"
)]
#[case(r#""\x4""#, "incomplete hex escape")]
#[case(r#""\x4z""#, "invalid hex escape")]
#[case(r#""\q""#, "unrecognized escape sequence")]
fn invalid_escape_sequences_report_parse_errors(
    #[case] code: &str,
    #[case] expected: &str,
) -> Result {
    let err = format!("{:?}", test().run(code).expect_parse_error()?);
    assert!(
        err.contains(expected),
        "expected parse error containing {expected:?}, got {err:?}"
    );
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
