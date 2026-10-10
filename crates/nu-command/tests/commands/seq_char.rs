use nu_protocol::ShellError;
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;

fn assert_invalid_seq_char_input(code: &str) -> Result {
    let err = test().run(code).expect_shell_error()?;
    assert_matches!(
        err,
        ShellError::Generic(err) if err.error == "seq char only accepts individual ASCII characters as parameters"
            && err.msg == "input should be a single ASCII character"
    );
    Ok(())
}

#[test]
fn fails_when_first_arg_is_multiple_chars() -> Result {
    assert_invalid_seq_char_input("seq char aa z")
}

#[test]
fn fails_when_second_arg_is_multiple_chars() -> Result {
    assert_invalid_seq_char_input("seq char a zz")
}

#[test]
fn generates_sequence_from_a_to_e() -> Result {
    test()
        .run("seq char a e | str join ''")
        .expect_value_eq("abcde")
}

#[test]
fn generates_sequence_from_e_to_a() -> Result {
    test()
        .run("seq char e a | str join ''")
        .expect_value_eq("edcba")
}

#[test]
fn fails_when_non_ascii_character_is_used_in_first_arg() -> Result {
    assert_invalid_seq_char_input("seq char ñ z")
}

#[test]
fn fails_when_non_ascii_character_is_used_in_second_arg() -> Result {
    assert_invalid_seq_char_input("seq char a ñ")
}

#[test]
fn joins_sequence_with_pipe() -> Result {
    test()
        .run("seq char a e | str join '|'")
        .expect_value_eq("a|b|c|d|e")
}
