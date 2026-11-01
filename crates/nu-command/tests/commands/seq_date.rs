use nu_protocol::{ParseError, ShellError, Type};
use nu_test_support::prelude::*;
use pretty_assertions::assert_matches;
use rstest::rstest;

#[test]
fn fails_on_datetime_input() -> Result {
    let err = test()
        .run("seq date --begin-date (date now)")
        .expect_parse_error()?;

    assert_matches!(err, ParseError::TypeMismatch(Type::String, Type::Date, _));
    Ok(())
}

#[test]
fn fails_when_increment_not_integer_or_duration() -> Result {
    let err = test()
        .run("seq date --begin-date 2020-01-01 --increment 1.1")
        .expect_parse_error()?;

    assert_matches!(
        err,
        ParseError::ExpectedWithStringMsg(expected, _)
            if expected == "one of a list of accepted shapes: [Duration, Int]"
    );
    Ok(())
}

#[rstest]
#[case::reversed_days("--days -9223372036854775808 --reverse", "int value too large")]
#[case::reversed_periods(
    "--periods -9223372036854775808 --reverse",
    "incrementing by the number of periods is too large"
)]
#[case::reversed_increment(
    "--increment (-9223372036854775807ns - 1ns) --reverse",
    "increment is too large"
)]
#[case::increment_with_periods(
    "--increment (-9223372036854775807ns - 1ns) --periods 2",
    "increment is too large"
)]
#[case::increment_towards_later_end_date(
    "--increment (-9223372036854775807ns - 1ns) --end-date 2020-01-02",
    "increment is too large"
)]
fn fails_instead_of_overflowing_on_i64_min(#[case] flags: &str, #[case] message: &str) -> Result {
    let code = format!("seq date --begin-date 2020-01-01 {flags}");
    let err = test().run(code.as_str()).expect_shell_error()?;

    assert_matches!(err, ShellError::Generic(err) if err.error == message);
    Ok(())
}

#[test]
fn reverse_ignores_days_when_periods_is_set() -> Result {
    let code = "seq date --begin-date 2020-01-01 --days -9223372036854775808 --periods 3 --reverse";
    let expected = "seq date --begin-date 2020-01-01 --periods 3 --reverse";

    let dates: Vec<String> = test().run(code)?;
    let expected: Vec<String> = test().run(expected)?;

    assert_eq!(dates, expected);
    Ok(())
}
