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
