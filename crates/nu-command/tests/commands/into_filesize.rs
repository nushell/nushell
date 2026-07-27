use nu_protocol::Filesize;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

#[test]
fn int() -> Result {
    test()
        .run("1 | into filesize")
        .expect_value_eq(Filesize::from(1))
}

#[test]
fn float() -> Result {
    test()
        .run("1.2 | into filesize")
        .expect_value_eq(Filesize::from(1))
}

#[test]
fn str() -> Result {
    test()
        .run("'2000' | into filesize")
        .expect_value_eq(Filesize::from(2000))
}

#[test]
fn str_newline() -> Result {
    test()
        .run_with_data("into filesize", "2000\n ")
        .expect_value_eq(Filesize::from(2000))
}

#[test]
fn str_many_newlines() -> Result {
    test()
        .run_with_data("into filesize", "2000\n \n ")
        .expect_value_eq(Filesize::from(2000))
}

#[test]
fn filesize() -> Result {
    test()
        .run("3kB | into filesize")
        .expect_value_eq(Filesize::from(3000))
}

#[test]
fn negative_filesize() -> Result {
    test()
        .run("-3kB | into filesize")
        .expect_value_eq(Filesize::from(-3000))
}

#[test]
fn negative_str_filesize() -> Result {
    test()
        .run("'-3kB' | into filesize")
        .expect_value_eq(Filesize::from(-3000))
}

#[test]
fn wrong_negative_str_filesize() -> Result {
    let err = test().run("'--3kB' | into filesize").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn large_negative_str_filesize() -> Result {
    let err = test()
        .run("'-10000PB' | into filesize")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn negative_str() -> Result {
    test()
        .run("'-1' | into filesize")
        .expect_value_eq(Filesize::from(-1))
}

#[test]
fn wrong_negative_str() -> Result {
    let err = test().run("'--1' | into filesize").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn positive_str_filesize() -> Result {
    test()
        .run("'+1kB' | into filesize")
        .expect_value_eq(Filesize::from(1000))
}

#[test]
fn wrong_positive_str_filesize() -> Result {
    let err = test().run("'++1kB' | into filesize").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn large_positive_str_filesize() -> Result {
    let err = test()
        .run("'+10000PB' | into filesize")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn positive_str() -> Result {
    test()
        .run("'+1' | into filesize")
        .expect_value_eq(Filesize::from(1))
}

#[test]
fn wrong_positive_str() -> Result {
    let err = test().run("'++1' | into filesize").expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}

#[test]
fn invalid_str() -> Result {
    let err = test()
        .run("'42.0 42.0 kB' | into filesize")
        .expect_shell_error()?;

    assert_matches!(
        err,
        ShellError::CantConvert { to_type, from_type, .. }
            if to_type == "filesize" && from_type == "string"
    );
    Ok(())
}
