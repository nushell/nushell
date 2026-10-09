use nu_test_support::prelude::*;
use rstest::rstest;

// Every rejected base shares this message.
const INVALID_BASE_MSG: &str = "Base has to be a finite number greater than 0 and not equal to 1";

#[test]
fn const_log() -> Result {
    test()
        .run("const LOG = 16 | math log 2; $LOG")
        .expect_value_eq(4.0)
}

#[test]
fn can_log_range_into_list() -> Result {
    let expected: Value = test().run("[1 2 3 4 5] | math log 2")?;
    test().run("1..5 | math log 2").expect_value_eq(expected)
}

#[test]
fn cannot_log_infinite_range() -> Result {
    let outcome = test().run("1.. | math log 2").expect_shell_error()?;

    assert!(matches!(outcome, ShellError::IncorrectValue { .. }));
    Ok(())
}

#[rstest]
#[case::one("1")]
#[case::one_float("1.0")]
#[case::zero("0")]
#[case::negative("-2")]
#[case::nan("NaN")]
#[case::infinite("inf")]
#[case::negative_infinite("-inf")]
fn cannot_log_invalid_base(#[case] base: &str) -> Result {
    let outcome = test()
        .run(format!("2 | math log {base}"))
        .expect_shell_error()?;

    match outcome {
        ShellError::UnsupportedInput { msg, .. } => {
            assert_eq!(msg, INVALID_BASE_MSG);
            Ok(())
        }
        err => Err(err.into()),
    }
}

#[rstest]
#[case::base_two("16 | math log 2", 4.0)]
#[case::base_ten("100 | math log 10", 2.0)]
#[case::base_four("16 | math log 4", 2.0)]
#[case::base_in_open_unit_interval("8 | math log 0.5", -3.0)]
fn can_log_valid_base(#[case] pipeline: &str, #[case] expected: f64) -> Result {
    test().run(pipeline).expect_value_eq(expected)
}

#[test]
fn cannot_log_nan_value() -> Result {
    let outcome = test().run("NaN | math log 2").expect_shell_error()?;

    match outcome {
        ShellError::UnsupportedInput { msg, .. } => {
            assert_eq!(
                msg,
                "'math log' undefined for values outside the open interval (0, Inf)."
            );
            Ok(())
        }
        err => Err(err.into()),
    }
}
