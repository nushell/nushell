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
#[case::reversed_days(
    "--days -9223372036854775808 --reverse",
    "The number of days is too large or too small"
)]
#[case::reversed_periods(
    "--periods -9223372036854775808 --reverse",
    "The period increment is too large or too small"
)]
#[case::reversed_increment(
    "--increment (-9223372036854775807ns - 1ns) --reverse",
    "The period increment is too large or too small"
)]
#[case::increment_with_periods(
    "--increment (-9223372036854775807ns - 1ns) --periods 2",
    "The period increment is too large or too small"
)]
#[case::increment_towards_later_end_date(
    "--increment (-9223372036854775807ns - 1ns) --end-date 2020-01-02",
    "The period increment is too large or too small"
)]
fn fails_instead_of_overflowing_on_i64_min(#[case] flags: &str, #[case] message: &str) -> Result {
    let code = format!("seq date --begin-date 2020-01-01 {flags}");
    let err = test().run(code.as_str()).expect_shell_error()?;

    assert_matches!(err, ShellError::Generic(err) if err.msg == message);
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

#[test]
fn days_ignored_with_days_zero() -> Result {
    let code = "seq date --days 0 --periods 3";
    let expected = "seq date --periods 3";

    let dates: Vec<String> = test().run(code)?;
    let expected: Vec<String> = test().run(expected)?;

    assert_eq!(dates, expected);
    Ok(())
}

#[rstest]
#[case("+ 10day")]
#[case("- 10day")]
fn correct_result_with_end_date(#[case] input: &str) -> Result {
    let last = format!("(date now) {} | format date '%Y-%m-%d'", input);
    let last: String = test().run(last)?;

    let code = format!("seq date --end-date {} | length", last);
    test().run(code).expect_value_eq(11)?;

    let code = format!("seq date --end-date {} | last", last);
    let date: String = test().run(code)?;
    assert_eq!(date, last);

    Ok(())
}

#[rstest]
#[case("+ 10day")]
#[case("- 10day")]
fn correct_result_with_begin_date(#[case] input: &str) -> Result {
    let now = "date now | format date '%Y-%m-%d'";
    let now: String = test().run(now)?;

    let first = format!("(date now) {} | format date '%Y-%m-%d'", input);
    let first: String = test().run(first)?;

    let code = format!("seq date --begin-date {} | length", first);
    test().run(code).expect_value_eq(11)?;

    let code = format!("seq date --begin-date {} | last", first);
    let date: String = test().run(code)?;
    assert_eq!(date, now);

    Ok(())
}

#[rstest]
#[case("--days 10", "2020-01-10")]
#[case("--days -10", "2019-12-23")]
#[case("--days 10 --reverse", "2019-12-23")]
#[case("--days -10 --reverse", "2020-01-10")]
#[case("--periods 10", "2020-01-10")]
#[case("--periods -10", "2019-12-23")]
#[case("--periods 10 --reverse", "2019-12-23")]
#[case("--periods -10 --reverse", "2020-01-10")]
fn correct_result_with_reverse_and_days(#[case] input: &str, #[case] expected: &str) -> Result {
    let code = format!("seq date --begin-date 2020-01-01 {} | length", input);
    test().run(code).expect_value_eq(10)?;

    let code = format!("seq date --begin-date 2020-01-01 {} | last", input);
    test().run(code).expect_value_eq(expected)
}

#[rstest]
#[case("")]
#[case("--increment 1")]
#[case("--increment 2")]
#[case("--increment -1")]
#[case("--increment 24hr")]
#[case("--increment 48hr")]
#[case("--increment -24hr")]
#[case("--increment 172800sec")]
fn midnight_time_with_whole_day_increments(#[case] input: &str) -> Result {
    let code = format!("seq date --output-format '%H:%M' {} | first", input);
    test().run(code).expect_value_eq("00:00")
}

#[rstest]
#[case("--increment 23hr")]
#[case("--increment 25hr")]
#[case("--increment 1ns")]
#[case("--increment -1ns")]
#[case("--increment 86400000000001ns")]
#[case("--increment 86399999999999ns")]
fn current_time_with_non_whole_day_increments(#[case] input: &str) -> Result {
    let code = format!("seq date --output-format '%H:%M' {} | first", input);
    let expected = "date now | format date '%H:%M'";

    let times: String = test().run(code)?;
    let expected: String = test().run(expected)?;

    assert_eq!(times, expected);
    Ok(())
}

#[rstest]
#[case("--days 0")]
#[case("--periods 0")]
fn zero_results_with_days_zero(#[case] input: &str) -> Result {
    let code = format!("seq date {} | length", input);
    test().run(code).expect_value_eq(0)
}
