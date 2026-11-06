use nu_test_support::prelude::*;

#[test]
fn format_duration() -> Result {
    let code = "1hr | format duration sec";
    test().run(code).expect_value_eq("3600 sec")
}

#[test]
fn format_duration_with_invalid_unit() -> Result {
    let code = "1hr | format duration MB";
    let err = test().run(code).expect_error()?;
    assert!(matches!(err, ShellError::InvalidUnit { .. }));
    Ok(())
}

#[test]
fn format_duration_iso8601() -> Result {
    for (input, expected) in [
        ("0ns", "PT0S"),
        ("1min", "PT1M"),
        ("1hr + 59sec", "PT1H59S"),
        ("-3day", "-PT72H"),
        ("1ns", "PT0.000000001S"),
        ("-1ns", "-PT0.000000001S"),
        (
            "9223372036854775807 | into duration",
            "PT2562047H47M16.854775807S",
        ),
        (
            "-9223372036854775808 | into duration",
            "-PT2562047H47M16.854775808S",
        ),
    ] {
        test()
            .run(format!("({input}) | format duration iso8601"))
            .expect_value_eq(expected)?;
    }
    Ok(())
}

#[test]
fn format_duration_iso8601_list() -> Result {
    test()
        .run("[1sec 2sec] | format duration iso8601 | str join ','")
        .expect_value_eq("PT1S,PT2S")
}

#[test]
fn format_duration_iso8601_cell_path() -> Result {
    test()
        .run("[{time: 1sec} {time: 2sec}] | format duration iso8601 time | get time | str join ','")
        .expect_value_eq("PT1S,PT2S")
}

#[test]
fn format_duration_iso8601_const() -> Result {
    test()
        .run("const result = (1sec | format duration iso8601); $result")
        .expect_value_eq("PT1S")
}

#[test]
fn format_duration_iso8601_ignores_float_precision() -> Result {
    test()
        .run("$env.config.float_precision = 0; 1ns | format duration iso8601")
        .expect_value_eq("PT0.000000001S")
}

#[test]
fn format_duration_iso8601_case_insensitive() -> Result {
    test()
        .run("1sec | format duration ISO8601")
        .expect_value_eq("PT1S")
}
