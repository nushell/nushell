use nu_test_support::prelude::*;

#[test]
fn string_fill_plain() -> Result {
    test()
        .run(r#""abc" | fill --alignment center --character "+" --width 5"#)
        .expect_value_eq("+abc+")
}

#[test]
fn string_fill_fancy() -> Result {
    let code = r#"
        $"(ansi red)a(ansi green)\u{65}\u{308}(ansi cyan)c(ansi reset)"
        | fill --alignment center --character "+" --width 5
    "#;

    test()
        .run(code)
        .expect_value_eq("+\u{1b}[31ma\u{1b}[32me\u{308}\u{1b}[36mc\u{1b}[0m+")
}

#[test]
fn string_fill_wide_characters() -> Result {
    // A wide character occupies two terminal columns, so `fill` must add two
    // fewer pad characters for it than it would for an ASCII character.
    test()
        .run(r#""\u{65e5}\u{672c}" | fill --character "." --width 8"#)
        .expect_value_eq("\u{65e5}\u{672c}....")?;
    test()
        .run(r#""\u{1f44d}" | fill --character "." --width 8"#)
        .expect_value_eq("\u{1f44d}......")?;
    // A ZWJ emoji sequence is one grapheme cluster but still two columns wide.
    test()
        .run(r#""\u{1f9d1}\u{200d}\u{1f3eb}" | fill --alignment center --character "+" --width 4"#)
        .expect_value_eq("+\u{1f9d1}\u{200d}\u{1f3eb}+")?;
    // A combining mark adds no column of its own.
    test()
        .run(r#""e\u{308}" | fill --character "." --width 3"#)
        .expect_value_eq("e\u{308}..")
}

#[test]
fn string_fill_control_characters() -> Result {
    // A tab and a carriage return each keep the single column they had before
    // `fill` measured in columns; they are not escape sequences to be ignored.
    test()
        .run(r#""a\tb" | fill --character "." --width 6"#)
        .expect_value_eq("a\tb...")?;
    test()
        .run(r#""a\rb" | fill --character "." --width 6"#)
        .expect_value_eq("a\rb...")?;
    // A multi-line string is as wide as its widest line.
    test()
        .run(r#""ab\ncd" | fill --character "." --width 6"#)
        .expect_value_eq("ab\ncd....")
}

#[test]
fn fill_valid_alignments() -> Result {
    test()
        .run("'nushell' | fill -w 10 -a l")
        .expect_value_eq("nushell   ")?;
    test()
        .run("'nushell' | fill -w 10 -a LEFT")
        .expect_value_eq("nushell   ")?;
    test()
        .run("'nushell' | fill -w 10 -a r")
        .expect_value_eq("   nushell")?;
    test()
        .run("'nushell' | fill -w 10 -a right")
        .expect_value_eq("   nushell")?;
    test()
        .run("'nushell' | fill -w 10 -a c")
        .expect_value_eq(" nushell  ")?;
    test()
        .run("'nushell' | fill -w 10 -a m")
        .expect_value_eq(" nushell  ")?;
    test()
        .run("'nushell' | fill -w 10 -a middle")
        .expect_value_eq(" nushell  ")?;
    test()
        .run("'nushell' | fill -w 10 -a cr")
        .expect_value_eq("  nushell ")?;
    test()
        .run("'nushell' | fill -w 10 -a mr")
        .expect_value_eq("  nushell ")?;
    test()
        .run("'nushell' | fill -w 10 -a centerright")
        .expect_value_eq("  nushell ")?;
    test()
        .run("'nushell' | fill -w 10 -a MiddleRight")
        .expect_value_eq("  nushell ")
}

#[test]
fn fill_invalid_alignment_errors() -> Result {
    let err = test()
        .run("'nushell' | fill -w 10 -a invalid")
        .expect_shell_error()?;
    match err {
        ShellError::InvalidValue { valid, actual, .. } => {
            assert_eq!(
                valid,
                "one of: left (l), right (r), center/middle (c/m), centerright/middleright (cr/mr)"
            );
            assert_eq!(actual, "'invalid'");
        }
        err => return Err(err.into()),
    }

    let err = test()
        .run("'nushell' | fill -w 10 -a ''")
        .expect_shell_error()?;
    match err {
        ShellError::InvalidValue { valid, actual, .. } => {
            assert_eq!(
                valid,
                "one of: left (l), right (r), center/middle (c/m), centerright/middleright (cr/mr)"
            );
            assert_eq!(actual, "''");
        }
        err => return Err(err.into()),
    }

    Ok(())
}
