use std::collections::HashMap;

use nu_test_support::prelude::*;

#[derive(Debug, IntoValue, Clone)]
struct Sample {
    first_name: &'static str,
    last_name: &'static str,
    rusty_at: &'static str,
}

#[rustfmt::skip]
static SAMPLE: [Sample; 3] = [
    Sample { first_name: "Andrés", last_name: "Robalino", rusty_at: "Ecuador" },
    Sample { first_name: "JT",     last_name: "Turner",   rusty_at: "Estados Unidos" },
    Sample { first_name: "Yehuda", last_name: "Katz",     rusty_at: "Estados Unidos" },
];

#[test]
fn summarizes_by_column_given() -> Result {
    let code = r#"
        $in
        | histogram rusty_at countries --percentage-type relative
        | where rusty_at == "Ecuador"
        | get countries
        | get 0
    "#;

    test()
        .run_with_data(code, SAMPLE.clone())
        .expect_value_eq("**************************************************")

    // 50%
}

#[test]
fn summarizes_by_column_given_with_normalize_percentage() -> Result {
    let code = r#"
        $in
        | histogram rusty_at countries
        | where rusty_at == "Ecuador"
        | get countries
        | get 0
    "#;

    test()
        .run_with_data(code, SAMPLE.clone())
        .expect_value_eq("*********************************")

    // 33%
}

#[test]
fn summarizes_by_values() -> Result {
    let code = r#"
        $in
        | get rusty_at
        | histogram
        | where value == "Estados Unidos"
        | get count
        | get 0
    "#;

    test()
        .run_with_data(code, SAMPLE.clone())
        .expect_value_eq(2)
}

#[test]
fn help() -> Result {
    let mut tester = test();
    let help_command: String = tester.run("help histogram")?;
    let help_short: String = tester.run("histogram -h")?;
    let help_long: String = tester.run("histogram --help")?;

    assert_eq!(help_short, help_command);
    assert_eq!(help_long, help_command);
    Ok(())
}

#[test]
fn count() -> Result {
    let code = "
        echo [[bit];  [1] [0] [0] [0] [0] [0] [0] [1] [1]]
        | histogram bit --percentage-type relative
        | sort-by count
        | reject frequency
    ";

    let outcome: Vec<HashMap<String, Value>> = test().run(code)?;
    assert_eq!(outcome.len(), 2);

    assert_eq!(outcome[0].len(), 4);
    outcome[0]["bit"].assert_eq(1);
    outcome[0]["count"].assert_eq(3);
    outcome[0]["quantile"].assert_eq(0.5);
    outcome[0]["percentage"].assert_eq("50.00%");

    assert_eq!(outcome[1].len(), 4);
    outcome[1]["bit"].assert_eq(0);
    outcome[1]["count"].assert_eq(6);
    outcome[1]["quantile"].assert_eq(1.0);
    outcome[1]["percentage"].assert_eq("100.00%");

    Ok(())
}

#[test]
fn count_with_normalize_percentage() -> Result {
    let code = "
        echo [[bit];  [1] [0] [0] [0] [0] [0] [0] [1]]
        | histogram bit --percentage-type normalize
        | sort-by count
        | reject frequency
    ";

    let outcome: Vec<HashMap<String, Value>> = test().run(code)?;
    assert_eq!(outcome.len(), 2);

    assert_eq!(outcome[0].len(), 4);
    outcome[0]["bit"].assert_eq(1);
    outcome[0]["count"].assert_eq(2);
    outcome[0]["quantile"].assert_eq(0.25);
    outcome[0]["percentage"].assert_eq("25.00%");

    assert_eq!(outcome[1].len(), 4);
    outcome[1]["bit"].assert_eq(0);
    outcome[1]["count"].assert_eq(6);
    outcome[1]["quantile"].assert_eq(0.75);
    outcome[1]["percentage"].assert_eq("75.00%");

    Ok(())
}

#[test]
fn column_name_conflicting_with_reserved_maps_to_value() -> Result {
    let code = "
        1..100
        | each { {count: ($in mod 11) } }
        | histogram count
        | columns
    ";

    let outcome: Vec<String> = test().run(code)?;
    let outcome: Vec<&str> = outcome.iter().map(|s| s.as_str()).collect();
    let outcome = outcome.as_slice();
    assert_eq!(outcome.len(), 5);
    assert_contains("count", outcome);
    assert_contains("frequency", outcome);
    assert_contains("percentage", outcome);
    assert_contains("quantile", outcome);
    // "count" is a reserved column name, so the value column should fall back to "value"
    assert_contains("value", outcome);
    Ok(())
}
