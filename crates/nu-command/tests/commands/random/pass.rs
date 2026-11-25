use nu_test_support::prelude::*;
use rstest::rstest;

#[rstest]
#[case::default("", 12)]
#[case::chars("--chars 20", 20)]
#[case::fewer_chars_than_types("--chars 2", 2)]
#[case::one_char("--chars 1", 1)]
#[case::max("--chars 128", 128)]
fn password_has_requested_length(#[case] flags: &str, #[case] expected: i64) -> Result {
    test()
        .run(format!("random pass {flags} | str length"))
        .expect_value_eq(expected)
}

// Each case draws 2560 characters from a pool of at most 26, so every character in the pool
// shows up.
#[rstest]
#[case::lowercase("--no-uppercase --no-numbers --no-symbols", "abcdefghjkmnopqrstuvwxyz")]
#[case::lowercase_with_ambiguous(
    "--no-uppercase --no-numbers --no-symbols --include-ambiguous",
    "abcdefghjklmnopqrstuvwxyz"
)]
#[case::lowercase_with_similar(
    "--no-uppercase --no-numbers --no-symbols --include-similar",
    "abcdefghijklmnopqrstuvwxyz"
)]
#[case::uppercase(
    "--no-lowercase --no-numbers --no-symbols",
    "ABCDEFGHIJKLMNPQRSTUVWXYZ"
)]
#[case::uppercase_with_ambiguous(
    "--no-lowercase --no-numbers --no-symbols --include-ambiguous",
    "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
)]
#[case::numbers("--no-uppercase --no-lowercase --no-symbols", "23456789")]
#[case::numbers_with_ambiguous(
    "--no-uppercase --no-lowercase --no-symbols --include-ambiguous",
    "0123456789"
)]
#[case::numbers_with_similar(
    "--no-uppercase --no-lowercase --no-symbols --include-similar",
    "123456789"
)]
fn password_uses_only_enabled_characters(#[case] flags: &str, #[case] expected: &str) -> Result {
    let chars: Vec<String> = test().run(format!(
        "1..20 | each {{ random pass --chars 128 {flags} | split chars }} | flatten | uniq | sort"
    ))?;
    let expected: Vec<String> = expected.chars().map(String::from).collect();
    assert_eq!(chars, expected);
    Ok(())
}

#[test]
fn require_each_type_gets_one_of_each() -> Result {
    let passwords: Vec<String> =
        test().run("1..20 | each { random pass --chars 4 --require-each-type }")?;
    for password in passwords {
        assert!(
            password.chars().any(|c| c.is_ascii_uppercase())
                && password.chars().any(|c| c.is_ascii_lowercase())
                && password.chars().any(|c| c.is_ascii_digit())
                && password.chars().any(|c| c.is_ascii_punctuation()),
            "{password:?} lacks a character type"
        );
    }
    Ok(())
}

#[rstest]
#[case::zero("--chars 0", "Password length must be greater than 0")]
#[case::negative("--chars -1", "Password length must be greater than 0")]
#[case::too_long("--chars 129", "Password length must be less than or equal to 128")]
#[case::require_each_type_too_short(
    "--chars 3 --require-each-type",
    "Password length must be at least 4 to include each character type"
)]
#[case::require_each_type_without_symbols(
    "--require-each-type --no-symbols",
    "You've enabled `require_each_type`"
)]
#[case::no_types(
    "--no-uppercase --no-lowercase --no-numbers --no-symbols",
    "At least one character type must be included"
)]
fn invalid_options_are_errors(#[case] flags: &str, #[case] expected: &str) -> Result {
    let err = test()
        .run(format!("random pass {flags}"))
        .expect_shell_error()?;
    assert_contains(expected, err.generic_msg()?);
    Ok(())
}
