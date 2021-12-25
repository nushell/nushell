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
fn to_json_raw_flag() -> TestResult {
    run_test(
        "[[a b]; [jim susie] [3 4]] | to json -r",
        r#"[{"a":"jim","b":"susie"},{"a":3,"b":4}]"#,
    )
}
