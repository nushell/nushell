use crate::tests::{run_test, TestResult};

#[test]
fn from_json_1() -> TestResult {
    run_test(r#"('{"name": "Fred"}' | from json).name"#, "Fred")
}

#[test]
fn from_json_2() -> TestResult {
    run_test(
        r#"('{"name": "Fred"}
                   {"name": "Sally"}' | from json -o).name.1"#,
        "Sally",
    )
}

#[test]
fn to_json_raw_flag_1() -> TestResult {
    run_test(
        "[[a b]; [jim susie] [3 4]] | to json -r",
        r#"[{"a": "jim","b": "susie"},{"a": 3,"b": 4}]"#,
    )
}

#[test]
fn to_json_raw_flag_2() -> TestResult {
    run_test(
        "[[\"a b\" c]; [jim susie] [3 4]] | to json -r",
        r#"[{"a b": "jim","c": "susie"},{"a b": 3,"c": 4}]"#,
    )
}

#[test]
fn to_json_raw_flag_3() -> TestResult {
    run_test(
        "[[\"a b\" \"c d\"]; [\"jim smith\" \"susie roberts\"] [3 4]] | to json -r",
        r#"[{"a b": "jim smith","c d": "susie roberts"},{"a b": 3,"c d": 4}]"#,
    )
}

#[test]
fn to_json_escaped() -> TestResult {
    run_test(
        r#"{foo: {bar: '[{"a":"b","c": 2}]'}} | to json --raw"#,
        r#"{"foo":{"bar": "[{\"a\":\"b\",\"c\": 2}]"}}"#,
    )
}
