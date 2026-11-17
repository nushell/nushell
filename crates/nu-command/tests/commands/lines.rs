use nu_protocol::{
    ByteStream, PipelineData, ShellError, Signals, Span, shell_error::io::ErrorKind,
};
use nu_test_support::prelude::*;

#[test]
fn lines() -> Result {
    let code = r#"
        open cargo_sample.toml -r
        | lines
        | skip while {|it| $it != "[dependencies]" }
        | skip 1
        | first
        | split column "="
        | get column0.0
        | str trim
    "#;

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq("rustyline")
}

#[test]
fn lines_proper_buffering() -> Result {
    let code = "
        open lines_test.txt -r
        | lines
        | str length
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq([8193, 3])
}

#[test]
fn lines_multi_value_split() -> Result {
    let code = "
        open sample-simple.json
        | get first second
        | lines
        | length
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(6)
}

/// test whether this handles CRLF and LF in the same input
#[test]
fn lines_mixed_line_endings() -> Result {
    test()
        .run(r#""foo\nbar\r\nquux" | lines | length"#)
        .expect_value_eq(3)
}

#[test]
fn lines_skip_empty_on_string() -> Result {
    test()
        .run(r#""foo\n\n\nbar\n\n\nqux" | lines --skip-empty"#)
        .expect_value_eq(["foo", "bar", "qux"])
}

#[test]
fn lines_keeps_empty_lines_without_skip_empty() -> Result {
    test()
        .run(r#""foo\n\nbar" | lines"#)
        .expect_value_eq(["foo", "", "bar"])
}

#[test]
fn lines_skip_empty_drops_whitespace_only_lines() -> Result {
    test()
        .run(r#""foo\n  \nbar" | lines --skip-empty"#)
        .expect_value_eq(["foo", "bar"])
}

#[test]
fn lines_skip_empty_on_list_stream() -> Result {
    test()
        .run(r#"["foo\n\n\nbar\n\n\nqux"] | each {} | lines --skip-empty"#)
        .expect_value_eq(["foo", "bar", "qux"])
}

#[test]
fn lines_skip_empty_on_byte_stream() -> Result {
    let input = PipelineData::ByteStream(
        ByteStream::read_string(
            "foo\n\n\nbar\n\n\nqux".into(),
            Span::test_data(),
            Signals::empty(),
        ),
        None,
    );

    test()
        .run_raw_with_data("lines --skip-empty", input)
        .and_then(|outcome| {
            outcome
                .body
                .into_value(Span::test_data())
                .map_err(Error::from)
        })
        .expect_value_eq(["foo", "bar", "qux"])
}

#[test]
fn lines_on_error() -> Result {
    let err = test().run("open . | lines").expect_shell_error()?;
    assert!(matches!(
        err,
        ShellError::Io(err) if matches!(err.kind, ErrorKind::Std(std::io::ErrorKind::IsADirectory, ..))
    ));
    Ok(())
}

#[test]
fn lines_handles_invalid_utf8() -> Result {
    let code = "
        open invalid_utf8.txt
        | lines
        | length
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_value_eq(3)
}

#[test]
fn lines_strict_fails_on_invalid_utf8() -> Result {
    let code = "
        open invalid_utf8.txt
        | lines --strict
    ";

    test()
        .cwd("tests/fixtures/formats")
        .run(code)
        .expect_shell_error()?;
    Ok(())
}
