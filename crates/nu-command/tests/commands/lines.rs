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

#[cfg(not(windows))]
#[test]
fn lines_on_error() -> Result {
    let err = test().run("open . | lines").expect_shell_error()?;
    assert_contains("Is a directory", err.to_string());
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
